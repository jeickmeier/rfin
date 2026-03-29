//! Bump functionality for scenario analysis and stress testing.
//!
//! Provides types and traits for applying parallel shocks and bumps to market
//! data. Used for risk metrics (DV01, CS01), scenario analysis, and regulatory
//! stress tests.
//!
//! # Conventions
//!
//! - Additive rate bumps are normalized into decimal form before application.
//!   For example, `1bp = 0.0001` and `2% = 0.02`.
//! - Inflation bumps are interpreted in annualized inflation-rate space rather
//!   than as direct CPI-level multipliers.
//! - FX percentage bumps are quoted in percent (`5.0 = +5%`) and strengthen the
//!   base currency against the quote currency.
//! - Bucketed rate bumps use market-standard triangular key-rate weights so the
//!   sum of bucketed DV01s matches the parallel DV01.
//!
//! # References
//!
//! - Key-rate risk methodology: `docs/REFERENCES.md#tuckman-serrat-fixed-income`

use super::scalars::{MarketScalar, ScalarTimeSeries};
use super::term_structures::{
    BaseCorrelationCurve, DiscountCurve, ForwardCurve, HazardCurve, InflationCurve, PriceCurve,
    VolatilityIndexCurve,
};
use crate::currency::Currency;
use crate::dates::Date;
use crate::types::CurveId;

// -----------------------------------------------------------------------------
// Bump Specification Types
// -----------------------------------------------------------------------------

/// Mode of applying a bump.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum BumpMode {
    /// Additive bump expressed in a normalized fractional form (e.g., 100bp = 0.01, 2% = 0.02).
    Additive,
    /// Multiplicative bump expressed as a factor (e.g., 1.1 = +10%, 0.9 = -10%).
    Multiplicative,
}

/// Type of bump to apply.
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BumpSpec {
    /// How the bump should be applied (additive vs multiplicative).
    pub mode: BumpMode,
    /// Units the value is expressed in, controlling normalization.
    pub units: BumpUnits,
    /// Raw magnitude provided by the caller (interpreted using `units`).
    pub value: f64,
    /// Type of bump (parallel or key-rate).
    #[serde(default)]
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
    ///
    /// Ensure the bump type is Parallel.
    pub fn validate_parallel(&self, context: &str) -> crate::Result<()> {
        if !matches!(self.bump_type, BumpType::Parallel) {
            return Err(crate::error::InputError::UnsupportedBump {
                reason: format!("{} only supports Parallel bumps", context),
            }
            .into());
        }
        Ok(())
    }

    /// Resolve standard bump units to a raw magnitude and a flag indicating if it is multiplicative.
    ///
    /// This handles the common logic found in curve bump implementations:
    /// - Additive/RateBp -> value / 10,000, not multiplicative
    /// - Additive/Percent -> value / 100, not multiplicative
    /// - Additive/Fraction -> value, not multiplicative
    /// - Multiplicative/Factor -> value, is multiplicative
    ///
    /// Returns `None` for other combinations (e.g. Multiplicative/Percent which isn't standard everywhere yet).
    pub fn resolve_standard_values(&self) -> Option<(f64, bool)> {
        match (self.mode, self.units) {
            (BumpMode::Additive, BumpUnits::RateBp) => Some((self.value / 10_000.0, false)),
            (BumpMode::Additive, BumpUnits::Percent) => Some((self.value / 100.0, false)),
            (BumpMode::Additive, BumpUnits::Fraction) => Some((self.value, false)),
            (BumpMode::Multiplicative, BumpUnits::Factor) => Some((self.value, true)),
            _ => None,
        }
    }
}

