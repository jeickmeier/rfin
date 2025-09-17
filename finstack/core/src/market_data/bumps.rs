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

// -----------------------------------------------------------------------------
// Bump Specification Types
// -----------------------------------------------------------------------------

/// Mode of applying a bump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
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
#[non_exhaustive]
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
///
/// # Examples
/// ```rust
/// use finstack_core::market_data::bumps::{BumpSpec, BumpMode, BumpUnits};
///
/// let additive = BumpSpec { mode: BumpMode::Additive, units: BumpUnits::RateBp, value: 15.0 };
/// assert_eq!(additive.mode, BumpMode::Additive);
/// assert_eq!(additive.units, BumpUnits::RateBp);
///
/// let multiplicative = BumpSpec::multiplier(1.05);
/// assert_eq!(multiplicative.mode, BumpMode::Multiplicative);
/// ```
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
// Bumpable trait for generic bump operations
// -----------------------------------------------------------------------------

/// Trait for types that can be bumped with a BumpSpec.
pub trait Bumpable: Sized {
    /// Apply a bump specification to create a new bumped instance.
    fn apply_bump(&self, spec: BumpSpec) -> Option<Self>;
}

/// Generic function to bump any curve that implements [`Bumpable`].
///
/// # Examples
/// ```rust
/// use finstack_core::market_data::bumps::{bump_curve, BumpSpec};
/// use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
/// use finstack_core::market_data::term_structures::CurveBuilder;
/// use finstack_core::math::interp::InterpStyle;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// let curve = DiscountCurve::builder("USD-OIS")
///     .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
///    .knots([(0.0, 1.0), (1.0, 0.99)])
///     .set_interp(InterpStyle::Linear)
///     .build()
///     .unwrap();
/// let bumped = bump_curve(&curve, BumpSpec::parallel_bp(25.0)).unwrap();
/// assert!(bumped.df(1.0) < curve.df(1.0));
/// ```
pub fn bump_curve<T: Bumpable>(curve: &T, spec: BumpSpec) -> Option<T> {
    curve.apply_bump(spec)
}

// -----------------------------------------------------------------------------
// Bumpable implementations for each curve type
// -----------------------------------------------------------------------------

impl Bumpable for DiscountCurve {
    fn apply_bump(&self, spec: BumpSpec) -> Option<Self> {
        if spec.mode == BumpMode::Additive && spec.units == BumpUnits::RateBp {
            Some(self.with_parallel_bump(spec.value))
        } else {
            None
        }
    }
}

impl Bumpable for ForwardCurve {
    fn apply_bump(&self, spec: BumpSpec) -> Option<Self> {
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
                BumpUnits::RateBp => id_bump_bp(self.id().as_str(), spec.value),
                BumpUnits::Percent => id_bump_pct(self.id().as_str(), spec.value),
                BumpUnits::Fraction => CurveId::new(format!("{}_bump_frac_{:.4}", self.id(), spec.value)),
                BumpUnits::Factor => CurveId::new(format!("{}_bump_factor_{:.4}", self.id(), spec.value)),
            },
            BumpMode::Multiplicative => CurveId::new(format!("{}_bump_factor_{:.4}", self.id(), spec.value)),
        };

        let bumped_rates: Vec<(F, F)> = self
            .knots()
            .iter()
            .copied()
            .zip(self.forwards().iter().copied())
            .map(|(t, r)| (t, transform(r)))
            .collect();

        // Preserve base date, reset lag, day count; interpolation style defaults
        // (original style is not publicly exposed)
        ForwardCurve::builder(bumped_id, self.tenor())
            .base_date(self.base_date())
            .reset_lag(self.reset_lag())
            .day_count(self.day_count())
            .knots(bumped_rates)
            .build()
            .ok()
    }
}

