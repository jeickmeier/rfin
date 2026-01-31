use std::sync::Arc;

use crate::market_data::bumps::{BumpSpec, BumpType, Bumpable};
use crate::market_data::term_structures::{
    BaseCorrelationCurve, DiscountCurve, ForwardCurve, HazardCurve, InflationCurve, PriceCurve,
    VolatilityIndexCurve,
};
use crate::types::CurveId;
use crate::Result;

// -----------------------------------------------------------------------------
// RebuildableWithId trait for preserving curve ID after bumping
// -----------------------------------------------------------------------------

/// Trait for curves that can be rebuilt with a new ID while preserving all other data.
///
/// This is used during market bumping operations where the bump produces a curve
/// with a modified ID (e.g., "USD-OIS_bump_+10bp") but we want to keep the original ID.
pub(crate) trait RebuildableWithId: Sized {
    /// Rebuild the curve with a new ID, preserving all other data.
    fn rebuild_with_id(&self, id: CurveId) -> Result<Self>;
}

impl RebuildableWithId for DiscountCurve {
    fn rebuild_with_id(&self, id: CurveId) -> Result<Self> {
        DiscountCurve::builder(id)
            .base_date(self.base_date())
            .day_count(self.day_count())
            .knots(self.knots().iter().copied().zip(self.dfs().iter().copied()))
            .set_interp(self.interp_style())
            .extrapolation(self.extrapolation())
            .allow_non_monotonic()
            .build()
    }
}

impl RebuildableWithId for ForwardCurve {
    fn rebuild_with_id(&self, id: CurveId) -> Result<Self> {
        ForwardCurve::builder(id, self.tenor())
            .base_date(self.base_date())
            .reset_lag(self.reset_lag())
            .day_count(self.day_count())
            .knots(
                self.knots()
                    .iter()
                    .copied()
                    .zip(self.forwards().iter().copied()),
            )
            .build()
    }
}

impl RebuildableWithId for HazardCurve {
    fn rebuild_with_id(&self, id: CurveId) -> Result<Self> {
        self.to_builder_with_id(id).build()
    }
}

impl RebuildableWithId for InflationCurve {
    fn rebuild_with_id(&self, id: CurveId) -> Result<Self> {
        InflationCurve::builder(id)
            .base_cpi(self.base_cpi())
            .knots(
                self.knots()
                    .iter()
                    .copied()
                    .zip(self.cpi_levels().iter().copied()),
            )
            .build()
    }
}

impl RebuildableWithId for BaseCorrelationCurve {
    fn rebuild_with_id(&self, id: CurveId) -> Result<Self> {
        BaseCorrelationCurve::builder(id)
            .knots(
                self.detachment_points()
                    .iter()
                    .copied()
                    .zip(self.correlations().iter().copied()),
            )
            .build()
    }
}

impl RebuildableWithId for VolatilityIndexCurve {
    fn rebuild_with_id(&self, id: CurveId) -> Result<Self> {
        VolatilityIndexCurve::builder(id)
            .base_date(self.base_date())
            .day_count(self.day_count())
            .spot_level(self.spot_level())
            .knots(
                self.knots()
                    .iter()
                    .copied()
                    .zip(self.levels().iter().copied()),
            )
            .build()
    }
}

impl RebuildableWithId for PriceCurve {
    fn rebuild_with_id(&self, id: CurveId) -> Result<Self> {
        PriceCurve::builder(id)
            .base_date(self.base_date())
            .day_count(self.day_count())
            .spot_price(self.spot_price())
            .knots(
                self.knots()
                    .iter()
                    .copied()
                    .zip(self.prices().iter().copied()),
            )
            .build()
    }
}

/// Unified storage for all curve types using an enum.
///
/// Downstream code rarely manipulates [`CurveStorage`] directly; it mostly
/// powers [`super::MarketContext`]'s heterogeneous map. When required, the helper
/// methods expose the inner `Arc` for each concrete curve type.
#[derive(Clone, Debug)]
pub enum CurveStorage {
    /// Discount factor curve
    Discount(Arc<DiscountCurve>),
    /// Forward rate curve
    Forward(Arc<ForwardCurve>),
    /// Credit hazard curve
    Hazard(Arc<HazardCurve>),
    /// Inflation index curve
    Inflation(Arc<InflationCurve>),
    /// Base correlation curve
    BaseCorrelation(Arc<BaseCorrelationCurve>),
    /// Forward price curve (commodities, indices)
    Price(Arc<PriceCurve>),
    /// Volatility index forward curve (VIX, VXN, VSTOXX)
    VolIndex(Arc<VolatilityIndexCurve>),
}

impl CurveStorage {
    /// Return the curve's unique identifier.
    pub fn id(&self) -> &CurveId {
        match self {
            Self::Discount(c) => c.id(),
            Self::Forward(c) => c.id(),
            Self::Hazard(c) => c.id(),
            Self::Inflation(c) => c.id(),
            Self::BaseCorrelation(c) => c.id(),
            Self::Price(c) => c.id(),
            Self::VolIndex(c) => c.id(),
        }
    }

