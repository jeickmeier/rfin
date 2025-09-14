# Market Context Serialization - Complete Architecture Redesign Plan

## Executive Summary

This document outlines a comprehensive plan to eliminate the architectural limitations currently preventing full serialization of MarketContext. The plan addresses the fundamental issue of trait object storage and provides a path to clean, maintainable serialization without string parsing hacks.

## Current Limitations

1. **Trait Object Storage**: `Arc<dyn Trait + Send + Sync>` prevents downcasting to concrete types
2. **Bumped Curve Wrappers**: Internal wrapper types that can't be serialized
3. **String Parsing Hacks**: Detecting bumped curves by parsing ID patterns like "_bump_100bp"

## Phase 1: Enum-Based Storage System

### 1.1 Design CurveStorage Enum

Replace trait object storage with a unified enum that can hold all curve types:

```rust
// finstack/core/src/market_data/storage.rs

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CurveStorage {
    Discount(Arc<DiscountCurve>),
    Forward(Arc<ForwardCurve>),
    Hazard(Arc<HazardCurve>),
    Inflation(Arc<InflationCurve>),
    BaseCorrelation(Arc<BaseCorrelationCurve>),
}

impl CurveStorage {
    /// Get as discount curve trait if this is a discount curve
    pub fn as_discount(&self) -> Option<Arc<dyn Discount + Send + Sync>> {
        match self {
            CurveStorage::Discount(curve) => Some(curve.clone() as Arc<dyn Discount + Send + Sync>),
            _ => None,
        }
    }
    
    /// Get as forward curve trait if this is a forward curve
    pub fn as_forward(&self) -> Option<Arc<dyn Forward + Send + Sync>> {
        match self {
            CurveStorage::Forward(curve) => Some(curve.clone() as Arc<dyn Forward + Send + Sync>),
            _ => None,
        }
    }
    
    /// Serialize to state representation
    pub fn to_state(&self) -> CurveState {
        match self {
            CurveStorage::Discount(curve) => CurveState::Discount(curve.to_state()),
            CurveStorage::Forward(curve) => CurveState::Forward(curve.to_state()),
            CurveStorage::Hazard(curve) => CurveState::Hazard(curve.to_state()),
            CurveStorage::Inflation(curve) => CurveState::Inflation(curve.to_state()),
            CurveStorage::BaseCorrelation(curve) => CurveState::BaseCorrelation((**curve).clone()),
        }
    }
}
```

### 1.2 Update MarketContext Structure

```rust
pub struct MarketContext {
    /// Unified curve storage by ID
    curves: HashMap<CurveId, CurveStorage>,
    
    /// Bump metadata tracked separately
    bumps: HashMap<CurveId, BumpMetadata>,
    
    /// FX matrix
    fx: Option<Arc<FxMatrix>>,
    
    /// Volatility surfaces
    surfaces: HashMap<CurveId, Arc<VolSurface>>,
    
    /// Prices and scalars
    prices: HashMap<CurveId, MarketScalar>,
    
    /// Time series
    series: HashMap<CurveId, ScalarTimeSeries>,
    
    /// Collateral mappings
    collateral: HashMap<String, CurveId>,
}
```

### 1.3 Implementation Timeline

- **Week 1**: Create CurveStorage enum and state types
- **Week 2**: Update MarketContext to use new storage
- **Week 3**: Migrate existing getter methods
- **Week 4**: Update tests and ensure backward compatibility

## Phase 2: Bump Metadata System

### 2.1 Design BumpMetadata Structure

```rust
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BumpMetadata {
    /// Original curve this was bumped from
    pub original_id: CurveId,
    
    /// Type of bump applied
    pub bump_type: BumpType,
    
    /// Bump specification
    pub spec: BumpSpec,
    
    /// Timestamp when bump was created
    pub created_at: Date,
    
    /// Optional description
    pub description: Option<String>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BumpType {
    ParallelRate,
    SpreadShift,
    VolatilityMultiplier,
    InflationShift,
    CorrelationShift,
    FxShift,
}
```

