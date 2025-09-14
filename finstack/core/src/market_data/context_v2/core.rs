//! MarketContext V2 - Core implementation with enum-based storage
//!
//! This module provides the new MarketContext implementation that uses
//! enum-based storage instead of trait objects, enabling complete serialization.

extern crate alloc;
use alloc::sync::Arc;
use hashbrown::HashMap;

use crate::market_data::storage::CurveStorage;
use crate::market_data::{
    credit_index::CreditIndexData,
    inflation_index::InflationIndex,
    primitives::{MarketScalar, ScalarTimeSeries},
    surfaces::vol_surface::VolSurface,
    term_structures::{
        base_correlation::BaseCorrelationCurve,
        discount_curve::DiscountCurve,
        forward_curve::ForwardCurve,
        hazard_curve::HazardCurve,
        inflation::InflationCurve,
    },
    traits::TermStructure,
};
use crate::money::fx::FxMatrix;
use crate::types::CurveId;
use crate::{error::InputError, Result};

/// MarketContext with enum-based storage
///
/// This is the main MarketContext implementation that uses `CurveStorage`
/// enum instead of trait objects, enabling complete serialization support.
///
/// # Key Features
/// - Complete serialization for all curve types
/// - Direct concrete type access with zero overhead
/// - Type-safe curve access with compile-time guarantees
/// - No string parsing for curve identification
/// - Clean, simple API with no confusion
///
/// # Example
/// ```rust,ignore
/// use finstack_core::market_data::MarketContext;
/// 
/// let context = MarketContext::new()
///     .insert_discount(discount_curve)
///     .insert_forward(forward_curve);
/// 
/// // Clean, direct API - returns concrete types
/// let disc = context.discount("USD-OIS")?;      // Arc<DiscountCurve>
/// let fwd = context.forward("USD-SOFR3M")?;     // Arc<ForwardCurve>
/// ```
#[derive(Clone, Default)]
pub struct MarketContext {
    /// All curves stored in unified enum-based map
    pub(super) curves: HashMap<CurveId, CurveStorage>,
    
    /// Foreign-exchange matrix
    pub fx: Option<Arc<FxMatrix>>,
    
    /// Volatility surfaces
    pub surfaces: HashMap<CurveId, Arc<VolSurface>>,
    
    /// Market scalars and prices
    pub prices: HashMap<CurveId, MarketScalar>,
    
    /// Generic time series
    pub series: HashMap<CurveId, ScalarTimeSeries>,
    
    /// Inflation indices
    pub(super) inflation_indices: HashMap<CurveId, Arc<InflationIndex>>,
    
    /// Credit index aggregates
    pub(super) credit_indices: HashMap<CurveId, Arc<CreditIndexData>>,
    
    /// Collateral CSA code mappings
    pub(super) collateral: HashMap<String, CurveId>,
}

impl MarketContext {
    /// Create an empty market context
    pub fn new() -> Self {
        Self::default()
    }

    // -----------------------------------------------------------------------------
    // Insertion Methods
    // -----------------------------------------------------------------------------

    /// Insert a discount curve
    pub fn insert_discount(mut self, curve: DiscountCurve) -> Self {
        let id = curve.id().clone();
        self.curves.insert(id, CurveStorage::new_discount(curve));
        self
    }

    /// Insert a forward curve
    pub fn insert_forward(mut self, curve: ForwardCurve) -> Self {
        let id = curve.id().clone();
        self.curves.insert(id, CurveStorage::new_forward(curve));
        self
    }

    /// Insert a hazard curve
    pub fn insert_hazard(mut self, curve: HazardCurve) -> Self {
        let id = curve.id().clone();
        self.curves.insert(id, CurveStorage::new_hazard(curve));
        self
    }

    /// Insert an inflation curve
    pub fn insert_inflation(mut self, curve: InflationCurve) -> Self {
        let id = curve.id().clone();
        self.curves.insert(id, CurveStorage::new_inflation(curve));
        self
    }

    /// Insert a base correlation curve
    pub fn insert_base_correlation(mut self, curve: BaseCorrelationCurve) -> Self {
        let id = curve.id().clone();
        self.curves.insert(id, CurveStorage::new_base_correlation(curve));
        self
    }

    /// Insert a volatility surface
    pub fn insert_surface(mut self, surface: VolSurface) -> Self {
        let id = surface.id().clone();
        self.surfaces.insert(id, Arc::new(surface));
        self
    }

    /// Insert a market scalar/price
    pub fn insert_price(mut self, id: impl AsRef<str>, price: MarketScalar) -> Self {
        self.prices.insert(CurveId::from(id.as_ref()), price);
        self
    }

    /// Insert a scalar time series
    pub fn insert_series(mut self, series: ScalarTimeSeries) -> Self {
        let id = series.id().clone();
        self.series.insert(id, series);
        self
    }

