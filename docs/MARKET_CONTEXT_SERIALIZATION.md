# MarketContext Serialization Implementation

## Overview
MarketContext serialization has been implemented using a state pattern approach to handle trait objects. This document outlines the current implementation status and identifies areas requiring future work.

## Current Implementation Status

### ✅ Fully Serializable Types (Have State Methods)

#### 1. **HazardCurve**
- **State Type**: `HazardCurveState`
- **Methods**: `to_state()`, `from_state()`
- **Location**: `finstack/core/src/market_data/term_structures/hazard_curve.rs`
- **Status**: ✅ Complete

#### 2. **VolSurface**
- **State Type**: `VolSurfaceState`
- **Methods**: `to_state()`, `from_state()`
- **Location**: `finstack/core/src/market_data/surfaces/vol_surface.rs`
- **Status**: ✅ Complete

#### 3. **ScalarTimeSeries**
- **State Type**: `ScalarTimeSeriesState`
- **Methods**: `to_state()`, `from_state()`
- **Location**: `finstack/core/src/market_data/primitives.rs`
- **Status**: ✅ Complete

#### 4. **BaseCorrelationCurve**
- **Directly Serializable**: Implements `Clone` + `Serialize`/`Deserialize`
- **Location**: `finstack/core/src/market_data/term_structures/base_correlation.rs`
- **Status**: ✅ Complete

#### 5. **MarketScalar**
- **Directly Serializable**: Implements `Clone` + `Serialize`/`Deserialize`
- **Location**: `finstack/core/src/market_data/primitives.rs`
- **Status**: ✅ Complete

### ⚠️ Partially Serializable Types

#### 1. **InflationIndex**
- **Current**: Can extract and reconstruct from observations
- **Location**: `finstack/core/src/market_data/inflation_index.rs`
- **Status**: ⚠️ Works but could benefit from dedicated state methods

#### 2. **CreditIndexData**
- **Current**: Can decompose and reconstruct components
- **Location**: `finstack/core/src/market_data/credit_index.rs`
- **Status**: ⚠️ Works through component serialization

### ❌ Types Requiring State Methods

#### 1. **DiscountCurve**
- **Location**: `finstack/core/src/market_data/term_structures/discount_curve.rs`
- **Current Issue**: No `Clone`, no state methods
- **Required Implementation**:
```rust
// Add to discount_curve.rs
#[cfg(feature = "serde")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscountCurveState {
    pub id: String,
    pub base_date: Date,
    pub knots: Vec<F>,
    pub dfs: Vec<F>,
    pub interp_style: InterpStyle,
}

impl DiscountCurve {
    #[cfg(feature = "serde")]
    pub fn to_state(&self) -> DiscountCurveState {
        // Extract internal state
    }
    
    #[cfg(feature = "serde")]
    pub fn from_state(state: DiscountCurveState) -> Result<Self> {
        // Reconstruct from state
    }
}
```

#### 2. **ForwardCurve**
- **Location**: `finstack/core/src/market_data/term_structures/forward_curve.rs`
- **Current Issue**: No `Clone`, no state methods
- **Required Implementation**:
```rust
// Add to forward_curve.rs
#[cfg(feature = "serde")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForwardCurveState {
    pub id: String,
    pub base_date: Date,
    pub tenor: F,
    pub knots: Vec<F>,
    pub rates: Vec<F>,
    pub interp_style: InterpStyle,
}

impl ForwardCurve {
    #[cfg(feature = "serde")]
    pub fn to_state(&self) -> ForwardCurveState {
        // Extract internal state
    }
    
    #[cfg(feature = "serde")]
    pub fn from_state(state: ForwardCurveState) -> Result<Self> {
        // Reconstruct from state
    }
}
```

#### 3. **InflationCurve**
- **Location**: `finstack/core/src/market_data/term_structures/inflation.rs`
- **Current Issue**: Has custom Serialize/Deserialize but no Clone or state methods
- **Required Implementation**:
```rust
// Add to inflation.rs
#[cfg(feature = "serde")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InflationCurveState {
    pub id: String,
    pub base_cpi: F,
    pub knots: Vec<F>,
    pub cpi_levels: Vec<F>,
    pub interp_style: InterpStyle,
}

impl InflationCurve {
    #[cfg(feature = "serde")]
    pub fn to_state(&self) -> InflationCurveState {
        // Extract internal state
    }
    
    #[cfg(feature = "serde")]
    pub fn from_state(state: InflationCurveState) -> Result<Self> {
        // Reconstruct from state
    }
}
```

