use crate::calibration::api::schema::HazardCurveParams;
use crate::calibration::quotes::CreditQuote;
use crate::calibration::solver::BootstrapTarget;
use crate::calibration::pricing::quote_factory;
use crate::instruments::cds::CDSConvention;
use finstack_core::dates::DayCountCtx;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::prelude::*;
use finstack_core::Result;

const HAZARD_HARD_MIN: f64 = 0.0;
// Safety cap: λ=10 implies ~99.995% 1Y default probability and can lead to numerical underflow
// in long-dated curves. Treat as a hard validation error during calibration.
const HAZARD_HARD_MAX: f64 = 10.0;

/// Bootstrapper for hazard curves from CDS quotes.
///
/// Implements sequential bootstrapping of hazard curves using CDS quotes
/// with different maturities. The bootstrapper derives CDS conventions from
/// currency and prices synthetic CDS instruments to solve for hazard rates
/// that match market quotes.
pub struct HazardBootstrapper {
    params: HazardCurveParams,
    convention: CDSConvention,
    base_context: MarketContext,
}

impl HazardBootstrapper {
    /// Creates a new hazard curve bootstrapper.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters defining the hazard curve structure
    /// * `base_context` - Market context containing discount curves
    ///
    /// # Returns
    ///
    /// A new `HazardBootstrapper` instance ready for calibration.
    ///
    /// # Note
    ///
    /// CDS conventions are automatically derived from the currency:
    /// - USD/CAD: ISDA North American
    /// - EUR/GBP/CHF: ISDA European
    /// - JPY/HKD/SGD/AUD/NZD: ISDA Asian
    pub fn new(params: HazardCurveParams, base_context: MarketContext) -> Self {
        // Derive convention from currency
        let convention = match params.currency {
            Currency::USD | Currency::CAD => CDSConvention::IsdaNa,
            Currency::EUR | Currency::GBP | Currency::CHF => CDSConvention::IsdaEu,
            Currency::JPY | Currency::HKD | Currency::SGD | Currency::AUD | Currency::NZD => {
                CDSConvention::IsdaAs
            }
            _ => CDSConvention::IsdaNa,
        };

        Self {
            params,
            convention,
            base_context,
        }
    }
}

impl BootstrapTarget for HazardBootstrapper {
    type Quote = CreditQuote;
    type Curve = HazardCurve;

    fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
        let dc = self.convention.day_count();
        let maturity = quote.maturity_date().ok_or(finstack_core::Error::Input(
            finstack_core::error::InputError::Invalid,
        ))?;
        dc.year_fraction(self.params.base_date, maturity, DayCountCtx::default())
    }

    fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        HazardCurve::builder(self.params.curve_id.to_string())
            .base_date(self.params.base_date)
            .day_count(self.convention.day_count())
            .issuer(self.params.entity.clone())
            .seniority(self.params.seniority)
            .currency(self.params.currency)
            .recovery_rate(self.params.recovery_rate)
            .knots(knots.to_vec())
            // Par spread interpolation is for *reporting* quoted spreads on the calibrated curve.
            // Positivity / no-arbitrage for survival is enforced via λ>=0 and the curve's
            // log-linear survival interpolation (in finstack_core).
            .par_interp(self.params.par_interp)
            .build()
    }

    fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
        let base_date = self.params.base_date;
        let (instrument, upfront_opt) =
            quote_factory::build_instrument_for_credit_quote(quote, &self.params, self.convention)?;

        let mut temp_ctx = self.base_context.clone();
        temp_ctx.insert_mut(std::sync::Arc::new(curve.clone()));

        let npv = instrument.value(&temp_ctx, base_date)?.amount();
        let adjusted = match upfront_opt {
            None => npv,
            Some(upfront) => npv - upfront.amount(),
        };

        Ok(adjusted / self.params.notional)
    }

    fn initial_guess(&self, _quote: &Self::Quote, previous_knots: &[(f64, f64)]) -> Result<f64> {
        let guess = previous_knots.last().map(|&(_, v)| v).unwrap_or(0.01);
        if guess.is_finite() {
            Ok(guess.clamp(HAZARD_HARD_MIN, HAZARD_HARD_MAX))
        } else {
            Ok(0.01)
        }
    }

    fn scan_points(&self, _quote: &Self::Quote, initial_guess: f64) -> Result<Vec<f64>> {
        // Bounded, maturity-agnostic scan grid (log-spaced) on [0, HAZARD_HARD_MAX].
        // This prevents the solver from spending effort in negative/absurd hazard regions.
        let max_h = HAZARD_HARD_MAX;
        let min_positive = 1e-10_f64;

        let center = if initial_guess.is_finite() {
            initial_guess.clamp(HAZARD_HARD_MIN, max_h)
        } else {
            0.01_f64
        };

        let mut pts = Vec::with_capacity(64);
        pts.push(0.0);
        pts.push(center);
        pts.push(max_h);

        let center_pos = center.max(min_positive);
        let log_center = center_pos.log10();
        let low_exp = (log_center - 4.0).max(min_positive.log10());
        let high_exp = (log_center + 2.0).min(max_h.log10());

        const N: usize = 48;
        if (high_exp - low_exp).abs() > 1e-12 {
            for i in 0..N {
                let t = i as f64 / (N - 1) as f64;
                let exp = low_exp + t * (high_exp - low_exp);
                let v = 10f64.powf(exp);
                if v.is_finite() && v >= 0.0 && v <= max_h {
                    pts.push(v);
                }
            }
        } else {
            pts.push(center_pos);
        }

        pts.sort_by(|a, b| a.total_cmp(b));
        pts.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
        Ok(pts)
    }

    fn validate_knot(&self, time: f64, value: f64) -> Result<()> {
        if !time.is_finite() || time <= 0.0 {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Invalid hazard knot time for {}: t={}",
                    self.params.curve_id, time
                ),
                category: "bootstrapping".to_string(),
            });
        }
        if !value.is_finite() {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Non-finite hazard rate for {} at t={:.6}",
                    self.params.curve_id, time
                ),
                category: "bootstrapping".to_string(),
            });
        }
        if value < HAZARD_HARD_MIN {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Negative hazard rate for {} at t={:.6}: {:.6}",
                    self.params.curve_id, time, value
                ),
                category: "bootstrapping".to_string(),
            });
        }
        if value > HAZARD_HARD_MAX {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Hazard rate out of bounds for {} at t={:.6}: {:.6} (max {:.6})",
                    self.params.curve_id, time, value, HAZARD_HARD_MAX
                ),
                category: "bootstrapping".to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibration::solver::BootstrapTarget;
    use crate::calibration::pricing::quote_factory;
    use crate::calibration::quotes::InstrumentConventions;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::term_structures::ParInterp;
    use time::Month;

    fn base_params() -> HazardCurveParams {
        HazardCurveParams {
            curve_id: CurveId::new("TEST-HAZ".to_string()),
            entity: "ACME".to_string(),
            seniority: finstack_core::market_data::term_structures::Seniority::Senior,
            currency: Currency::USD,
            base_date: Date::from_calendar_date(2025, Month::January, 1).expect("valid base_date"),
            discount_curve_id: CurveId::new("USD-OIS".to_string()),
            recovery_rate: 0.4,
            notional: 1.0,
            method: crate::calibration::config::CalibrationMethod::Bootstrap,
            interpolation: InterpStyle::Linear,
            par_interp: ParInterp::Linear,
        }
    }

    #[test]
    fn validate_knot_rejects_negative_hazard() {
        let target = HazardBootstrapper::new(base_params(), MarketContext::default());
        let err = target
            .validate_knot(1.0, -1e-6)
            .expect_err("should reject negative hazard");
        assert!(err.to_string().to_lowercase().contains("negative hazard"));
    }

    #[test]
    fn validate_knot_rejects_hazard_above_max() {
        let target = HazardBootstrapper::new(base_params(), MarketContext::default());
        let err = target
            .validate_knot(1.0, HAZARD_HARD_MAX + 1e-6)
            .expect_err("should reject excessive hazard");
        assert!(err.to_string().to_lowercase().contains("out of bounds"));
    }

    #[test]
    fn build_curve_preserves_par_interp_and_monotone_survival() {
        let mut p = base_params();
        p.par_interp = ParInterp::LogLinear;
        let target = HazardBootstrapper::new(p, MarketContext::default());

        let curve = target
            .build_curve(&[(1.0, 0.02), (5.0, 0.03)])
            .expect("curve build should succeed");
        assert_eq!(curve.par_interp(), ParInterp::LogLinear);

        let s1 = curve.sp(1.0);
        let s5 = curve.sp(5.0);
        let s10 = curve.sp(10.0);
        assert!((0.0..=1.0).contains(&s1));
        assert!((0.0..=1.0).contains(&s5));
        assert!((0.0..=1.0).contains(&s10));
        assert!(s1 >= s5 && s5 >= s10);
    }

    #[test]
    fn credit_factory_returns_upfront_for_cds_upfront() {
        let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("date");
        let mut params = base_params();
        params.base_date = base_date;
        params.discount_curve_id = "USD-OIS".into();
        params.notional = 1_000_000.0;
        params.recovery_rate = 0.4;

        let hazard = HazardCurve::builder("CRV")
            .base_date(base_date)
            .knots([(0.0, 0.01), (5.0, 0.015)])
            .build()
            .expect("hazard curve");
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (5.0, 0.95)])
            .build()
            .expect("disc");

        let ctx = MarketContext::new()
            .insert_discount(disc)
            .insert_hazard(hazard.clone());

        let quote = CreditQuote::CDSUpfront {
            entity: "ABC".to_string(),
            maturity: base_date + time::Duration::days(365),
            upfront_pct: 2.0,
            running_spread_bp: 100.0,
            recovery_rate: 0.4,
            currency: Currency::USD,
            conventions: InstrumentConventions::default(),
        };

        let (inst, upfront_opt) =
            quote_factory::build_instrument_for_credit_quote(&quote, &params, CDSConvention::IsdaNa)
                .expect("factory build");
        let upfront = upfront_opt.expect("expected upfront");

        let pv = inst.value(&ctx, base_date).expect("pv").amount();
        let adjusted = pv - upfront.amount();
        assert!(adjusted.is_finite());
    }
}
