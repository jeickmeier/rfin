//! Jump-to-Default metric for single-name CDS.
//!
//! Calculates the instantaneous P&L if the reference entity defaults immediately.
//! This is a key risk metric that measures the immediate impact of a credit event.
//!
//! ## Full JTD Formula (with accrued premium)
//! ```text
//! JTD = signed(LGD × Notional) ∓ signed(Accrued Premium)
//! ```
//!
//! Where:
//! - **LGD** = 1 - Recovery Rate (Loss Given Default)
//! - **Accrued Premium** = Premium accrued from last coupon date to default
//!
//! ## Interpretation
//! - For protection **buyer** (PayFixed):
//!   - Receives: LGD × Notional (protection payout)
//!   - Pays: Accrued premium (payable on default per ISDA)
//!   - Net JTD = LGD × Notional - Accrued Premium (positive = gain)
//!
//! - For protection **seller** (ReceiveFixed):
//!   - Pays: LGD × Notional (protection payout)
//!   - Receives: Accrued premium
//!   - Net JTD = Accrued Premium - LGD × Notional (negative = loss)
//!
//! ## Note on Accrued Premium
//!
//! Under ISDA standard documentation, accrued premium is payable upon default
//! (unlike bond coupons which may be forgiven). This calculator includes the
//! accrued premium in the JTD to give a more accurate P&L impact.

use crate::instruments::cds::CDSPricer;
use crate::instruments::cds::{CreditDefaultSwap, PayReceive};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::DayCountCtx;
use finstack_core::Result;
use rust_decimal::prelude::ToPrimitive;

/// Jump-to-default calculator for single-name CDS (includes accrued premium).
pub struct JumpToDefaultCalculator;

impl MetricCalculator for JumpToDefaultCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let as_of = context.as_of;

        // Loss given default (protection payout)
        let lgd = 1.0 - cds.protection.recovery_rate;
        let protection_payout = cds.notional.amount() * lgd;

        // Calculate accrued premium from last coupon date to as_of
        let accrued_premium = calculate_accrued_premium(cds, as_of)?;

        // Apply sign based on position:
        // - Protection buyer: receives protection, pays accrued → JTD = protection - accrued
        // - Protection seller: pays protection, receives accrued → JTD = accrued - protection
        let signed_jtd = match cds.side {
            PayReceive::PayFixed => protection_payout - accrued_premium,
            PayReceive::ReceiveFixed => accrued_premium - protection_payout,
        };

        Ok(signed_jtd)
    }
}

/// Jump-to-default calculator (LGD only, excludes accrued premium).
///
/// This simplified version only considers the protection leg payout.
/// Use `JumpToDefaultCalculator` for a more complete P&L impact.
pub struct JumpToDefaultLgdOnlyCalculator;

impl MetricCalculator for JumpToDefaultLgdOnlyCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;

        // Loss given default
        let lgd = 1.0 - cds.protection.recovery_rate;

        // Jump-to-default amount (unsigned)
        let jtd_amount = cds.notional.amount() * lgd;

        // Apply sign based on position
        let signed_jtd = match cds.side {
            PayReceive::PayFixed => jtd_amount,      // Buyer gains
            PayReceive::ReceiveFixed => -jtd_amount, // Seller loses
        };

        Ok(signed_jtd)
    }
}

/// Calculate accrued premium from the last coupon date to the given date.
fn calculate_accrued_premium(
    cds: &CreditDefaultSwap,
    as_of: finstack_core::dates::Date,
) -> Result<f64> {
    // Find the last scheduled coupon date before as_of, respecting:
    // - IMM dates if configured by the pricer (default)
    // - Business day adjustments via `calendar_id`
    // - Stubs via the instrument spec
    let premium_start = cds.premium.start;
    let premium_end = cds.premium.end;

    if as_of <= premium_start || as_of >= premium_end {
        return Ok(0.0);
    }

    let pricer = CDSPricer::new();
    let schedule = pricer.generate_schedule(cds, as_of)?;
    if schedule.is_empty() {
        return Ok(0.0);
    }

    // Find the most recent date in schedule <= as_of (should exist since schedule[0] = start)
    let mut last_coupon = schedule[0];
    for &d in &schedule {
        if d <= as_of {
            last_coupon = d;
        } else {
            break;
        }
    }

    // Calculate accrual fraction from last_coupon to as_of
    let accrual_fraction =
        cds.premium
            .dc
            .year_fraction(last_coupon, as_of, DayCountCtx::default())?;

    // Spread in decimal
    let spread = cds.premium.spread_bp.to_f64().unwrap_or(0.0) / 10_000.0;

    // Accrued premium = Notional × Spread × Accrual Fraction
    let accrued = cds.notional.amount() * spread * accrual_fraction;

    Ok(accrued)
}
