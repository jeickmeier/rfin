//! Enum-based storage for all curve types
//!
//! This module provides `CurveStorage` which replaces trait object storage
//! (`Arc<dyn Trait + Send + Sync>`) with a concrete enum, enabling complete
//! serialization support while maintaining API compatibility.

extern crate alloc;
use alloc::sync::Arc;

use crate::market_data::term_structures::{
    base_correlation::BaseCorrelationCurve,
    discount_curve::DiscountCurve,
    forward_curve::ForwardCurve,
    hazard_curve::HazardCurve,
    inflation::InflationCurve,
};
use crate::market_data::traits::{Discount, Forward, Inflation as InflationTrait, Survival, TermStructure};
use crate::types::CurveId;

/// Unified storage for all curve types
///
/// This enum replaces trait object storage (`Arc<dyn Trait>`) with concrete
/// type storage, enabling full serialization support while maintaining
/// backward API compatibility through conversion methods.
#[derive(Clone, Debug)]
pub enum CurveStorage {
    /// Discount factor curve
    Discount(Arc<DiscountCurve>),
    /// Forward rate curve
    Forward(Arc<ForwardCurve>),
    /// Credit hazard/survival curve
    Hazard(Arc<HazardCurve>),
    /// Inflation/CPI curve
    Inflation(Arc<InflationCurve>),
    /// Base correlation curve for credit tranche pricing
    BaseCorrelation(Arc<BaseCorrelationCurve>),
}

impl CurveStorage {
    /// Get the curve's unique identifier
    #[inline]
    pub fn id(&self) -> &CurveId {
        match self {
            Self::Discount(c) => c.id(),
            Self::Forward(c) => c.id(),
            Self::Hazard(c) => c.id(),
            Self::Inflation(c) => c.id(),
            Self::BaseCorrelation(c) => c.id(),
        }
    }

    /// Get as discount curve trait (backward compatibility)
    pub fn as_discount(&self) -> Option<Arc<dyn Discount + Send + Sync>> {
        match self {
            Self::Discount(curve) => Some(curve.clone() as Arc<dyn Discount + Send + Sync>),
            _ => None,
        }
    }

    /// Get as forward curve trait (backward compatibility)
    pub fn as_forward(&self) -> Option<Arc<dyn Forward + Send + Sync>> {
        match self {
            Self::Forward(curve) => Some(curve.clone() as Arc<dyn Forward + Send + Sync>),
            _ => None,
        }
    }

    /// Get as survival curve trait (backward compatibility)
    pub fn as_survival(&self) -> Option<Arc<dyn Survival + Send + Sync>> {
        match self {
            Self::Hazard(curve) => Some(curve.clone() as Arc<dyn Survival + Send + Sync>),
            _ => None,
        }
    }

    /// Get as inflation curve trait (backward compatibility)
    pub fn as_inflation(&self) -> Option<Arc<dyn InflationTrait + Send + Sync>> {
        match self {
            Self::Inflation(curve) => Some(curve.clone() as Arc<dyn InflationTrait + Send + Sync>),
            _ => None,
        }
    }

    /// Get concrete discount curve (advanced use cases)
    pub fn as_concrete_discount(&self) -> Option<&Arc<DiscountCurve>> {
        match self {
            Self::Discount(curve) => Some(curve),
            _ => None,
        }
    }

    /// Get concrete forward curve (advanced use cases)
    pub fn as_concrete_forward(&self) -> Option<&Arc<ForwardCurve>> {
        match self {
            Self::Forward(curve) => Some(curve),
            _ => None,
        }
    }

    /// Get concrete hazard curve (advanced use cases)
    pub fn as_concrete_hazard(&self) -> Option<&Arc<HazardCurve>> {
        match self {
            Self::Hazard(curve) => Some(curve),
            _ => None,
        }
    }

    /// Get concrete inflation curve (advanced use cases)
    pub fn as_concrete_inflation(&self) -> Option<&Arc<InflationCurve>> {
        match self {
            Self::Inflation(curve) => Some(curve),
            _ => None,
        }
    }

    /// Get concrete base correlation curve (advanced use cases)
    pub fn as_concrete_base_correlation(&self) -> Option<&Arc<BaseCorrelationCurve>> {
        match self {
            Self::BaseCorrelation(curve) => Some(curve),
            _ => None,
        }
    }

