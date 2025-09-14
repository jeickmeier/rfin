//! Bump functionality for scenario analysis and stress testing
//!
//! This module provides functionality to apply parallel shocks and bumps
//! to market data curves, surfaces, and scalars.

use crate::types::CurveId;
use crate::F;
use super::term_structures::{
    discount_curve::DiscountCurve,
    forward_curve::ForwardCurve,
    hazard_curve::HazardCurve,
    inflation::InflationCurve,
    base_correlation::BaseCorrelationCurve,
};
use super::traits::TermStructure;

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

#[inline]
pub(crate) fn id_bump_pct(id: &str, pct: F) -> CurveId {
    CurveId::new(format!("{}_bump_{:.0}pct", id, pct))
}

// -----------------------------------------------------------------------------
// Curve bump helpers (centralized)
// -----------------------------------------------------------------------------

/// Apply a bump to a `DiscountCurve`. Currently supports additive rate shifts in bp.
pub fn bump_discount_curve(curve: &DiscountCurve, spec: BumpSpec) -> Option<DiscountCurve> {
    if spec.mode == BumpMode::Additive && spec.units == BumpUnits::RateBp {
        Some(curve.with_parallel_bump(spec.value))
    } else {
        None
    }
}

/// Apply a bump to a `ForwardCurve`.
/// - Additive: units=RateBp/Fraction/Percent apply as simple additive to forward rates
/// - Multiplicative: units=Factor scales all forward rates
pub fn bump_forward_curve(curve: &ForwardCurve, spec: BumpSpec) -> Option<ForwardCurve> {
    // Determine transformation of forward rates
    let transform: Box<dyn Fn(F) -> F> = match (spec.mode, spec.units) {
        (BumpMode::Additive, BumpUnits::RateBp | BumpUnits::Fraction) => {
            let bump = spec.additive_fraction()?;
            Box::new(move |r| r + bump)
        }
        (BumpMode::Additive, BumpUnits::Percent) => {
            let bump = spec.additive_fraction()?; // percent -> fraction
            Box::new(move |r| r + bump)
        }
        (BumpMode::Multiplicative, BumpUnits::Factor) => {
            let factor = spec.value;
            Box::new(move |r| r * factor)
        }
        _ => return None,
    };

    let bumped_id = match spec.mode {
        BumpMode::Additive => match spec.units {
            BumpUnits::RateBp => id_bump_bp(curve.id().as_str(), spec.value),
            BumpUnits::Percent => id_bump_pct(curve.id().as_str(), spec.value),
            BumpUnits::Fraction => CurveId::new(format!("{}_bump_frac_{:.4}", curve.id(), spec.value)),
            BumpUnits::Factor => CurveId::new(format!("{}_bump_factor_{:.4}", curve.id(), spec.value)),
        },
        BumpMode::Multiplicative => CurveId::new(format!("{}_bump_factor_{:.4}", curve.id(), spec.value)),
    };

    let bumped_rates: Vec<(F, F)> = curve
        .knots()
        .iter()
        .copied()
        .zip(curve.forwards().iter().copied())
        .map(|(t, r)| (t, transform(r)))
        .collect();

    // Preserve base date, reset lag, day count; interpolation style defaults
    // (original style is not publicly exposed)
    ForwardCurve::builder(bumped_id, curve.tenor())
        .base_date(curve.base_date())
        .reset_lag(curve.reset_lag())
        .day_count(curve.day_count())
        .knots(bumped_rates)
        .build()
        .ok()
}

