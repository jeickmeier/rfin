//! Bump functionality for scenario analysis and stress testing.
//!
//! Provides types and traits for applying parallel shocks and bumps to market
//! data. Used for risk metrics (DV01, CS01), scenario analysis, and regulatory
//! stress tests.

use super::scalars::{MarketScalar, ScalarTimeSeries};
use super::term_structures::{
    base_correlation::BaseCorrelationCurve, discount_curve::DiscountCurve,
    forward_curve::ForwardCurve, hazard_curve::HazardCurve, inflation::InflationCurve,
};
use crate::currency::Currency;
use crate::dates::Date;
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
    /// Triangular key-rate bump with explicit bucket neighbors (market standard).
    ///
    /// This implements the market-standard key-rate DV01 methodology (per Tuckman/Fabozzi)
    /// where the triangular weight is defined by the **bucket grid**, not curve knots.
    /// This ensures that the sum of all bucketed DV01s equals the parallel DV01.
    ///
    /// # Weight Function
    ///
    /// For a bump at `target_bucket` with neighbors `prev_bucket` and `next_bucket`:
    /// - w(t) = 0                                    if t ≤ prev_bucket
    /// - w(t) = (t - prev_bucket) / (target - prev) if prev_bucket < t ≤ target_bucket
    /// - w(t) = (next_bucket - t) / (next - target) if target_bucket < t < next_bucket
    /// - w(t) = 0                                    if t ≥ next_bucket
    ///
    /// # Key Property
    ///
    /// For any time t, the sum of all bucket weights equals 1.0:
    /// `Σᵢ wᵢ(t) = 1.0`
    ///
    /// This ensures that sum of bucketed DV01 = parallel DV01.
    TriangularKeyRate {
        /// Previous bucket time in years (use 0.0 for first bucket)
        prev_bucket: f64,
        /// Target bucket time in years (peak of the triangle)
        target_bucket: f64,
        /// Next bucket time in years (use f64::INFINITY for last bucket)
        next_bucket: f64,
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

    /// Create a triangular key-rate bump with explicit bucket neighbors.
    ///
    /// This is the market-standard implementation (per Tuckman/Fabozzi) where the
    /// triangular weight is defined by the bucket grid, ensuring that the sum of
    /// all bucketed DV01s equals the parallel DV01.
    ///
    /// # Arguments
    /// * `prev_bucket` - Previous bucket time in years (use 0.0 for first bucket)
    /// * `target_bucket` - Target bucket time in years (peak of the triangle)
    /// * `next_bucket` - Next bucket time in years (use f64::INFINITY for last bucket)
    /// * `bump_bp` - Bump size in basis points (e.g., 1.0 = 1bp)
    ///
    /// # Example
    /// ```rust
    /// use finstack_core::market_data::bumps::BumpSpec;
    ///
    /// // For the 5Y bucket with neighbors at 3Y and 7Y
    /// let spec = BumpSpec::triangular_key_rate_bp(3.0, 5.0, 7.0, 1.0);
    ///
    /// // For the first bucket (3M) with no previous neighbor
    /// let first = BumpSpec::triangular_key_rate_bp(0.0, 0.25, 0.5, 1.0);
    ///
    /// // For the last bucket (30Y) with no next neighbor
    /// let last = BumpSpec::triangular_key_rate_bp(20.0, 30.0, f64::INFINITY, 1.0);
    /// ```
    pub fn triangular_key_rate_bp(
        prev_bucket: f64,
        target_bucket: f64,
        next_bucket: f64,
        bump_bp: f64,
    ) -> Self {
        Self {
            mode: BumpMode::Additive,
            units: BumpUnits::RateBp,
            value: bump_bp,
            bump_type: BumpType::TriangularKeyRate {
                prev_bucket,
                target_bucket,
                next_bucket,
            },
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

/// Unified bump description spanning curves, surfaces, FX, and scalar prices.
#[derive(Debug, Clone)]
pub enum MarketBump {
    /// Standard curve/surface/price bumps addressed by `CurveId`.
    Curve {
        /// Identifier of the curve/surface/price entry.
        id: CurveId,
        /// How to bump the entry (parallel/key-rate, additive/multiplicative).
        spec: BumpSpec,
    },
    /// FX rate percentage shock (positive strengthens the base currency).
    FxPct {
        /// Base currency.
        base: Currency,
        /// Quote currency.
        quote: Currency,
        /// Percentage change (e.g., 5.0 = +5%).
        pct: f64,
        /// Valuation date used for the FX lookup.
        as_of: Date,
    },
    /// Volatility surface bucket bump (percentage multiplier).
    VolBucketPct {
        /// Surface identifier.
        surface_id: CurveId,
        /// Optional expiry filters (year fractions).
        expiries: Option<Vec<f64>>,
        /// Optional strike filters.
        strikes: Option<Vec<f64>>,
        /// Percentage change to apply to matching buckets.
        pct: f64,
    },
    /// Base correlation bucket bump (additive points).
    BaseCorrBucketPts {
        /// Curve identifier.
        surface_id: CurveId,
        /// Optional detachment filters (percent, e.g., 3.0 for 3%).
        detachments: Option<Vec<f64>>,
        /// Absolute correlation points to add (0.02 = +2 points).
        points: f64,
    },
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
///
/// This trait provides a uniform interface for applying market data bumps
/// (parallel shifts, key-rate bumps, etc.) across different curve and surface types.
///
/// # Error Handling
///
/// Returns `Err` with a descriptive message when:
/// - The bump type is not supported for this curve type
/// - The mode/units combination is invalid
/// - The curve reconstruction fails after applying the bump
/// - Input validation fails (e.g., invalid recovery rate for hazard curves)
pub trait Bumpable: Sized {
    /// Apply a bump specification to create a new bumped instance.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::UnsupportedBump`](crate::error::InputError::UnsupportedBump)
    /// if the bump operation is not supported for this type.
    fn apply_bump(&self, spec: BumpSpec) -> crate::Result<Self>;
}

// -----------------------------------------------------------------------------
// Bumpable implementations for each curve type
// -----------------------------------------------------------------------------

impl Bumpable for DiscountCurve {
    fn apply_bump(&self, spec: BumpSpec) -> crate::Result<Self> {
        use crate::error::InputError;

        if spec.mode != BumpMode::Additive || spec.units != BumpUnits::RateBp {
            return Err(InputError::UnsupportedBump {
                reason: format!(
                    "DiscountCurve only supports Additive/RateBp bumps, got {:?}/{:?}",
                    spec.mode, spec.units
                ),
            }
            .into());
        }

        match spec.bump_type {
            BumpType::Parallel => self.try_with_parallel_bump(spec.value),
            BumpType::TriangularKeyRate {
                prev_bucket,
                target_bucket,
                next_bucket,
            } => self.try_with_triangular_key_rate_bump_neighbors(
                prev_bucket,
                target_bucket,
                next_bucket,
                spec.value,
            ),
        }
    }
}

impl Bumpable for ForwardCurve {
    fn apply_bump(&self, spec: BumpSpec) -> crate::Result<Self> {
        use crate::error::InputError;

        match spec.bump_type {
            BumpType::Parallel => {
                // Simple pattern matching without boxed closures
                let (bump_amount, is_multiplicative) = match (spec.mode, spec.units) {
                    (
                        BumpMode::Additive,
                        BumpUnits::RateBp | BumpUnits::Fraction | BumpUnits::Percent,
                    ) => {
                        let frac = spec.additive_fraction().ok_or_else(|| InputError::UnsupportedBump {
                            reason: "ForwardCurve: additive bump requires RateBp, Percent, or Fraction units".to_string(),
                        })?;
                        (frac, false)
                    }
                    (BumpMode::Multiplicative, BumpUnits::Factor) => (spec.value, true),
                    _ => {
                        return Err(InputError::UnsupportedBump {
                            reason: format!(
                                "ForwardCurve parallel bump requires Additive/{{RateBp,Percent,Fraction}} or Multiplicative/Factor, got {:?}/{:?}",
                                spec.mode, spec.units
                            ),
                        }
                        .into());
                    }
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
            }
            BumpType::TriangularKeyRate {
                prev_bucket,
                target_bucket,
                next_bucket,
            } => {
                // For triangular key-rate bumps, only support additive rate bumps
                if spec.mode == BumpMode::Additive && spec.units == BumpUnits::RateBp {
                    self.try_with_triangular_key_rate_bump_neighbors(
                        prev_bucket,
                        target_bucket,
                        next_bucket,
                        spec.value,
                    )
                } else {
                    Err(InputError::UnsupportedBump {
                        reason: format!(
                            "ForwardCurve key-rate bump requires Additive/RateBp, got {:?}/{:?}",
                            spec.mode, spec.units
                        ),
                    }
                    .into())
                }
            }
        }
    }
}

impl Bumpable for HazardCurve {
    fn apply_bump(&self, spec: BumpSpec) -> crate::Result<Self> {
        use crate::error::InputError;

        // HazardCurve currently only supports parallel bumps
        if !matches!(spec.bump_type, BumpType::Parallel) {
            return Err(InputError::UnsupportedBump {
                reason: "HazardCurve only supports Parallel bumps, not key-rate bumps".to_string(),
            }
            .into());
        }

        // Recovery must be within [0, 1) for par spread ⇢ hazard conversions
        let recovery = self.recovery_rate();
        if !recovery.is_finite() || !(0.0..1.0).contains(&recovery) {
            return Err(InputError::UnsupportedBump {
                reason: format!(
                    "HazardCurve bump requires recovery rate in [0, 1), got {}",
                    recovery
                ),
            }
            .into());
        }

        // Interpret RateBp/Percent as **par spread** shocks; convert to hazard using 1/(1 - recovery).
        let shift = match (spec.mode, spec.units) {
            (BumpMode::Additive, BumpUnits::RateBp | BumpUnits::Fraction | BumpUnits::Percent) => {
                let spread = spec.additive_fraction().ok_or_else(|| InputError::UnsupportedBump {
                    reason: "HazardCurve additive bump failed to compute fraction".to_string(),
                })?;
                spread / (1.0 - recovery)
            }
            _ => {
                return Err(InputError::UnsupportedBump {
                    reason: format!(
                        "HazardCurve only supports Additive/{{RateBp,Percent,Fraction}} bumps, got {:?}/{:?}",
                        spec.mode, spec.units
                    ),
                }
                .into());
            }
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
    }
}

impl Bumpable for InflationCurve {
    fn apply_bump(&self, spec: BumpSpec) -> crate::Result<Self> {
        use crate::error::InputError;

        // InflationCurve currently only supports parallel bumps
        if !matches!(spec.bump_type, BumpType::Parallel) {
            return Err(InputError::UnsupportedBump {
                reason: "InflationCurve only supports Parallel bumps, not key-rate bumps"
                    .to_string(),
            }
            .into());
        }

        let factor = match (spec.mode, spec.units) {
            (BumpMode::Additive, BumpUnits::Percent | BumpUnits::Fraction) => {
                let frac = spec.additive_fraction().ok_or_else(|| InputError::UnsupportedBump {
                    reason: "InflationCurve additive bump failed to compute fraction".to_string(),
                })?;
                1.0 + frac
            }
            (BumpMode::Multiplicative, BumpUnits::Factor) => spec.value,
            _ => {
                return Err(InputError::UnsupportedBump {
                    reason: format!(
                        "InflationCurve only supports Additive/{{Percent,Fraction}} or Multiplicative/Factor, got {:?}/{:?}",
                        spec.mode, spec.units
                    ),
                }
                .into());
            }
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
    }
}

impl Bumpable for BaseCorrelationCurve {
    fn apply_bump(&self, spec: BumpSpec) -> crate::Result<Self> {
        use crate::error::InputError;

        // BaseCorrelationCurve currently only supports parallel bumps
        if !matches!(spec.bump_type, BumpType::Parallel) {
            return Err(InputError::UnsupportedBump {
                reason: "BaseCorrelationCurve only supports Parallel bumps, not key-rate bumps"
                    .to_string(),
            }
            .into());
        }

        let (add, mul) = match (spec.mode, spec.units) {
            (BumpMode::Additive, BumpUnits::Percent | BumpUnits::Fraction) => {
                let frac = spec.additive_fraction().ok_or_else(|| InputError::UnsupportedBump {
                    reason: "BaseCorrelationCurve additive bump failed to compute fraction"
                        .to_string(),
                })?;
                (frac, 1.0)
            }
            (BumpMode::Multiplicative, BumpUnits::Factor) => (0.0, spec.value),
            _ => {
                return Err(InputError::UnsupportedBump {
                    reason: format!(
                        "BaseCorrelationCurve only supports Additive/{{Percent,Fraction}} or Multiplicative/Factor, got {:?}/{:?}",
                        spec.mode, spec.units
                    ),
                }
                .into());
            }
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
    }
}

impl Bumpable for MarketScalar {
    fn apply_bump(&self, spec: BumpSpec) -> crate::Result<Self> {
        use crate::error::InputError;

        match self {
            MarketScalar::Unitless(v) => match (spec.mode, spec.units) {
                (
                    BumpMode::Additive,
                    BumpUnits::RateBp | BumpUnits::Percent | BumpUnits::Fraction,
                ) => {
                    let frac = spec.additive_fraction().ok_or_else(|| InputError::UnsupportedBump {
                        reason: "MarketScalar::Unitless additive bump failed to compute fraction"
                            .to_string(),
                    })?;
                    Ok(MarketScalar::Unitless(v + frac))
                }
                (BumpMode::Multiplicative, BumpUnits::Factor) => {
                    Ok(MarketScalar::Unitless(v * spec.value))
                }
                _ => Err(InputError::UnsupportedBump {
                    reason: format!(
                        "MarketScalar::Unitless only supports Additive/{{RateBp,Percent,Fraction}} or Multiplicative/Factor, got {:?}/{:?}",
                        spec.mode, spec.units
                    ),
                }
                .into()),
            },
            MarketScalar::Price(m) => match (spec.mode, spec.units) {
                (BumpMode::Additive, BumpUnits::Percent | BumpUnits::Fraction) => {
                    let frac = spec.additive_fraction().ok_or_else(|| InputError::UnsupportedBump {
                        reason: "MarketScalar::Price additive bump failed to compute fraction"
                            .to_string(),
                    })?;
                    let factor = 1.0 + frac;
                    Ok(MarketScalar::Price(*m * factor))
                }
                (BumpMode::Multiplicative, BumpUnits::Factor) => {
                    Ok(MarketScalar::Price(*m * spec.value))
                }
                _ => Err(InputError::UnsupportedBump {
                    reason: format!(
                        "MarketScalar::Price only supports Additive/{{Percent,Fraction}} or Multiplicative/Factor, got {:?}/{:?}",
                        spec.mode, spec.units
                    ),
                }
                .into()),
            },
        }
    }
}

impl Bumpable for ScalarTimeSeries {
    fn apply_bump(&self, spec: BumpSpec) -> crate::Result<Self> {
        use crate::error::InputError;

        // Only parallel bumps are supported for now
        if !matches!(spec.bump_type, BumpType::Parallel) {
            return Err(InputError::UnsupportedBump {
                reason: "ScalarTimeSeries only supports Parallel bumps, not key-rate bumps"
                    .to_string(),
            }
            .into());
        }

        let bumped_obs: Vec<(crate::dates::Date, f64)> = match (spec.mode, spec.units) {
            (BumpMode::Additive, BumpUnits::RateBp | BumpUnits::Percent | BumpUnits::Fraction) => {
                let delta = spec.additive_fraction().ok_or_else(|| InputError::UnsupportedBump {
                    reason: "ScalarTimeSeries additive bump failed to compute fraction".to_string(),
                })?;
                self.observations()
                    .into_iter()
                    .map(|(d, v)| (d, v + delta))
                    .collect()
            }
            (BumpMode::Multiplicative, BumpUnits::Factor) => self
                .observations()
                .into_iter()
                .map(|(d, v)| (d, v * spec.value))
                .collect(),
            _ => {
                return Err(InputError::UnsupportedBump {
                    reason: format!(
                        "ScalarTimeSeries only supports Additive/{{RateBp,Percent,Fraction}} or Multiplicative/Factor, got {:?}/{:?}",
                        spec.mode, spec.units
                    ),
                }
                .into());
            }
        };

        ScalarTimeSeries::new(self.id().as_str(), bumped_obs, self.currency())
            .map(|s| s.with_interpolation(self.interpolation()))
    }
}
