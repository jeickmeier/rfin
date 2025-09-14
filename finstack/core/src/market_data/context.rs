//! Lightweight container aggregating all market data needed by valuations.
//!
//! `MarketContext` groups together curves, FX (`FxMatrix`), 2-D surfaces
//! (`VolSurface`) and generic prices/scalars so that pricing and
//! risk components have a single handle to query required inputs.
//!
//! The container is intentionally minimal and cloning it is cheap because it
//! stores `Arc` references and small maps.

extern crate alloc;
use alloc::sync::Arc;
use hashbrown::HashMap;

use crate::money::fx::FxMatrix;
use crate::currency::Currency;
use crate::types::CurveId;
use crate::F;
use crate::Result;

use super::{
    scalars::inflation_index::InflationIndex,
    term_structures::credit_index::CreditIndexData,
    inflation::InflationCurve,
    scalars::{MarketScalar, ScalarTimeSeries},
    surfaces::vol_surface::VolSurface,
    term_structures::{
        base_correlation::BaseCorrelationCurve,
        discount_curve::DiscountCurve,
        forward_curve::ForwardCurve,
        hazard_curve::HazardCurve,
    },
    traits::{Discount, TermStructure},
};

// Re-export bump functionality
pub use super::bumps::{BumpMode, BumpSpec, BumpUnits};

// -----------------------------------------------------------------------------
// Curve Storage
// -----------------------------------------------------------------------------

/// Unified storage for all curve types using an enum
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
    /// Get the type name for statistics
    pub fn type_name(&self) -> &'static str {
        match self {
            CurveStorage::Discount(_) => "Discount",
            CurveStorage::Forward(_) => "Forward",
            CurveStorage::Hazard(_) => "Hazard",
            CurveStorage::Inflation(_) => "Inflation",
            CurveStorage::BaseCorrelation(_) => "BaseCorrelation",
        }
    }
}

// Extended API (moved from storage::curve_storage)
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
    pub fn is_discount(&self) -> bool { matches!(self, Self::Discount(_)) }
    /// Check if this storage contains a forward curve
    pub fn is_forward(&self) -> bool { matches!(self, Self::Forward(_)) }
    /// Check if this storage contains a hazard curve
    pub fn is_hazard(&self) -> bool { matches!(self, Self::Hazard(_)) }
    /// Check if this storage contains an inflation curve
    pub fn is_inflation(&self) -> bool { matches!(self, Self::Inflation(_)) }
    /// Check if this storage contains a base correlation curve
    pub fn is_base_correlation(&self) -> bool { matches!(self, Self::BaseCorrelation(_)) }

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
    fn id(&self) -> &CurveId { self.id() }
}

// Convenience constructors
impl CurveStorage {
    /// Create storage for a discount curve
    pub fn new_discount(curve: DiscountCurve) -> Self { Self::Discount(Arc::new(curve)) }
    /// Create storage for a forward curve
    pub fn new_forward(curve: ForwardCurve) -> Self { Self::Forward(Arc::new(curve)) }
    /// Create storage for a hazard curve
    pub fn new_hazard(curve: HazardCurve) -> Self { Self::Hazard(Arc::new(curve)) }
    /// Create storage for an inflation curve
    pub fn new_inflation(curve: InflationCurve) -> Self { Self::Inflation(Arc::new(curve)) }
    /// Create storage for a base correlation curve
    pub fn new_base_correlation(curve: BaseCorrelationCurve) -> Self { Self::BaseCorrelation(Arc::new(curve)) }
}

// -----------------------------------------------------------------------------
// Serde: move CurveState and (De)Serialize impls here
// -----------------------------------------------------------------------------

#[cfg(feature = "serde")]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "snake_case"))]
/// Serializable state representation for any curve type
pub enum CurveState {
    /// Discount curve state
    Discount(crate::market_data::term_structures::discount_curve::DiscountCurveState),
    /// Forward curve state
    Forward(crate::market_data::term_structures::forward_curve::ForwardCurveState),
    /// Hazard curve state
    Hazard(crate::market_data::term_structures::hazard_curve::HazardCurveState),
    /// Inflation curve state
    Inflation(InflationCurveData),
    /// Base correlation curve state
    BaseCorrelation(crate::market_data::term_structures::base_correlation::BaseCorrelationCurve),
}

