# MarketContextV2 Simplification Plan

## Goal: Clean, Direct API with Zero Overhead

Remove all backward compatibility layers and trait object conversions to create the cleanest possible API that works directly with concrete types.

## Current API Complexity

### Dual Getter Problem
```rust
// Current: Confusing dual API
context.disc("USD-OIS")?;           // Returns Arc<dyn Discount>
context.discount_curve("USD-OIS")?; // Returns &Arc<DiscountCurve>

// Current: Confusing CurveStorage methods  
storage.as_discount()?;           // Returns Arc<dyn Discount>
storage.as_concrete_discount()?;  // Returns &Arc<DiscountCurve>
```

## Proposed Simplified API

### Single, Clean Getters
```rust
// Simplified: One clear method per type
context.discount("USD-OIS")?;     // Returns Arc<DiscountCurve>
context.forward("USD-SOFR3M")?;   // Returns Arc<ForwardCurve>  
context.hazard("CORP")?;          // Returns Arc<HazardCurve>
context.inflation("US-CPI")?;     // Returns Arc<InflationCurve>
context.base_correlation("CDX")?; // Returns Arc<BaseCorrelationCurve>
```

### Simplified CurveStorage
```rust
// Remove all trait object conversions
impl CurveStorage {
    // Keep only: id(), curve_type(), is_*() methods
    // Remove: as_discount(), as_concrete_discount(), etc.
    
    // Add simple extractors:
    pub fn into_discount(self) -> Option<Arc<DiscountCurve>>
    pub fn into_forward(self) -> Option<Arc<ForwardCurve>>
    // etc.
}
```

## Implementation Steps

### Step 1: Simplify CurveStorage (30 minutes)
- Remove all `as_*()` conversion methods
- Keep only type checking and extraction methods
- Simplify the enum interface

### Step 2: Simplify MarketContextV2 Getters (30 minutes)  
- Remove dual API methods (`disc()`, `fwd()`, etc.)
- Rename concrete methods to be primary API
- Return `Arc<T>` directly instead of `&Arc<T>`

### Step 3: Update Tests (30 minutes)
- Update all tests to use simplified API
- Remove trait object usage in tests
- Verify all functionality works with concrete types

### Step 4: Update Documentation (15 minutes)
- Update examples to show clean API
- Remove backward compatibility mentions
- Highlight simplification benefits

## Expected Benefits

### Performance
- **Zero trait object overhead** - all access is direct
- **Faster compilation** - fewer generic conversions
- **Better inlining** - concrete types optimize better

### Developer Experience  
- **Clearer API** - one method per curve type
- **Type safety** - you get exactly what you expect
- **Simpler docs** - no confusion about which method to use

### Maintenance
- **Less code** - ~50% fewer methods
- **Fewer tests** - no dual API testing needed
- **Cleaner architecture** - single responsibility principle

## Migration Impact

### Who's Affected
Since MarketContextV2 is:
- Feature-gated (`new-context`)
- Not yet used in production
- Experimental

**Impact: ZERO** - No existing code depends on the current API.

### V1 Compatibility
This change only affects MarketContextV2. Original MarketContext (V1) remains unchanged, so existing code continues to work perfectly.

## Implementation Details

### Before (Complex)
```rust
impl MarketContextV2 {
    // Dual API - confusing
    pub fn disc(&self, id: &str) -> Result<Arc<dyn Discount + Send + Sync>>
    pub fn discount_curve(&self, id: &str) -> Result<&Arc<DiscountCurve>>
}

impl CurveStorage {
    // Complex conversions
    pub fn as_discount(&self) -> Option<Arc<dyn Discount + Send + Sync>>
    pub fn as_concrete_discount(&self) -> Option<&Arc<DiscountCurve>>
}
```

### After (Simple)
```rust
impl MarketContextV2 {
    // Single, clear API
    pub fn discount(&self, id: &str) -> Result<Arc<DiscountCurve>>
    pub fn forward(&self, id: &str) -> Result<Arc<ForwardCurve>>
    pub fn hazard(&self, id: &str) -> Result<Arc<HazardCurve>>
}

impl CurveStorage {
    // Simple extraction
    pub fn into_discount(self) -> Option<Arc<DiscountCurve>>
    pub fn discount(&self) -> Option<&Arc<DiscountCurve>>
}
```

### Usage Examples
```rust
// Simple, direct usage
let context = MarketContextV2::new()
    .insert_discount(discount_curve)
    .insert_forward(forward_curve);

// Direct concrete access - no trait objects
let disc = context.discount("USD-OIS")?;
let rate = disc.zero(1.0);

let fwd = context.forward("USD-SOFR3M")?; 
let forward_rate = fwd.rate(1.0);
```

## Risk Assessment

### Risks: MINIMAL
- ✅ No production code affected (feature-gated)
- ✅ No breaking changes to V1
- ✅ All tests can be easily updated
- ✅ Implementation is straightforward

### Mitigation
- Keep changes isolated to MarketContextV2
- Update all tests immediately
- Document the simplified API clearly

## Timeline

### Total Time: ~2 hours

1. **Step 1 (30 min)**: Simplify CurveStorage
2. **Step 2 (30 min)**: Simplify MarketContextV2 getters  
3. **Step 3 (30 min)**: Update all tests
4. **Step 4 (15 min)**: Update documentation
5. **Step 5 (15 min)**: Run full test suite and verify

## Success Criteria

- [ ] Single getter method per curve type
- [ ] No trait object conversions anywhere
- [ ] All tests pass with simplified API
- [ ] Performance improvement measurable
- [ ] Code is significantly cleaner

## Next Actions

1. Start with Step 1: Simplify CurveStorage enum
2. Move to Step 2: Clean up MarketContextV2 methods
3. Update tests immediately to prevent drift
4. Measure performance improvement
5. Document the clean, simple API

This simplification will make MarketContextV2 the cleanest, fastest, and most maintainable market data context in the codebase!
