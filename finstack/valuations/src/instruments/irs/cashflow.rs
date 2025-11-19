//! Cashflow construction helpers for interest rate swaps.
//!
//! This module centralizes cashflow schedule generation for `InterestRateSwap`:
//! - Fixed-leg and floating-leg `CashFlowSchedule` builders
//! - Signed dated flows used by `CashflowProvider`
//! - Combined full schedules with `CFKind` metadata
//!
//! Pricing logic (discounting, forwards, PV) lives in `pricer.rs` and consumes
//! these schedules where appropriate.

use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::Result;

use crate::cashflow::builder::{
    CashFlowSchedule, FixedCouponSpec, FloatingCouponSpec, FloatingRateSpec, Notional,
};
use crate::cashflow::traits::DatedFlows;
use crate::instruments::irs::{InterestRateSwap, PayReceive};

/// Build an unsigned fixed-leg cashflow schedule for an IRS.
///
/// The resulting schedule has positive notionals and coupon amounts; caller is
/// responsible for applying `PayReceive` sign conventions.
///
/// # Arguments
///
/// * `irs` - The interest rate swap for which to build the schedule
///
/// # Returns
///
/// A `CashFlowSchedule` containing all fixed leg cashflows with `CFKind::Fixed`
/// or `CFKind::Stub` classifications. Amounts are unsigned (positive).
///
/// # Errors
///
/// Returns an error if the cashflow schedule cannot be built (e.g., invalid
/// date ranges or calendar lookups fail).
///
/// # Examples
///
/// ```
/// use finstack_valuations::instruments::irs::{InterestRateSwap, cashflow};
///
/// # fn example() -> finstack_core::Result<()> {
/// let irs = InterestRateSwap::example()?;
/// let schedule = cashflow::fixed_leg_schedule(&irs)?;
///
/// // Schedule contains fixed coupon flows
/// assert!(!schedule.flows.is_empty());
/// # Ok(())
/// # }
/// ```
pub fn fixed_leg_schedule(irs: &InterestRateSwap) -> Result<CashFlowSchedule> {
    let mut fixed_b = CashFlowSchedule::builder();
    fixed_b
        .principal(irs.notional, irs.fixed.start, irs.fixed.end)
        .fixed_cf(FixedCouponSpec {
            coupon_type: crate::cashflow::builder::CouponType::Cash,
            rate: irs.fixed.rate,
            freq: irs.fixed.freq,
            dc: irs.fixed.dc,
            bdc: irs.fixed.bdc,
            calendar_id: irs.fixed.calendar_id.as_deref().map(String::from),
            stub: irs.fixed.stub,
        });
    fixed_b.build()
}

/// Build an unsigned floating-leg cashflow schedule for an IRS.
///
/// The schedule encodes reset dates, payment dates, and accrual metadata. The
/// amounts are unsigned; caller is responsible for applying `PayReceive`.
///
/// # Arguments
///
/// * `irs` - The interest rate swap for which to build the schedule
///
/// # Returns
///
/// A `CashFlowSchedule` containing all floating leg cashflows with `CFKind::FloatReset`
/// classifications. Amounts are unsigned (positive) and represent notional × spread
/// × accrual factor (forward rates must be added separately by the pricer).
///
/// # Errors
///
/// Returns an error if the cashflow schedule cannot be built (e.g., invalid
/// date ranges or calendar lookups fail).
///
/// # Examples
///
/// ```
/// use finstack_valuations::instruments::irs::{InterestRateSwap, cashflow};
///
/// # fn example() -> finstack_core::Result<()> {
/// let irs = InterestRateSwap::example()?;
/// let schedule = cashflow::float_leg_schedule(&irs)?;
///
/// // Schedule contains floating rate reset flows
/// assert!(!schedule.flows.is_empty());
/// # Ok(())
/// # }
/// ```
pub fn float_leg_schedule(irs: &InterestRateSwap) -> Result<CashFlowSchedule> {
    let mut float_b = CashFlowSchedule::builder();
    float_b
        .principal(irs.notional, irs.float.start, irs.float.end)
        .floating_cf(FloatingCouponSpec {
            rate_spec: FloatingRateSpec {
                index_id: irs.float.forward_curve_id.to_owned(),
                spread_bp: irs.float.spread_bp,
                gearing: 1.0,
                floor_bp: None,
                cap_bp: None,
                reset_freq: irs.float.freq,
                reset_lag_days: irs.float.reset_lag_days,
                dc: irs.float.dc,
                bdc: irs.float.bdc,
                calendar_id: irs.float.calendar_id.as_deref().map(String::from),
            },
            coupon_type: crate::cashflow::builder::CouponType::Cash,
            freq: irs.float.freq,
            stub: irs.float.stub,
        });
    float_b.build()
}

