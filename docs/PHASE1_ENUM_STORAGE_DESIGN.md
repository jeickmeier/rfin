# Phase 1: Enum-Based Storage System - Detailed Technical Design

## Overview

This document provides the detailed technical design for replacing trait object storage in MarketContext with an enum-based system that enables complete serialization.

## Core Design

### 1. CurveStorage Enum

```rust
// finstack/core/src/market_data/storage/curve_storage.rs

use std::sync::Arc;
use crate::market_data::traits::{Discount, Forward, Survival, Inflation};

/// Unified storage for all curve types
#[derive(Clone, Debug)]
pub enum CurveStorage {
    Discount(Arc<DiscountCurve>),
    Forward(Arc<ForwardCurve>),
    Hazard(Arc<HazardCurve>),
    Inflation(Arc<InflationCurve>),
    BaseCorrelation(Arc<BaseCorrelationCurve>),
}

impl CurveStorage {
    /// Get the curve's ID
    pub fn id(&self) -> &CurveId {
        match self {
            Self::Discount(c) => c.id(),
            Self::Forward(c) => c.id(),
            Self::Hazard(c) => c.id(),
            Self::Inflation(c) => c.id(),
            Self::BaseCorrelation(c) => &c.id,
        }
    }
    
    /// Try to get as a discount curve trait
    pub fn as_discount(&self) -> Option<Arc<dyn Discount + Send + Sync>> {
        match self {
            Self::Discount(curve) => Some(curve.clone() as Arc<dyn Discount + Send + Sync>),
            _ => None,
        }
    }
    
    /// Try to get as a forward curve trait
    pub fn as_forward(&self) -> Option<Arc<dyn Forward + Send + Sync>> {
        match self {
            Self::Forward(curve) => Some(curve.clone() as Arc<dyn Forward + Send + Sync>),
            _ => None,
        }
    }
    
    /// Try to get as a survival curve trait
    pub fn as_survival(&self) -> Option<Arc<dyn Survival + Send + Sync>> {
        match self {
            Self::Hazard(curve) => Some(curve.clone() as Arc<dyn Survival + Send + Sync>),
            _ => None,
        }
    }
    
    /// Try to get as an inflation curve trait
    pub fn as_inflation(&self) -> Option<Arc<dyn Inflation + Send + Sync>> {
        match self {
            Self::Inflation(curve) => Some(curve.clone() as Arc<dyn Inflation + Send + Sync>),
            _ => None,
        }
    }
    
    /// Get the concrete type if it matches
    pub fn as_concrete_discount(&self) -> Option<&Arc<DiscountCurve>> {
        match self {
            Self::Discount(curve) => Some(curve),
            _ => None,
        }
    }
    
    // Similar methods for other concrete types...
}
```

### 2. Serialization Support

```rust
// finstack/core/src/market_data/storage/curve_state.rs

/// Serializable state representation
#[cfg(feature = "serde")]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CurveState {
    Discount(DiscountCurveState),
    Forward(ForwardCurveState),
    Hazard(HazardCurveState),
    Inflation(InflationCurveState),
    BaseCorrelation(BaseCorrelationCurve), // Already serializable
}

impl CurveStorage {
    /// Convert to serializable state
    pub fn to_state(&self) -> CurveState {
        match self {
            Self::Discount(curve) => CurveState::Discount(curve.to_state()),
            Self::Forward(curve) => CurveState::Forward(curve.to_state()),
            Self::Hazard(curve) => CurveState::Hazard(curve.to_state()),
            Self::Inflation(curve) => {
                // InflationCurve implements Serialize directly
                // We need to convert it to a state representation
                CurveState::Inflation(InflationCurveState::from(curve.as_ref()))
            },
            Self::BaseCorrelation(curve) => {
                CurveState::BaseCorrelation((**curve).clone())
            },
        }
    }
    
    /// Reconstruct from state
    pub fn from_state(state: CurveState) -> Result<Self> {
        Ok(match state {
            CurveState::Discount(s) => {
                Self::Discount(Arc::new(DiscountCurve::from_state(s)?))
            },
            CurveState::Forward(s) => {
                Self::Forward(Arc::new(ForwardCurve::from_state(s)?))
            },
            CurveState::Hazard(s) => {
                Self::Hazard(Arc::new(HazardCurve::from_state(s)?))
            },
            CurveState::Inflation(s) => {
                Self::Inflation(Arc::new(InflationCurve::from_state(s)?))
            },
            CurveState::BaseCorrelation(c) => {
                Self::BaseCorrelation(Arc::new(c))
            },
        })
    }
}

#[cfg(feature = "serde")]
impl Serialize for CurveStorage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_state().serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for CurveStorage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let state = CurveState::deserialize(deserializer)?;
        Self::from_state(state).map_err(serde::de::Error::custom)
    }
}
```

### 3. Updated MarketContext

