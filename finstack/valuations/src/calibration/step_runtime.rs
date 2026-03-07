use crate::calibration::api::schema::{CalibrationStep, StepParams};
use crate::calibration::config::CalibrationConfig;
use crate::calibration::hull_white::{
    calibrate_hull_white_to_swaptions_with_frequency_and_initial_guess, HullWhiteParams,
    SwapFrequency, SwaptionQuote,
};
use crate::calibration::targets::base_correlation::BaseCorrelationBootstrapper;
use crate::calibration::targets::discount::DiscountCurveTarget;
use crate::calibration::targets::forward::ForwardCurveTarget;
use crate::calibration::targets::hazard::HazardBootstrapper;
use crate::calibration::targets::inflation::InflationBootstrapper;
use crate::calibration::targets::student_t::StudentTCalibrator;
use crate::calibration::targets::swaption::SwaptionVolBootstrapper;
use crate::calibration::targets::vol::VolSurfaceBootstrapper;
use crate::calibration::validation::ValidationMode;
use crate::calibration::{CalibrationReport, CurveValidator, SurfaceValidator};
use crate::market::quotes::market_quote::MarketQuote;
use crate::market::quotes::vol::VolQuote;
use finstack_core::dates::{DayCount, DayCountCtx};
use finstack_core::explain::TraceEntry;
use finstack_core::market_data::context::{CurveStorage, MarketContext};
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::surfaces::{VolSurface, VolSurfaceAxis};
use finstack_core::market_data::term_structures::CreditIndexData;
use finstack_core::types::CurveId;
use finstack_core::Result;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq)]
struct OrderedF64(f64);

impl Eq for OrderedF64 {}

