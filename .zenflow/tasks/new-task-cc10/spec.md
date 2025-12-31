# Technical Specification: Code Cleanup - "Kill List"

## Difficulty Assessment: Medium

This task requires removing redundant and vestigial code across multiple files in the valuations crate. While the individual changes are straightforward, the task requires careful verification to ensure no breaking changes and proper testing after modifications.

## Technical Context

- **Language**: Rust
- **Primary Crate**: `finstack-valuations`
- **Dependencies**: `finstack-core` (for types like `Money`, `Currency`)
- **Target Files**:
  - `finstack/valuations/src/attribution/factors.rs`
  - `finstack/valuations/src/attribution/parallel.rs`
  - `finstack/valuations/src/instruments/common/models/monte_carlo/payoff/rates.rs`

## Current State Analysis

### 1. Dead Function: `freeze_all_market` (Line 533)

**Location**: `finstack/valuations/src/attribution/factors.rs:533`

**Current Implementation**:
```rust
pub fn freeze_all_market(market_t0: &MarketContext, _market_t1: &MarketContext) -> MarketContext {
    market_t0.clone()
}
```

**Analysis**:
- Function accepts `market_t1` parameter but completely ignores it (prefixed with `_`)
- Function is essentially a wrapper around `.clone()`
- Used in exactly one place: `finstack/valuations/src/attribution/parallel.rs:126`
- Has a dedicated test: `test_freeze_all_market()` at line 578

**Impact**:
- Direct usage: 1 call site in `parallel.rs`
- Test coverage: 1 test in `factors.rs`

### 2. Redundant `compute_forward_rate` Methods

**Location**: `finstack/valuations/src/instruments/common/models/monte_carlo/payoff/rates.rs`

**Current Implementation**:

**CapPayoff** (line 97):
```rust
fn compute_forward_rate(&self, short_rate: f64, _idx: usize) -> f64 {
    // Simple approximation: forward rate ≈ short rate
    short_rate
}
```

**FloorPayoff** (line 191):
```rust
fn compute_forward_rate(&self, short_rate: f64, _idx: usize) -> f64 {
    short_rate
}
```

**Analysis**:
- Both methods are identical pass-through stubs
- The `_idx` parameter is unused in both implementations
- Methods are private and only called internally within the same struct
- Call sites:
  - `CapPayoff::on_event` at line 119
  - `FloorPayoff::on_event` at line 208
- Comments suggest this is a simplification pending full Hull-White implementation

**Impact**:
- These are helper methods, not part of a trait interface
- Can be inlined without breaking changes
- No external dependencies

### 3. Unused `_idx` Parameters

**Instances Found**: 5 files contain functions with unused `_idx` parameters

**Specific Cases**:
1. `compute_forward_rate` in both `CapPayoff` and `FloorPayoff` (already covered above)
2. Additional instances in:
   - `finstack/valuations/src/calibration/solver/global.rs`
   - `finstack/valuations/src/instruments/structured_credit/pricing/stochastic/tree/tree.rs`
   - `finstack/valuations/src/instruments/swaption/pricing/tree_valuator.rs`
   - `finstack/valuations/src/instruments/common/models/trees/hull_white_tree.rs`

**Note**: Need to verify if these are trait implementations before removal.

## Implementation Approach

### Phase 1: Remove `freeze_all_market`

1. **Replace call site** in `parallel.rs:126`:
   ```rust
   // Before
   let market_frozen = freeze_all_market(market_t0, market_t1);

   // After
   let market_frozen = market_t0.clone();
   ```

2. **Remove function** from `factors.rs`:
   - Delete function definition (lines 521-537)
   - Delete test `test_freeze_all_market` (lines 577-588)

3. **Remove import** if present:
   - Check and remove any `use` statements in `parallel.rs`

### Phase 2: Inline `compute_forward_rate` Stubs

1. **Update CapPayoff::on_event** (line 119):
   ```rust
   // Before
   let forward_rate = self.compute_forward_rate(short_rate, self.next_fixing_idx);

   // After
   let forward_rate = short_rate; // Simple approximation: forward rate ≈ short rate
   ```

2. **Update FloorPayoff::on_event** (line 208):
   ```rust
   // Before
   let forward_rate = self.compute_forward_rate(short_rate, self.next_fixing_idx);

   // After
   let forward_rate = short_rate; // Simple approximation: forward rate ≈ short rate
   ```

