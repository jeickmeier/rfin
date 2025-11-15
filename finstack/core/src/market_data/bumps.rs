//! Bump functionality for scenario analysis and stress testing.
//!
//! Provides types and traits for applying parallel shocks and bumps to market
//! data. Used for risk metrics (DV01, CS01), scenario analysis, and regulatory
//! stress tests.

use super::term_structures::{
    base_correlation::BaseCorrelationCurve, discount_curve::DiscountCurve,
    forward_curve::ForwardCurve, hazard_curve::HazardCurve, inflation::InflationCurve,
};
use crate::types::CurveId;

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

/// Type of bump to apply.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum BumpType {
    /// Parallel shift across all maturities.
    #[default]
    Parallel,
    /// Key-rate bump at specific maturity.
    KeyRate { 
        /// Time in years at which to apply the key-rate bump.
        time_years: f64 
    },
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
/// use finstack_core::market_data::bumps::{BumpSpec, BumpMode, BumpUnits, BumpType};
///
/// let additive = BumpSpec { mode: BumpMode::Additive, units: BumpUnits::RateBp, value: 15.0, bump_type: BumpType::Parallel };
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
    pub value: f64,
    /// Type of bump (parallel or key-rate).
    #[cfg_attr(feature = "serde", serde(default))]
    pub bump_type: BumpType,
}

impl BumpSpec {
    /// Create an additive bump specified in basis points (e.g., 100.0 = 100bp = 1%).
    pub fn parallel_bp(bump_bp: f64) -> Self {
        Self {
            mode: BumpMode::Additive,
            units: BumpUnits::RateBp,
            value: bump_bp,
            bump_type: BumpType::Parallel,
        }
    }

    /// Create a key-rate bump at a specific time in years, specified in basis points.
    /// 
    /// # Arguments
    /// * `time_years` - The time in years at which to apply the key-rate bump
    /// * `bump_bp` - The bump size in basis points (e.g., 100.0 = 100bp = 1%)
    pub fn key_rate_bp(time_years: f64, bump_bp: f64) -> Self {
        Self {
            mode: BumpMode::Additive,
            units: BumpUnits::RateBp,
            value: bump_bp,
            bump_type: BumpType::KeyRate { time_years },
        }
    }

    /// Create a multiplicative bump given as a factor (e.g., 1.1 = +10%).
    pub fn multiplier(factor: f64) -> Self {
        Self {
            mode: BumpMode::Multiplicative,
            units: BumpUnits::Factor,
            value: factor,
            bump_type: BumpType::Parallel,
        }
    }

    /// Create an additive inflation shift specified in percent (e.g., 2.0 = +2%).
    pub fn inflation_shift_pct(bump_pct: f64) -> Self {
        Self {
            mode: BumpMode::Additive,
            units: BumpUnits::Percent,
            value: bump_pct,
            bump_type: BumpType::Parallel,
        }
    }

    /// Create an additive correlation shift specified in percent (e.g., 10.0 = +10%).
    pub fn correlation_shift_pct(bump_pct: f64) -> Self {
        Self {
            mode: BumpMode::Additive,
            units: BumpUnits::Percent,
            value: bump_pct,
            bump_type: BumpType::Parallel,
        }
    }

