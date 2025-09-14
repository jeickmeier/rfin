//! Enum-based storage for all curve types
//!
//! This module provides `CurveStorage` which uses concrete enum storage  
//! for optimal performance and complete serialization support.

extern crate alloc;
use alloc::sync::Arc;

use crate::market_data::term_structures::{
    base_correlation::BaseCorrelationCurve,
    discount_curve::DiscountCurve,
    forward_curve::ForwardCurve,
    hazard_curve::HazardCurve,
    inflation::InflationCurve,
};
use crate::market_data::traits::TermStructure;
use crate::types::CurveId;

/// Unified storage for all curve types
///
/// This enum replaces trait object storage (`Arc<dyn Trait>`) with concrete
/// type storage, enabling full serialization support and direct type access
/// with zero overhead.
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

    /// Get discount curve if this storage contains one
    pub fn discount(&self) -> Option<&Arc<DiscountCurve>> {
        match self {
            Self::Discount(curve) => Some(curve),
            _ => None,
        }
    }

    /// Get forward curve if this storage contains one
    pub fn forward(&self) -> Option<&Arc<ForwardCurve>> {
        match self {
            Self::Forward(curve) => Some(curve),
            _ => None,
        }
    }

    /// Get hazard curve if this storage contains one
    pub fn hazard(&self) -> Option<&Arc<HazardCurve>> {
        match self {
            Self::Hazard(curve) => Some(curve),
            _ => None,
        }
    }

    /// Get inflation curve if this storage contains one
    pub fn inflation(&self) -> Option<&Arc<InflationCurve>> {
        match self {
            Self::Inflation(curve) => Some(curve),
            _ => None,
        }
    }

    /// Get base correlation curve if this storage contains one
    pub fn base_correlation(&self) -> Option<&Arc<BaseCorrelationCurve>> {
        match self {
            Self::BaseCorrelation(curve) => Some(curve),
            _ => None,
        }
    }

    /// Extract discount curve, consuming the storage
    pub fn into_discount(self) -> Option<Arc<DiscountCurve>> {
        match self {
            Self::Discount(curve) => Some(curve),
            _ => None,
        }
    }

    /// Extract forward curve, consuming the storage  
    pub fn into_forward(self) -> Option<Arc<ForwardCurve>> {
        match self {
            Self::Forward(curve) => Some(curve),
            _ => None,
        }
    }

    /// Extract hazard curve, consuming the storage
    pub fn into_hazard(self) -> Option<Arc<HazardCurve>> {
        match self {
            Self::Hazard(curve) => Some(curve),
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
    pub fn new_discount(curve: DiscountCurve) -> Self {
        Self::Discount(Arc::new(curve))
    }

    /// Create storage for a forward curve
    pub fn new_forward(curve: ForwardCurve) -> Self {
        Self::Forward(Arc::new(curve))
    }

    /// Create storage for a hazard curve
    pub fn new_hazard(curve: HazardCurve) -> Self {
        Self::Hazard(Arc::new(curve))
    }

    /// Create storage for an inflation curve
    pub fn new_inflation(curve: InflationCurve) -> Self {
        Self::Inflation(Arc::new(curve))
    }

    /// Create storage for a base correlation curve
    pub fn new_base_correlation(curve: BaseCorrelationCurve) -> Self {
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
        let storage = CurveStorage::new_discount(disc_curve);
        
        assert!(storage.is_discount());
        assert!(!storage.is_forward());
        assert_eq!(storage.curve_type(), "Discount");
        assert_eq!(storage.id().as_str(), "TEST-DISC");
    }

    #[test]
    fn test_direct_access() {
        let disc_curve = test_discount_curve();
        let storage = CurveStorage::new_discount(disc_curve);
        
        // Direct concrete access
        let discount_curve = storage.discount().unwrap();
        assert_eq!(discount_curve.id().as_str(), "TEST-DISC");
        assert!((discount_curve.df(1.0) - 0.95).abs() < 1e-12);
        
        // Should not access other types
        assert!(storage.forward().is_none());
        assert!(storage.hazard().is_none());
    }

    #[test]
    fn test_extraction() {
        let disc_curve = test_discount_curve();
        let storage = CurveStorage::new_discount(disc_curve);
        
        // Extract the curve by consuming storage
        let extracted = storage.into_discount().unwrap();
        assert_eq!(extracted.id().as_str(), "TEST-DISC");
        assert!((extracted.df(1.0) - 0.95).abs() < 1e-12);
    }

    #[test]
    fn test_all_curve_types() {
        let disc = CurveStorage::new_discount(test_discount_curve());
        let fwd = CurveStorage::new_forward(test_forward_curve());
        let hazard = CurveStorage::new_hazard(test_hazard_curve());
        
        assert!(disc.is_discount());
        assert!(fwd.is_forward());
        assert!(hazard.is_hazard());
        
        assert_eq!(disc.curve_type(), "Discount");
        assert_eq!(fwd.curve_type(), "Forward");
        assert_eq!(hazard.curve_type(), "Hazard");
    }
}