/// Unified bump description spanning curves, surfaces, FX, and scalar prices.
///
/// This enum is the heterogeneous input consumed by
/// [`MarketContext::bump`](crate::market_data::context::MarketContext::bump).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
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
        let (val, is_multiplicative) = spec.resolve_standard_values().ok_or_else(|| {
            crate::error::InputError::UnsupportedBump {
                reason: format!(
                    "DiscountCurve only supports Additive/{{RateBp,Percent,Fraction}} bumps, got {:?}/{:?}",
                    spec.mode, spec.units
                ),
            }
        })?;

        if is_multiplicative {
            return Err(crate::error::InputError::UnsupportedBump {
                reason: "DiscountCurve does not support Multiplicative bumps".to_string(),
            }
            .into());
        }

        // Internal DiscountCurve methods expect bump in Basis Points (BP).
        // Convert normalized value back to BP.
        let bp = val * 10_000.0;

        match spec.bump_type {
            BumpType::Parallel => self.with_parallel_bump(bp),
            BumpType::TriangularKeyRate {
                prev_bucket,
                target_bucket,
                next_bucket,
            } => self.with_triangular_key_rate_bump_neighbors(
                prev_bucket,
                target_bucket,
                next_bucket,
                bp,
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
                // Simple pattern matching without boxed closures
                let (bump_amount, is_multiplicative) = spec.resolve_standard_values().ok_or_else(|| {
                    InputError::UnsupportedBump {
                        reason: format!(
                            "ForwardCurve parallel bump requires Additive/{{RateBp,Percent,Fraction}} or Multiplicative/Factor, got {:?}/{:?}",
                            spec.mode, spec.units
                        ),
                    }
                })?;

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
                    .interp(self.interp_style())
                    .extrapolation(self.extrapolation())
                    .build()
            }
            BumpType::TriangularKeyRate {
                prev_bucket,
                target_bucket,
                next_bucket,
            } => {
                // For triangular key-rate bumps, only support additive rate bumps
                if spec.mode == BumpMode::Additive && spec.units == BumpUnits::RateBp {
                    self.with_triangular_key_rate_bump_neighbors(
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
        // Interpret RateBp/Percent as **par spread** shocks; convert to hazard using 1/(1 - recovery).
        let (spread, is_multiplicative) = spec.resolve_standard_values().ok_or_else(|| {
            InputError::UnsupportedBump {
                reason: format!(
                    "HazardCurve only supports Additive/{{RateBp,Percent,Fraction}} bumps, got {:?}/{:?}",
                    spec.mode, spec.units
                ),
            }
        })?;

        if is_multiplicative {
            return Err(InputError::UnsupportedBump {
                reason: "HazardCurve does not support Multiplicative bumps".to_string(),
            }
            .into());
        }

        let shift = spread / (1.0 - recovery);

        let bumped_id = match spec.units {
            BumpUnits::RateBp => id_spread_bp(self.id().as_str(), spec.value),
            BumpUnits::Percent => id_bump_pct(self.id().as_str(), spec.value),
            BumpUnits::Fraction => CurveId::new(format!("{}_shift_{:.4}", self.id(), spec.value)),
            BumpUnits::Factor => {
                CurveId::new(format!("{}_shift_factor_{:.4}", self.id(), spec.value))
            }
        };

        #[cfg(feature = "tracing")]
        for (t, lambda) in self.knot_points() {
            let shifted = lambda + shift;
            if shifted < 0.0 {
                tracing::warn!(
                    curve_id = %self.id(),
                    time = t,
                    hazard_rate = lambda,
                    hazard_shift = shift,
                    "Hazard curve bump clamped negative hazard rate to zero"
                );
            }
        }

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

        let factor = {
            let (raw_val, is_multiplicative) = spec.resolve_standard_values().ok_or_else(|| {
                InputError::UnsupportedBump {
                     reason: format!(
                        "InflationCurve only supports Additive/{{Percent,Fraction}} or Multiplicative/Factor, got {:?}/{:?}",
                        spec.mode, spec.units
                    ),
                }
            })?;
            (raw_val, is_multiplicative)
        };

        let bumped_id = match spec.units {
            // RateBp: 1 bp = 0.0001 (standard basis point convention).
            // Bumps are applied as absolute shifts to the inflation rate at each knot.
            BumpUnits::RateBp => id_bump_bp(self.id().as_str(), spec.value),
            BumpUnits::Percent => id_bump_pct(self.id().as_str(), spec.value),
            _ => CurveId::new(format!("{}_bump_{:.4}", self.id(), spec.value)),
        };

        let bumped_points: Vec<(f64, f64)> = self
            .knots()
            .iter()
            .copied()
            .map(|t| {
                if t <= 0.0 {
                    return (t, self.base_cpi());
                }

                let zero_rate = self.inflation_rate(0.0, t);
                let bumped_zero_rate = if factor.1 {
                    (1.0 + zero_rate) * factor.0 - 1.0
                } else {
                    zero_rate + factor.0
                };
                let bumped_cpi = self.base_cpi() * (1.0 + bumped_zero_rate).powf(t);
                (t, bumped_cpi)
            })
            .collect();

        InflationCurve::builder(bumped_id)
            .base_cpi(self.base_cpi())
            .base_date(self.base_date())
            .day_count(self.day_count())
            .indexation_lag_months(self.indexation_lag_months())
            .knots(bumped_points)
            .interp(self.interp_style())
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

        let (add, mul) = {
            let (raw_val, is_multiplicative) = spec.resolve_standard_values().ok_or_else(|| {
                InputError::UnsupportedBump {
                     reason: format!(
                        "BaseCorrelationCurve only supports Additive/{{Percent,Fraction}} or Multiplicative/Factor, got {:?}/{:?}",
                         spec.mode, spec.units
                    ),
                }
            })?;

            if is_multiplicative {
                (0.0, raw_val)
            } else {
                (raw_val, 1.0)
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

impl Bumpable for VolatilityIndexCurve {
    fn apply_bump(&self, spec: BumpSpec) -> crate::Result<Self> {
        use crate::error::InputError;

        match spec.bump_type {
            BumpType::Parallel => {
                // Vol index curves support both additive and multiplicative bumps
                match (spec.mode, spec.units) {
                    (BumpMode::Additive, BumpUnits::RateBp) => {
                        let bump = spec.value / 10_000.0;
                        self.with_parallel_bump(bump)
                    }
                    (BumpMode::Additive, BumpUnits::Fraction) => {
                        self.with_parallel_bump(spec.value)
                    }
                    (BumpMode::Additive, BumpUnits::Percent) => {
                        let frac = spec.value / 100.0;
                        self.with_parallel_bump(frac)
                    }
                    (BumpMode::Multiplicative, BumpUnits::Factor) => {
                        // spec.value is the target factor (e.g., 1.10 for +10%)
                        let pct = spec.value - 1.0;
                        self.with_percentage_bump(pct)
                    }
                    (BumpMode::Multiplicative, BumpUnits::Percent) => {
                        // spec.value is the percentage (e.g., 10 for +10%)
                        let pct = spec.value / 100.0;
                        self.with_percentage_bump(pct)
                    }
                    _ => Err(InputError::UnsupportedBump {
                        reason: format!(
                            "VolatilityIndexCurve parallel bump: unsupported mode/units {:?}/{:?}",
                            spec.mode, spec.units
                        ),
                    }
                    .into()),
                }
            }
            BumpType::TriangularKeyRate {
                prev_bucket,
                target_bucket,
                next_bucket,
            } => {
                let bump = match (spec.mode, spec.units) {
                    (BumpMode::Additive, BumpUnits::RateBp) => spec.value / 10_000.0,
                    (BumpMode::Additive, BumpUnits::Fraction) => spec.value,
                    (BumpMode::Additive, BumpUnits::Percent) => spec.value / 100.0,
                    _ => {
                        return Err(InputError::UnsupportedBump {
                            reason: format!(
                                "VolatilityIndexCurve key-rate bump requires Additive mode, got {:?}/{:?}",
                                spec.mode, spec.units
                            ),
                        }
                        .into());
                    }
                };
                self.with_triangular_key_rate_bump_neighbors(
                    prev_bucket,
                    target_bucket,
                    next_bucket,
                    bump,
                )
            }
        }
    }
}

impl Bumpable for MarketScalar {
    fn apply_bump(&self, spec: BumpSpec) -> crate::Result<Self> {
        use crate::error::InputError;

        let (raw_val, is_multiplicative) = spec.resolve_standard_values().ok_or_else(|| {
            InputError::UnsupportedBump {
                reason: format!(
                    "MarketScalar only supports Additive/{{RateBp,Percent,Fraction}} or Multiplicative/Factor, got {:?}/{:?}",
                    spec.mode, spec.units
                ),
            }
        })?;

        match self {
            MarketScalar::Unitless(v) => {
                let new_val = if is_multiplicative {
                    v * raw_val
                } else {
                    v + raw_val
                };
                Ok(MarketScalar::Unitless(new_val))
            }
            MarketScalar::Price(m) => {
                let new_val = if is_multiplicative {
                    *m * raw_val
                } else {
                    // Additive bump on Price: interpreted as proportional shift.
                    // A raw_val of 0.01 means a +1% price change, not +0.01 absolute.
                    *m * (1.0 + raw_val)
                };
                Ok(MarketScalar::Price(new_val))
            }
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

        let (raw_val, is_multiplicative) = spec.resolve_standard_values().ok_or_else(|| {
            InputError::UnsupportedBump {
                reason: format!(
                    "ScalarTimeSeries only supports Additive/{{RateBp,Percent,Fraction}} or Multiplicative/Factor, got {:?}/{:?}",
                    spec.mode, spec.units
                ),
            }
        })?;

        let bumped_obs: Vec<(crate::dates::Date, f64)> = if is_multiplicative {
            self.observations()
                .into_iter()
                .map(|(d, v)| (d, v * raw_val))
                .collect()
        } else {
            self.observations()
                .into_iter()
                .map(|(d, v)| (d, v + raw_val))
                .collect()
        };

        ScalarTimeSeries::new(self.id().as_str(), bumped_obs, self.currency())
            .map(|s| s.with_interpolation(self.interpolation()))
    }
}

impl Bumpable for PriceCurve {
    fn apply_bump(&self, spec: BumpSpec) -> crate::Result<Self> {
        use crate::error::InputError;

        match spec.bump_type {
            BumpType::Parallel => {
                // Price curves support both additive and multiplicative bumps
                match (spec.mode, spec.units) {
                    (BumpMode::Additive, BumpUnits::Fraction) => {
                        // Interpret fraction as absolute price units
                        self.with_parallel_bump(spec.value)
                    }
                    (BumpMode::Additive, BumpUnits::Percent) => {
                        // Interpret percent as percentage of current price
                        let pct = spec.value / 100.0;
                        self.with_percentage_bump(pct)
                    }
                    (BumpMode::Multiplicative, BumpUnits::Factor) => {
                        // spec.value is the target factor (e.g., 1.10 for +10%)
                        let pct = spec.value - 1.0;
                        self.with_percentage_bump(pct)
                    }
                    (BumpMode::Multiplicative, BumpUnits::Percent) => {
                        // spec.value is the percentage (e.g., 10 for +10%)
                        let pct = spec.value / 100.0;
                        self.with_percentage_bump(pct)
                    }
                    _ => Err(InputError::UnsupportedBump {
                        reason: format!(
                            "PriceCurve parallel bump: unsupported mode/units {:?}/{:?}. \
                             Use Additive/{{Fraction,Percent}} or Multiplicative/{{Factor,Percent}}",
                            spec.mode, spec.units
                        ),
                    }
                    .into()),
                }
            }
            BumpType::TriangularKeyRate {
                prev_bucket,
                target_bucket,
                next_bucket,
            } => {
                let bump = match (spec.mode, spec.units) {
                    (BumpMode::Additive, BumpUnits::Fraction) => spec.value,
                    (BumpMode::Additive, BumpUnits::Percent) => {
                        // Compute percentage of spot as additive bump
                        spec.value / 100.0 * self.spot_price()
                    }
                    _ => {
                        return Err(InputError::UnsupportedBump {
                            reason: format!(
                                "PriceCurve key-rate bump requires Additive mode, got {:?}/{:?}",
                                spec.mode, spec.units
                            ),
                        }
                        .into());
                    }
                };
                self.with_triangular_key_rate_bump_neighbors(
                    prev_bucket,
                    target_bucket,
                    next_bucket,
                    bump,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_standard_values() {
        // Additive RateBp (divided by 10,000)
        let spec = BumpSpec {
            mode: BumpMode::Additive,
            units: BumpUnits::RateBp,
            value: 50.0,
            bump_type: BumpType::Parallel,
        };
        assert_eq!(spec.resolve_standard_values(), Some((0.0050, false)));

        // Additive Percent (divided by 100)
        let spec = BumpSpec {
            mode: BumpMode::Additive,
            units: BumpUnits::Percent,
            value: 2.0,
            bump_type: BumpType::Parallel,
        };
        assert_eq!(spec.resolve_standard_values(), Some((0.02, false)));

        // Additive Fraction (raw value)
        let spec = BumpSpec {
            mode: BumpMode::Additive,
            units: BumpUnits::Fraction,
            value: 0.05,
            bump_type: BumpType::Parallel,
        };
        assert_eq!(spec.resolve_standard_values(), Some((0.05, false)));

        // Multiplicative Factor (raw value, is_multiplicative=true)
        let spec = BumpSpec {
            mode: BumpMode::Multiplicative,
            units: BumpUnits::Factor,
            value: 1.10,
            bump_type: BumpType::Parallel,
        };
        assert_eq!(spec.resolve_standard_values(), Some((1.10, true)));

        // Unsupported combination (Multiplicative/Percent) -> None
        let spec = BumpSpec {
            mode: BumpMode::Multiplicative,
            units: BumpUnits::Percent,
            value: 10.0,
            bump_type: BumpType::Parallel,
        };
        assert_eq!(spec.resolve_standard_values(), None);
    }

    #[test]
    fn test_forward_curve_bump() -> crate::Result<()> {
        use crate::dates::Date;
        use crate::market_data::term_structures::ForwardCurve;
        use time::Month;

        let base = Date::from_calendar_date(2025, Month::January, 1).map_err(|_| {
            crate::error::InputError::InvalidDate {
                year: 2025,
                month: 1,
                day: 1,
            }
        })?;
        let height_check = 0.04; // rate at 1.0 (knot)

        let fc = ForwardCurve::builder("USD-TEST-3M", 0.25)
            .base_date(base)
            .knots([(0.5, 0.038), (1.0, height_check)]) // Minimum 2 points required
            .build()?;

        // 1. Additive RateBp
        let spec = BumpSpec::parallel_bp(10.0); // +10bps = +0.0010
        let bumped = fc.apply_bump(spec)?;
        assert!((bumped.rate(1.0) - 0.0410).abs() < 1e-12);

        // 2. Additive Percent
        let spec_pct = BumpSpec::inflation_shift_pct(0.5); // Using helper, treated as Additive Percent
                                                           // ForwardCurve generic logic handles Additive/Percent -> value/100
                                                           // So 0.5 -> 0.005. rate = 0.04 + 0.005 = 0.045
        let bumped_pct = fc.apply_bump(spec_pct)?;
        assert!((bumped_pct.rate(1.0) - 0.045).abs() < 1e-12);

        // 3. Multiplicative Factor
        let spec_mul = BumpSpec::multiplier(1.10); // +10%
        let bumped_mul = fc.apply_bump(spec_mul)?;
        assert!((bumped_mul.rate(1.0) - 0.044).abs() < 1e-12);

        Ok(())
    }

    #[test]
    fn test_hazard_curve_bump() -> crate::Result<()> {
        use crate::dates::Date;
        use crate::market_data::term_structures::HazardCurve;
        use time::Month;

        let base = Date::from_calendar_date(2025, Month::January, 1).map_err(|_| {
            crate::error::InputError::InvalidDate {
                year: 2025,
                month: 1,
                day: 1,
            }
        })?;
        let hc = HazardCurve::builder("CDS-TEST")
            .base_date(base)
            .recovery_rate(0.40)
            .knots([(1.0, 0.02)]) // lambda = 0.02
            .build()?;

        // Additive RateBp
        // shift = 10bps / (1 - R) = 0.0010 / 0.6 = 0.001666...
        let spec = BumpSpec::parallel_bp(10.0);
        let bumped = hc.apply_bump(spec)?;
        let expected_lambda = 0.02 + (0.0010 / 0.60);
        assert!((bumped.hazard_rate(1.0) - expected_lambda).abs() < 1e-10);

        // Verify Multiplicative raises error
        let spec_mul = BumpSpec::multiplier(1.10);
        assert!(hc.apply_bump(spec_mul).is_err());

        Ok(())
    }

    #[test]
    fn test_hazard_curve_negative_bump_clamps_to_zero() -> crate::Result<()> {
        use crate::dates::Date;
        use crate::market_data::term_structures::HazardCurve;
        use time::Month;

        let base = Date::from_calendar_date(2025, Month::January, 1).map_err(|_| {
            crate::error::InputError::InvalidDate {
                year: 2025,
                month: 1,
                day: 1,
            }
        })?;
        let hc = HazardCurve::builder("CDS-CLAMP")
            .base_date(base)
            .recovery_rate(0.40)
            .knots([(1.0, 0.0010)])
            .build()?;

        let spec = BumpSpec::parallel_bp(-20.0);
        let bumped = hc.apply_bump(spec)?;

        assert_eq!(bumped.hazard_rate(1.0), 0.0);
        Ok(())
    }

    #[test]
    fn test_discount_curve_bump() -> crate::Result<()> {
        use crate::dates::Date;
        use crate::market_data::term_structures::DiscountCurve;
        use time::Month;

        let base = Date::from_calendar_date(2025, Month::January, 1).map_err(|_| {
            crate::error::InputError::InvalidDate {
                year: 2025,
                month: 1,
                day: 1,
            }
        })?;
        // Flat curve: 5% continuously compounded -> DF(1) = exp(-0.05) ≈ 0.951229
        let dc = DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots([(0.5, 0.975309912), (1.0, 0.9512294245)]) // Minimum 2 points
            .build()?;

        // 1. Additive RateBp
        let spec = BumpSpec::parallel_bp(100.0); // +100bps = +1%
        let bumped = dc.apply_bump(spec)?;
        // New rate = 5% + 1% = 6%
        // DF(1) = exp(-0.06) ≈ 0.9417645336
        assert!((bumped.df(1.0) - 0.9417645336).abs() < 1e-8);

        // 2. Additive Percent (New capability!)
        // 1% additive bump (same as 100bp)
        let spec_pct = BumpSpec::inflation_shift_pct(1.0);
        let bumped_pct = dc.apply_bump(spec_pct)?;
        assert!((bumped_pct.df(1.0) - 0.9417645336).abs() < 1e-8);

        // 3. Verify Multiplicative raises error
        let spec_mul = BumpSpec::multiplier(1.10);
        assert!(dc.apply_bump(spec_mul).is_err());

        Ok(())
    }
}