#### 4. **FxMatrix**
- **Location**: `finstack/core/src/money/fx.rs`
- **Current Issue**: Complex internal state with provider trait
- **Required Implementation**:
```rust
// Add to fx.rs
#[cfg(feature = "serde")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FxMatrixState {
    pub quotes: Vec<((Currency, Currency), F)>,
    pub pivot_currency: Option<Currency>,
    pub default_policy: FxConversionPolicy,
}

impl FxMatrix {
    #[cfg(feature = "serde")]
    pub fn to_state(&self) -> FxMatrixState {
        // Extract cached quotes and configuration
    }
    
    #[cfg(feature = "serde")]
    pub fn from_state(state: FxMatrixState) -> Result<Self> {
        // Reconstruct with static provider
    }
}
```

## Current Workarounds

### Bumped Curves
- **Solution**: Store bump specifications and reconstruct by applying bumps
- **Implementation**: `BumpInfo` struct with original curve ID and `BumpSpec`
- **Limitation**: Requires original curve to exist in context

### Collateral Mappings
- **Issue**: Requires `&'static str` for CSA codes
- **Workaround**: Using `Box::leak()` to create static strings
- **Better Solution**: Refactor to use `String` or `Arc<str>` instead

## Implementation Priority

### High Priority (Core Functionality)
1. **DiscountCurve** - Most commonly used curve type
2. **ForwardCurve** - Essential for forward rate agreements
3. **InflationCurve** - Needed for inflation-linked instruments

### Medium Priority (Enhanced Features)
4. **FxMatrix** - Important for multi-currency support
5. **InflationIndex** - Improve with dedicated state methods

### Low Priority (Already Working)
6. **Collateral Mappings** - Works but could use cleaner implementation

## Testing Coverage

### Current Test Coverage
- ✅ Empty context serialization
- ✅ HazardCurve serialization
- ✅ BaseCorrelationCurve serialization
- ✅ VolSurface serialization
- ✅ MarketScalar prices
- ✅ ScalarTimeSeries
- ✅ Comprehensive multi-type context
- ✅ JSON round-trip

### Tests Needed After Implementation
- [ ] DiscountCurve serialization
- [ ] ForwardCurve serialization
- [ ] InflationCurve serialization
- [ ] Bumped curve reconstruction
- [ ] FxMatrix serialization
- [ ] Cross-version compatibility

## Migration Guide

When adding state methods to a type:

1. **Define State Struct**: Create a serializable struct containing all necessary data
2. **Implement `to_state()`**: Extract internal state to the state struct
3. **Implement `from_state()`**: Reconstruct the type from state struct
4. **Update `context.rs`**: Modify serialization logic to use state methods
5. **Add Tests**: Create test cases for the new serialization
6. **Update Documentation**: Mark type as complete in this document

## Example Implementation Pattern

```rust
// Standard pattern for adding state methods
impl YourCurveType {
    #[cfg(feature = "serde")]
    pub fn to_state(&self) -> YourCurveState {
        YourCurveState {
            id: self.id.to_string(),
            // ... extract other fields
        }
    }
    
    #[cfg(feature = "serde")]
    pub fn from_state(state: YourCurveState) -> crate::Result<Self> {
        // Use builder pattern if available
        Self::builder(&state.id)
            // ... set other fields
            .build()
    }
}
```

## Known Issues

### Interpolation Types
Some interpolation types (`CubicHermite`, `MonotoneConvex`) don't implement `Clone`, which causes serialization issues. These need to be fixed in:
- `finstack/core/src/math/interp/cubic_hermite.rs`
- `finstack/core/src/math/interp/monotone_convex.rs`

Add `#[derive(Clone)]` to these types to enable full serialization support.

## File Locations Summary

| Component | File | Status |
|-----------|------|--------|
| MarketContext serialization | `finstack/core/src/market_data/context.rs` | ✅ |
| Serialization types | `finstack/core/src/market_data/context_serde.rs` | ✅ |
| Tests | `finstack/core/src/market_data/test_context_serde.rs` | ✅ |
| DiscountCurve | `finstack/core/src/market_data/term_structures/discount_curve.rs` | ❌ |
| ForwardCurve | `finstack/core/src/market_data/term_structures/forward_curve.rs` | ❌ |
| InflationCurve | `finstack/core/src/market_data/term_structures/inflation.rs` | ❌ |
| FxMatrix | `finstack/core/src/money/fx.rs` | ❌ |