### 2.2 Implement Bump Application

```rust
impl MarketContext {
    /// Apply bumps with proper metadata tracking
    pub fn bump(&self, bumps: HashMap<CurveId, BumpSpec>) -> Result<Self> {
        let mut new_context = self.clone();
        
        for (curve_id, bump_spec) in bumps {
            // Get the original curve
            let original = self.curves.get(&curve_id)
                .ok_or_else(|| InputError::NotFound { id: curve_id.to_string() })?;
            
            // Apply bump to create new curve
            let bumped_curve = apply_bump(original, &bump_spec)?;
            
            // Generate deterministic ID for bumped curve
            let bumped_id = generate_bump_id(&curve_id, &bump_spec);
            
            // Store the bumped curve
            new_context.curves.insert(bumped_id.clone(), bumped_curve);
            
            // Store bump metadata
            new_context.bumps.insert(bumped_id, BumpMetadata {
                original_id: curve_id,
                bump_type: bump_spec.to_bump_type(),
                spec: bump_spec,
                created_at: Date::today(),
                description: None,
            });
        }
        
        Ok(new_context)
    }
}
```

### 2.3 Implementation Timeline

- **Week 1**: Design and implement BumpMetadata types
- **Week 2**: Create bump application system
- **Week 3**: Implement metadata serialization
- **Week 4**: Migrate existing bump functionality

## Phase 3: Structured Metadata System

### 3.1 Replace String Parsing

Instead of parsing IDs like "USD-OIS_bump_100bp", use structured metadata:

```rust
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CurveMetadata {
    /// Base identifier (e.g., "USD-OIS")
    pub base_id: String,
    
    /// Curve type
    pub curve_type: CurveType,
    
    /// Optional transformations applied
    pub transformations: Vec<Transformation>,
    
    /// Tags for categorization
    pub tags: HashSet<String>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Transformation {
    Bump { spec: BumpSpec },
    Shift { amount: F },
    Scale { factor: F },
    Composite { operations: Vec<Transformation> },
}
```

### 3.2 ID Generation System

```rust
/// Generate deterministic IDs for transformed curves
pub struct IdGenerator {
    counter: AtomicU64,
}

impl IdGenerator {
    /// Generate ID for bumped curve
    pub fn bump_id(&self, base: &CurveId, spec: &BumpSpec) -> CurveId {
        let suffix = match spec {
            BumpSpec::ParallelBp(bp) => format!("bump_{:.0}bp", bp),
            BumpSpec::Multiplier(factor) => format!("mult_{:.2}", factor),
            // ... other cases
        };
        CurveId::new(format!("{}_{}", base, suffix))
    }
    
    /// Generate unique ID for complex transformations
    pub fn transform_id(&self, base: &CurveId) -> CurveId {
        let id = self.counter.fetch_add(1, Ordering::SeqCst);
        CurveId::new(format!("{}_t{:08x}", base, id))
    }
}
```

### 3.3 Implementation Timeline

- **Week 1**: Design metadata structures
- **Week 2**: Implement ID generation system
- **Week 3**: Migrate existing ID parsing code
- **Week 4**: Update serialization to use metadata

## Phase 4: Migration Strategy

### 4.1 Backward Compatibility Layer

```rust
/// Compatibility wrapper for existing code
pub struct MarketContextCompat {
    inner: MarketContext,
}

impl MarketContextCompat {
    /// Get discount curve with old trait object interface
    pub fn disc(&self, id: impl AsRef<str>) -> Result<Arc<dyn Discount + Send + Sync>> {
        self.inner.curves
            .get(id.as_ref())
            .and_then(|storage| storage.as_discount())
            .ok_or_else(|| InputError::NotFound { id: id.as_ref().to_string() }.into())
    }
}
```

### 4.2 Migration Steps