    /// Borrow the discount curve when the variant matches.
    pub fn discount(&self) -> Option<&Arc<DiscountCurve>> {
        match self {
            Self::Discount(curve) => Some(curve),
            _ => None,
        }
    }

    /// Borrow the forward curve when the variant matches.
    pub fn forward(&self) -> Option<&Arc<ForwardCurve>> {
        match self {
            Self::Forward(curve) => Some(curve),
            _ => None,
        }
    }

    /// Borrow the hazard curve when the variant matches.
    pub fn hazard(&self) -> Option<&Arc<HazardCurve>> {
        match self {
            Self::Hazard(curve) => Some(curve),
            _ => None,
        }
    }

    /// Borrow the inflation curve when the variant matches.
    pub fn inflation(&self) -> Option<&Arc<InflationCurve>> {
        match self {
            Self::Inflation(curve) => Some(curve),
            _ => None,
        }
    }

    /// Borrow the base correlation curve when the variant matches.
    pub fn base_correlation(&self) -> Option<&Arc<BaseCorrelationCurve>> {
        match self {
            Self::BaseCorrelation(curve) => Some(curve),
            _ => None,
        }
    }

    /// Borrow the volatility index curve when the variant matches.
    pub fn vol_index(&self) -> Option<&Arc<VolatilityIndexCurve>> {
        match self {
            Self::VolIndex(curve) => Some(curve),
            _ => None,
        }
    }

    /// Borrow the price curve when the variant matches.
    pub fn price(&self) -> Option<&Arc<PriceCurve>> {
        match self {
            Self::Price(curve) => Some(curve),
            _ => None,
        }
    }

    /// Return `true` when this storage contains a discount curve.
    pub fn is_discount(&self) -> bool {
        matches!(self, Self::Discount(_))
    }
    /// Return `true` when this storage contains a forward curve.
    pub fn is_forward(&self) -> bool {
        matches!(self, Self::Forward(_))
    }
    /// Return `true` when this storage contains a hazard curve.
    pub fn is_hazard(&self) -> bool {
        matches!(self, Self::Hazard(_))
    }
    /// Return `true` when this storage contains an inflation curve.
    pub fn is_inflation(&self) -> bool {
        matches!(self, Self::Inflation(_))
    }
    /// Return `true` when this storage contains a base correlation curve.
    pub fn is_base_correlation(&self) -> bool {
        matches!(self, Self::BaseCorrelation(_))
    }
    /// Return `true` when this storage contains a volatility index curve.
    pub fn is_vol_index(&self) -> bool {
        matches!(self, Self::VolIndex(_))
    }

    /// Return `true` when this storage contains a price curve.
    pub fn is_price(&self) -> bool {
        matches!(self, Self::Price(_))
    }

