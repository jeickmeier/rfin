//! Serializable state representations for all curve types
//!
//! This module provides the `CurveState` enum and related types for
//! serializing curve data extracted from `CurveStorage`.

extern crate alloc;

#[cfg(feature = "serde")]
use alloc::sync::Arc;
#[cfg(feature = "serde")]
use crate::market_data::term_structures::{
    base_correlation::BaseCorrelationCurve,
    discount_curve::{DiscountCurve, DiscountCurveState},
    forward_curve::{ForwardCurve, ForwardCurveState},
    hazard_curve::{HazardCurve, HazardCurveState},
    inflation::InflationCurve,
};

#[cfg(feature = "serde")]
use super::curve_storage::CurveStorage;

/// Serializable state representation for any curve type
///
/// This enum provides a unified serialization format for all curves,
/// enabling complete serialization without trait object limitations.
#[cfg(feature = "serde")]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "snake_case"))]
pub enum CurveState {
    /// Discount curve state
    Discount(DiscountCurveState),
    /// Forward curve state
    Forward(ForwardCurveState),
    /// Hazard curve state
    Hazard(HazardCurveState),
    /// Inflation curve state (special handling due to direct Serialize)
    Inflation(InflationCurveData),
    /// Base correlation curve (already serializable)
    BaseCorrelation(BaseCorrelationCurve),
}

/// Wrapper for InflationCurve serialization
///
/// InflationCurve implements Serialize directly but we need a consistent
/// state pattern. This wrapper extracts the necessary data.
#[cfg(feature = "serde")]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InflationCurveData {
    /// Curve identifier
    pub id: String,
    /// Base CPI level
    pub base_cpi: crate::F,
    /// Time/CPI level pairs
    pub knot_points: Vec<(crate::F, crate::F)>,
    /// Interpolation style
    pub interp_style: crate::market_data::interp::InterpStyle,
}

#[cfg(feature = "serde")]
impl CurveStorage {
    /// Convert to serializable state
    pub fn to_state(&self) -> crate::Result<CurveState> {
        Ok(match self {
            Self::Discount(curve) => CurveState::Discount(curve.to_state()),
            Self::Forward(curve) => CurveState::Forward(curve.to_state()),
            Self::Hazard(curve) => CurveState::Hazard(curve.to_state()),
            Self::Inflation(curve) => {
                // Extract inflation curve data
                let knot_points: Vec<(crate::F, crate::F)> = curve
                    .knots()
                    .iter()
                    .zip(curve.cpi_levels().iter())
                    .map(|(&t, &cpi)| (t, cpi))
                    .collect();

                CurveState::Inflation(InflationCurveData {
                    id: curve.id().to_string(),
                    base_cpi: curve.base_cpi(),
                    knot_points,
                    interp_style: crate::market_data::interp::InterpStyle::LogLinear, // Default for inflation
                })
            }
            Self::BaseCorrelation(curve) => {
                CurveState::BaseCorrelation((**curve).clone())
            }
        })
    }

    /// Reconstruct from serializable state
    pub fn from_state(state: CurveState) -> crate::Result<Self> {
        Ok(match state {
            CurveState::Discount(s) => {
                Self::Discount(Arc::new(DiscountCurve::from_state(s).map_err(|_| crate::Error::Internal)?))
            }
            CurveState::Forward(s) => {
                Self::Forward(Arc::new(ForwardCurve::from_state(s)?))
            }
            CurveState::Hazard(s) => {
                Self::Hazard(Arc::new(HazardCurve::from_state(s)?))
            }
            CurveState::Inflation(s) => {
                let curve = InflationCurve::builder(s.id)
                    .base_cpi(s.base_cpi)
                    .knots(s.knot_points)
                    .set_interp(s.interp_style)
                    .build()?;
                Self::Inflation(Arc::new(curve))
            }
            CurveState::BaseCorrelation(c) => {
                Self::BaseCorrelation(Arc::new(c))
            }
        })
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for CurveStorage {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_state()
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for CurveStorage {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let state = CurveState::deserialize(deserializer)?;
        Self::from_state(state).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use crate::dates::Date;
    use crate::market_data::{
        interp::InterpStyle,
        storage::CurveStorage,
        term_structures::{
            discount_curve::DiscountCurve,
            hazard_curve::HazardCurve,
        },
    };

    fn test_discount_curve() -> DiscountCurve {
        DiscountCurve::builder("TEST-DISC")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
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
    #[cfg(feature = "serde")]
    fn test_state_round_trip_discount() {
        let curve = test_discount_curve();
        let storage = CurveStorage::new_discount(curve);
        
        // Convert to state and back
        let state = storage.to_state().unwrap();
        let restored = CurveStorage::from_state(state).unwrap();
        
        // Verify IDs match
        assert_eq!(storage.id(), restored.id());
        
        // Verify type is preserved
        assert!(restored.is_discount());
        
        // Verify functionality is preserved
        let original_curve = storage.discount().unwrap();
        let restored_curve = restored.discount().unwrap();
        assert!((original_curve.df(1.0) - restored_curve.df(1.0)).abs() < 1e-12);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_state_round_trip_hazard() {
        let curve = test_hazard_curve();
        let storage = CurveStorage::new_hazard(curve);
        
        // Convert to state and back
        let state = storage.to_state().unwrap();
        let restored = CurveStorage::from_state(state).unwrap();
        
        // Verify preservation
        assert_eq!(storage.id(), restored.id());
        assert!(restored.is_hazard());
        
        let original_curve = storage.hazard().unwrap();
        let restored_curve = restored.hazard().unwrap();
        assert!((original_curve.sp(1.0) - restored_curve.sp(1.0)).abs() < 1e-12);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_json_serialization() {
        let curve = test_discount_curve();
        let storage = CurveStorage::new_discount(curve);
        
        // Serialize to JSON
        let json = serde_json::to_string(&storage).unwrap();
        
        // Verify it contains expected structure
        assert!(json.contains("\"type\":\"discount\""));
        assert!(json.contains("TEST-DISC"));
        
        // Deserialize and verify
        let restored: CurveStorage = serde_json::from_str(&json).unwrap();
        assert_eq!(storage.id(), restored.id());
        assert!(restored.is_discount());
    }

    #[test]
    fn test_type_checking() {
        let disc = CurveStorage::new_discount(test_discount_curve());
        let hazard = CurveStorage::new_hazard(test_hazard_curve());
        
        // Type checks
        assert!(disc.is_discount());
        assert!(!disc.is_hazard());
        assert!(hazard.is_hazard());
        assert!(!hazard.is_discount());
        
        // Curve type strings
        assert_eq!(disc.curve_type(), "Discount");
        assert_eq!(hazard.curve_type(), "Hazard");
    }

    #[test]
    fn test_access_safety() {
        let storage = CurveStorage::new_discount(test_discount_curve());
        
        // Should access correct type
        assert!(storage.discount().is_some());
        
        // Should not access incorrect types
        assert!(storage.forward().is_none());
        assert!(storage.hazard().is_none());
        assert!(storage.inflation().is_none());
    }
}
