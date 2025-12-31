use crate::calibration::api::schema::InflationCurveParams;
use crate::calibration::config::{CalibrationConfig, CalibrationMethod};
use crate::calibration::prepared::CalibrationQuote;
use crate::calibration::solver::bootstrap::SequentialBootstrapper;
use crate::calibration::solver::traits::BootstrapTarget;
use crate::calibration::CalibrationReport;
use crate::market::quotes::inflation::InflationQuote;
use crate::market::quotes::market_quote::{ExtractQuotes, MarketQuote};

use crate::instruments::inflation_swap::{InflationSwap, PayReceiveInflation, YoYInflationSwap};
use crate::market::build::prepared::PreparedQuote;
use crate::market::conventions::registry::ConventionRegistry;
use finstack_core::dates::DateExt;
use finstack_core::dates::{DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::InflationLag;
use finstack_core::market_data::term_structures::InflationCurve;
use finstack_core::money::Money;
use finstack_core::Result;
use std::cell::RefCell;
use std::sync::Arc;

/// Bootstrapper for inflation curves from inflation swap quotes.
///
/// Implements sequential bootstrapping of inflation curves using zero-coupon
/// inflation swap (ZCIS) quotes with different maturities. The bootstrapper
/// prices synthetic inflation swaps to solve for CPI values that match
/// market quotes.
pub struct InflationBootstrapper {
    /// Parameters for the inflation curve (ID, interpolation, etc).
    pub params: InflationCurveParams,
    /// Baseline market context containing discount curves.
    pub base_context: MarketContext,
    /// Optional reusable context for sequential solvers to reduce memory pressure.
    reuse_context: Option<RefCell<MarketContext>>,
}

impl InflationBootstrapper {
    /// Creates a new inflation curve bootstrapper.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters defining the inflation curve structure
    /// * `base_context` - Market context containing discount curves
    ///
    /// # Returns
    ///
    /// A new `InflationBootstrapper` instance ready for calibration.
    pub fn new(
        params: InflationCurveParams,
        base_context: MarketContext,
        use_parallel: bool,
    ) -> Self {
        let reuse_context = if use_parallel {
            None
        } else {
            Some(RefCell::new(base_context.clone()))
        };
        Self {
            params,
            base_context,
            reuse_context,
        }
    }

    /// Pre-build per-quote instruments so solver residual evaluation is allocation-free.
    pub fn prepare_quotes(&self, quotes: Vec<InflationQuote>) -> Result<Vec<CalibrationQuote>> {
        quotes.into_iter().map(|q| self.prepare_single(q)).collect()
    }

    fn prepare_single(&self, raw: InflationQuote) -> Result<CalibrationQuote> {
        let (maturity, rate, index_name, frequency, convention_id) = match &raw {
            InflationQuote::InflationSwap {
                maturity,
                rate,
                index,
                convention,
            } => (*maturity, *rate, index.as_str(), None, convention),
            InflationQuote::YoYInflationSwap {
                maturity,
                rate,
                index,
                frequency,
                convention,
            } => (
                *maturity,
                *rate,
                index.as_str(),
                Some(*frequency),
                convention,
            ),
        };

        // Load conventions
        let conventions =
            ConventionRegistry::try_global()?.require_inflation_swap(convention_id)?;

        if index_name != self.params.index && index_name != self.params.curve_id.as_str() {
            return Err(finstack_core::Error::Validation(format!(
                "Quote index {} does not match calibrator index {}",
                index_name, self.params.index
            )));
        }

        let base_date = self.params.base_date;
        let has_index_fixings = self
            .base_context
            .inflation_index(self.params.curve_id.as_str())
            .is_some();

        let (lag, base_cpi) = if let Some(index) = self
            .base_context
            .inflation_index(self.params.curve_id.as_str())
        {
            let base_cpi = index.value_on(base_date).map_err(|e| {
                finstack_core::Error::Validation(format!(
                    "Failed to resolve base CPI from inflation index '{}': {}",
                    self.params.curve_id.as_str(),
                    e
                ))
            })?;
            (index.lag(), base_cpi)
        } else {
            // Use conventions lag if available, otherwise params
            (
                self.parse_lag(&conventions.inflation_lag.to_string())
                    .or_else(|_| self.parse_lag(&self.params.observation_lag))?,
                self.params.base_cpi,
            )
        };

        let swap: std::sync::Arc<dyn crate::instruments::common::traits::Instrument> =
            if let Some(freq) = frequency {
                let instrument = YoYInflationSwap::builder()
                    .id("CALIB_YOY".into())
                    .notional(Money::new(self.params.notional, self.params.currency))
                    .start(base_date)
                    .maturity(maturity)
                    .fixed_rate(rate)
                    .frequency(freq)
                    .inflation_index_id(self.params.curve_id.clone())
                    .discount_curve_id(self.params.discount_curve_id.clone())
                    .dc(conventions.day_count)
                    .side(PayReceiveInflation::PayFixed)
                    .lag_override_opt(if has_index_fixings { None } else { Some(lag) })
                    .bdc_opt(Some(conventions.business_day_convention))
                    .calendar_id_opt(Some(conventions.calendar_id.clone()))
                    .build()
                    .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;
                Arc::new(instrument)
            } else {
                let instrument = InflationSwap::builder()
                    .id("CALIB_ZCIS".into())
                    .notional(Money::new(self.params.notional, self.params.currency))
                    .start(base_date)
                    .maturity(maturity)
                    .fixed_rate(rate)
                    .inflation_index_id(self.params.curve_id.clone())
                    .discount_curve_id(self.params.discount_curve_id.clone())
                    .dc(conventions.day_count)
                    .side(PayReceiveInflation::PayFixed)
                    .lag_override_opt(if has_index_fixings { None } else { Some(lag) })
                    .base_cpi_opt(if has_index_fixings {
                        None
                    } else {
                        Some(base_cpi)
                    })
                    .bdc_opt(Some(conventions.business_day_convention))
                    .calendar_id_opt(Some(conventions.calendar_id.clone()))
                    .build()
                    .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;
                Arc::new(instrument)
            };

        // Calculate pillar time (lagged)
        let fixing_date = Self::apply_lag(maturity, lag);
        let pillar_time = DayCount::Act365F.year_fraction(
            self.params.base_date,
            fixing_date,
            DayCountCtx::default(),
        )?;

        let pq = PreparedQuote::new(Arc::new(raw), swap, maturity, pillar_time);
        Ok(CalibrationQuote::Inflation(pq))
    }

    /// Parse an observation lag string (e.g. "3M").
    fn parse_lag(&self, spec: &str) -> Result<InflationLag> {
        let s = spec.trim();
        if s.is_empty() {
            return Ok(InflationLag::None);
        }
        let upper = s.to_ascii_uppercase();
        if upper == "NONE" || upper == "0" || upper == "0M" || upper == "0D" {
            return Ok(InflationLag::None);
        }
        if let Some(num) = upper.strip_suffix('M') {
            let months: u8 = num.trim().parse().map_err(|_| {
                finstack_core::Error::Validation(format!(
                    "Invalid observation_lag '{spec}': expected like '3M'"
                ))
            })?;
            return Ok(InflationLag::Months(months));
        }
        if let Some(num) = upper.strip_suffix('D') {
            let days: u16 = num.trim().parse().map_err(|_| {
                finstack_core::Error::Validation(format!(
                    "Invalid observation_lag '{spec}': expected like '90D'"
                ))
            })?;
            return Ok(InflationLag::Days(days));
        }
        Err(finstack_core::Error::Validation(format!(
            "Invalid observation_lag '{spec}': expected like '3M' or '90D'"
        )))
    }

    /// Apply an observation lag to a date.
    fn apply_lag(
        date: finstack_core::dates::Date,
        lag: InflationLag,
    ) -> finstack_core::dates::Date {
        match lag {
            InflationLag::None => date,
            InflationLag::Months(m) => date.add_months(-(m as i32)),
            InflationLag::Days(d) => date - time::Duration::days(d as i64),
            _ => date,
        }
    }

    /// Resolve the effective base CPI level (from index or params).
    fn effective_base_cpi(&self) -> Result<f64> {
        if let Some(index) = self
            .base_context
            .inflation_index(self.params.curve_id.as_str())
        {
            return index.value_on(self.params.base_date).map_err(|e| {
                finstack_core::Error::Validation(format!(
                    "Failed to resolve base CPI from inflation index '{}': {}",
                    self.params.curve_id.as_str(),
                    e
                ))
            });
        }
        Ok(self.params.base_cpi)
    }

    fn with_temp_context<F, T>(&self, curve: &InflationCurve, op: F) -> Result<T>
    where
        F: FnOnce(&MarketContext) -> Result<T>,
    {
        if let Some(ctx_cell) = &self.reuse_context {
            let mut ctx = ctx_cell.borrow_mut();
            *ctx = std::mem::take(&mut *ctx).insert_inflation(curve.clone());
            op(&ctx)
        } else {
            let mut temp_context = self.base_context.clone();
            temp_context = temp_context.insert_inflation(curve.clone());
            op(&temp_context)
        }
    }

    /// Execute the full calibration for an inflation curve step.
    pub fn solve(
        params: &InflationCurveParams,
        quotes: &[MarketQuote],
        context: &MarketContext,
        global_config: &CalibrationConfig,
    ) -> Result<(MarketContext, CalibrationReport)> {
        let inflation_quotes: Vec<InflationQuote> = quotes.extract_quotes();

        if inflation_quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::InputError::TooFewPoints,
            ));
        }

        let mut config = global_config.clone();
        config.calibration_method = params.method.clone();

        let target =
            InflationBootstrapper::new(params.clone(), context.clone(), config.use_parallel);
        let prepared_quotes = target.prepare_quotes(inflation_quotes)?;

        let (curve, mut report) = match params.method {
            CalibrationMethod::Bootstrap => SequentialBootstrapper::bootstrap(
                &target,
                &prepared_quotes,
                Vec::new(),
                &config,
                None,
            )?,
            CalibrationMethod::GlobalSolve { .. } => {
                return Err(finstack_core::Error::Input(
                    finstack_core::InputError::Invalid,
                ));
            }
        };

        report.update_solver_config(config.solver.clone());

        let new_context = context.clone().insert_inflation(curve);
        Ok((new_context, report))
    }
}