    /// Insert an inflation index
    pub fn insert_inflation_index(mut self, id: impl AsRef<str>, index: InflationIndex) -> Self {
        let curve_id = CurveId::from(id.as_ref());
        self.inflation_indices.insert(curve_id, Arc::new(index));
        self
    }

    /// Insert a credit index
    pub fn insert_credit_index(mut self, id: impl AsRef<str>, data: CreditIndexData) -> Self {
        let curve_id = CurveId::from(id.as_ref());
        self.credit_indices.insert(curve_id, Arc::new(data));
        self
    }

    /// Insert FX matrix
    pub fn insert_fx(mut self, fx: FxMatrix) -> Self {
        self.fx = Some(Arc::new(fx));
        self
    }

    /// Map collateral CSA code to discount curve ID
    pub fn map_collateral(mut self, csa_code: impl Into<String>, disc_id: CurveId) -> Self {
        self.collateral.insert(csa_code.into(), disc_id);
        self
    }

    // -----------------------------------------------------------------------------
    // Clean, Direct Getter Methods
    // -----------------------------------------------------------------------------

    /// Get discount curve by ID
    pub fn discount(&self, id: impl AsRef<str>) -> Result<Arc<DiscountCurve>> {
        let curve_id = CurveId::from(id.as_ref());
        self.curves
            .get(&curve_id)
            .and_then(|storage| storage.discount())
            .cloned()
            .ok_or_else(|| {
                crate::Error::Input(InputError::NotFound {
                    id: id.as_ref().to_string(),
                })
            })
    }

    /// Get forward curve by ID
    pub fn forward(&self, id: impl AsRef<str>) -> Result<Arc<ForwardCurve>> {
        let curve_id = CurveId::from(id.as_ref());
        self.curves
            .get(&curve_id)
            .and_then(|storage| storage.forward())
            .cloned()
            .ok_or_else(|| {
                crate::Error::Input(InputError::NotFound {
                    id: id.as_ref().to_string(),
                })
            })
    }

    /// Get hazard curve by ID
    pub fn hazard(&self, id: impl AsRef<str>) -> Result<Arc<HazardCurve>> {
        let curve_id = CurveId::from(id.as_ref());
        self.curves
            .get(&curve_id)
            .and_then(|storage| storage.hazard())
            .cloned()
            .ok_or_else(|| {
                crate::Error::Input(InputError::NotFound {
                    id: id.as_ref().to_string(),
                })
            })
    }

    /// Get inflation curve by ID
    pub fn inflation(&self, id: impl AsRef<str>) -> Result<Arc<InflationCurve>> {
        let curve_id = CurveId::from(id.as_ref());
        self.curves
            .get(&curve_id)
            .and_then(|storage| storage.inflation())
            .cloned()
            .ok_or_else(|| {
                crate::Error::Input(InputError::NotFound {
                    id: id.as_ref().to_string(),
                })
            })
    }

    /// Get base correlation curve by ID
    pub fn base_correlation(&self, id: impl AsRef<str>) -> Result<Arc<BaseCorrelationCurve>> {
        let curve_id = CurveId::from(id.as_ref());
        self.curves
            .get(&curve_id)
            .and_then(|storage| storage.base_correlation())
            .cloned()
            .ok_or_else(|| {
                crate::Error::Input(InputError::NotFound {
                    id: id.as_ref().to_string(),
                })
            })
    }

    // -----------------------------------------------------------------------------
    // Compatibility Methods (for downstream code migration)
    // -----------------------------------------------------------------------------

    /// Get discount curve by ID (compatibility alias)
    pub fn disc(&self, id: impl AsRef<str>) -> Result<Arc<DiscountCurve>> {
        self.discount(id)
    }

    /// Get forward curve by ID (compatibility alias)
    pub fn fwd(&self, id: impl AsRef<str>) -> Result<Arc<ForwardCurve>> {
        self.forward(id)
    }

    /// Get inflation curve by ID (compatibility alias)
    pub fn infl(&self, id: impl AsRef<str>) -> Result<Arc<InflationCurve>> {
        self.inflation(id)
    }

    /// Access to curves (for API compatibility)
    pub fn curves(&self) -> &Self {
        self
    }


    /// Get volatility surface by ID
    pub fn surface(&self, id: impl AsRef<str>) -> Result<Arc<VolSurface>> {
        let curve_id = CurveId::from(id.as_ref());
        self.surfaces
            .get(&curve_id)
            .cloned()
            .ok_or_else(|| {
                crate::Error::Input(InputError::NotFound {
                    id: id.as_ref().to_string(),
                })
            })
    }

