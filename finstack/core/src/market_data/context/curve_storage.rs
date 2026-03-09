//! Internal curve storage helpers for [`MarketContext`](super::MarketContext).
//!
//! This submodule contains the enum-based storage and reconstruction helpers that
//! let the public context API handle heterogeneous curve types through a single
//! internal representation.

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
        self.to_builder_with_id(id).build()
    }
}

impl RebuildableWithId for ForwardCurve {
    fn rebuild_with_id(&self, id: CurveId) -> Result<Self> {
        self.to_builder_with_id(id).build()
    }
}

impl RebuildableWithId for HazardCurve {
    fn rebuild_with_id(&self, id: CurveId) -> Result<Self> {
        self.to_builder_with_id(id).build()
    }
}

impl RebuildableWithId for InflationCurve {
    fn rebuild_with_id(&self, id: CurveId) -> Result<Self> {
        self.to_builder_with_id(id).build()
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
        self.to_builder_with_id(id).build()
    }
}

impl RebuildableWithId for PriceCurve {
    fn rebuild_with_id(&self, id: CurveId) -> Result<Self> {
        self.to_builder_with_id(id).build()
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
        fn bump_curve_preserving_id<C>(
            original: &C,
            original_id: &CurveId,
            spec: BumpSpec,
            id_of: fn(&C) -> &CurveId,
        ) -> Result<C>
        where
            C: Bumpable + RebuildableWithId,
        {
            let bumped = original.apply_bump(spec)?;
            if id_of(&bumped) != original_id {
                bumped.rebuild_with_id(original_id.clone())
            } else {
                Ok(bumped)
            }
        }

        match self {
            Self::Discount(original) => {
                let curve = bump_curve_preserving_id(
                    original.as_ref(),
                    original_id,
                    spec,
                    DiscountCurve::id,
                )?;
                Ok(Self::Discount(Arc::new(curve)))
            }
            Self::Forward(original) => {
                let curve = bump_curve_preserving_id(
                    original.as_ref(),
                    original_id,
                    spec,
                    ForwardCurve::id,
                )?;
                Ok(Self::Forward(Arc::new(curve)))
            }
            Self::Hazard(original) => {
                let curve = bump_curve_preserving_id(
                    original.as_ref(),
                    original_id,
                    spec,
                    HazardCurve::id,
                )?;
                Ok(Self::Hazard(Arc::new(curve)))
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
                        .base_date(original.base_date())
                        .day_count(original.day_count())
                        .indexation_lag_months(original.indexation_lag_months())
                        .knots(points)
                        .interp(original.interp_style())
                        .build()?;
                    return Ok(Self::Inflation(Arc::new(rebuilt)));
                }

                let curve = bump_curve_preserving_id(
                    original.as_ref(),
                    original_id,
                    spec,
                    InflationCurve::id,
                )?;
                Ok(Self::Inflation(Arc::new(curve)))
            }
            Self::BaseCorrelation(original) => {
                let curve = bump_curve_preserving_id(
                    original.as_ref(),
                    original_id,
                    spec,
                    BaseCorrelationCurve::id,
                )?;
                Ok(Self::BaseCorrelation(Arc::new(curve)))
            }
            Self::VolIndex(original) => {
                let curve = bump_curve_preserving_id(
                    original.as_ref(),
                    original_id,
                    spec,
                    VolatilityIndexCurve::id,
                )?;
                Ok(Self::VolIndex(Arc::new(curve)))
            }
            Self::Price(original) => {
                let curve =
                    bump_curve_preserving_id(original.as_ref(), original_id, spec, PriceCurve::id)?;
                Ok(Self::Price(Arc::new(curve)))
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::dates::{Date, DayCount};
    use crate::math::interp::{ExtrapolationPolicy, InterpStyle};
    use serde_json::Value;
    use time::Month;

    fn test_date() -> Date {
        Date::from_calendar_date(2025, Month::January, 1).expect("valid test date")
    }

    fn json(curve: &impl serde::Serialize) -> Value {
        serde_json::to_value(curve).expect("curve should serialize")
    }

    #[test]
    fn forward_bump_preserves_interp_and_extrapolation() {
        let curve = ForwardCurve::builder("FWD", 0.25)
            .base_date(test_date())
            .reset_lag(0)
            .day_count(DayCount::Act365F)
            .interp(InterpStyle::LogLinear)
            .extrapolation(ExtrapolationPolicy::FlatZero)
            .knots([(0.5, 0.02), (1.0, 0.025), (2.0, 0.03)])
            .build()
            .expect("curve builds");
        let original = json(&curve);

        let storage = CurveStorage::from(curve);
        let bumped = storage
            .apply_bump_preserving_id(&CurveId::from("FWD"), BumpSpec::parallel_bp(1.0))
            .expect("bump succeeds");
        let bumped_curve = bumped.forward().expect("forward curve");
        let bumped_json = json(bumped_curve.as_ref());

        assert_eq!(bumped_curve.interp_style(), InterpStyle::LogLinear);
        assert_eq!(bumped_json["reset_lag"], original["reset_lag"]);
        assert_eq!(bumped_json["day_count"], original["day_count"]);
        assert_eq!(bumped_json["interp_style"], original["interp_style"]);
        assert_eq!(bumped_json["extrapolation"], original["extrapolation"]);
    }

    #[test]
    fn inflation_bump_preserves_lag_day_count_and_interp() {
        let curve = InflationCurve::builder("CPI")
            .base_date(test_date())
            .base_cpi(300.0)
            .day_count(DayCount::Act360)
            .indexation_lag_months(2)
            .interp(InterpStyle::LogLinear)
            .knots([(0.0, 300.0), (5.0, 325.0), (10.0, 350.0)])
            .build()
            .expect("curve builds");
        let original = json(&curve);

        let storage = CurveStorage::from(curve);
        let bumped = storage
            .apply_bump_preserving_id(&CurveId::from("CPI"), BumpSpec::inflation_shift_pct(1.0))
            .expect("bump succeeds");
        let bumped_curve = bumped.inflation().expect("inflation curve");
        let bumped_json = json(bumped_curve.as_ref());

        assert_eq!(bumped_curve.day_count(), DayCount::Act360);
        assert_eq!(bumped_curve.indexation_lag_months(), 2);
        assert_eq!(bumped_curve.interp_style(), InterpStyle::LogLinear);
        assert_eq!(bumped_json["base_date"], original["base_date"]);
        assert_eq!(bumped_json["day_count"], original["day_count"]);
        assert_eq!(
            bumped_json["indexation_lag_months"],
            original["indexation_lag_months"]
        );
        assert_eq!(bumped_json["interp_style"], original["interp_style"]);
        assert_eq!(bumped_json["extrapolation"], original["extrapolation"]);
    }

    #[test]
    fn discount_bump_preserves_forward_controls() {
        let curve = DiscountCurve::builder("DISC")
            .base_date(test_date())
            .day_count(DayCount::Act365F)
            .interp(InterpStyle::Linear)
            .extrapolation(ExtrapolationPolicy::FlatForward)
            .knots([(0.5, 1.0), (1.0, 1.001), (2.0, 1.002)])
            .allow_non_monotonic_with_floor()
            .min_forward_tenor(1e-8)
            .build()
            .expect("curve builds");
        let original = json(&curve);

        let storage = CurveStorage::from(curve);
        let bumped = storage
            .apply_bump_preserving_id(&CurveId::from("DISC"), BumpSpec::parallel_bp(1.0))
            .expect("bump succeeds");
        let bumped_curve = bumped.discount().expect("discount curve");
        let bumped_json = json(bumped_curve.as_ref());

        assert_eq!(bumped_curve.interp_style(), InterpStyle::Linear);
        assert_eq!(
            bumped_json["allow_non_monotonic"],
            original["allow_non_monotonic"]
        );
        assert_eq!(
            bumped_json["min_forward_rate"],
            original["min_forward_rate"]
        );
        assert_eq!(
            bumped_json["min_forward_tenor"],
            original["min_forward_tenor"]
        );
    }
}
