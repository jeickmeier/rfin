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