3. **Remove methods**:
   - Delete `CapPayoff::compute_forward_rate` (lines 93-102)
   - Delete `FloorPayoff::compute_forward_rate` (lines 191-193)

### Phase 3: Audit Other Unused Parameters

1. **Investigate each file** with unused `_idx` parameters
2. **Determine if parameters are**:
   - Part of a trait interface (keep with `#[allow(unused)]`)
   - Truly unused and can be removed
   - Vestigial from incomplete implementation
3. **Document findings** and remove where safe

## Source Code Structure Changes

### Files to Modify:
1. `finstack/valuations/src/attribution/factors.rs`
   - Remove: `freeze_all_market` function and test (~17 lines)

2. `finstack/valuations/src/attribution/parallel.rs`
   - Modify: Replace function call with direct `.clone()` (1 line)

3. `finstack/valuations/src/instruments/common/models/monte_carlo/payoff/rates.rs`
   - Remove: Two `compute_forward_rate` methods (~12 lines total)
   - Modify: Two call sites to inline the logic (2 lines)

### Files to Investigate:
4. `finstack/valuations/src/calibration/solver/global.rs`
5. `finstack/valuations/src/instruments/structured_credit/pricing/stochastic/tree/tree.rs`
6. `finstack/valuations/src/instruments/swaption/pricing/tree_valuator.rs`
7. `finstack/valuations/src/instruments/common/models/trees/hull_white_tree.rs`

## Verification Approach

### Testing Strategy

1. **Run valuations tests**:
   ```bash
   make test-rust
   ```
   Focus on:
   - Attribution tests (for `freeze_all_market` removal)
   - Monte Carlo payoff tests (for `compute_forward_rate` changes)

2. **Run linting**:
   ```bash
   make lint-rust
   ```
   Verify no new warnings introduced

3. **Spot check specific tests**:
   ```bash
   cargo test --package finstack-valuations attribution
   cargo test --package finstack-valuations rates_payoff
   ```

### Manual Verification

1. **Code Review Checklist**:
   - [ ] All call sites of removed functions are updated
   - [ ] No imports of removed functions remain
   - [ ] Tests that depend on removed code are also removed
   - [ ] Inline comments explain simplifications where appropriate
   - [ ] No new clippy warnings introduced

2. **Semantic Verification**:
   - [ ] `freeze_all_market` replacement maintains same behavior (cloning T₀ market)
   - [ ] `compute_forward_rate` removal maintains pass-through logic
   - [ ] Attribution calculations remain unchanged

## Data Model / API / Interface Changes

**No API changes**: All removed functions are internal implementation details, not public APIs.

**Behavioral equivalence**:
- `freeze_all_market(m0, m1)` → `m0.clone()` : Semantically identical
- `self.compute_forward_rate(r, idx)` → `r` : Semantically identical for current stub implementation

## Risk Assessment

**Low Risk**:
- Removed code is clearly vestigial or redundant
- Changes are localized with clear call site boundaries
- Comprehensive test coverage exists for affected modules

**Potential Issues**:
- If `freeze_all_market` was intended for future use (keeping structure of T₁ market), removing it loses that intent
  - **Mitigation**: Add comment at call site explaining the simplification
- Removing `compute_forward_rate` makes future Hull-White implementation slightly more work
  - **Mitigation**: Add TODO comment indicating where forward rate calculation would go in full implementation

## Success Criteria

1. All identified redundant code removed
2. All tests pass (`make test-rust`)
3. No new linting warnings (`make lint-rust`)
4. Code compiles without errors or warnings
5. Attribution parallel tests still pass
6. Monte Carlo payoff tests still pass
7. Verification report documents:
   - Lines of code removed
   - Tests updated
   - Any discovered edge cases

## Future Considerations

1. **Hull-White Implementation**: When implementing full Hull-White model, the forward rate calculation logic will need to be re-added to `CapPayoff` and `FloorPayoff`. The TODO comments should guide this.

2. **Market Context Freezing**: If future requirements need to preserve T₁ market structure while using T₀ data, a proper implementation of `freeze_all_market` should be created at that time.

3. **Parameter Audit**: Consider a broader codebase audit for unused parameters using clippy's `unused_variables` lint at `warn` level.