    /// Get market scalar by ID
    pub fn price(&self, id: impl AsRef<str>) -> Result<&MarketScalar> {
        let curve_id = CurveId::from(id.as_ref());
        self.prices.get(&curve_id).ok_or_else(|| {
            crate::Error::Input(InputError::NotFound {
                id: id.as_ref().to_string(),
            })
        })
    }

    /// Get scalar time series by ID
    pub fn series(&self, id: impl AsRef<str>) -> Result<&ScalarTimeSeries> {
        let curve_id = CurveId::from(id.as_ref());
        self.series.get(&curve_id).ok_or_else(|| {
            crate::Error::Input(InputError::NotFound {
                id: id.as_ref().to_string(),
            })
        })
    }

    /// Get inflation index by ID
    pub fn inflation_index(&self, id: impl AsRef<str>) -> Option<Arc<InflationIndex>> {
        let curve_id = CurveId::from(id.as_ref());
        self.inflation_indices.get(&curve_id).cloned()
    }

    /// Get credit index by ID
    pub fn credit_index(&self, id: impl AsRef<str>) -> Result<Arc<CreditIndexData>> {
        let curve_id = CurveId::from(id.as_ref());
        self.credit_indices
            .get(&curve_id)
            .cloned()
            .ok_or_else(|| {
                crate::Error::Input(InputError::NotFound {
                    id: id.as_ref().to_string(),
                })
            })
    }

    /// Resolve collateral discount curve for CSA code
    pub fn collateral(&self, csa_code: &str) -> Result<Arc<DiscountCurve>> {
        let curve_id = self.collateral.get(csa_code).ok_or_else(|| {
            crate::Error::Input(InputError::NotFound {
                id: format!("collateral:{}", csa_code),
            })
        })?;
        self.discount(curve_id.as_str())
    }

    // -----------------------------------------------------------------------------
    // Advanced Access and Introspection Methods
    // -----------------------------------------------------------------------------

    /// Get any curve by ID (returns the storage enum)
    pub fn curve(&self, id: impl AsRef<str>) -> Option<&CurveStorage> {
        let curve_id = CurveId::from(id.as_ref());
        self.curves.get(&curve_id)
    }

    /// Get mutable access to curves (for advanced operations)
    pub fn curves_mut(&mut self) -> &mut HashMap<CurveId, CurveStorage> {
        &mut self.curves
    }

    /// Get all curve IDs
    pub fn curve_ids(&self) -> impl Iterator<Item = &CurveId> {
        self.curves.keys()
    }

    /// Get curves by type
    pub fn curves_of_type<'a>(&'a self, curve_type: &'a str) -> impl Iterator<Item = (&'a CurveId, &'a CurveStorage)> + 'a {
        self.curves
            .iter()
            .filter(move |(_, storage)| storage.curve_type() == curve_type)
    }

    /// Count curves by type
    pub fn count_by_type(&self) -> HashMap<&'static str, usize> {
        let mut counts = HashMap::new();
        for storage in self.curves.values() {
            *counts.entry(storage.curve_type()).or_insert(0) += 1;
        }
        counts
    }


    // -----------------------------------------------------------------------------
    // Statistics and Introspection
    // -----------------------------------------------------------------------------

    /// Get summary statistics about the context
    pub fn stats(&self) -> ContextStats {
        ContextStats {
            curve_counts: self.count_by_type(),
            total_curves: self.curves.len(),
            has_fx: self.fx.is_some(),
            surface_count: self.surfaces.len(),
            price_count: self.prices.len(),
            series_count: self.series.len(),
            inflation_index_count: self.inflation_indices.len(),
            credit_index_count: self.credit_indices.len(),
            collateral_mapping_count: self.collateral.len(),
        }
    }

    /// Check if context is empty
    pub fn is_empty(&self) -> bool {
        self.curves.is_empty()
            && self.fx.is_none()
            && self.surfaces.is_empty()
            && self.prices.is_empty()
            && self.series.is_empty()
            && self.inflation_indices.is_empty()
            && self.credit_indices.is_empty()
            && self.collateral.is_empty()
    }

    /// Get the total number of market data objects
    pub fn total_objects(&self) -> usize {
        self.curves.len()
            + self.surfaces.len()
            + self.prices.len()
            + self.series.len()
            + self.inflation_indices.len()
            + self.credit_indices.len()
            + if self.fx.is_some() { 1 } else { 0 }
    }
}

/// Summary statistics for MarketContext
#[derive(Debug, Clone)]
pub struct ContextStats {
    /// Count of curves by type
    pub curve_counts: HashMap<&'static str, usize>,
    /// Total number of curves
    pub total_curves: usize,
    /// Whether FX matrix is present
    pub has_fx: bool,
    /// Number of volatility surfaces
    pub surface_count: usize,
    /// Number of market prices/scalars
    pub price_count: usize,
    /// Number of time series
    pub series_count: usize,
    /// Number of inflation indices
    pub inflation_index_count: usize,
    /// Number of credit indices
    pub credit_index_count: usize,
    /// Number of collateral mappings
    pub collateral_mapping_count: usize,
}

