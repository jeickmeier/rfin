//! Cashflow emission helpers.
//!
//! This module contains functions that emit cashflows on specific dates based on
//! coupon schedules, amortization specs, and fee specifications. These functions
//! are called by the build pipeline to generate deterministic cashflow sequences.
//!
//! ## Responsibilities
//!
//! - Emit fixed coupon cashflows with PIK capitalization
//! - Emit floating coupon cashflows with forward rate lookups
//! - Emit amortization payments according to various schedules
//! - Emit periodic and fixed fee cashflows
//! - Emit credit event cashflows (defaults, prepayments, recoveries)
//! - Track outstanding balances through PIK and amortization
//!
//! ## Organization
//!
//! This module is organized into submodules by emission type:
//! - `helpers`: Common helper functions for PIK flows and reset date calculation
//! - `coupons`: Fixed and floating coupon emission
//! - `amortization`: Principal amortization emission
//! - `fees`: Periodic, commitment, usage, and facility fee emission
//! - `credit`: Default, prepayment, and recovery emission
//! - `tests`: Comprehensive test suite for emission functions
//!
//! ## Design
//!
//! Each `emit_*_on` function takes a date and relevant schedules, computes the
//! appropriate cashflows for that date, and returns both the flows and any PIK
//! amount that should capitalize into the outstanding balance.

use crate::primitives::CashFlow;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;

mod amortization;
pub(crate) mod coupons;
pub(crate) mod credit;
mod fees;
mod helpers;

// ---------------------------------------------------------------------------
// Shared f64 ↔ Decimal conversion helpers
//
// These are accessible to all submodules (coupons, fees, etc.) via `super::`.
// They are deliberately kept private to this module cluster — callers outside
// the `emission` module should never need to construct Decimals from raw f64s.
// ---------------------------------------------------------------------------

/// Convert an f64 to [`Decimal`], returning an error for non-finite values.
///
/// This prevents silent masking of NaN/Infinity values as zero, which would
/// result in zero cashflows instead of a proper error indicating data corruption.
///
/// Used by both coupon and fee emission to ensure consistent, audit-visible
/// handling of degenerate floating-point inputs.
fn f64_to_decimal(value: f64, _context: &str) -> finstack_core::Result<rust_decimal::Decimal> {
    use finstack_core::{InputError, NonFiniteKind};
    if value.is_nan() {
        return Err(InputError::NonFiniteValue {
            kind: NonFiniteKind::NaN,
        }
        .into());
    }
    if value.is_infinite() {
        let kind = if value.is_sign_positive() {
            NonFiniteKind::PosInfinity
        } else {
            NonFiniteKind::NegInfinity
        };
        return Err(InputError::NonFiniteValue { kind }.into());
    }
    rust_decimal::Decimal::try_from(value)
        .map_err(|_| finstack_core::Error::from(InputError::ConversionOverflow))
}

/// Convert [`Decimal`] to f64, returning an error if conversion fails.
///
/// While `Decimal` values are always finite, conversion to f64 can fail for
/// very large values that exceed f64's representable range (~1.8 × 10^308).
fn decimal_to_f64(value: rust_decimal::Decimal, _context: &str) -> finstack_core::Result<f64> {
    use rust_decimal::prelude::ToPrimitive;
    value
        .to_f64()
        .ok_or_else(|| finstack_core::Error::from(finstack_core::InputError::ConversionOverflow))
}

/// Parameters for emitting revolving-credit fee cashflows for one accrual period.
#[derive(Debug, Clone, Copy)]
pub struct RevolvingFeeEmissionConfig {
    /// Payment date for all emitted fee cashflows.
    pub payment_date: Date,
    /// Drawn balance used as the base for usage fees.
    pub drawn_balance: f64,
    /// Undrawn balance used as the base for commitment fees.
    pub undrawn_balance: f64,
    /// Total commitment amount used as the base for facility fees.
    pub commitment_amount: f64,
    /// Commitment fee quote in basis points.
    pub commitment_fee_bp: f64,
    /// Usage fee quote in basis points.
    pub usage_fee_bp: f64,
    /// Facility fee quote in basis points.
    pub facility_fee_bp: f64,
    /// Accrual factor for the period, expressed in years.
    pub year_fraction: f64,
    /// Currency applied to all emitted fee cashflows.
    pub currency: Currency,
}

/// Emit all revolving-credit fee cashflows for a single accrual period.
pub fn emit_revolving_credit_fees(flows: &mut Vec<CashFlow>, cfg: &RevolvingFeeEmissionConfig) {
    if let Some(cf) = fees::emit_commitment_fee_on(
        cfg.payment_date,
        cfg.undrawn_balance,
        cfg.commitment_fee_bp,
        cfg.year_fraction,
        cfg.currency,
    ) {
        flows.push(cf);
    }

    if let Some(cf) = fees::emit_usage_fee_on(
        cfg.payment_date,
        cfg.drawn_balance,
        cfg.usage_fee_bp,
        cfg.year_fraction,
        cfg.currency,
    ) {
        flows.push(cf);
    }

    if let Some(cf) = fees::emit_facility_fee_on(
        cfg.payment_date,
        cfg.commitment_amount,
        cfg.facility_fee_bp,
        cfg.year_fraction,
        cfg.currency,
    ) {
        flows.push(cf);
    }
}

// Re-export coupon emission (internal to builder module)
pub(crate) use coupons::{emit_fixed_coupons_on, emit_float_coupons_on};

// Re-export amortization emission and types (internal to builder module)
pub(super) use amortization::{emit_amortization_on, AmortizationParams};

// Re-export fee emission (internal to builder module)
pub(super) use fees::emit_fees_on;

// Re-export helper utilities (internal to builder module)
pub(super) use helpers::compute_reset_date;

// Re-export inflation coupon emission for inflation-linked instruments.
pub use coupons::emit_inflation_coupons;

// Re-export credit event emission (used by credit model tests)
pub use credit::{emit_default_on, emit_prepayment_on};

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod revolving_credit_fee_tests {
    use super::{emit_revolving_credit_fees, RevolvingFeeEmissionConfig};
    use crate::primitives::CFKind;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use time::Month;

    #[test]
    fn emits_all_non_zero_revolving_fee_kinds() {
        let payment_date = Date::from_calendar_date(2025, Month::March, 31).expect("valid date");
        let mut flows = Vec::new();

        emit_revolving_credit_fees(
            &mut flows,
            &RevolvingFeeEmissionConfig {
                payment_date,
                drawn_balance: 400_000.0,
                undrawn_balance: 600_000.0,
                commitment_amount: 1_000_000.0,
                commitment_fee_bp: 25.0,
                usage_fee_bp: 15.0,
                facility_fee_bp: 10.0,
                year_fraction: 0.25,
                currency: Currency::USD,
            },
        );

        assert_eq!(flows.len(), 3);
        assert_eq!(flows[0].kind, CFKind::CommitmentFee);
        assert_eq!(flows[1].kind, CFKind::UsageFee);
        assert_eq!(flows[2].kind, CFKind::FacilityFee);
    }
}
