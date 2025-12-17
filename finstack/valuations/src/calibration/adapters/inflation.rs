use crate::calibration::api::schema::InflationCurveParams;
use crate::calibration::domain::quotes::InflationQuote;
use crate::calibration::domain::solver::BootstrapTarget;
use crate::instruments::common::traits::Instrument;
use crate::instruments::inflation_swap::{InflationSwap, PayReceiveInflation};
use finstack_core::dates::{DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::inflation_index::InflationLag;
use finstack_core::market_data::term_structures::InflationCurve;
use finstack_core::money::Money;
use finstack_core::prelude::DateExt;
use finstack_core::Result;

/// Bootstrapper for inflation curves from inflation swap quotes.
///
/// Implements sequential bootstrapping of inflation curves using zero-coupon
/// inflation swap (ZCIS) quotes with different maturities. The bootstrapper
/// prices synthetic inflation swaps to solve for CPI values that match
/// market quotes.
pub struct InflationBootstrapper {
    params: InflationCurveParams,
    base_context: MarketContext,
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
    pub fn new(params: InflationCurveParams, base_context: MarketContext) -> Self {
        Self {
            params,
            base_context,
        }
    }

    fn parse_lag(spec: &str) -> Result<InflationLag> {
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

    fn effective_lag(&self) -> Result<InflationLag> {
        if let Some(index) = self
            .base_context
            .inflation_index_ref(self.params.curve_id.as_str())
        {
            return Ok(index.lag());
        }
        Self::parse_lag(&self.params.observation_lag)
    }

    fn effective_base_cpi(&self) -> Result<f64> {
        if let Some(index) = self
            .base_context
            .inflation_index_ref(self.params.curve_id.as_str())
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
}

impl BootstrapTarget for InflationBootstrapper {
    type Quote = InflationQuote;
    type Curve = InflationCurve;

    fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
        let maturity = quote.maturity_date().ok_or(finstack_core::Error::Input(
            finstack_core::error::InputError::Invalid,
        ))?;
        // Align time-axis with `InflationSwap` pricing conventions:
        // use Act365F and apply observation lag to the index fixing date.
        let lag = self.effective_lag()?;
        let fixing_date = Self::apply_lag(maturity, lag);
        DayCount::Act365F.year_fraction(self.params.base_date, fixing_date, DayCountCtx::default())
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
        full_knots.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .expect("Time values should be comparable")
        });

        InflationCurve::builder(self.params.curve_id.to_string())
            .base_cpi(base_cpi)
            .knots(full_knots)
            .set_interp(self.params.interpolation)
            .build()
    }

    fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
        let (maturity, rate, index_name) = match quote {
            InflationQuote::InflationSwap {
                maturity,
                rate,
                index,
                ..
            } => (*maturity, *rate, index),
            _ => {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ))
            }
        };

        if index_name != &self.params.index && index_name != self.params.curve_id.as_str() {
            return Err(finstack_core::Error::Validation(format!(
                "Quote index {} does not match calibrator index {}",
                index_name, self.params.index
            )));
        }

        let base_date = self.params.base_date;
        let has_index_fixings = self
            .base_context
            .inflation_index_ref(self.params.curve_id.as_str())
            .is_some();
        let lag = self.effective_lag()?;
        let base_cpi = self.effective_base_cpi()?;

        let swap = InflationSwap::builder()
            .id("CALIB_ZCIS".into())
            .notional(Money::new(self.params.notional, self.params.currency))
            .start(base_date)
            .maturity(maturity)
            .fixed_rate(rate)
            .inflation_index_id(self.params.curve_id.clone())
            .discount_curve_id(self.params.discount_curve_id.clone())
            .dc(DayCount::ActAct)
            .side(PayReceiveInflation::PayFixed)
            .lag_override_opt(if has_index_fixings { None } else { Some(lag) })
            .base_cpi_opt(if has_index_fixings {
                None
            } else {
                Some(base_cpi)
            })
            .bdc_opt(None)
            .calendar_id_opt(None)
            .build()
            .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;

        // Context needs the curve being calibrated + discount curve
        let temp_context = self.base_context.clone().insert_inflation(curve.clone());

        let pv = swap.value(&temp_context, base_date)?.amount();
        Ok(pv / swap.notional.amount())
    }

    fn initial_guess(&self, quote: &Self::Quote, _previous_knots: &[(f64, f64)]) -> Result<f64> {
        let t = self.quote_time(quote)?;
        let rate = match quote {
            InflationQuote::InflationSwap { rate, .. } => *rate,
            _ => 0.02,
        };
        let base_cpi = self.effective_base_cpi()?;
        Ok(base_cpi * (1.0 + rate).powf(t))
    }
}