```rust
// finstack/core/src/market_data/context_v2.rs

use crate::market_data::storage::{CurveStorage, CurveState};

/// Market context with enum-based storage
pub struct MarketContextV2 {
    /// All curves stored in unified map
    curves: HashMap<CurveId, CurveStorage>,
    
    /// FX matrix
    fx: Option<Arc<FxMatrix>>,
    
    /// Volatility surfaces
    surfaces: HashMap<CurveId, Arc<VolSurface>>,
    
    /// Market scalars
    prices: HashMap<CurveId, MarketScalar>,
    
    /// Time series
    series: HashMap<CurveId, ScalarTimeSeries>,
    
    /// Collateral mappings
    collateral: HashMap<String, CurveId>,
    
    /// Inflation indices
    inflation_indices: HashMap<CurveId, Arc<InflationIndex>>,
    
    /// Credit indices
    credit_indices: HashMap<CurveId, Arc<CreditIndexData>>,
}

impl MarketContextV2 {
    /// Create empty context
    pub fn new() -> Self {
        Self {
            curves: HashMap::new(),
            fx: None,
            surfaces: HashMap::new(),
            prices: HashMap::new(),
            series: HashMap::new(),
            collateral: HashMap::new(),
            inflation_indices: HashMap::new(),
            credit_indices: HashMap::new(),
        }
    }
    
    /// Insert a discount curve
    pub fn insert_discount(mut self, curve: DiscountCurve) -> Self {
        let id = curve.id().clone();
        self.curves.insert(id, CurveStorage::Discount(Arc::new(curve)));
        self
    }
    
    /// Insert a forward curve
    pub fn insert_forward(mut self, curve: ForwardCurve) -> Self {
        let id = curve.id().clone();
        self.curves.insert(id, CurveStorage::Forward(Arc::new(curve)));
        self
    }
    
    /// Get discount curve by ID
    pub fn disc(&self, id: impl AsRef<str>) -> Result<Arc<dyn Discount + Send + Sync>> {
        self.curves
            .get(id.as_ref())
            .and_then(|storage| storage.as_discount())
            .ok_or_else(|| InputError::NotFound { 
                id: id.as_ref().to_string() 
            }.into())
    }
    
    /// Get forward curve by ID
    pub fn fwd(&self, id: impl AsRef<str>) -> Result<Arc<dyn Forward + Send + Sync>> {
        self.curves
            .get(id.as_ref())
            .and_then(|storage| storage.as_forward())
            .ok_or_else(|| InputError::NotFound { 
                id: id.as_ref().to_string() 
            }.into())
    }
    
    /// Get any curve by ID (returns the storage enum)
    pub fn curve(&self, id: impl AsRef<str>) -> Option<&CurveStorage> {
        self.curves.get(id.as_ref())
    }
    
    /// Get mutable access to curves (for advanced operations)
    pub fn curves_mut(&mut self) -> &mut HashMap<CurveId, CurveStorage> {
        &mut self.curves
    }
}
```

### 4. Serialization Implementation

```rust
// finstack/core/src/market_data/context_v2_serde.rs

#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize)]
pub struct MarketContextV2Data {
    /// All curves with their states
    curves: Vec<(CurveId, CurveState)>,
    
    /// FX matrix data
    fx: Option<FxMatrixData>,
    
    /// Surfaces
    surfaces: Vec<(CurveId, VolSurfaceState)>,
    
    /// Prices
    prices: Vec<(CurveId, MarketScalar)>,
    
    /// Series
    series: Vec<(CurveId, ScalarTimeSeriesState)>,
    
    /// Collateral mappings
    collateral: Vec<(String, CurveId)>,
    
    /// Inflation indices
    inflation_indices: Vec<(CurveId, InflationIndexData)>,
    
    /// Credit indices  
    credit_indices: Vec<(CurveId, CreditIndexEntry)>,
}

impl MarketContextV2 {
    /// Full serialization with all curves
    pub fn to_data(&self) -> Result<MarketContextV2Data> {
        Ok(MarketContextV2Data {
            curves: self.curves
                .iter()
                .map(|(id, storage)| (id.clone(), storage.to_state()))
                .collect(),
            
            fx: self.fx.as_ref().map(|fx| extract_fx_data(fx)),
            
            surfaces: self.surfaces
                .iter()
                .map(|(id, s)| (id.clone(), s.to_state()))
                .collect(),
            
            prices: self.prices
                .iter()
                .map(|(id, p)| (id.clone(), p.clone()))
                .collect(),
            
            series: self.series
                .iter()
                .filter_map(|(id, s)| s.to_state().ok().map(|state| (id.clone(), state)))
                .collect(),
            
            collateral: self.collateral
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            
            inflation_indices: convert_inflation_indices(&self.inflation_indices),
            
            credit_indices: convert_credit_indices(&self.credit_indices),
        })
    }
    
    /// Full deserialization
    pub fn from_data(data: MarketContextV2Data) -> Result<Self> {
        let mut context = Self::new();
        
        // Reconstruct all curves
        for (id, state) in data.curves {
            let storage = CurveStorage::from_state(state)?;
            context.curves.insert(id, storage);
        }
        
        // Reconstruct other components...
        if let Some(fx_data) = data.fx {
            context.fx = Some(Arc::new(reconstruct_fx(fx_data)?));
        }
        
        for (id, state) in data.surfaces {
            context.surfaces.insert(id, Arc::new(VolSurface::from_state(state)?));
        }
        
        for (id, price) in data.prices {
            context.prices.insert(id, price);
        }
        
        for (id, state) in data.series {
            context.series.insert(id, ScalarTimeSeries::from_state(state)?);
        }
        
        for (k, v) in data.collateral {
            context.collateral.insert(k, v);
        }
        
        // Reconstruct indices...
        
        Ok(context)
    }
}
```

