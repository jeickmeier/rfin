//! Bump functionality for scenario analysis and stress testing
//!
//! This module provides functionality to apply parallel shocks and bumps
//! to market data curves, surfaces, and scalars.

extern crate alloc;
use alloc::sync::Arc;

use crate::dates::Date;
use crate::types::CurveId;
use crate::F;

use super::{
    traits::{Discount, Forward, TermStructure},
};

// -----------------------------------------------------------------------------
// Bump Specification Types
// -----------------------------------------------------------------------------

/// Mode of applying a bump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum BumpMode {
    /// Additive bump expressed in a normalized fractional form (e.g., 100bp = 0.01, 2% = 0.02).
    Additive,
    /// Multiplicative bump expressed as a factor (e.g., 1.1 = +10%, 0.9 = -10%).
    Multiplicative,
}

/// Units for the bump magnitude. These control normalization to fraction or factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum BumpUnits {
    /// Basis points for rates/spreads (100bp = 0.01).
    RateBp,
    /// Percent units (2.0 = 2%).
    Percent,
    /// Direct fraction (0.02 = 2%).
    Fraction,
    /// Direct factor (1.10 = +10%). Only valid for Multiplicative mode.
    Factor,
}

/// Unified bump specification capturing mode, units, and value.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct BumpSpec {
    /// How the bump should be applied (additive vs multiplicative).
    pub mode: BumpMode,
    /// Units the value is expressed in, controlling normalization.
    pub units: BumpUnits,
    /// Raw magnitude provided by the caller (interpreted using `units`).
    pub value: F,
}

impl BumpSpec {
    /// Create an additive bump specified in basis points (e.g., 100.0 = 100bp = 1%).
    pub fn parallel_bp(bump_bp: F) -> Self {
        Self {
            mode: BumpMode::Additive,
            units: BumpUnits::RateBp,
            value: bump_bp,
        }
    }

    /// Create a multiplicative bump given as a factor (e.g., 1.1 = +10%).
    pub fn multiplier(factor: F) -> Self {
        Self {
            mode: BumpMode::Multiplicative,
            units: BumpUnits::Factor,
            value: factor,
        }
    }

    /// Create an additive spread shift in basis points for credit curves.
    pub fn spread_shift_bp(bump_bp: F) -> Self {
        Self::parallel_bp(bump_bp)
    }

    /// Create an additive inflation shift specified in percent (e.g., 2.0 = +2%).
    pub fn inflation_shift_pct(bump_pct: F) -> Self {
        Self {
            mode: BumpMode::Additive,
            units: BumpUnits::Percent,
            value: bump_pct,
        }
    }

    /// Create an additive correlation shift specified in percent (e.g., 10.0 = +10%).
    pub fn correlation_shift_pct(bump_pct: F) -> Self {
        Self {
            mode: BumpMode::Additive,
            units: BumpUnits::Percent,
            value: bump_pct,
        }
    }

    /// If additive, return the bump as a normalized fraction (e.g., 100bp -> 0.01, 2% -> 0.02).
    pub(crate) fn additive_fraction(&self) -> Option<F> {
        if self.mode != BumpMode::Additive {
            return None;
        }
        let frac = match self.units {
            BumpUnits::RateBp => self.value / 10_000.0,
            BumpUnits::Percent => self.value / 100.0,
            BumpUnits::Fraction => self.value,
            BumpUnits::Factor => return None,
        };
        Some(frac)
    }
}

// -----------------------------------------------------------------------------
// Wrapper Curves for Bumping
// -----------------------------------------------------------------------------

/// Wrapper for a discount curve with a parallel rate bump applied.
///
/// This applies the formula: df_bumped(t) = df_original(t) * exp(-bump * t)
/// where bump is in rate units (e.g., 0.0001 for 1bp).
#[derive(Clone)]
pub struct BumpedDiscountCurve {
    pub(crate) original: Arc<dyn Discount + Send + Sync>,
    pub(crate) bump_rate: F,
    pub(crate) bumped_id: CurveId,
}

impl BumpedDiscountCurve {
    pub(crate) fn new(original: Arc<dyn Discount + Send + Sync>, bump_bp: F, bumped_id: CurveId) -> Self {
        Self {
            original,
            bump_rate: bump_bp / 10_000.0, // Convert bp to rate
            bumped_id,
        }
    }
}

impl TermStructure for BumpedDiscountCurve {
    fn id(&self) -> &CurveId {
        &self.bumped_id
    }
}

impl Discount for BumpedDiscountCurve {
    #[inline]
    fn base_date(&self) -> Date {
        self.original.base_date()
    }

    #[inline]
    fn df(&self, t: F) -> F {
        let original_df = self.original.df(t);
        original_df * (-self.bump_rate * t).exp()
    }
}

/// Wrapper for a forward curve with a parallel rate bump applied.
#[derive(Clone)]
pub struct BumpedForwardCurve {
    pub(crate) original: Arc<dyn Forward + Send + Sync>,
    pub(crate) bump_rate: F,
    pub(crate) bumped_id: CurveId,
}

impl BumpedForwardCurve {
    pub(crate) fn new(original: Arc<dyn Forward + Send + Sync>, bump_bp: F, bumped_id: CurveId) -> Self {
        Self {
            original,
            bump_rate: bump_bp / 10_000.0, // Convert bp to rate
            bumped_id,
        }
    }
}

impl TermStructure for BumpedForwardCurve {
    fn id(&self) -> &CurveId {
        &self.bumped_id
    }
}

impl Forward for BumpedForwardCurve {
    #[inline]
    fn rate(&self, t: F) -> F {
        self.original.rate(t) + self.bump_rate
    }
}

// -----------------------------------------------------------------------------
// ID formatting helpers
// -----------------------------------------------------------------------------

#[inline]
pub(crate) fn id_bump_bp(id: &str, bp: F) -> CurveId {
    CurveId::new(format!("{}_bump_{:.0}bp", id, bp))
}

#[inline]
pub(crate) fn id_spread_bp(id: &str, bp: F) -> CurveId {
    CurveId::new(format!("{}_spread_{:.0}bp", id, bp))
}
