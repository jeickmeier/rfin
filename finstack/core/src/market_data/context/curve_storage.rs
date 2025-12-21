use std::sync::Arc;

use crate::market_data::term_structures::{
    base_correlation::BaseCorrelationCurve, discount_curve::DiscountCurve, forward_curve::ForwardCurve,
    hazard_curve::HazardCurve, inflation::InflationCurve,
};
use crate::types::CurveId;

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

    /// Return a human-readable curve type (useful for diagnostics/logging).
    pub fn curve_type(&self) -> &'static str {
        match self {
            Self::Discount(_) => "Discount",
            Self::Forward(_) => "Forward",
            Self::Hazard(_) => "Hazard",
            Self::Inflation(_) => "Inflation",
            Self::BaseCorrelation(_) => "BaseCorrelation",
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