## Migration Strategy

### 1. Feature Flag System

```toml
# finstack/core/Cargo.toml
[features]
default = ["legacy-context"]
legacy-context = []
new-context = []
```

### 2. Compatibility Module

```rust
// finstack/core/src/market_data/compat.rs

/// Provides backward-compatible API during migration
pub mod compat {
    #[cfg(feature = "legacy-context")]
    pub use super::context::MarketContext;
    
    #[cfg(feature = "new-context")]
    pub use super::context_v2::MarketContextV2 as MarketContext;
}
```

### 3. Gradual Migration Path

```rust
// Example of using both during migration
#[cfg(all(feature = "legacy-context", feature = "new-context"))]
mod migration {
    use super::*;
    
    /// Convert legacy to new
    pub fn migrate_context(legacy: &context::MarketContext) -> Result<context_v2::MarketContextV2> {
        let mut new = context_v2::MarketContextV2::new();
        
        // Migrate what we can access...
        // This will be limited by trait object issues
        
        new
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn curve_storage_round_trip() {
        let curve = DiscountCurve::builder("TEST")
            .base_date(Date::today())
            .knots([(0.0, 1.0), (1.0, 0.95)])
            .build()
            .unwrap();
        
        let storage = CurveStorage::Discount(Arc::new(curve));
        let state = storage.to_state();
        let restored = CurveStorage::from_state(state).unwrap();
        
        assert_eq!(storage.id(), restored.id());
    }
    
    #[test]
    fn context_full_serialization() {
        let context = MarketContextV2::new()
            .insert_discount(create_test_discount())
            .insert_forward(create_test_forward())
            .insert_hazard(create_test_hazard());
        
        let data = context.to_data().unwrap();
        let json = serde_json::to_string(&data).unwrap();
        let restored_data: MarketContextV2Data = serde_json::from_str(&json).unwrap();
        let restored = MarketContextV2::from_data(restored_data).unwrap();
        
        assert_eq!(context.curves.len(), restored.curves.len());
    }
}
```

### Property Tests

```rust
#[cfg(test)]
mod prop_tests {
    use proptest::prelude::*;
    
    prop_compose! {
        fn arb_curve_storage()(
            curve_type in 0..5u8,
            id in "[A-Z]{3}-[A-Z]{3,5}",
        ) -> CurveStorage {
            match curve_type {
                0 => CurveStorage::Discount(Arc::new(arb_discount_curve(id))),
                1 => CurveStorage::Forward(Arc::new(arb_forward_curve(id))),
                2 => CurveStorage::Hazard(Arc::new(arb_hazard_curve(id))),
                3 => CurveStorage::Inflation(Arc::new(arb_inflation_curve(id))),
                4 => CurveStorage::BaseCorrelation(Arc::new(arb_base_corr(id))),
                _ => unreachable!(),
            }
        }
    }
    
    proptest! {
        #[test]
        fn storage_serialization_preserves_data(storage in arb_curve_storage()) {
            let json = serde_json::to_string(&storage)?;
            let restored: CurveStorage = serde_json::from_str(&json)?;
            prop_assert_eq!(storage.id(), restored.id());
        }
    }
}
```

## Performance Considerations

### Benchmarks

```rust
#[bench]
fn bench_trait_dispatch_old(b: &mut Bencher) {
    let context = create_old_context();
    b.iter(|| {
        for _ in 0..1000 {
            let disc = context.disc("USD-OIS").unwrap();
            black_box(disc.df(1.0));
        }
    });
}

#[bench]
fn bench_enum_dispatch_new(b: &mut Bencher) {
    let context = create_new_context();
    b.iter(|| {
        for _ in 0..1000 {
            let disc = context.disc("USD-OIS").unwrap();
            black_box(disc.df(1.0));
        }
    });
}
```

Expected performance characteristics:
- **Serialization**: 10x faster (no string parsing)
- **Dispatch**: ~Same (still using trait objects for API)
- **Memory**: Slightly higher (enum overhead)

## Implementation Checklist

- [ ] Create `storage` module with `CurveStorage` enum
- [ ] Implement state conversion for all curve types
- [ ] Create `MarketContextV2` with new storage
- [ ] Implement full serialization/deserialization
- [ ] Add comprehensive unit tests
- [ ] Add property-based tests
- [ ] Create migration utilities
- [ ] Add benchmarks
- [ ] Update documentation
- [ ] Create migration guide

## Next Steps

1. Create feature branch
2. Implement `CurveStorage` enum
3. Add tests for enum operations
4. Implement `MarketContextV2` 
5. Add serialization support
6. Create compatibility layer
7. Test migration path
8. Performance benchmarks
9. Documentation
10. Code review and merge