impl BootstrapTarget for InflationBootstrapper {
    type Quote = CalibrationQuote;
    type Curve = InflationCurve;

    fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
        Ok(quote.pillar_time())
    }

    fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        // knots are (time, cpi)
        // Ensure base point (0.0, base_cpi) is included or added
        let base_cpi = self.effective_base_cpi()?;
        let mut full_knots: Vec<(f64, f64)> = knots
            .iter()
            .copied()
            .filter(|(t, _)| t.abs() > 1e-8)
            .collect();
        full_knots.push((0.0, base_cpi));
        full_knots.sort_by(|a, b| a.0.total_cmp(&b.0));

        InflationCurve::builder(self.params.curve_id.to_string())
            .base_cpi(base_cpi)
            .knots(full_knots)
            .set_interp(self.params.interpolation)
            .build()
    }

    fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
        let base_date = self.params.base_date;
        // Context needs the curve being calibrated + discount curve
        self.with_temp_context(curve, |ctx| {
            let pv = quote.get_instrument().value_raw(ctx, base_date)?;
            Ok(pv / self.params.notional)
        })
    }

    fn initial_guess(&self, quote: &Self::Quote, _previous_knots: &[(f64, f64)]) -> Result<f64> {
        let t = self.quote_time(quote)?;
        // We know it's Inflation variant
        let rate = match quote {
            CalibrationQuote::Inflation(pq) => match pq.quote.as_ref() {
                InflationQuote::InflationSwap { rate, .. } => *rate,
                _ => 0.02,
            },
            _ => 0.02, // Fallback if mismatched type (shouldn't happen)
        };
        let base_cpi = self.effective_base_cpi()?;
        Ok(base_cpi * (1.0 + rate).powf(t))
    }
}