impl PartialOrd for OrderedF64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl From<f64> for OrderedF64 {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl OrderedF64 {
    fn into_inner(self) -> f64 {
        self.0
    }
}

/// Normalized output key for a step.
pub(crate) enum OutputKey {
    Curve(CurveId),
    Surface(CurveId),
    Scalar(String),
}

/// Normalized output payload for a step.
pub(crate) enum StepOutput {
    Curve(CurveStorage),
    Surface(Arc<VolSurface>),
    Scalar { key: String, value: MarketScalar },
    Scalars(Vec<(String, MarketScalar)>),
}

fn interpolate_svi_params(
    target_expiry: f64,
    params_by_expiry: &BTreeMap<OrderedF64, finstack_core::math::volatility::svi::SviParams>,
) -> Result<finstack_core::math::volatility::svi::SviParams> {
    let Some((&first_key, &first_params)) = params_by_expiry.iter().next() else {
        return Err(finstack_core::Error::Input(
            finstack_core::InputError::TooFewPoints,
        ));
    };

    if params_by_expiry.len() == 1 || target_expiry <= first_key.into_inner() {
        return Ok(first_params);
    }

    let Some((&last_key, &last_params)) = params_by_expiry.iter().next_back() else {
        return Ok(first_params);
    };
    if target_expiry >= last_key.into_inner() {
        return Ok(last_params);
    }

    let mut lower = (first_key.into_inner(), first_params);
    let mut upper = (last_key.into_inner(), last_params);

    for (&expiry_key, &params) in params_by_expiry {
        let expiry = expiry_key.into_inner();
        if expiry <= target_expiry {
            lower = (expiry, params);
        }
        if expiry >= target_expiry {
            upper = (expiry, params);
            break;
        }
    }

    if (upper.0 - lower.0).abs() < f64::EPSILON {
        return Ok(lower.1);
    }

    let w = (target_expiry - lower.0) / (upper.0 - lower.0);
    Ok(finstack_core::math::volatility::svi::SviParams {
        a: lower.1.a + w * (upper.1.a - lower.1.a),
        b: lower.1.b + w * (upper.1.b - lower.1.b),
        rho: lower.1.rho + w * (upper.1.rho - lower.1.rho),
        m: lower.1.m + w * (upper.1.m - lower.1.m),
        sigma: lower.1.sigma + w * (upper.1.sigma - lower.1.sigma),
    })
}

/// Aggregated outcome of a single calibration step.
pub(crate) struct StepOutcome {
    pub output: StepOutput,
    pub credit_index_update: Option<(String, CreditIndexData)>,
    pub report: CalibrationReport,
}

fn attach_validation_result(
    report: CalibrationReport,
    validation: Result<()>,
    global_config: &CalibrationConfig,
) -> CalibrationReport {
    match validation {
        Ok(()) => report.with_validation_result(true, None),
        Err(err) => match global_config.validation_mode {
            ValidationMode::Error => report.with_validation_result(false, Some(err.to_string())),
            ValidationMode::Warn => {
                let mut report = report;
                report.update_metadata("validation_warning", err.to_string());
                report
            }
        },
    }
}

/// Compute the output key for batching without executing the step.
pub(crate) fn output_key(step: &CalibrationStep) -> OutputKey {
    match &step.params {
        StepParams::Discount(p) => OutputKey::Curve(p.curve_id.clone()),
        StepParams::Forward(p) => OutputKey::Curve(p.curve_id.clone()),
        StepParams::Hazard(p) => OutputKey::Curve(p.curve_id.clone()),
        StepParams::Inflation(p) => OutputKey::Curve(p.curve_id.clone()),
        StepParams::BaseCorrelation(p) => {
            OutputKey::Curve(CurveId::from(format!("{}_CORR", p.index_id)))
        }
        StepParams::VolSurface(p) => OutputKey::Surface(CurveId::from(p.surface_id.as_str())),
        StepParams::SwaptionVol(p) => OutputKey::Surface(CurveId::from(p.surface_id.as_str())),
        StepParams::StudentT(p) => {
            OutputKey::Scalar(format!("{}_STUDENT_T_DF", p.tranche_instrument_id))
        }
        StepParams::HullWhite(p) => OutputKey::Scalar(format!("{}_HW1F", p.curve_id.as_str())),
        StepParams::SviSurface(p) => OutputKey::Surface(CurveId::from(p.surface_id.as_str())),
    }
}

/// Apply a normalized step output into the mutable market context.
pub(crate) fn apply_output(
    context: &mut MarketContext,
    output: StepOutput,
    credit_index_update: Option<(String, CreditIndexData)>,
) {
    match output {
        StepOutput::Curve(curve) => {
            *context = std::mem::take(context).insert(curve);
        }
        StepOutput::Surface(surface) => {
            *context = std::mem::take(context).insert_surface(surface);
        }
        StepOutput::Scalar { key, value } => {
            *context = std::mem::take(context).insert_price(&key, value);
        }
        StepOutput::Scalars(values) => {
            let mut updated = std::mem::take(context);
            for (key, value) in values {
                updated = updated.insert_price(&key, value);
            }
            *context = updated;
        }
    }

    if let Some((id, data)) = credit_index_update {
        *context = std::mem::take(context).insert_credit_index(id, data);
    }
}

/// Execute calibration logic for the provided [`StepParams`].
pub(crate) fn execute_params(
    params: &StepParams,
    quotes: &[MarketQuote],
    context: &MarketContext,
    global_config: &CalibrationConfig,
) -> Result<StepOutcome> {
    match params {
        StepParams::Discount(p) => {
            let (ctx, report) = DiscountCurveTarget::solve(p, quotes, context, global_config)?;
            let curve = ctx.get_discount(&p.curve_id)?;
            let output = StepOutput::Curve(curve.clone().into());
            let report = attach_validation_result(
                report,
                curve.validate(&global_config.validation),
                global_config,
            );
            Ok(StepOutcome {
                output,
                credit_index_update: None,
                report,
            })
        }
        StepParams::Forward(p) => {
            let (ctx, report) = ForwardCurveTarget::solve(p, quotes, context, global_config)?;
            let curve = ctx.get_forward(&p.curve_id)?;
            let output = StepOutput::Curve(curve.clone().into());
            let report = attach_validation_result(
                report,
                curve.validate(&global_config.validation),
                global_config,
            );
            Ok(StepOutcome {
                output,
                credit_index_update: None,
                report,
            })
        }
        StepParams::Hazard(p) => {
            let (ctx, report) = HazardBootstrapper::solve(p, quotes, context, global_config)?;
            let curve = ctx.get_hazard(&p.curve_id)?;
            let output = StepOutput::Curve(curve.clone().into());
            let report = attach_validation_result(
                report,
                curve.validate(&global_config.validation),
                global_config,
            );
            Ok(StepOutcome {
                output,
                credit_index_update: None,
                report,
            })
        }
        StepParams::Inflation(p) => {
            let (ctx, report) = InflationBootstrapper::solve(p, quotes, context, global_config)?;
            let curve = ctx.get_inflation(&p.curve_id)?;
            let output = StepOutput::Curve(curve.clone().into());
            let report = attach_validation_result(
                report,
                curve.validate(&global_config.validation),
                global_config,
            );
            Ok(StepOutcome {
                output,
                credit_index_update: None,
                report,
            })
        }
        StepParams::BaseCorrelation(p) => {
            let (ctx, report) =
                BaseCorrelationBootstrapper::solve(p, quotes, context, global_config)?;
            let curve_id = CurveId::from(format!("{}_CORR", p.index_id));
            let curve = ctx.get_base_correlation(curve_id.as_str())?;
            let output = StepOutput::Curve(curve.clone().into());
            let report = attach_validation_result(
                report,
                curve.validate(&global_config.validation),
                global_config,
            );
            let credit_index_update = ctx
                .credit_index(&p.index_id)
                .ok()
                .map(|idx| (p.index_id.clone(), idx.as_ref().clone()));
            Ok(StepOutcome {
                output,
                credit_index_update,
                report,
            })
        }
        StepParams::VolSurface(p) => {
            let (surface, report) =
                VolSurfaceBootstrapper::solve(p, quotes, context, global_config)?;
            // Preserve context insertion behavior
            let mut new_report = report.clone();
            new_report
                .explanation
                .get_or_insert_with(|| finstack_core::explain::ExplanationTrace::new("vol_surface"))
                .push(
                    TraceEntry::ComputationStep {
                        name: "surface_built".to_string(),
                        description: "Vol surface constructed".to_string(),
                        metadata: None,
                    },
                    global_config.explain.max_entries,
                );
            let new_report = attach_validation_result(
                new_report,
                surface.validate(&global_config.validation),
                global_config,
            );
            Ok(StepOutcome {
                output: StepOutput::Surface(surface.into()),
                credit_index_update: None,
                report: new_report,
            })
        }
        StepParams::SwaptionVol(p) => {
            let (surface, report) =
                SwaptionVolBootstrapper::solve(p, quotes, context, global_config)?;
            let report = if surface.secondary_axis() == VolSurfaceAxis::Strike {
                attach_validation_result(
                    report,
                    surface.validate(&global_config.validation),
                    global_config,
                )
            } else {
                report
            };
            Ok(StepOutcome {
                output: StepOutput::Surface(surface.into()),
                credit_index_update: None,
                report,
            })
        }
        StepParams::StudentT(p) => {
            let (_, calibrated_df, report) =
                StudentTCalibrator::solve(p, quotes, context, global_config)?;
            let scalar_key = format!("{}_STUDENT_T_DF", p.tranche_instrument_id);
            Ok(StepOutcome {
                output: StepOutput::Scalar {
                    key: scalar_key,
                    value: MarketScalar::Unitless(calibrated_df),
                },
                credit_index_update: None,
                report,
            })
        }
        StepParams::HullWhite(p) => {
            let disc_curve = context.get_discount(&p.curve_id)?;
            let df = |t: f64| disc_curve.df(t);
            let dc = DayCount::Act365F;

            // Extract swaption quotes from MarketQuote::Vol(VolQuote::SwaptionVol { .. })
            let hw_quotes: Vec<SwaptionQuote> = quotes
                .iter()
                .filter_map(|q| {
                    if let MarketQuote::Vol(VolQuote::SwaptionVol {
                        expiry,
                        maturity,
                        vol,
                        quote_type,
                        ..
                    }) = q
                    {
                        let t_exp = dc
                            .year_fraction(p.base_date, *expiry, DayCountCtx::default())
                            .ok()?;
                        let t_ten = dc
                            .year_fraction(*expiry, *maturity, DayCountCtx::default())
                            .ok()?;
                        if t_exp <= 0.0 || t_ten <= 0.0 {
                            return None;
                        }
                        let is_normal = quote_type.eq_ignore_ascii_case("normal");
                        Some(SwaptionQuote {
                            expiry: t_exp,
                            tenor: t_ten,
                            volatility: *vol,
                            is_normal_vol: is_normal,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            let initial_guess = match (p.initial_kappa, p.initial_sigma) {
                (Some(kappa), Some(sigma)) => Some(HullWhiteParams::new(kappa, sigma)?),
                (None, None) => None,
                _ => {
                    return Err(finstack_core::Error::Validation(
                        "Hull-White calibration requires both `initial_kappa` and `initial_sigma` when overriding defaults"
                            .to_string(),
                    ))
                }
            };
            let frequency = match p.currency {
                finstack_core::currency::Currency::EUR | finstack_core::currency::Currency::GBP => {
                    SwapFrequency::Annual
                }
                _ => SwapFrequency::SemiAnnual,
            };
            let (hw_params, report) =
                calibrate_hull_white_to_swaptions_with_frequency_and_initial_guess(
                    &df,
                    &hw_quotes,
                    frequency,
                    initial_guess,
                )?;

            Ok(StepOutcome {
                output: StepOutput::Scalars(vec![
                    (
                        format!("{}_HW1F_KAPPA", p.curve_id.as_str()),
                        MarketScalar::Unitless(hw_params.kappa),
                    ),
                    (
                        format!("{}_HW1F_SIGMA", p.curve_id.as_str()),
                        MarketScalar::Unitless(hw_params.sigma),
                    ),
                ]),
                credit_index_update: None,
                report,
            })
        }
        StepParams::SviSurface(p) => {
            if p.target_expiries.is_empty() || p.target_strikes.is_empty() {
                return Err(finstack_core::Error::Input(
                    finstack_core::InputError::TooFewPoints,
                ));
            }

            let option_quotes: Vec<&VolQuote> = quotes
                .iter()
                .filter_map(|quote| match quote {
                    MarketQuote::Vol(vol_quote) => Some(vol_quote),
                    _ => None,
                })
                .filter(|quote| match quote {
                    VolQuote::OptionVol { underlying, .. } => {
                        underlying.as_str() == p.underlying_ticker.as_str()
                    }
                    VolQuote::SwaptionVol { .. } => false,
                })
                .collect();

            if option_quotes.is_empty() {
                return Err(finstack_core::Error::Input(
                    finstack_core::InputError::TooFewPoints,
                ));
            }

            let spot = if let Some(spot) = p.spot_override {
                spot
            } else {
                match context.price(&p.underlying_ticker)? {
                    MarketScalar::Price(money) => money.amount(),
                    MarketScalar::Unitless(value) => *value,
                }
            };

            let discount = p
                .discount_curve_id
                .as_ref()
                .map(|curve_id| context.get_discount(curve_id))
                .transpose()?;

            let forward_fn = |t: f64| -> f64 {
                if let Some(curve) = discount.as_ref() {
                    let r = curve.zero(t);
                    spot * (r * t).exp()
                } else {
                    spot
                }
            };

            let mut quotes_by_expiry: BTreeMap<OrderedF64, Vec<(f64, f64)>> = BTreeMap::new();
            for quote in option_quotes {
                if let VolQuote::OptionVol {
                    expiry,
                    strike,
                    vol,
                    ..
                } = quote
                {
                    let t = DayCount::Act365F.year_fraction(
                        p.base_date,
                        *expiry,
                        DayCountCtx::default(),
                    )?;
                    if t > 0.0 {
                        quotes_by_expiry
                            .entry(t.into())
                            .or_default()
                            .push((*strike, *vol));
                    }
                }
            }

            if quotes_by_expiry.is_empty() {
                return Err(finstack_core::Error::Input(
                    finstack_core::InputError::TooFewPoints,
                ));
            }

            let mut params_by_expiry = BTreeMap::new();
            let mut residuals = BTreeMap::new();

            for (&expiry_key, expiry_quotes) in &quotes_by_expiry {
                if expiry_quotes.len() < 5 {
                    return Err(finstack_core::Error::Validation(format!(
                        "SVI surface calibration requires at least 5 strikes per expiry; got {} at t={:.6}",
                        expiry_quotes.len(),
                        expiry_key.into_inner()
                    )));
                }

                let expiry = expiry_key.into_inner();
                let forward = forward_fn(expiry);
                let strikes: Vec<f64> = expiry_quotes.iter().map(|(strike, _)| *strike).collect();
                let vols: Vec<f64> = expiry_quotes.iter().map(|(_, vol)| *vol).collect();

                let params = finstack_core::math::volatility::svi::calibrate_svi(
                    &strikes, &vols, forward, expiry,
                )?;

                for (idx, (strike, market_vol)) in expiry_quotes.iter().enumerate() {
                    let log_moneyness = (*strike / forward).ln();
                    let model_vol = params.implied_vol(log_moneyness, expiry);
                    residuals.insert(
                        format!("svi_t{expiry:.6}_k{strike:.6}_i{idx}"),
                        (model_vol - *market_vol).abs(),
                    );
                }

                params_by_expiry.insert(expiry_key, params);
            }

            let mut grid = Vec::with_capacity(p.target_expiries.len() * p.target_strikes.len());
            for &target_expiry in &p.target_expiries {
                let params = interpolate_svi_params(target_expiry, &params_by_expiry)?;
                let forward = forward_fn(target_expiry);
                for &target_strike in &p.target_strikes {
                    let log_moneyness = (target_strike / forward).ln();
                    let vol = params.implied_vol(log_moneyness, target_expiry);
                    if !vol.is_finite() || vol <= 0.0 {
                        return Err(finstack_core::Error::Validation(format!(
                            "SVI surface produced invalid implied vol at t={target_expiry:.6}, strike={target_strike:.6}",
                        )));
                    }
                    grid.push(vol);
                }
            }

            let surface =
                VolSurface::from_grid(&p.surface_id, &p.target_expiries, &p.target_strikes, &grid)?;
            let mut report = CalibrationReport::new(
                residuals,
                params_by_expiry.len(),
                true,
                "SVI surface calibration completed",
            )
            .with_model_version("SVI v1.0");
            report.update_solver_config(global_config.solver.clone());

            Ok(StepOutcome {
                output: StepOutput::Surface(surface.into()),
                credit_index_update: None,
                report,
            })
        }
    }
}

/// Execute a calibration step and normalize its output/result.
pub(crate) fn execute(
    step: &CalibrationStep,
    quotes: &[MarketQuote],
    context: &MarketContext,
    global_config: &CalibrationConfig,
) -> Result<StepOutcome> {
    execute_params(&step.params, quotes, context, global_config)
}

/// Execute [`StepParams`] directly and apply the output to a cloned context.
pub(crate) fn execute_params_and_apply(
    params: &StepParams,
    quotes: &[MarketQuote],
    context: &MarketContext,
    global_config: &CalibrationConfig,
) -> Result<(MarketContext, CalibrationReport)> {
    let outcome = execute_params(params, quotes, context, global_config)?;
    let StepOutcome {
        output,
        credit_index_update,
        report,
    } = outcome;

    let mut new_context = context.clone();
    apply_output(&mut new_context, output, credit_index_update);
    Ok((new_context, report))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::calibration::api::schema::{HullWhiteStepParams, StudentTParams, SviSurfaceParams};
    use crate::instruments::credit_derivatives::cds_tranche::{
        CDSTranche, CDSTranchePricer, CDSTranchePricerConfig, TrancheSide,
    };
    use crate::instruments::Attributes;
    use crate::market::conventions::ids::{
        CdsConventionKey, CdsDocClause, OptionConventionId, SwaptionConventionId,
    };
    use crate::market::quotes::cds_tranche::CDSTrancheQuote;
    use crate::market::quotes::ids::QuoteId;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{Date, DayCount, DayCountCtx};
    use finstack_core::market_data::term_structures::{
        BaseCorrelationCurve, CreditIndexData, DiscountCurve, HazardCurve,
    };
    use finstack_core::money::Money;
    use finstack_core::types::{CurveId, UnderlyingId};
    use std::sync::Arc;
    use time::Month;

    fn build_flat_discount_curve(rate: f64, base_date: Date, curve_id: &str) -> DiscountCurve {
        DiscountCurve::builder(curve_id)
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-rate).exp()),
                (5.0, (-rate * 5.0).exp()),
                (10.0, (-rate * 10.0).exp()),
            ])
            .build()
            .expect("flat discount curve should build")
    }

    fn build_student_t_market(base_date: Date, correlation: f64) -> MarketContext {
        let discount = build_flat_discount_curve(0.03, base_date, "USD-OIS");
        let hazard = HazardCurve::builder("CDX_HAZARD")
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .recovery_rate(0.40)
            .knots([(1.0, 0.0010), (5.0, 0.0012), (10.0, 0.0015)])
            .build()
            .expect("hazard curve");
        let base_corr = BaseCorrelationCurve::builder("CDX_CORR")
            .knots([(3.0, correlation), (7.0, correlation)])
            .build()
            .expect("base correlation curve");
        let credit_index = CreditIndexData::builder()
            .num_constituents(125)
            .recovery_rate(0.40)
            .index_credit_curve(Arc::new(hazard.clone()))
            .base_correlation_curve(Arc::new(base_corr.clone()))
            .build()
            .expect("credit index");

        MarketContext::new()
            .insert_discount(discount)
            .insert_hazard(hazard)
            .insert_base_correlation(base_corr)
            .insert_credit_index("CDX.NA.IG", credit_index)
    }

    fn build_student_t_quote(base_date: Date, df: f64, correlation: f64) -> CDSTrancheQuote {
        let market = build_student_t_market(base_date, correlation);
        let maturity = Date::from_calendar_date(2030, Month::March, 20).expect("valid maturity");
        let tranche = CDSTranche::builder()
            .id("TRANCHE-1".into())
            .index_name("CDX.NA.IG".to_string())
            .series(1)
            .attach_pct(3.0)
            .detach_pct(7.0)
            .notional(Money::new(0.04, Currency::USD))
            .maturity(maturity)
            .running_coupon_bp(500.0)
            .frequency("3M".parse().expect("tenor"))
            .day_count(DayCount::Act360)
            .bdc(finstack_core::dates::BusinessDayConvention::Following)
            .calendar_id_opt(None)
            .discount_curve_id(CurveId::from("USD-OIS"))
            .credit_index_id(CurveId::from("CDX.NA.IG"))
            .side(TrancheSide::SellProtection)
            .effective_date_opt(None)
            .accumulated_loss(0.0)
            .standard_imm_dates(true)
            .attributes(Attributes::new())
            .build()
            .expect("tranche");

        let pricer = CDSTranchePricer::with_params(
            CDSTranchePricerConfig::default().with_student_t_copula(df),
        );
        let upfront_pct = pricer
            .calculate_upfront(&tranche, &market, base_date)
            .expect("upfront")
            / tranche.notional.amount();

        CDSTrancheQuote::CDSTranche {
            id: QuoteId::new("TRANCHE-1"),
            index: "CDX.NA.IG".to_string(),
            attachment: 0.03,
            detachment: 0.07,
            maturity,
            upfront_pct,
            running_spread_bp: 500.0,
            convention: CdsConventionKey {
                currency: Currency::USD,
                doc_clause: CdsDocClause::Cr14,
            },
        }
    }

    #[test]
    fn student_t_step_calibrates_and_returns_scalar_output() {
        let base_date = Date::from_calendar_date(2025, Month::March, 20).expect("valid date");
        let params = StepParams::StudentT(StudentTParams {
            tranche_instrument_id: "TRANCHE-1".to_string(),
            base_correlation_curve_id: "CDX_CORR".to_string(),
            discount_curve_id: Some("USD-OIS".into()),
            initial_df: 6.0,
            df_bounds: (2.5, 12.0),
            correlation: 0.3,
        });
        let quotes = vec![MarketQuote::CDSTranche(build_student_t_quote(
            base_date, 6.0, 0.3,
        ))];
        let context = build_student_t_market(base_date, 0.25);

        let outcome = execute_params(&params, &quotes, &context, &CalibrationConfig::default())
            .expect("Student-t step should calibrate");

        match outcome.output {
            StepOutput::Scalar { key, value } => {
                assert_eq!(key, "TRANCHE-1_STUDENT_T_DF");
                let MarketScalar::Unitless(calibrated_df) = value else {
                    panic!("expected unitless calibrated df");
                };
                assert!(
                    (calibrated_df - 6.0).abs() < 0.5,
                    "expected calibrated df near 6.0, got {calibrated_df}"
                );
            }
            _ => panic!("expected scalar output"),
        }
    }

    #[test]
    fn hull_white_step_persists_both_kappa_and_sigma_scalars() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let params = StepParams::HullWhite(HullWhiteStepParams {
            curve_id: "USD-OIS".into(),
            currency: Currency::USD,
            base_date,
            initial_kappa: Some(0.04),
            initial_sigma: Some(0.008),
        });
        let quotes = vec![
            MarketQuote::Vol(VolQuote::SwaptionVol {
                expiry: Date::from_calendar_date(2026, Month::January, 1).expect("expiry"),
                maturity: Date::from_calendar_date(2031, Month::January, 1).expect("maturity"),
                strike: 0.03,
                vol: 0.0050,
                quote_type: "normal".to_string(),
                convention: SwaptionConventionId::new("USD"),
            }),
            MarketQuote::Vol(VolQuote::SwaptionVol {
                expiry: Date::from_calendar_date(2027, Month::January, 1).expect("expiry"),
                maturity: Date::from_calendar_date(2032, Month::January, 1).expect("maturity"),
                strike: 0.03,
                vol: 0.0060,
                quote_type: "normal".to_string(),
                convention: SwaptionConventionId::new("USD"),
            }),
        ];
        let context = MarketContext::new()
            .insert_discount(build_flat_discount_curve(0.03, base_date, "USD-OIS"));

        let outcome = execute_params(&params, &quotes, &context, &CalibrationConfig::default())
            .expect("Hull-White step should calibrate");

        match outcome.output {
            StepOutput::Scalars(values) => {
                assert!(
                    values
                        .iter()
                        .any(|(key, _)| key.starts_with("USD-OIS_") && key.ends_with("KAPPA")),
                    "expected calibrated kappa scalar output"
                );
                assert!(
                    values
                        .iter()
                        .any(|(key, _)| key.starts_with("USD-OIS_") && key.ends_with("SIGMA")),
                    "expected calibrated sigma scalar output"
                );
            }
            _ => panic!("expected multiple scalar outputs for Hull-White calibration"),
        }
    }

    #[test]
    fn svi_surface_step_builds_surface_from_option_vol_quotes() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let expiry_1 = Date::from_calendar_date(2025, Month::July, 1).expect("valid date");
        let expiry_2 = Date::from_calendar_date(2026, Month::January, 1).expect("valid date");
        let time_dc = DayCount::Act365F;
        let t1 = time_dc
            .year_fraction(base_date, expiry_1, DayCountCtx::default())
            .expect("valid year fraction");
        let t2 = time_dc
            .year_fraction(base_date, expiry_2, DayCountCtx::default())
            .expect("valid year fraction");

        let params = StepParams::SviSurface(SviSurfaceParams {
            surface_id: "SPX-SVI".to_string(),
            base_date,
            underlying_ticker: "SPX".to_string(),
            discount_curve_id: Some("USD-OIS".into()),
            target_expiries: vec![t1, t2],
            target_strikes: vec![80.0, 90.0, 100.0, 110.0, 120.0],
            spot_override: Some(100.0),
        });

        let quotes = vec![
            MarketQuote::Vol(VolQuote::OptionVol {
                underlying: UnderlyingId::new("SPX"),
                expiry: expiry_1,
                strike: 80.0,
                vol: 0.30,
                option_type: "Call".to_string(),
                convention: OptionConventionId::new("USD-EQ"),
            }),
            MarketQuote::Vol(VolQuote::OptionVol {
                underlying: UnderlyingId::new("SPX"),
                expiry: expiry_1,
                strike: 90.0,
                vol: 0.24,
                option_type: "Call".to_string(),
                convention: OptionConventionId::new("USD-EQ"),
            }),
            MarketQuote::Vol(VolQuote::OptionVol {
                underlying: UnderlyingId::new("SPX"),
                expiry: expiry_1,
                strike: 100.0,
                vol: 0.20,
                option_type: "Call".to_string(),
                convention: OptionConventionId::new("USD-EQ"),
            }),
            MarketQuote::Vol(VolQuote::OptionVol {
                underlying: UnderlyingId::new("SPX"),
                expiry: expiry_1,
                strike: 110.0,
                vol: 0.22,
                option_type: "Call".to_string(),
                convention: OptionConventionId::new("USD-EQ"),
            }),
            MarketQuote::Vol(VolQuote::OptionVol {
                underlying: UnderlyingId::new("SPX"),
                expiry: expiry_1,
                strike: 120.0,
                vol: 0.27,
                option_type: "Call".to_string(),
                convention: OptionConventionId::new("USD-EQ"),
            }),
            MarketQuote::Vol(VolQuote::OptionVol {
                underlying: UnderlyingId::new("SPX"),
                expiry: expiry_2,
                strike: 80.0,
                vol: 0.32,
                option_type: "Call".to_string(),
                convention: OptionConventionId::new("USD-EQ"),
            }),
            MarketQuote::Vol(VolQuote::OptionVol {
                underlying: UnderlyingId::new("SPX"),
                expiry: expiry_2,
                strike: 90.0,
                vol: 0.27,
                option_type: "Call".to_string(),
                convention: OptionConventionId::new("USD-EQ"),
            }),
            MarketQuote::Vol(VolQuote::OptionVol {
                underlying: UnderlyingId::new("SPX"),
                expiry: expiry_2,
                strike: 100.0,
                vol: 0.23,
                option_type: "Call".to_string(),
                convention: OptionConventionId::new("USD-EQ"),
            }),
            MarketQuote::Vol(VolQuote::OptionVol {
                underlying: UnderlyingId::new("SPX"),
                expiry: expiry_2,
                strike: 110.0,
                vol: 0.24,
                option_type: "Call".to_string(),
                convention: OptionConventionId::new("USD-EQ"),
            }),
            MarketQuote::Vol(VolQuote::OptionVol {
                underlying: UnderlyingId::new("SPX"),
                expiry: expiry_2,
                strike: 120.0,
                vol: 0.28,
                option_type: "Call".to_string(),
                convention: OptionConventionId::new("USD-EQ"),
            }),
        ];

        let context = MarketContext::new()
            .insert_discount(build_flat_discount_curve(0.03, base_date, "USD-OIS"));

        let outcome = execute_params(&params, &quotes, &context, &CalibrationConfig::default())
            .expect("SVI step should build a surface");

        match outcome.output {
            StepOutput::Surface(surface) => {
                assert_eq!(surface.id(), &CurveId::from("SPX-SVI"));
                assert_eq!(surface.grid_shape(), (2, 5));
                let atm_vol = surface
                    .value_checked(t1, 100.0)
                    .expect("ATM point should exist");
                assert!(atm_vol.is_finite(), "ATM SVI vol should be finite");
                assert!(
                    atm_vol > 0.0 && atm_vol < 1.0,
                    "ATM SVI vol should be in a realistic range, got {atm_vol}"
                );
            }
            _ => panic!("expected surface output"),
        }
    }
}