1. **Stage 1: Parallel Implementation**
   - Implement new system alongside existing
   - Add feature flag `new-storage` for opt-in

2. **Stage 2: Gradual Migration**
   - Update internal code to use new system
   - Keep public API unchanged
   - Run both systems in parallel for validation

3. **Stage 3: Switch Default**
   - Make new system default
   - Old system behind `legacy-storage` flag

4. **Stage 4: Deprecation**
   - Mark old system as deprecated
   - Provide migration guide

5. **Stage 5: Removal**
   - Remove old system in next major version

### 4.3 Timeline

- **Month 1**: Implement parallel system
- **Month 2**: Internal migration and testing
- **Month 3**: Switch defaults and deprecation
- **Month 4**: Clean up and documentation

## Phase 5: Testing Strategy

### 5.1 Test Categories

1. **Unit Tests**
   - CurveStorage enum operations
   - BumpMetadata serialization
   - ID generation determinism

2. **Integration Tests**
   - Full serialization round-trip
   - Bump application and tracking
   - Backward compatibility

3. **Property Tests**
   - Serialization invariants
   - Bump composition properties
   - Metadata consistency

### 5.2 Test Implementation

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn serialization_round_trip(context in arb_market_context()) {
            let serialized = serde_json::to_string(&context)?;
            let deserialized: MarketContext = serde_json::from_str(&serialized)?;
            prop_assert_eq!(context, deserialized);
        }
        
        #[test]
        fn bump_metadata_preserved(bumps in arb_bump_specs()) {
            let context = create_test_context();
            let bumped = context.bump(bumps.clone())?;
            
            for (id, spec) in bumps {
                let bumped_id = generate_bump_id(&id, &spec);
                let metadata = bumped.bumps.get(&bumped_id);
                prop_assert!(metadata.is_some());
                prop_assert_eq!(metadata.unwrap().spec, spec);
            }
        }
    }
}
```

## Benefits of New Architecture

### 1. **Complete Serialization**
- All curve types can be serialized
- Bump metadata is preserved
- No information loss

### 2. **Type Safety**
- Enum ensures exhaustive matching
- No runtime type errors
- Compile-time guarantees

### 3. **Performance**
- Direct access to concrete types
- No dynamic dispatch overhead for serialization
- Efficient memory layout

### 4. **Maintainability**
- No string parsing
- Clear separation of concerns
- Self-documenting code

### 5. **Extensibility**
- Easy to add new curve types
- Bump system can be extended
- Metadata system is flexible

## Risk Mitigation

### Risks and Mitigations

1. **Breaking Changes**
   - Mitigation: Compatibility layer and gradual migration

2. **Performance Regression**
   - Mitigation: Benchmark before/after, optimize hot paths

3. **Serialization Format Changes**
   - Mitigation: Version field in serialized data

4. **Complex Migration**
   - Mitigation: Feature flags and parallel systems

## Success Criteria

- [ ] All curve types fully serializable
- [ ] No string parsing for curve detection
- [ ] Bump metadata tracked separately
- [ ] 100% backward compatibility maintained
- [ ] Performance within 5% of current
- [ ] All tests passing
- [ ] Documentation complete

## Estimated Timeline

- **Phase 1**: 4 weeks - Enum-based storage
- **Phase 2**: 4 weeks - Bump metadata system  
- **Phase 3**: 4 weeks - Structured metadata
- **Phase 4**: 4 months - Migration (can overlap)
- **Phase 5**: 2 weeks - Testing and validation

**Total**: 4-6 months for complete implementation

## Next Immediate Steps

1. Create feature branch `feature/market-context-redesign`
2. Implement CurveStorage enum (Phase 1.1)
3. Write comprehensive tests for new enum
4. Create proof-of-concept for migration strategy
5. Get stakeholder approval before proceeding

## Conclusion

This plan provides a clear path to eliminate all serialization limitations in MarketContext while maintaining backward compatibility. The phased approach minimizes risk and allows for validation at each stage.