impl Bumpable for HazardCurve {
    fn apply_bump(&self, spec: BumpSpec) -> Option<Self> {
        let shift = match (spec.mode, spec.units) {
            (BumpMode::Additive, BumpUnits::RateBp | BumpUnits::Fraction | BumpUnits::Percent) => {
                spec.additive_fraction()?
            }
            _ => return None,
        };

        let bumped_id = match spec.units {
            BumpUnits::RateBp => id_spread_bp(self.id().as_str(), spec.value),
            BumpUnits::Percent => id_bump_pct(self.id().as_str(), spec.value),
            BumpUnits::Fraction => CurveId::new(format!("{}_shift_{:.4}", self.id(), spec.value)),
            BumpUnits::Factor => CurveId::new(format!("{}_shift_factor_{:.4}", self.id(), spec.value)),
        };

        let shifted_points: Vec<(F, F)> = self
            .knot_points()
            .map(|(t, lambda)| (t, (lambda + shift).max(0.0)))
            .collect();

        // Rebuild a proper curve with the bumped ID, preserving key metadata
        HazardCurve::builder(bumped_id)
            .base_date(self.base_date())
            .recovery_rate(self.recovery_rate())
            .day_count(self.day_count())
            .knots(shifted_points)
            .par_spreads(self.par_spread_points())
            .build()
            .ok()
    }
}

impl Bumpable for InflationCurve {
    fn apply_bump(&self, spec: BumpSpec) -> Option<Self> {
        let factor = match (spec.mode, spec.units) {
            (BumpMode::Additive, BumpUnits::Percent | BumpUnits::Fraction) => 1.0 + spec.additive_fraction()?,
            (BumpMode::Multiplicative, BumpUnits::Factor) => spec.value,
            _ => return None,
        };

        let bumped_id = match spec.mode {
            BumpMode::Additive => match spec.units {
                BumpUnits::Percent => id_bump_pct(self.id().as_str(), spec.value),
                BumpUnits::Fraction => CurveId::new(format!("{}_bump_frac_{:.4}", self.id(), spec.value)),
                BumpUnits::RateBp => id_bump_bp(self.id().as_str(), spec.value),
                BumpUnits::Factor => CurveId::new(format!("{}_bump_factor_{:.4}", self.id(), spec.value)),
            },
            BumpMode::Multiplicative => CurveId::new(format!("{}_bump_factor_{:.4}", self.id(), spec.value)),
        };

        let bumped_points: Vec<(F, F)> = self
            .knots()
            .iter()
            .copied()
            .zip(self.cpi_levels().iter().copied())
            .map(|(t, cpi)| (t, cpi * factor))
            .collect();

        InflationCurve::builder(bumped_id)
            .base_cpi(self.base_cpi())
            .knots(bumped_points)
            .build()
            .ok()
    }
}

impl Bumpable for BaseCorrelationCurve {
    fn apply_bump(&self, spec: BumpSpec) -> Option<Self> {
        let (add, mul) = match (spec.mode, spec.units) {
            (BumpMode::Additive, BumpUnits::Percent | BumpUnits::Fraction) => (spec.additive_fraction()?, 1.0),
            (BumpMode::Multiplicative, BumpUnits::Factor) => (0.0, spec.value),
            _ => return None,
        };

        let bumped_id = match spec.mode {
            BumpMode::Additive => match spec.units {
                BumpUnits::Percent => id_bump_pct(self.id().as_str(), spec.value),
                BumpUnits::Fraction => CurveId::new(format!("{}_bump_frac_{:.4}", self.id(), spec.value)),
                BumpUnits::RateBp => id_bump_bp(self.id().as_str(), spec.value),
                BumpUnits::Factor => CurveId::new(format!("{}_bump_factor_{:.4}", self.id(), spec.value)),
            },
            BumpMode::Multiplicative => CurveId::new(format!("{}_bump_factor_{:.4}", self.id(), spec.value)),
        };

        let bumped_points: Vec<(F, F)> = self
            .detachment_points()
            .iter()
            .copied()
            .zip(self.correlations().iter().copied())
            .map(|(d, c)| (d, (c * mul + add).clamp(0.0, 1.0)))
            .collect();

        BaseCorrelationCurve::builder(bumped_id)
            .knots(bumped_points)
            .build()
            .ok()
    }
}
