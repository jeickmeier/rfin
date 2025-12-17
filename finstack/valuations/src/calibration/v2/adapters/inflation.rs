use crate::calibration::v2::api::schema::InflationCurveParams;
use crate::calibration::v2::domain::quotes::InflationQuote;
use crate::calibration::v2::domain::solver::BootstrapTarget;
use crate::instruments::common::traits::Instrument;
use crate::instruments::inflation_swap::{InflationSwap, PayReceiveInflation};
use finstack_core::dates::{DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::InflationCurve;
use finstack_core::money::Money;
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
}

impl BootstrapTarget for InflationBootstrapper {
    type Quote = InflationQuote;
    type Curve = InflationCurve;

    fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
        let maturity = quote.maturity_date().ok_or(finstack_core::Error::Input(
            finstack_core::error::InputError::Invalid,
        ))?;
        // Use ActAct for time axis, matching v1 default
        DayCount::ActAct.year_fraction(self.params.base_date, maturity, DayCountCtx::default())
    }

    fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        // knots are (time, cpi)
        // Ensure base point (0.0, base_cpi) is included or added
        let mut full_knots = knots.to_vec();
        // Check if 0.0 exists
        if !full_knots.iter().any(|(t, _)| t.abs() < 1e-8) {
            full_knots.insert(0, (0.0, self.params.base_cpi));
        }
        full_knots.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .expect("Time values should be comparable")
        });

        InflationCurve::builder(self.params.curve_id.to_string())
            .base_cpi(self.params.base_cpi)
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

        if index_name != &self.params.index {
            return Err(finstack_core::Error::Validation(format!(
                "Quote index {} does not match calibrator index {}",
                index_name, self.params.index
            )));
        }

        let base_date = self.params.base_date;
        let swap = InflationSwap::builder()
            .id("CALIB_ZCIS".into())
            .notional(Money::new(1_000_000.0, self.params.currency))
            .start(base_date)
            .maturity(maturity)
            .fixed_rate(rate)
            .inflation_index_id(self.params.index.clone().into())
            .discount_curve_id(self.params.discount_curve_id.clone())
            .dc(DayCount::ActAct)
            .side(PayReceiveInflation::PayFixed)
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
        Ok(self.params.base_cpi * (1.0 + rate).powf(t))
    }
}