impl core::fmt::Display for ContextStats {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "MarketContext Statistics:")?;
        writeln!(f, "  Total Objects: {}", self.total_curves + self.surface_count + self.price_count + self.series_count)?;
        writeln!(f, "  Curves: {}", self.total_curves)?;
        for (curve_type, count) in &self.curve_counts {
            writeln!(f, "    {}: {}", curve_type, count)?;
        }
        writeln!(f, "  Surfaces: {}", self.surface_count)?;
        writeln!(f, "  Prices: {}", self.price_count)?;
        writeln!(f, "  Series: {}", self.series_count)?;
        writeln!(f, "  Inflation Indices: {}", self.inflation_index_count)?;
        writeln!(f, "  Credit Indices: {}", self.credit_index_count)?;
        writeln!(f, "  FX Matrix: {}", if self.has_fx { "Yes" } else { "No" })?;
        writeln!(f, "  Collateral Mappings: {}", self.collateral_mapping_count)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::Date;
    use crate::market_data::interp::InterpStyle;

    fn test_discount_curve() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap()
    }

    fn test_forward_curve() -> ForwardCurve {
        ForwardCurve::builder("USD-SOFR3M", 0.25)
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .knots([(0.0, 0.03), (1.0, 0.035), (5.0, 0.04)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap()
    }

    fn test_hazard_curve() -> HazardCurve {
        HazardCurve::builder("CORP-HAZARD")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .recovery_rate(0.4)
            .knots([(0.0, 0.01), (1.0, 0.015), (5.0, 0.02)])
            .build()
            .unwrap()
    }

    #[test]
    fn test_context_creation_and_insertion() {
        let context = MarketContext::new()
            .insert_discount(test_discount_curve())
            .insert_forward(test_forward_curve())
            .insert_hazard(test_hazard_curve());

        // Verify curves were inserted
        assert_eq!(context.curves.len(), 3);
        assert!(context.discount("USD-OIS").is_ok());
        assert!(context.forward("USD-SOFR3M").is_ok());
        assert!(context.hazard("CORP-HAZARD").is_ok());
    }

    #[test]
    fn test_direct_concrete_api() {
        let context = MarketContext::new()
            .insert_discount(test_discount_curve());

        // Test direct concrete API - clean and simple
        let disc = context.discount("USD-OIS").unwrap();
        assert!((disc.df(1.0) - 0.95).abs() < 1e-12);
        assert_eq!(disc.id().as_str(), "USD-OIS");
    }

    #[test]
    fn test_error_handling() {
        let context = MarketContext::new();
        
        // Should return NotFound errors for missing curves
        assert!(context.discount("NONEXISTENT").is_err());
        assert!(context.forward("NONEXISTENT").is_err());
        assert!(context.hazard("NONEXISTENT").is_err());
        assert!(context.inflation("NONEXISTENT").is_err());
        assert!(context.base_correlation("NONEXISTENT").is_err());
    }

    #[test]
    fn test_curve_type_filtering() {
        let context = MarketContext::new()
            .insert_discount(test_discount_curve())
            .insert_forward(test_forward_curve())
            .insert_hazard(test_hazard_curve());

        let discount_curves: Vec<_> = context.curves_of_type("Discount").collect();
        assert_eq!(discount_curves.len(), 1);
        
        let counts = context.count_by_type();
        assert_eq!(counts.get("Discount"), Some(&1));
        assert_eq!(counts.get("Forward"), Some(&1));
        assert_eq!(counts.get("Hazard"), Some(&1));
    }

    #[test]
    fn test_context_stats() {
        let context = MarketContext::new()
            .insert_discount(test_discount_curve())
            .insert_price("SPOT_GOLD", MarketScalar::Unitless(2000.0));

        let stats = context.stats();
        assert_eq!(stats.total_curves, 1);
        assert_eq!(stats.price_count, 1);
        assert!(!stats.has_fx);
        
        // Test display
        let display = format!("{}", stats);
        assert!(display.contains("Total Objects: 2"));
        assert!(display.contains("Discount: 1"));
    }

    #[test]
    fn test_collateral_mapping() {
        let context = MarketContext::new()
            .insert_discount(test_discount_curve())
            .map_collateral("USD-CSA", CurveId::new("USD-OIS"));

        let collateral_curve = context.collateral("USD-CSA").unwrap();
        assert!((collateral_curve.df(1.0) - 0.95).abs() < 1e-12);
    }

    #[test]
    fn test_empty_context() {
        let context = MarketContext::new();
        assert!(context.is_empty());
        assert_eq!(context.total_objects(), 0);
    }
}