    /// Return a human-readable curve type (useful for diagnostics/logging).
    pub fn curve_type(&self) -> &'static str {
        match self {
            Self::Discount(_) => "Discount",
            Self::Forward(_) => "Forward",
            Self::Hazard(_) => "Hazard",
            Self::Inflation(_) => "Inflation",
            Self::BaseCorrelation(_) => "BaseCorrelation",
            Self::Price(_) => "Price",
            Self::VolIndex(_) => "VolIndex",
        }
    }

    /// Apply a bump to this curve storage, preserving the original ID.
    ///
    /// After bumping, if the bumped curve has a different ID (e.g., "USD-OIS_bump_+10bp"),
    /// it is rebuilt with the original ID to maintain context consistency.
    ///
    /// # Special Cases
    ///
    /// - `InflationCurve` with `TriangularKeyRate` bump: Custom point-level bumping
    ///   that modifies the CPI level at the target bucket.
    pub(crate) fn apply_bump_preserving_id(
        &self,
        original_id: &CurveId,
        spec: BumpSpec,
    ) -> Result<Self> {
        match self {
            Self::Discount(original) => {
                let bumped = original.apply_bump(spec)?;
                let final_curve = if bumped.id() != original_id {
                    bumped.rebuild_with_id(original_id.clone())?
                } else {
                    bumped
                };
                Ok(Self::Discount(Arc::new(final_curve)))
            }
            Self::Forward(original) => {
                let bumped = original.apply_bump(spec)?;
                let final_curve = if bumped.id() != original_id {
                    bumped.rebuild_with_id(original_id.clone())?
                } else {
                    bumped
                };
                Ok(Self::Forward(Arc::new(final_curve)))
            }
            Self::Hazard(original) => {
                let bumped = original.apply_bump(spec)?;
                let final_curve = if bumped.id() != original_id {
                    bumped.rebuild_with_id(original_id.clone())?
                } else {
                    bumped
                };
                Ok(Self::Hazard(Arc::new(final_curve)))
            }
            Self::Inflation(original) => {
                // Special handling for TriangularKeyRate bumps on InflationCurve
                if let BumpType::TriangularKeyRate { target_bucket, .. } = spec.bump_type {
                    // Only support additive bumps for this special case
                    let (delta, is_multiplicative) =
                        spec.resolve_standard_values().ok_or_else(|| {
                            crate::error::InputError::UnsupportedBump {
                                reason: "InflationCurve key-rate bump requires additive bump"
                                    .to_string(),
                            }
                        })?;

                    if is_multiplicative {
                        return Err(crate::error::InputError::UnsupportedBump {
                            reason:
                                "InflationCurve key-rate bump does not support multiplicative bumps"
                                    .to_string(),
                        }
                        .into());
                    }
                    let mut points: Vec<(f64, f64)> = original
                        .knots()
                        .iter()
                        .copied()
                        .zip(original.cpi_levels().iter().copied())
                        .collect();
                    if let Some((idx, _)) = points.iter().enumerate().min_by(|a, b| {
                        let da = (a.1 .0 - target_bucket).abs();
                        let db = (b.1 .0 - target_bucket).abs();
                        da.total_cmp(&db)
                    }) {
                        points[idx].1 *= 1.0 + delta;
                    }

                    let rebuilt = InflationCurve::builder(original_id.clone())
                        .base_cpi(original.base_cpi())
                        .knots(points)
                        .build()?;
                    return Ok(Self::Inflation(Arc::new(rebuilt)));
                }

                let bumped = original.apply_bump(spec)?;
                let final_curve = if bumped.id() != original_id {
                    bumped.rebuild_with_id(original_id.clone())?
                } else {
                    bumped
                };
                Ok(Self::Inflation(Arc::new(final_curve)))
            }
            Self::BaseCorrelation(original) => {
                let bumped = original.apply_bump(spec)?;
                let final_curve = if bumped.id() != original_id {
                    bumped.rebuild_with_id(original_id.clone())?
                } else {
                    bumped
                };
                Ok(Self::BaseCorrelation(Arc::new(final_curve)))
            }
            Self::VolIndex(original) => {
                let bumped = original.apply_bump(spec)?;
                let final_curve = if bumped.id() != original_id {
                    bumped.rebuild_with_id(original_id.clone())?
                } else {
                    bumped
                };
                Ok(Self::VolIndex(Arc::new(final_curve)))
            }
            Self::Price(original) => {
                let bumped = original.apply_bump(spec)?;
                let final_curve = if bumped.id() != original_id {
                    bumped.rebuild_with_id(original_id.clone())?
                } else {
                    bumped
                };
                Ok(Self::Price(Arc::new(final_curve)))
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Curve Conversions
// -----------------------------------------------------------------------------

impl From<DiscountCurve> for CurveStorage {
    fn from(c: DiscountCurve) -> Self {
        Self::Discount(Arc::new(c))
    }
}
impl From<Arc<DiscountCurve>> for CurveStorage {
    fn from(c: Arc<DiscountCurve>) -> Self {
        Self::Discount(c)
    }
}

impl From<ForwardCurve> for CurveStorage {
    fn from(c: ForwardCurve) -> Self {
        Self::Forward(Arc::new(c))
    }
}
impl From<Arc<ForwardCurve>> for CurveStorage {
    fn from(c: Arc<ForwardCurve>) -> Self {
        Self::Forward(c)
    }
}

impl From<HazardCurve> for CurveStorage {
    fn from(c: HazardCurve) -> Self {
        Self::Hazard(Arc::new(c))
    }
}
impl From<Arc<HazardCurve>> for CurveStorage {
    fn from(c: Arc<HazardCurve>) -> Self {
        Self::Hazard(c)
    }
}

impl From<InflationCurve> for CurveStorage {
    fn from(c: InflationCurve) -> Self {
        Self::Inflation(Arc::new(c))
    }
}
impl From<Arc<InflationCurve>> for CurveStorage {
    fn from(c: Arc<InflationCurve>) -> Self {
        Self::Inflation(c)
    }
}

impl From<BaseCorrelationCurve> for CurveStorage {
    fn from(c: BaseCorrelationCurve) -> Self {
        Self::BaseCorrelation(Arc::new(c))
    }
}
impl From<Arc<BaseCorrelationCurve>> for CurveStorage {
    fn from(c: Arc<BaseCorrelationCurve>) -> Self {
        Self::BaseCorrelation(c)
    }
}

impl From<VolatilityIndexCurve> for CurveStorage {
    fn from(c: VolatilityIndexCurve) -> Self {
        Self::VolIndex(Arc::new(c))
    }
}
impl From<Arc<VolatilityIndexCurve>> for CurveStorage {
    fn from(c: Arc<VolatilityIndexCurve>) -> Self {
        Self::VolIndex(c)
    }
}

impl From<PriceCurve> for CurveStorage {
    fn from(c: PriceCurve) -> Self {
        Self::Price(Arc::new(c))
    }
}
impl From<Arc<PriceCurve>> for CurveStorage {
    fn from(c: Arc<PriceCurve>) -> Self {
        Self::Price(c)
    }
}
