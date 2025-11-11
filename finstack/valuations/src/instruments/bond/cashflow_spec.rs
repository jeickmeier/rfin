//! Thin facade for bond cashflow specification.
//!
//! This module provides a clean, ergonomic API for bonds by wrapping the canonical
//! builder coupon specs (`FixedCouponSpec`, `FloatingCouponSpec`) with convenience
//! constructors that apply sensible defaults.

use crate::cashflow::builder::specs::{
    CouponType, FixedCouponSpec, FloatingCouponSpec, FloatingRateSpec,
};
use crate::cashflow::builder::AmortizationSpec;
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
use finstack_core::types::CurveId;

/// Thin facade over canonical builder coupon specs for bond cashflows.
///
/// Wraps `FixedCouponSpec` and `FloatingCouponSpec` from the cashflow builder,
/// providing convenience constructors with sensible defaults for common bond use cases.
/// This ensures parity with all builder features (floors/caps, BDC, calendars, PIK, etc.)
/// while keeping the bond API simple.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CashflowSpec {
    /// Fixed-rate bond using the canonical `FixedCouponSpec`.
    Fixed(FixedCouponSpec),

    /// Floating-rate note using the canonical `FloatingCouponSpec`.
    Floating(FloatingCouponSpec),

    /// Amortizing bond (principal payments during life).
    Amortizing {
        /// Base cashflow specification (fixed or floating).
        base: Box<CashflowSpec>,
        /// Amortization schedule.
        schedule: AmortizationSpec,
    },
}

impl CashflowSpec {
    /// Create a fixed-rate specification with sensible defaults.
    ///
    /// Defaults:
    /// - `coupon_type`: Cash (100% cash payment)
    /// - `bdc`: Following
    /// - `stub`: None
    /// - `calendar_id`: None
    ///
    /// For full control, construct `FixedCouponSpec` directly and wrap in `CashflowSpec::Fixed(...)`.
    pub fn fixed(coupon: f64, freq: Frequency, dc: DayCount) -> Self {
        Self::Fixed(FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: coupon,
            freq,
            dc,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        })
    }

    /// Create a floating-rate specification with sensible defaults.
    ///
    /// Defaults:
    /// - `coupon_type`: Cash (100% cash payment)
    /// - `gearing`: 1.0
    /// - `reset_lag_days`: 2 (T-2 convention)
    /// - `floor_bp`: None
    /// - `cap_bp`: None
    /// - `reset_freq`: Same as payment frequency
    /// - `bdc`: Following
    /// - `stub`: None
    /// - `calendar_id`: None
    ///
    /// For full control (floors/caps/gearing), construct `FloatingCouponSpec` directly
    /// and wrap in `CashflowSpec::Floating(...)`.
    pub fn floating(index_id: CurveId, margin_bp: f64, freq: Frequency, dc: DayCount) -> Self {
        Self::Floating(FloatingCouponSpec {
            rate_spec: FloatingRateSpec {
                index_id,
                spread_bp: margin_bp,
                gearing: 1.0,
                floor_bp: None,
                cap_bp: None,
                reset_freq: freq,
                reset_lag_days: 2,
                dc,
                bdc: BusinessDayConvention::Following,
                calendar_id: None,
            },
            coupon_type: CouponType::Cash,
            freq,
            stub: StubKind::None,
        })
    }

    /// Create an amortizing bond specification.
    ///
    /// The base spec (fixed or floating) determines the coupon payments,
    /// while the amortization schedule specifies principal reductions.
    pub fn amortizing(base: CashflowSpec, schedule: AmortizationSpec) -> Self {
        Self::Amortizing {
            base: Box::new(base),
            schedule,
        }
    }

    /// Get the payment frequency from this specification.
    pub fn frequency(&self) -> Frequency {
        match self {
            Self::Fixed(spec) => spec.freq,
            Self::Floating(spec) => spec.freq,
            Self::Amortizing { base, .. } => base.frequency(),
        }
    }

    /// Get the day count convention from this specification.
    pub fn day_count(&self) -> DayCount {
        match self {
            Self::Fixed(spec) => spec.dc,
            Self::Floating(spec) => spec.rate_spec.dc,
            Self::Amortizing { base, .. } => base.day_count(),
        }
    }
}

impl Default for CashflowSpec {
    /// Default to semi-annual fixed bond with 30/360 day count (US convention).
    fn default() -> Self {
        Self::fixed(0.0, Frequency::semi_annual(), DayCount::Thirty360)
    }
}