/// Apply a bump to a `HazardCurve`.
/// - Additive: units=RateBp/Fraction/Percent adds to hazard rate λ(t) and clamps at 0.
pub fn bump_hazard_curve(curve: &HazardCurve, spec: BumpSpec) -> Option<HazardCurve> {
    let shift = match (spec.mode, spec.units) {
        (BumpMode::Additive, BumpUnits::RateBp | BumpUnits::Fraction | BumpUnits::Percent) => {
            spec.additive_fraction()?
        }
        _ => return None,
    };

    let bumped_id = match spec.units {
        BumpUnits::RateBp => id_spread_bp(curve.id().as_str(), spec.value),
        BumpUnits::Percent => id_bump_pct(curve.id().as_str(), spec.value),
        BumpUnits::Fraction => CurveId::new(format!("{}_shift_{:.4}", curve.id(), spec.value)),
        BumpUnits::Factor => CurveId::new(format!("{}_shift_factor_{:.4}", curve.id(), spec.value)),
    };

    let shifted_points: Vec<(F, F)> = curve
        .knot_points()
        .map(|(t, lambda)| (t, (lambda + shift).max(0.0)))
        .collect();

    // Rebuild a proper curve with the bumped ID, preserving key metadata
    HazardCurve::builder(bumped_id)
        .base_date(curve.base_date())
        .recovery_rate(curve.recovery_rate())
        .day_count(curve.day_count())
        .knots(shifted_points)
        .par_spreads(curve.par_spread_points())
        .build()
        .ok()
}

/// Apply a bump to an `InflationCurve`.
/// - Additive Percent/Fraction: scales CPI levels by (1 + shift)
/// - Multiplicative Factor: scales CPI levels by `factor`
pub fn bump_inflation_curve(curve: &InflationCurve, spec: BumpSpec) -> Option<InflationCurve> {
    let factor = match (spec.mode, spec.units) {
        (BumpMode::Additive, BumpUnits::Percent | BumpUnits::Fraction) => 1.0 + spec.additive_fraction()?,
        (BumpMode::Multiplicative, BumpUnits::Factor) => spec.value,
        _ => return None,
    };

    let bumped_id = match spec.mode {
        BumpMode::Additive => match spec.units {
            BumpUnits::Percent => id_bump_pct(curve.id().as_str(), spec.value),
            BumpUnits::Fraction => CurveId::new(format!("{}_bump_frac_{:.4}", curve.id(), spec.value)),
            BumpUnits::RateBp => id_bump_bp(curve.id().as_str(), spec.value),
            BumpUnits::Factor => CurveId::new(format!("{}_bump_factor_{:.4}", curve.id(), spec.value)),
        },
        BumpMode::Multiplicative => CurveId::new(format!("{}_bump_factor_{:.4}", curve.id(), spec.value)),
    };

    let bumped_points: Vec<(F, F)> = curve
        .knots()
        .iter()
        .copied()
        .zip(curve.cpi_levels().iter().copied())
        .map(|(t, cpi)| (t, cpi * factor))
        .collect();

    InflationCurve::builder(bumped_id)
        .base_cpi(curve.base_cpi())
        .knots(bumped_points)
        .build()
        .ok()
}

/// Apply a bump to a `BaseCorrelationCurve`.
/// - Additive Percent/Fraction: adds to correlation; clamped to [0,1]
/// - Multiplicative Factor: scales correlation; clamped to [0,1]
pub fn bump_base_correlation_curve(curve: &BaseCorrelationCurve, spec: BumpSpec) -> Option<BaseCorrelationCurve> {
    let (add, mul) = match (spec.mode, spec.units) {
        (BumpMode::Additive, BumpUnits::Percent | BumpUnits::Fraction) => (spec.additive_fraction()?, 1.0),
        (BumpMode::Multiplicative, BumpUnits::Factor) => (0.0, spec.value),
        _ => return None,
    };

    let bumped_id = match spec.mode {
        BumpMode::Additive => match spec.units {
            BumpUnits::Percent => id_bump_pct(curve.id().as_str(), spec.value),
            BumpUnits::Fraction => CurveId::new(format!("{}_bump_frac_{:.4}", curve.id(), spec.value)),
            BumpUnits::RateBp => id_bump_bp(curve.id().as_str(), spec.value),
            BumpUnits::Factor => CurveId::new(format!("{}_bump_factor_{:.4}", curve.id(), spec.value)),
        },
        BumpMode::Multiplicative => CurveId::new(format!("{}_bump_factor_{:.4}", curve.id(), spec.value)),
    };

    let bumped_points: Vec<(F, F)> = curve
        .detachment_points()
        .iter()
        .copied()
        .zip(curve.correlations().iter().copied())
        .map(|(d, c)| (d, (c * mul + add).clamp(0.0, 1.0)))
        .collect();

    BaseCorrelationCurve::builder(bumped_id)
        .points(bumped_points)
        .build()
        .ok()
}
