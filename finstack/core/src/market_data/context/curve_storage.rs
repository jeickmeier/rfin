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

macro_rules! for_each_context_curve {
    ($macro:ident) => {
        $macro! {
            Discount => { accessor: discount, is_accessor: is_discount, ty: DiscountCurve, type_name: "Discount" },
            Forward => { accessor: forward, is_accessor: is_forward, ty: ForwardCurve, type_name: "Forward" },
            Hazard => { accessor: hazard, is_accessor: is_hazard, ty: HazardCurve, type_name: "Hazard" },
            Inflation => { accessor: inflation, is_accessor: is_inflation, ty: InflationCurve, type_name: "Inflation" },
            BaseCorrelation => {
                accessor: base_correlation,
                is_accessor: is_base_correlation,
                ty: BaseCorrelationCurve,
                type_name: "BaseCorrelation"
            },
            Price => { accessor: price, is_accessor: is_price, ty: PriceCurve, type_name: "Price" },
            VolIndex => { accessor: vol_index, is_accessor: is_vol_index, ty: VolatilityIndexCurve, type_name: "VolIndex" }
        }
    };
}

pub(crate) use for_each_context_curve;

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

macro_rules! impl_simple_rebuildable_with_id {
    ($($ty:ty),* $(,)?) => {
        $(
            impl RebuildableWithId for $ty {
                fn rebuild_with_id(&self, id: CurveId) -> Result<Self> {
                    self.to_builder_with_id(id).build()
                }
            }
        )*
    };
}

impl_simple_rebuildable_with_id!(
    DiscountCurve,
    ForwardCurve,
    HazardCurve,
    InflationCurve,
    VolatilityIndexCurve,
    PriceCurve,
);

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

macro_rules! define_curve_storage {
    ($( $variant:ident => {
        accessor: $accessor:ident,
        is_accessor: $is_accessor:ident,
        ty: $ty:ident,
        type_name: $type_name:literal
    } ),* $(,)?) => {
        /// Unified storage for all curve types using an enum.
        ///
        /// Downstream code rarely manipulates [`CurveStorage`] directly; it mostly
        /// powers [`super::MarketContext`]'s heterogeneous map. When required, the helper
        /// methods expose the inner `Arc` for each concrete curve type.
        #[derive(Clone, Debug)]
        pub enum CurveStorage {
            $(
                #[doc = concat!($type_name, " curve")]
                $variant(Arc<$ty>),
            )*
        }

        impl CurveStorage {
            /// Return the curve's unique identifier.
            pub fn id(&self) -> &CurveId {
                match self {
                    $( Self::$variant(curve) => curve.id(), )*
                }
            }

            $(
                #[doc = concat!("Borrow the ", $type_name, " curve when the variant matches.")]
                pub fn $accessor(&self) -> Option<&Arc<$ty>> {
                    match self {
                        Self::$variant(curve) => Some(curve),
                        _ => None,
                    }
                }

                #[doc = concat!("Return `true` when this storage contains a ", $type_name, " curve.")]
                pub fn $is_accessor(&self) -> bool {
                    matches!(self, Self::$variant(_))
                }
            )*

            /// Return a human-readable curve type (useful for diagnostics/logging).
            pub fn curve_type(&self) -> &'static str {
                match self {
                    $( Self::$variant(_) => $type_name, )*
                }
            }
        }

        $(
            impl From<$ty> for CurveStorage {
                fn from(curve: $ty) -> Self {
                    Self::$variant(Arc::new(curve))
                }
            }

            impl From<Arc<$ty>> for CurveStorage {
                fn from(curve: Arc<$ty>) -> Self {
                    Self::$variant(curve)
                }
            }
        )*
    };
}

for_each_context_curve!(define_curve_storage);

impl CurveStorage {
    /// Roll this curve storage forward by the provided number of days.
    pub(crate) fn roll_forward_storage(&self, days: i64) -> Result<Self> {
        match self {
            Self::Discount(curve) => Ok(Self::Discount(Arc::new(curve.roll_forward(days)?))),
            Self::Forward(curve) => Ok(Self::Forward(Arc::new(curve.roll_forward(days)?))),
            Self::Hazard(curve) => Ok(Self::Hazard(Arc::new(curve.roll_forward(days)?))),
            Self::Inflation(curve) => Ok(Self::Inflation(Arc::new(curve.roll_forward(days)?))),
            Self::BaseCorrelation(curve) => Ok(Self::BaseCorrelation(Arc::clone(curve))),
            Self::Price(curve) => Ok(Self::Price(Arc::new(curve.roll_forward(days)?))),
            Self::VolIndex(curve) => Ok(Self::VolIndex(Arc::new(curve.roll_forward(days)?))),
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
        &mut self,
        original_id: &CurveId,
        spec: BumpSpec,
    ) -> Result<()> {
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
            Self::Discount(arc) => {
                // In-place bump: Arc::make_mut deep-clones only if refcount > 1
                Arc::make_mut(arc).bump_in_place(&spec)?;
                Ok(())
            }
            Self::Forward(arc) => {
                Arc::make_mut(arc).bump_in_place(&spec)?;
                Ok(())
            }
            Self::Hazard(arc) => {
                Arc::make_mut(arc).bump_in_place(&spec)?;
                Ok(())
            }
            Self::Inflation(original) => {
                // Special handling for TriangularKeyRate bumps on InflationCurve
                if let BumpType::TriangularKeyRate { target_bucket, .. } = spec.bump_type {
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
                    *self = Self::Inflation(Arc::new(rebuilt));
                    return Ok(());
                }

                let curve = bump_curve_preserving_id(
                    original.as_ref(),
                    original_id,
                    spec,
                    InflationCurve::id,
                )?;
                *self = Self::Inflation(Arc::new(curve));
                Ok(())
            }
            Self::BaseCorrelation(original) => {
                let curve = bump_curve_preserving_id(
                    original.as_ref(),
                    original_id,
                    spec,
                    BaseCorrelationCurve::id,
                )?;
                *self = Self::BaseCorrelation(Arc::new(curve));
                Ok(())
            }
            Self::VolIndex(original) => {
                let curve = bump_curve_preserving_id(
                    original.as_ref(),
                    original_id,
                    spec,
                    VolatilityIndexCurve::id,
                )?;
                *self = Self::VolIndex(Arc::new(curve));
                Ok(())
            }
            Self::Price(original) => {
                let curve =
                    bump_curve_preserving_id(original.as_ref(), original_id, spec, PriceCurve::id)?;
                *self = Self::Price(Arc::new(curve));
                Ok(())
            }
        }
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

        let mut storage = CurveStorage::from(curve);
        storage
            .apply_bump_preserving_id(&CurveId::from("FWD"), BumpSpec::parallel_bp(1.0))
            .expect("bump succeeds");
        let bumped_curve = storage.forward().expect("forward curve");
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

        let mut storage = CurveStorage::from(curve);
        storage
            .apply_bump_preserving_id(&CurveId::from("CPI"), BumpSpec::inflation_shift_pct(1.0))
            .expect("bump succeeds");
        let bumped_curve = storage.inflation().expect("inflation curve");
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

        let mut storage = CurveStorage::from(curve);
        storage
            .apply_bump_preserving_id(&CurveId::from("DISC"), BumpSpec::parallel_bp(1.0))
            .expect("bump succeeds");
        let bumped_curve = storage.discount().expect("discount curve");
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
