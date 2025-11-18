//! Floating leg present value calculation for interest rate swaps.
//!
//! Computes the present value of all future floating rate payments by
//! projecting forward rates from the forward curve and discounting.
//!
//! # Calculation
//!
//! ## Term-Rate Swaps (LIBOR-style)
//!
//! ```text
//! PV_float = Σ Notional × (Forward_i + Spread) × α_i × DF(T_i)
//! ```
//!
//! where:
//! - `Forward_i` = forward rate for period i from the forward curve
//! - `Spread` = quoted spread in basis points
//! - `α_i` = accrual factor for period i
//! - `DF(T_i)` = discount factor to payment date i
//!
//! ## OIS/Overnight Swaps (RFR-style)
//!
//! For swaps with compounded-in-arrears floating legs, uses the
//! discount-only identity:
//! ```text
//! PV_float = Notional × (DF(start) - DF(end)) + Spread_Annuity
//! ```
//!
//! This is exact when the forward curve matches the discount curve.
//!
//! # References
//!
//! - **ISDA 2006 Definitions**: Sections 4.1-4.2 (term rates)
//! - **ISDA 2021 Definitions**: Section 4.5 (compounded RFR)

use crate::instruments::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext};

/// Present value calculator for the floating leg of an interest rate swap.
///
/// Projects forward rates and discounts floating coupon payments. Automatically
/// detects OIS swaps (overnight compounding) and uses the appropriate pricing method.
pub struct FloatLegPvCalculator;

impl MetricCalculator for FloatLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let irs: &InterestRateSwap = context.instrument_as()?;
        let as_of = context.as_of;

        // Use the same discount curve as the main IRS pricer (fixed-leg curve)
        let disc = context.curves.get_discount(&irs.fixed.discount_curve_id)?;

        let pv_money = if irs.is_ois() {
            // OIS / compounded RFR swap: reuse discount-only helper for consistency with npv()
            irs.pv_compounded_float_leg(&disc, as_of)?
        } else {
            // Non-OIS swap: requires forward curve for float leg pricing
            let fwd = context.curves.get_forward(&irs.float.forward_curve_id)?;
            irs.pv_float_leg(&disc, fwd.as_ref(), as_of)?
        };

        Ok(pv_money.amount())
    }
}