    /// Check if this storage contains a specific curve type
    pub fn is_discount(&self) -> bool {
        matches!(self, Self::Discount(_))
    }

    /// Check if this storage contains a forward curve
    pub fn is_forward(&self) -> bool {
        matches!(self, Self::Forward(_))
    }

    /// Check if this storage contains a hazard curve
    pub fn is_hazard(&self) -> bool {
        matches!(self, Self::Hazard(_))
    }

    /// Check if this storage contains an inflation curve
    pub fn is_inflation(&self) -> bool {
        matches!(self, Self::Inflation(_))
    }

    /// Check if this storage contains a base correlation curve
    pub fn is_base_correlation(&self) -> bool {
        matches!(self, Self::BaseCorrelation(_))
    }

    /// Get the curve type as a string (for debugging/logging)
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

impl TermStructure for CurveStorage {
    #[inline]
    fn id(&self) -> &CurveId {
        self.id()
    }
}

// Convenience constructors
impl CurveStorage {
    /// Create storage for a discount curve
    pub fn discount(curve: DiscountCurve) -> Self {
        Self::Discount(Arc::new(curve))
    }

    /// Create storage for a forward curve
    pub fn forward(curve: ForwardCurve) -> Self {
        Self::Forward(Arc::new(curve))
    }

    /// Create storage for a hazard curve
    pub fn hazard(curve: HazardCurve) -> Self {
        Self::Hazard(Arc::new(curve))
    }

    /// Create storage for an inflation curve
    pub fn inflation(curve: InflationCurve) -> Self {
        Self::Inflation(Arc::new(curve))
    }

    /// Create storage for a base correlation curve
    pub fn base_correlation(curve: BaseCorrelationCurve) -> Self {
        Self::BaseCorrelation(Arc::new(curve))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::Date;
    use crate::market_data::interp::InterpStyle;

    fn test_discount_curve() -> DiscountCurve {
        DiscountCurve::builder("TEST-DISC")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap()
    }

    fn test_forward_curve() -> ForwardCurve {
        ForwardCurve::builder("TEST-FWD", 0.25)
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .knots([(0.0, 0.03), (1.0, 0.035), (5.0, 0.04)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap()
    }

    fn test_hazard_curve() -> HazardCurve {
        HazardCurve::builder("TEST-HAZARD")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .recovery_rate(0.4)
            .knots([(0.0, 0.01), (1.0, 0.015), (5.0, 0.02)])
            .build()
            .unwrap()
    }

    #[test]
    fn test_curve_storage_creation() {
        let disc_curve = test_discount_curve();
        let storage = CurveStorage::discount(disc_curve);
        
        assert!(storage.is_discount());
        assert!(!storage.is_forward());
        assert_eq!(storage.curve_type(), "Discount");
        assert_eq!(storage.id().as_str(), "TEST-DISC");
    }

    #[test]
    fn test_trait_conversion() {
        let disc_curve = test_discount_curve();
        let storage = CurveStorage::discount(disc_curve);
        
        let discount_trait = storage.as_discount().unwrap();
        assert!((discount_trait.df(1.0) - 0.95).abs() < 1e-12);
        
        // Should not convert to other types
        assert!(storage.as_forward().is_none());
        assert!(storage.as_survival().is_none());
    }

    #[test]
    fn test_concrete_access() {
        let disc_curve = test_discount_curve();
        let storage = CurveStorage::discount(disc_curve);
        
        let concrete = storage.as_concrete_discount().unwrap();
        assert_eq!(concrete.id().as_str(), "TEST-DISC");
        assert!((concrete.df(1.0) - 0.95).abs() < 1e-12);
    }

    #[test]
    fn test_all_curve_types() {
        let disc = CurveStorage::discount(test_discount_curve());
        let fwd = CurveStorage::forward(test_forward_curve());
        let hazard = CurveStorage::hazard(test_hazard_curve());
        
        assert!(disc.is_discount());
        assert!(fwd.is_forward());
        assert!(hazard.is_hazard());
        
        assert_eq!(disc.curve_type(), "Discount");
        assert_eq!(fwd.curve_type(), "Forward");
        assert_eq!(hazard.curve_type(), "Hazard");
    }
}