    /// If additive, return the bump as a normalized fraction (e.g., 100bp -> 0.01, 2% -> 0.02).
    pub(crate) fn additive_fraction(&self) -> Option<f64> {
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
pub(crate) fn id_bump_bp(id: &str, bp: f64) -> CurveId {
    CurveId::new(format!("{}_bump_{:.0}bp", id, bp))
}

#[inline]
pub(crate) fn id_spread_bp(id: &str, bp: f64) -> CurveId {
    CurveId::new(format!("{}_spread_{:.0}bp", id, bp))
}

#[inline]
pub(crate) fn id_bump_pct(id: &str, pct: f64) -> CurveId {
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

// -----------------------------------------------------------------------------
// Bumpable implementations for each curve type
// -----------------------------------------------------------------------------

impl Bumpable for DiscountCurve {
    fn apply_bump(&self, spec: BumpSpec) -> Option<Self> {
        if spec.mode == BumpMode::Additive && spec.units == BumpUnits::RateBp {
            match spec.bump_type {
                BumpType::Parallel => self.try_with_parallel_bump(spec.value).ok(),
                BumpType::KeyRate { time_years } => {
                    self.try_with_key_rate_bump_years(time_years, spec.value).ok()
                }
            }
        } else {
            None
        }
    }
}

impl Bumpable for ForwardCurve {
    fn apply_bump(&self, spec: BumpSpec) -> Option<Self> {
        match spec.bump_type {
            BumpType::Parallel => {
                // Simple pattern matching without boxed closures
                let (bump_amount, is_multiplicative) = match (spec.mode, spec.units) {
                    (BumpMode::Additive, BumpUnits::RateBp | BumpUnits::Fraction | BumpUnits::Percent) => {
                        (spec.additive_fraction()?, false)
                    }
                    (BumpMode::Multiplicative, BumpUnits::Factor) => (spec.value, true),
                    _ => return None,
                };

                let bumped_id = match spec.units {
                    BumpUnits::RateBp => id_bump_bp(self.id().as_str(), spec.value),
                    BumpUnits::Percent => id_bump_pct(self.id().as_str(), spec.value),
                    _ => CurveId::new(format!("{}_bump_{:.4}", self.id(), spec.value)),
                };

                let bumped_rates: Vec<(f64, f64)> = self
                    .knots()
                    .iter()
                    .copied()
                    .zip(self.forwards().iter().copied())
                    .map(|(t, r)| {
                        let new_rate = if is_multiplicative {
                            r * bump_amount
                        } else {
                            r + bump_amount
                        };
                        (t, new_rate)
                    })
                    .collect();

                ForwardCurve::builder(bumped_id, self.tenor())
                    .base_date(self.base_date())
                    .reset_lag(self.reset_lag())
                    .day_count(self.day_count())
                    .knots(bumped_rates)
                    .build()
                    .ok()
            }
            BumpType::KeyRate { time_years } => {
                // For key-rate bumps, only support additive rate bumps
                if spec.mode == BumpMode::Additive && spec.units == BumpUnits::RateBp {
                    self.try_with_key_rate_bump_years(time_years, spec.value).ok()
                } else {
                    None
                }
            }
        }
    }
}

impl Bumpable for HazardCurve {
    fn apply_bump(&self, spec: BumpSpec) -> Option<Self> {
        // HazardCurve currently only supports parallel bumps
        if !matches!(spec.bump_type, BumpType::Parallel) {
            return None;
        }
        
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
            BumpUnits::Factor => {
                CurveId::new(format!("{}_shift_factor_{:.4}", self.id(), spec.value))
            }
        };

        let shifted_points: Vec<(f64, f64)> = self
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
        // InflationCurve currently only supports parallel bumps
        if !matches!(spec.bump_type, BumpType::Parallel) {
            return None;
        }
        
        let factor = match (spec.mode, spec.units) {
            (BumpMode::Additive, BumpUnits::Percent | BumpUnits::Fraction) => {
                1.0 + spec.additive_fraction()?
            }
            (BumpMode::Multiplicative, BumpUnits::Factor) => spec.value,
            _ => return None,
        };

        let bumped_id = match spec.units {
            BumpUnits::RateBp => id_bump_bp(self.id().as_str(), spec.value),
            BumpUnits::Percent => id_bump_pct(self.id().as_str(), spec.value),
            _ => CurveId::new(format!("{}_bump_{:.4}", self.id(), spec.value)),
        };

        let bumped_points: Vec<(f64, f64)> = self
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
        // BaseCorrelationCurve currently only supports parallel bumps
        if !matches!(spec.bump_type, BumpType::Parallel) {
            return None;
        }
        
        let (add, mul) = match (spec.mode, spec.units) {
            (BumpMode::Additive, BumpUnits::Percent | BumpUnits::Fraction) => {
                (spec.additive_fraction()?, 1.0)
            }
            (BumpMode::Multiplicative, BumpUnits::Factor) => (0.0, spec.value),
            _ => return None,
        };

        let bumped_id = match spec.units {
            BumpUnits::RateBp => id_bump_bp(self.id().as_str(), spec.value),
            BumpUnits::Percent => id_bump_pct(self.id().as_str(), spec.value),
            _ => CurveId::new(format!("{}_bump_{:.4}", self.id(), spec.value)),
        };

        let bumped_points: Vec<(f64, f64)> = self
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