/// Build signed dated flows for an IRS, suitable for `CashflowProvider`.
///
/// Returns a vector of `(date, amount)` with signs applied according to
/// `PayReceive` and fixed/float leg direction.
///
/// # Arguments
///
/// * `irs` - The interest rate swap for which to build dated flows
///
/// # Returns
///
/// A vector of `(Date, Money)` tuples with:
/// - Fixed leg flows: positive for ReceiveFixed, negative for PayFixed
/// - Floating leg flows: negative for ReceiveFixed, positive for PayFixed
///
/// # Errors
///
/// Returns an error if either leg's cashflow schedule cannot be built.
///
/// # Examples
///
/// ```
/// use finstack_valuations::instruments::irs::{InterestRateSwap, cashflow, PayReceive};
///
/// # fn example() -> finstack_core::Result<()> {
/// let irs = InterestRateSwap::example()?;
/// let flows = cashflow::signed_dated_flows(&irs)?;
///
/// // PayFixed swap has negative fixed leg flows, positive float leg flows
/// assert!(!flows.is_empty());
/// # Ok(())
/// # }
/// ```
pub fn signed_dated_flows(irs: &InterestRateSwap) -> Result<DatedFlows> {
    let fixed_sched = fixed_leg_schedule(irs)?;
    let float_sched = float_leg_schedule(irs)?;

    let mut flows: Vec<(Date, Money)> = Vec::new();

    // Fixed leg: sign depends on PayReceive
    for cf in fixed_sched.flows {
        if cf.kind == crate::cashflow::primitives::CFKind::Fixed
            || cf.kind == crate::cashflow::primitives::CFKind::Stub
        {
            let amt = match irs.side {
                PayReceive::ReceiveFixed => cf.amount,
                PayReceive::PayFixed => cf.amount * -1.0,
            };
            flows.push((cf.date, amt));
        }
    }

    // Floating leg: opposite sign to fixed leg
    for cf in float_sched.flows {
        if cf.kind == crate::cashflow::primitives::CFKind::FloatReset {
            let amt = match irs.side {
                PayReceive::ReceiveFixed => cf.amount * -1.0,
                PayReceive::PayFixed => cf.amount,
            };
            flows.push((cf.date, amt));
        }
    }

    Ok(flows)
}

/// Build a full, signed cashflow schedule with `CFKind` metadata for an IRS.
///
/// This combines fixed and floating leg schedules, applies sign conventions,
/// and sorts flows by date and CFKind priority.
///
/// # Arguments
///
/// * `irs` - The interest rate swap for which to build the full schedule
///
/// # Returns
///
/// A `CashFlowSchedule` containing all cashflows from both legs with:
/// - Proper sign conventions applied based on `PayReceive`
/// - `CFKind` metadata preserved for each flow
/// - Flows sorted by date and kind priority
///
/// # Errors
///
/// Returns an error if either leg's cashflow schedule cannot be built.
///
/// # Examples
///
/// ```
/// use finstack_valuations::instruments::irs::{InterestRateSwap, cashflow};
///
/// # fn example() -> finstack_core::Result<()> {
/// let irs = InterestRateSwap::example()?;
/// let schedule = cashflow::full_signed_schedule(&irs)?;
///
/// // Combined schedule has flows from both fixed and floating legs
/// assert!(!schedule.flows.is_empty());
///
/// // Flows are sorted by date
/// for i in 1..schedule.flows.len() {
///     assert!(schedule.flows[i].date >= schedule.flows[i-1].date);
/// }
/// # Ok(())
/// # }
/// ```
pub fn full_signed_schedule(irs: &InterestRateSwap) -> Result<CashFlowSchedule> {
    use finstack_core::cashflow::primitives::{CFKind, CashFlow};

    let fixed_sched = fixed_leg_schedule(irs)?;
    let float_sched = float_leg_schedule(irs)?;

    // Combine flows from both legs with proper CFKind classification
    let mut all_flows: Vec<CashFlow> = Vec::new();

    // Add fixed leg flows
    for cf in fixed_sched.flows {
        if cf.kind == CFKind::Fixed || cf.kind == CFKind::Stub {
            let amt = match irs.side {
                PayReceive::ReceiveFixed => cf.amount,
                PayReceive::PayFixed => cf.amount * -1.0,
            };
            all_flows.push(CashFlow {
                date: cf.date,
                reset_date: cf.reset_date,
                amount: amt,
                kind: cf.kind, // Preserve precise CFKind
                accrual_factor: cf.accrual_factor,
                rate: cf.rate,
            });
        }
    }

    // Add floating leg flows
    for cf in float_sched.flows {
        if cf.kind == CFKind::FloatReset {
            let amt = match irs.side {
                PayReceive::ReceiveFixed => cf.amount * -1.0,
                PayReceive::PayFixed => cf.amount,
            };
            all_flows.push(CashFlow {
                date: cf.date,
                reset_date: cf.reset_date,
                amount: amt,
                kind: cf.kind, // Preserve precise CFKind
                accrual_factor: cf.accrual_factor,
                rate: cf.rate,
            });
        }
    }

    // Sort flows by date and CFKind priority
    all_flows.sort_by(|a, b| {
        use core::cmp::Ordering;
        match a.date.cmp(&b.date) {
            Ordering::Equal => {
                // Use kind ranking logic from cashflow builder
                let rank_a = match a.kind {
                    CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => 0,
                    CFKind::Fee => 1,
                    CFKind::Amortization => 2,
                    CFKind::PIK => 3,
                    CFKind::Notional => 4,
                    _ => 5,
                };
                let rank_b = match b.kind {
                    CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => 0,
                    CFKind::Fee => 1,
                    CFKind::Amortization => 2,
                    CFKind::PIK => 3,
                    CFKind::Notional => 4,
                    _ => 5,
                };
                rank_a.cmp(&rank_b)
            }
            other => other,
        }
    });

    // Create notional spec for swap (notional doesn't amortize)
    let notional = Notional::par(irs.notional.amount(), irs.notional.currency());

    Ok(CashFlowSchedule {
        flows: all_flows,
        notional,
        day_count: irs.fixed.dc, // Use fixed leg day count as representative
        meta: Default::default(),
    })
}
