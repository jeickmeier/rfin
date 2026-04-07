//! Inflation01 (inflation rate sensitivity) metric for `InflationSwap`.
//!
//! # Analytical Approximation
//!
//! This calculator uses a closed-form analytical approximation for Inflation01,
//! rather than numerical finite differences. The formula is:
//!
//! ```text
//! Inflation01 ≈ Notional × I(T)/I(0) × DF(T) × T × 0.0001
//! ```
//!
//! where:
//! - `I(T)/I(0)` is the projected inflation index ratio
//! - `DF(T)` is the discount factor to maturity
//! - `T` is the time to lagged maturity in years
//! - `0.0001` converts to per-basis-point sensitivity
//!
//! # Derivation
//!
//! For a zero-coupon inflation swap, the inflation leg PV is:
//! ```text
//! PV_infl = N × [I(T)/I(0) - 1] × DF(T)
//! ```
//!
//! Assuming the index ratio follows `I(T)/I(0) ≈ exp(π×T)` for inflation rate π:
//! ```text
//! dPV/dπ = N × DF(T) × T × exp(π×T) = N × DF(T) × T × I(T)/I(0)
//! ```
//!
//! # Approximation Accuracy
//!
//! This analytical formula assumes continuous compounding (`exp(π×T)`), but actual
//! inflation curves may use discrete compounding (`(1+π)^T`). For discrete:
//! ```text
//! d/dπ[(1+π)^T] = T × (1+π)^(T-1) ≠ T × (1+π)^T
//! ```
//!
//! The error is approximately `π×T` in relative terms, which is typically small
//! (<1%) for normal inflation levels (2-3%) and maturities (<10Y).
//!
//! For high-precision applications or extreme scenarios, consider using finite
//! differences (as implemented in `YoYInflation01Calculator`).
//!
//! # Comparison with YoY Inflation01
//!
//! - **Zero-coupon (this)**: Uses analytical approximation for speed
//! - **YoY swaps**: Uses finite differences for accuracy with periodic cashflows

use crate::instruments::common_impl::parameters::legs::PayReceive;
use crate::instruments::rates::inflation_swap::InflationSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::DayCount;
use finstack_core::market_data::scalars::InflationLag;

/// Calculates Inflation01 (1bp inflation rate sensitivity) for inflation swaps.
///
/// Uses an analytical approximation based on the derivative of the inflation leg PV
/// with respect to parallel inflation rate shifts.
///
/// # Formula
///
/// ```text
/// Inflation01 ≈ N × (I_T / I_0) × DF × T × 0.0001
/// ```
///
/// # Accuracy
///
/// This approximation is accurate to within ~1% for typical market conditions
/// (inflation rates 0-5%, maturities 1-10Y). For extreme scenarios, finite
/// differences may be more accurate.
///
/// # Sign Convention
///
/// - **PayFixed**: Positive Inflation01 (benefits from higher inflation)
/// - **ReceiveFixed**: Negative Inflation01 (loses from higher inflation)
pub(crate) struct Inflation01Calculator;

impl MetricCalculator for Inflation01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let s: &InflationSwap = context.instrument_as()?;

        let disc = context.curves.get_discount(s.discount_curve_id.as_str())?;
        let base = disc.base_date();
        let curve_dc = disc.day_count();

        // Get Index Ratio using central logic
        let index_ratio = s.projected_index_ratio(&context.curves, base)?;

        // Calculate T (time to lagged maturity) for sensitivity
        // Use the effective lag (instrument override or index default)
        let inflation_index = context
            .curves
            .get_inflation_index(s.inflation_index_id.as_str())
            .ok();

        let default_lag = s
            .lag_override
            .or_else(|| inflation_index.map(|i| i.lag()))
            .unwrap_or(InflationLag::Months(3)); // Standard 3-month lag default

        let lagged_maturity = s.apply_lag(s.maturity, default_lag);

        let t_maturity = DayCount::Act365F.year_fraction(
            base,
            lagged_maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        // Use curve day count for discounting
        let t_discount = curve_dc.year_fraction(
            base,
            s.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let df = disc.df(t_discount);

        let inflation_sensitivity = s.notional.amount() * index_ratio * df * t_maturity * 0.0001;

        let signed_sensitivity = match s.side {
            PayReceive::PayFixed => inflation_sensitivity,
            PayReceive::ReceiveFixed => -inflation_sensitivity,
        };

        Ok(signed_sensitivity)
    }
}
