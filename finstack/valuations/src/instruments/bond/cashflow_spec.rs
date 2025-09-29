//! Simplified cashflow specification for bonds.
//!
//! This module provides a cleaner API for specifying bond cashflows by making
//! mutually exclusive options explicit through an enum instead of multiple
//! optional fields that could conflict.

use crate::cashflow::builder::CashFlowSchedule;
use crate::cashflow::primitives::AmortizationSpec;
use finstack_core::dates::{DayCount, Frequency};
use finstack_core::types::CurveId;


/// Specification for how bond cashflows should be generated.
///
/// This enum makes it clear which cashflow generation methods are mutually exclusive,
/// eliminating confusion about which fields should be set together.
#[derive(Clone, Debug)]
pub enum CashflowSpec {
    /// Standard fixed-rate bond with regular coupon payments.
    Fixed {
        /// Annual coupon rate (e.g., 0.05 for 5%).
        coupon: f64,
        /// Payment frequency.
        freq: Frequency,
        /// Day count convention for accrual.
        dc: DayCount,
    },

    /// Floating-rate note with index-linked coupons.
    Floating {
        /// Forward curve identifier for the floating index.
        index_id: CurveId,
        /// Margin over the index in basis points.
        margin_bp: f64,
        /// Gearing multiplier on the index rate.
        gearing: f64,
        /// Reset lag in days.
        reset_lag_days: i32,
        /// Payment frequency.
        freq: Frequency,
        /// Day count convention for accrual.
        dc: DayCount,
    },

    /// User-provided custom cashflow schedule.
    Custom(CashFlowSchedule),

    /// Amortizing bond (principal payments during life).
    Amortizing {
        /// Base cashflow specification (fixed or floating).
        base: Box<CashflowSpec>,
        /// Amortization schedule.
        schedule: AmortizationSpec,
    },
}

impl CashflowSpec {
    /// Create a standard fixed-rate specification.
    pub fn fixed(coupon: f64, freq: Frequency, dc: DayCount) -> Self {
        Self::Fixed { coupon, freq, dc }
    }

    /// Create a floating-rate specification.
    pub fn floating(index_id: CurveId, margin_bp: f64, freq: Frequency, dc: DayCount) -> Self {
        Self::Floating {
            index_id,
            margin_bp,
            gearing: 1.0,
            reset_lag_days: 2,
            freq,
            dc,
        }
    }

    /// Create a custom cashflow specification.
    pub fn custom(schedule: CashFlowSchedule) -> Self {
        Self::Custom(schedule)
    }

    /// Create an amortizing bond specification.
    pub fn amortizing(base: CashflowSpec, schedule: AmortizationSpec) -> Self {
        Self::Amortizing {
            base: Box::new(base),
            schedule,
        }
    }

    /// Get the payment frequency from this specification.
    pub fn frequency(&self) -> Option<Frequency> {
        match self {
            Self::Fixed { freq, .. } => Some(*freq),
            Self::Floating { freq, .. } => Some(*freq),
            Self::Custom(_) => None, // Custom schedules don't have regular frequency
            Self::Amortizing { base, .. } => base.frequency(),
        }
    }

    /// Get the day count convention from this specification.
    pub fn day_count(&self) -> Option<DayCount> {
        match self {
            Self::Fixed { dc, .. } => Some(*dc),
            Self::Floating { dc, .. } => Some(*dc),
            Self::Custom(_) => None, // Custom schedules may have mixed day counts
            Self::Amortizing { base, .. } => base.day_count(),
        }
    }
}

impl Default for CashflowSpec {
    /// Default to semi-annual fixed bond with 30/360 day count.
    fn default() -> Self {
        Self::Fixed {
            coupon: 0.0,
            freq: Frequency::semi_annual(),
            dc: DayCount::Thirty360,
        }
    }
}