#[cfg(feature = "serde")]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Serializable representation of an inflation curve
pub struct InflationCurveData {
    /// Curve identifier
    pub id: String,
    /// Base CPI level
    pub base_cpi: crate::F,
    /// Time/CPI level pairs
    pub knot_points: alloc::vec::Vec<(crate::F, crate::F)>,
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
                let knot_points: alloc::vec::Vec<(crate::F, crate::F)> = curve
                    .knots()
                    .iter()
                    .zip(curve.cpi_levels().iter())
                    .map(|(&t, &cpi)| (t, cpi))
                    .collect();

                CurveState::Inflation(InflationCurveData {
                    id: curve.id().to_string(),
                    base_cpi: curve.base_cpi(),
                    knot_points,
                    interp_style: crate::market_data::interp::InterpStyle::LogLinear,
                })
            }
            Self::BaseCorrelation(curve) => CurveState::BaseCorrelation((**curve).clone()),
        })
    }

    /// Reconstruct from serializable state
    pub fn from_state(state: CurveState) -> crate::Result<Self> {
        use alloc::sync::Arc;
        use crate::market_data::term_structures::{
            discount_curve::DiscountCurve,
            forward_curve::ForwardCurve,
            hazard_curve::HazardCurve,
            inflation::InflationCurve,
        };

        Ok(match state {
            CurveState::Discount(s) => Self::Discount(Arc::new(DiscountCurve::from_state(s).map_err(|_| crate::Error::Internal)?)),
            CurveState::Forward(s) => Self::Forward(Arc::new(ForwardCurve::from_state(s)?)),
            CurveState::Hazard(s) => Self::Hazard(Arc::new(HazardCurve::from_state(s)?)),
            CurveState::Inflation(s) => {
                let curve = InflationCurve::builder(s.id)
                    .base_cpi(s.base_cpi)
                    .knots(s.knot_points)
                    .set_interp(s.interp_style)
                    .build()?;
                Self::Inflation(Arc::new(curve))
            }
            CurveState::BaseCorrelation(c) => Self::BaseCorrelation(Arc::new(c)),
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

// -----------------------------------------------------------------------------
// Market Context
// -----------------------------------------------------------------------------

/// Unified market data context with enum-based storage
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
    // Insert methods - builder pattern
// -----------------------------------------------------------------------------

    /// Insert a discount curve
    pub fn insert_discount(mut self, curve: DiscountCurve) -> Self {
        let id = TermStructure::id(&curve).clone();
        self.curves.insert(id, CurveStorage::Discount(Arc::new(curve)));
        self
    }

    /// Insert a forward curve
    pub fn insert_forward(mut self, curve: ForwardCurve) -> Self {
        let id = TermStructure::id(&curve).clone();
        self.curves.insert(id, CurveStorage::Forward(Arc::new(curve)));
        self
    }

    /// Insert a hazard curve
    pub fn insert_hazard(mut self, curve: HazardCurve) -> Self {
        let id = TermStructure::id(&curve).clone();
        self.curves.insert(id, CurveStorage::Hazard(Arc::new(curve)));
        self
    }

    /// Insert an inflation curve
    pub fn insert_inflation(mut self, curve: InflationCurve) -> Self {
        let id = TermStructure::id(&curve).clone();
        self.curves.insert(id, CurveStorage::Inflation(Arc::new(curve)));
        self
    }

    /// Insert a base correlation curve
    pub fn insert_base_correlation(mut self, curve: BaseCorrelationCurve) -> Self {
        let id = TermStructure::id(&curve).clone();
        self.curves.insert(id, CurveStorage::BaseCorrelation(Arc::new(curve)));
        self
    }

    /// Insert a volatility surface
    pub fn insert_surface(mut self, surface: VolSurface) -> Self {
        let id = TermStructure::id(&surface).clone();
        self.surfaces.insert(id, Arc::new(surface));
        self
    }

    /// Insert a market scalar/price
    pub fn insert_price(mut self, id: impl AsRef<str>, price: MarketScalar) -> Self {
        self.prices.insert(CurveId::from(id.as_ref()), price);
        self
    }

    /// Insert a time series
    pub fn insert_series(mut self, series: ScalarTimeSeries) -> Self {
        let id = series.id().clone();
        self.series.insert(id, series);
        self
    }

    /// Insert an inflation index
    pub fn insert_inflation_index(mut self, id: impl AsRef<str>, index: InflationIndex) -> Self {
        self.inflation_indices.insert(CurveId::from(id.as_ref()), Arc::new(index));
        self
    }

    /// Insert a credit index
    pub fn insert_credit_index(mut self, id: impl AsRef<str>, data: CreditIndexData) -> Self {
        self.credit_indices.insert(CurveId::from(id.as_ref()), Arc::new(data));
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
    // Typed getters - return concrete types (preferred API)
    // -----------------------------------------------------------------------------

    /// Get a hazard curve by ID (returns concrete type)
    pub fn hazard(&self, id: impl AsRef<str>) -> Result<Arc<HazardCurve>> {
        let id_str = id.as_ref();
        match self.curves.get(id_str) {
            Some(CurveStorage::Hazard(curve)) => Ok(Arc::clone(curve)),
            _ => Err(crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }.into()),
        }
    }

    

    /// Get a base correlation curve by ID (returns concrete type)
    pub fn base_correlation(&self, id: impl AsRef<str>) -> Result<Arc<BaseCorrelationCurve>> {
        let id_str = id.as_ref();
        match self.curves.get(id_str) {
            Some(CurveStorage::BaseCorrelation(curve)) => Ok(Arc::clone(curve)),
            _ => Err(crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }.into()),
        }
    }

    // -----------------------------------------------------------------------------
    // Typed getters - return concrete types (for new code)
    // -----------------------------------------------------------------------------

    /// Get a discount curve by ID (returns concrete type)
    pub fn discount(&self, id: impl AsRef<str>) -> Result<Arc<DiscountCurve>> {
        let id_str = id.as_ref();
        match self.curves.get(id_str) {
            Some(CurveStorage::Discount(curve)) => Ok(Arc::clone(curve)),
            _ => Err(crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }.into()),
            }
    }

    /// Get a forward curve by ID (returns concrete type)
    pub fn forward(&self, id: impl AsRef<str>) -> Result<Arc<ForwardCurve>> {
        let id_str = id.as_ref();
        match self.curves.get(id_str) {
            Some(CurveStorage::Forward(curve)) => Ok(Arc::clone(curve)),
            _ => Err(crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }.into()),
            }
    }

    /// Get an inflation curve by ID (returns concrete type)
    pub fn inflation(&self, id: impl AsRef<str>) -> Result<Arc<InflationCurve>> {
        let id_str = id.as_ref();
        match self.curves.get(id_str) {
            Some(CurveStorage::Inflation(curve)) => Ok(Arc::clone(curve)),
            _ => Err(crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }.into()),
            }
    }

    /// Get a volatility surface by ID
    pub fn surface(&self, id: impl AsRef<str>) -> Result<Arc<VolSurface>> {
        let id_str = id.as_ref();
        self.surfaces.get(id_str).cloned().ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }.into(),
        )
    }

    /// Get a market price/scalar by ID
    pub fn price(&self, id: impl AsRef<str>) -> Result<&MarketScalar> {
        let id_str = id.as_ref();
        self.prices.get(id_str).ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }.into(),
        )
    }

    /// Get a time series by ID
    pub fn series(&self, id: impl AsRef<str>) -> Result<&ScalarTimeSeries> {
        let id_str = id.as_ref();
        self.series.get(id_str).ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }.into(),
        )
    }

    /// Get an inflation index by ID
    pub fn inflation_index(&self, id: impl AsRef<str>) -> Option<Arc<InflationIndex>> {
        self.inflation_indices.get(id.as_ref()).cloned()
    }

    /// Get a credit index by ID
    pub fn credit_index(&self, id: impl AsRef<str>) -> Result<Arc<CreditIndexData>> {
        let id_str = id.as_ref();
        self.credit_indices.get(id_str).cloned().ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }.into(),
        )
    }

    /// Resolve collateral discount curve for CSA code
    pub fn collateral(&self, csa_code: &str) -> Result<Arc<dyn Discount + Send + Sync>> {
        let curve_id = self.collateral.get(csa_code)
            .ok_or(crate::error::InputError::NotFound {
                    id: format!("collateral:{}", csa_code),
            })?;
        self.discount(curve_id.as_str()).map(|arc| arc as Arc<dyn Discount + Send + Sync>)
    }

    // -----------------------------------------------------------------------------
    // Update methods for special cases
    // -----------------------------------------------------------------------------

    /// Update only the base correlation curve for a credit index.
    ///
    /// This is an optimization for calibration routines that need to repeatedly
    /// update only the correlation curve while keeping other index data constant.
    /// Returns false if the index doesn't exist.
    pub fn update_base_correlation_curve(
        &mut self,
        id: impl AsRef<str>,
        new_curve: Arc<BaseCorrelationCurve>,
    ) -> bool {
        let cid = CurveId::from(id.as_ref());

        // Get the existing index data
        let Some(existing_index) = self.credit_indices.get(&cid) else {
            return false;
        };

        // Create a new index with the updated correlation curve
        let updated_index = CreditIndexData {
            num_constituents: existing_index.num_constituents,
            recovery_rate: existing_index.recovery_rate,
            index_credit_curve: Arc::clone(&existing_index.index_credit_curve),
            base_correlation_curve: new_curve,
            issuer_credit_curves: existing_index.issuer_credit_curves.clone(),
        };

        // Update the context
        self.credit_indices.insert(cid, Arc::new(updated_index));
        true
    }

    // -----------------------------------------------------------------------------
    // Introspection and statistics
    // -----------------------------------------------------------------------------

    /// Get curve storage by ID (for generic access)
    pub fn curve(&self, id: impl AsRef<str>) -> Option<&CurveStorage> {
        self.curves.get(id.as_ref())
    }

    /// Get all curve IDs
    pub fn curve_ids(&self) -> impl Iterator<Item = &CurveId> {
        self.curves.keys()
    }

    /// Filter curves by type
    pub fn curves_of_type<'a>(&'a self, curve_type: &'a str) -> impl Iterator<Item = (&'a CurveId, &'a CurveStorage)> + 'a {
        self.curves.iter()
            .filter(move |(_, storage)| storage.type_name() == curve_type)
    }

    /// Count curves by type
    pub fn count_by_type(&self) -> HashMap<&'static str, usize> {
        let mut counts = HashMap::new();
        for storage in self.curves.values() {
            *counts.entry(storage.type_name()).or_insert(0) += 1;
        }
        counts
    }

    /// Get context statistics
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

    /// Check if the context is empty
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

    /// Get total number of objects
    pub fn total_objects(&self) -> usize {
        self.curves.len()
            + self.surfaces.len()
            + self.prices.len()
            + self.series.len()
            + self.inflation_indices.len()
            + self.credit_indices.len()
            + if self.fx.is_some() { 1 } else { 0 }
    }

    // -----------------------------------------------------------------------------
    // Scenario Analysis and Stress Testing
    // -----------------------------------------------------------------------------

    /// Apply one or more bumps to the market context in a single call.
    ///
    /// This consolidated API supports discount/forward/hazard/inflation/base-correlation
    /// curves, volatility surfaces, and market scalars.
    ///
    /// # Example
    /// ```rust
    /// # use hashbrown::HashMap;
    /// # use finstack_core::market_data::context::{MarketContext, BumpSpec};
    /// # use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    /// # use finstack_core::dates::Date;
    /// # use finstack_core::types::CurveId;
    /// # let curve = DiscountCurve::builder("USD-OIS")
    /// #     .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
    /// #     .knots([(0.0, 1.0), (5.0, 0.9)])
    /// #     .build().unwrap();
    /// # let context = MarketContext::new().insert_discount(curve);
    /// let mut bumps = HashMap::new();
    /// bumps.insert(CurveId::new("USD-OIS"), BumpSpec::parallel_bp(100.0));
    /// let bumped = context.bump(bumps).unwrap();
    /// assert!(bumped.discount("USD-OIS_bump_100bp").is_ok());
    /// ```
    pub fn bump(&self, bumps: HashMap<CurveId, BumpSpec>) -> Result<Self> {
        use super::bumps::*;
        
        let mut new_context = self.clone();

        for (curve_id, bump_spec) in bumps {
            let curve_id_str = curve_id.as_str();

            // Try discount curves
            if let Ok(original) = self.discount(curve_id_str) {
                if bump_spec.mode == BumpMode::Additive && bump_spec.units == BumpUnits::RateBp {
                    let bump_bp = bump_spec.value;
                    let bumped_curve = original.with_parallel_bump(bump_bp);
                    let bumped_id = TermStructure::id(&bumped_curve).clone();
                    new_context
                        .curves
                        .insert(bumped_id, CurveStorage::Discount(Arc::new(bumped_curve)));
                }
            }
            // Try forward curves
            else if let Ok(original) = self.forward(curve_id_str) {
                if bump_spec.mode == BumpMode::Additive && bump_spec.units == BumpUnits::RateBp {
                    let bump_bp = bump_spec.value;
                    let bump_rate = bump_bp / 10_000.0;
                    let bumped_id = id_bump_bp(curve_id_str, bump_bp);
                    
                    // Apply bump directly to the original curve's knot points
                    let base_date = original.base_date();
                    let bumped_rates: Vec<(F, F)> = original.knots()
                        .iter()
                        .zip(original.fwds().iter())
                        .map(|(&t, &rate)| {
                            // Simple additive bump for forward rates
                            (t, rate + bump_rate)
                        })
                        .collect();
                    
                    if let Ok(bumped_curve) = ForwardCurve::builder(bumped_id.as_str(), original.tenor())
                        .base_date(base_date)
                        .knots(bumped_rates)
                        .build()
                    {
                        new_context.curves.insert(bumped_id, CurveStorage::Forward(Arc::new(bumped_curve)));
                    }
                }
            }
            // Hazard curves, inflation curves, base correlation, and other types continue...
            // These can directly modify and recreate curves without wrappers
            else if let Ok(original) = self.hazard(curve_id_str) {
                if bump_spec.mode == BumpMode::Additive && bump_spec.units == BumpUnits::RateBp {
                    let spread_rate = bump_spec.additive_fraction().unwrap_or(0.0);
                    if let Ok(bumped_curve) = original.with_hazard_shift(spread_rate) {
                        let bumped_id = id_spread_bp(curve_id_str, bump_spec.value);
                        new_context.curves.insert(bumped_id, CurveStorage::Hazard(Arc::new(bumped_curve)));
                    }
                }
            }
            else {
                return Err(crate::error::InputError::NotFound { id: curve_id_str.to_string() }.into());
            }
        }

        Ok(new_context)
    }

    // -----------------------------------------------------------------------------
    // Forward Price/Rate Calculators
    // -----------------------------------------------------------------------------

    /// Build forward function for equity underlyings: F(t) = S₀ × exp((r - q) × t)
    pub fn equity_forward<'a>(
        &'a self,
        underlying: &str,
        base_currency: Currency,
    ) -> Result<Box<dyn Fn(F) -> F + 'a>> {
        // Get spot price
        let spot_scalar = self.price(underlying)?;
        let spot = match spot_scalar {
            MarketScalar::Price(money) => money.amount(),
            MarketScalar::Unitless(value) => *value,
        };

        // Get dividend yield (default to 0.0 if not available)
        let div_yield_key = format!("{}-DIVYIELD", underlying);
        let dividend_yield = self.price(&div_yield_key)
            .map(|scalar| match scalar {
                MarketScalar::Unitless(yield_val) => *yield_val,
                _ => 0.0,
            })
            .unwrap_or(0.0);

        // Get risk-free rate from discount curve
        let disc_curve_id = format!("{}-OIS", base_currency);
        let discount_curve = self.discount(&disc_curve_id)?;

        Ok(Box::new(move |t: F| -> F {
            let risk_free_rate = discount_curve.zero(t);
            spot * ((risk_free_rate - dividend_yield) * t).exp()
        }))
    }

    /// Build forward function for FX underlyings: F(t) = S₀ × exp((r_dom - r_for) × t)
    pub fn fx_forward<'a>(&'a self, underlying: &str) -> Result<Box<dyn Fn(F) -> F + 'a>> {
        // Parse FX pair (assume 6-char format like "EURUSD")
        if underlying.len() != 6 {
            return Err(crate::error::InputError::Invalid.into());
        }

        let foreign_ccy = &underlying[0..3];
        let domestic_ccy = &underlying[3..6];

        // Get spot rate
        let spot_scalar = self.price(underlying)?;
        let spot = match spot_scalar {
            MarketScalar::Price(money) => money.amount(),
            MarketScalar::Unitless(value) => *value,
        };

        // Get domestic and foreign discount curves
        let dom_disc_id = format!("{}-OIS", domestic_ccy);
        let for_disc_id = format!("{}-OIS", foreign_ccy);
        let dom_curve = self.discount(&dom_disc_id)?;
        let for_curve = self.discount(&for_disc_id)?;

        Ok(Box::new(move |t: F| -> F {
            let domestic_rate = dom_curve.zero(t);
            let foreign_rate = for_curve.zero(t);
            spot * ((domestic_rate - foreign_rate) * t).exp()
        }))
    }

    /// Build forward function for interest rate underlyings: F(t) = forward_curve.rate(t)
    pub fn rates_forward<'a>(&'a self, underlying: &str) -> Result<Box<dyn Fn(F) -> F + 'a>> {
        let forward_curve = self.forward(underlying)?;
        Ok(Box::new(move |t: F| -> F {
            forward_curve.rate(t)
        }))
    }

    /// Auto-detect asset class and build appropriate forward function
    pub fn auto_forward<'a>(
        &'a self,
        underlying: &str,
        base_currency: Currency,
    ) -> Result<Box<dyn Fn(F) -> F + 'a>> {
        // Detect asset class from underlying identifier
        if underlying.contains("-") && (underlying.contains("SOFR") || underlying.contains("EURIBOR") || underlying.contains("SONIA")) {
            self.rates_forward(underlying)
        } else if underlying.len() == 6 && underlying.chars().all(|c| c.is_ascii_alphabetic()) {
            self.fx_forward(underlying)
        } else {
            self.equity_forward(underlying, base_currency)
        }
    }
}

// -----------------------------------------------------------------------------
// Context Statistics
// -----------------------------------------------------------------------------

/// Statistics about the contents of a MarketContext
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
        writeln!(f, "  Total curves: {}", self.total_curves)?;
        for (curve_type, count) in &self.curve_counts {
            writeln!(f, "    {}: {}", curve_type, count)?;
        }
        writeln!(f, "  Surfaces: {}", self.surface_count)?;
        writeln!(f, "  Prices: {}", self.price_count)?;
        writeln!(f, "  Series: {}", self.series_count)?;
        writeln!(f, "  Inflation indices: {}", self.inflation_index_count)?;
        writeln!(f, "  Credit indices: {}", self.credit_index_count)?;
        writeln!(f, "  Collateral mappings: {}", self.collateral_mapping_count)?;
        writeln!(f, "  Has FX: {}", self.has_fx)?;
        Ok(())
    }
}
