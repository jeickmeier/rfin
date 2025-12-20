# Implementation Plan: Finstack Code Consolidation

## Overview

This plan breaks down the "Marge List" code consolidation into incremental, testable milestones. Each phase can be implemented, tested, and merged independently to minimize risk.

**Priority Order**: Phases 1-3 are high-value refactorings that eliminate the most duplication. Phases 4-6 are lower-priority polish.

---

## Phase 1: Market Data Curve Restoration [HIGH PRIORITY]

**Impact**: 327 lines → ~80 lines (75% reduction)  
**Estimated Time**: 3-4 days

### [x] Step: Technical Specification
✅ Created comprehensive technical specification in spec.md

### [x] Step 1.1: Add bitflags dependency and CurveRestoreFlags
<!-- chat-id: d2b8f327-d90c-40e6-bdab-d62951c6e506 -->
**File**: `finstack/valuations/Cargo.toml`, `finstack/valuations/src/attribution/factors.rs`

**Tasks**:
- ✅ Add `bitflags = "2.4"` to valuations Cargo.toml
- ✅ Define `CurveRestoreFlags` bitflags enum in factors.rs (lines after existing imports)
- ✅ Add constants: DISCOUNT, FORWARD, HAZARD, INFLATION, CORRELATION
- ✅ Add convenience combinations: RATES, CREDIT
- ✅ Write unit tests for bitflag operations (union, intersection, complement)

**Verification**:
```bash
cd finstack/valuations
cargo test --lib attribution::factors::tests::test_curve_restore_flags
```

**Acceptance**:
- ✅ Bitflags compile without warnings
- ✅ All combination operations work correctly
- ✅ Tests pass for flag manipulation

---

### [ ] Step 1.2: Create unified MarketSnapshot struct
**File**: `finstack/valuations/src/attribution/factors.rs`

**Tasks**:
- Add `MarketSnapshot` struct after existing snapshot types (~line 70)
- Include all 5 curve type HashMap fields
- Derive Clone, Debug, Default
- Implement `MarketSnapshot::extract(market, flags)` method
- Add unit tests for extraction with various flag combinations

**Verification**:
```bash
cargo test --lib attribution::factors::tests::test_market_snapshot_extract
```

**Acceptance**:
- ✅ Struct compiles and derives work
- ✅ Extract method correctly filters by flags
- ✅ Tests cover single flags, combinations, and empty markets

---

### [ ] Step 1.3: Implement unified restore_market() function
**File**: `finstack/valuations/src/attribution/factors.rs`

**Tasks**:
- Implement `restore_market(current, snapshot, flags)` function
- Use bitflag complement to determine preserved curves
- Insert preserved curves first, then snapshot curves
- Copy FX, surfaces, scalars (always preserved)
- Add helper to copy scalars (extract existing `copy_scalars` if not already separate)

**Verification**:
```bash
cargo test --lib attribution::factors::tests::test_restore_market_unified
```

**Acceptance**:
- ✅ Function compiles and handles all flag combinations
- ✅ Preserved curves are not overwritten
- ✅ Snapshot curves are correctly inserted
- ✅ FX/surfaces/scalars always copied

---

### [ ] Step 1.4: Refactor existing restore_*_curves() as wrappers
**File**: `finstack/valuations/src/attribution/factors.rs`

**Tasks**:
- Update `restore_rates_curves()` to call `restore_market()` with RATES flag
- Update `restore_credit_curves()` to call `restore_market()` with CREDIT flag
- Update `restore_inflation_curves()` to call `restore_market()` with INFLATION flag
- Update `restore_correlations()` to call `restore_market()` with CORRELATION flag
- Keep function signatures unchanged (backward compatibility)

**Verification**:
```bash
cargo test --lib attribution::factors
cargo test --test integration_attribution
```

**Acceptance**:
- ✅ All existing tests pass unchanged
- ✅ Wrapper functions are 5-10 lines each (down from ~80)
- ✅ No change in behavior (golden output tests)

---

### [ ] Step 1.5: Add equivalence tests (old vs new)
**File**: `finstack/valuations/src/attribution/factors.rs` (test module)

**Tasks**:
- Create test helper `assert_market_contexts_equal()`
- Add equivalence test for each restore function
- Compare curve counts, curve IDs, FX presence
- Verify DF values match at sample dates

**Verification**:
```bash
cargo test --lib attribution::factors::tests::test_restore_equivalence
```

**Acceptance**:
- ✅ Old and new implementations produce identical results
- ✅ Tests cover all 4 restore functions
- ✅ Edge cases: empty markets, missing curves, mixed types

---

### [ ] Step 1.6: Phase 1 integration and documentation
**Files**: `finstack/valuations/src/attribution/factors.rs`, `finstack/valuations/CHANGELOG.md`

**Tasks**:
- Add module-level documentation explaining unified approach
- Document CurveRestoreFlags with examples
- Add inline comments to `restore_market()` explaining logic
- Run full test suite and benchmarks
- Update CHANGELOG with refactoring notes

**Verification**:
```bash
make test-rust
make lint-rust
cd finstack/valuations && cargo bench --bench attribution
cargo doc --no-deps --open
```

**Acceptance**:
- ✅ All tests pass (valuations + integration)
- ✅ No clippy warnings
- ✅ Benchmarks show <5% regression (ideally 0%)
- ✅ Documentation builds and is clear
- ✅ Ready for PR review

---

## Phase 2: Monte Carlo Payoff Consolidation [MEDIUM PRIORITY]

**Impact**: ~150 lines → ~50 lines per pair (66% reduction)  
**Estimated Time**: 2-3 days

### [ ] Step 2.1: Merge CapPayoff and FloorPayoff
**File**: `finstack/valuations/src/instruments/common/models/monte_carlo/payoff/rates.rs`

**Tasks**:
- Add `RatesPayoffType` enum (Cap, Floor)
- Create unified `RatesPayoff` struct with `payoff_type` field
- Merge `impl Payoff for CapPayoff` and `FloorPayoff` into single impl
- Use match on `payoff_type` for the one diverging line
- Keep `CapPayoff` and `FloorPayoff` as type aliases (deprecated)

**Verification**:
```bash
cargo test --lib instruments::common::models::monte_carlo::payoff::tests
```

**Acceptance**:
- ✅ Unified struct compiles and tests pass
- ✅ Behavior identical to original implementations
- ✅ Type aliases maintain backward compatibility

---

### [ ] Step 2.2: Merge LookbackCall and LookbackPut
**File**: `finstack/valuations/src/instruments/common/models/monte_carlo/payoff/lookback.rs`

**Tasks**:
- Add `LookbackDirection` enum (Call, Put)
- Create unified `Lookback` struct with `direction` field
- Implement `new()` to initialize `extreme_spot` based on direction
- Merge `on_event()` implementations with match on direction
- Add type aliases for backward compatibility

**Verification**:
```bash
cargo test --lib instruments::common::models::monte_carlo::payoff::lookback
```

**Acceptance**:
- ✅ Unified struct compiles and tests pass
- ✅ Extreme tracking (min/max) works correctly
- ✅ Backward-compatible aliases work

---

### [ ] Step 2.3: Monte Carlo integration tests
**Files**: Existing MC integration tests

**Tasks**:
- Run full MC test suite with new unified payoffs
- Verify pricing matches original implementations
- Test edge cases: zero strike, extreme volatility, long maturities
- Compare against analytical formulas where available

**Verification**:
```bash
cargo test --test integration_monte_carlo
cargo bench --bench monte_carlo
```

**Acceptance**:
- ✅ All integration tests pass
- ✅ Prices match within 1bp of old implementation
- ✅ No performance regression

---

## Phase 3: Parameter Reduction via Context Structs [MEDIUM PRIORITY]

**Impact**: 15-parameter functions → 2-parameter functions  
**Estimated Time**: 3-4 days

### [ ] Step 3.1: Create AllocationContext and AllocationOutput
**File**: `finstack/valuations/src/instruments/structured_credit/pricing/waterfall.rs`

**Tasks**:
- Add `AllocationContext<'a>` struct before allocation functions (~line 90200)
- Include all 11 input parameters as fields
- Add `AllocationOutput<'a>` struct for mutable outputs (3 fields)
- Add constructor methods with validation

**Verification**:
```bash
cargo build --lib
```

**Acceptance**:
- ✅ Structs compile with correct lifetimes
- ✅ Fields are public and accessible
- ✅ Validation methods enforce invariants

---

### [ ] Step 3.2: Refactor allocate_pro_rata() and allocate_sequential()
**File**: `finstack/valuations/src/instruments/structured_credit/pricing/waterfall.rs`

**Tasks**:
- Update `allocate_pro_rata()` signature to take context structs
- Keep internal logic identical initially
- Update `allocate_sequential()` similarly
- Update all call sites to construct context structs

**Verification**:
```bash
cargo test --lib instruments::structured_credit::pricing::waterfall
```

**Acceptance**:
- ✅ Functions compile with new signatures
- ✅ All tests pass unchanged
- ✅ Call sites updated correctly

---

### [ ] Step 3.3: Create unified execute_waterfall_core()
**File**: `finstack/valuations/src/instruments/structured_credit/pricing/waterfall.rs`

**Tasks**:
- Implement `execute_waterfall_core()` with optional workspace parameter
- Merge logic from `execute_waterfall_with_explanation()` and `execute_waterfall_with_workspace()`
- Use `Option<&mut WaterfallWorkspace>` to branch between local and workspace state
- Update wrapper functions to call core implementation

**Verification**:
```bash
cargo test --lib instruments::structured_credit::pricing::waterfall
cargo test --test integration_waterfall
```

**Acceptance**:
- ✅ Core function handles both workspace and non-workspace cases
- ✅ Wrapper functions are thin (2-3 lines)
- ✅ All tests pass with identical results

---

### [ ] Step 3.4: Create AttributionInput context struct
**Files**: 
- `finstack/valuations/src/attribution/parallel.rs`
- `finstack/valuations/src/attribution/waterfall.rs`
- `finstack/valuations/src/attribution/metrics_based.rs`

**Tasks**:
- Add `AttributionInput<'a>` struct in `attribution/mod.rs` or `types.rs`
- Add `AttributionMethod` enum (Parallel, Waterfall, MetricsBased)
- Refactor `attribute_pnl_parallel()` to use context struct
- Refactor `attribute_pnl_waterfall()` similarly
- Refactor `attribute_pnl_metrics_based()` similarly

**Verification**:
```bash
cargo test --lib attribution
cargo test --test integration_attribution
```

**Acceptance**:
- ✅ Context struct reduces parameter counts to 2-3
- ✅ All attribution methods use unified input struct
- ✅ Tests pass unchanged

---

## Phase 4: Trait-Based Market Data Extraction [LOWER PRIORITY]

**Impact**: 6 functions → 1 generic + 6 trait impls  
**Estimated Time**: 2 days

### [ ] Step 4.1: Define MarketExtractable trait
**File**: `finstack/valuations/src/attribution/factors.rs`

**Tasks**:
- Add `MarketExtractable` trait with `extract(market) -> Self` method
- Add documentation explaining trait purpose
- Add generic `extract::<T>(market)` helper function

**Verification**:
```bash
cargo build --lib
```

**Acceptance**:
- ✅ Trait compiles and is well-documented
- ✅ Generic helper works with type inference

---

### [ ] Step 4.2: Implement trait for all snapshot types
**File**: `finstack/valuations/src/attribution/factors.rs`

**Tasks**:
- Implement `MarketExtractable` for `RatesCurvesSnapshot`
- Implement for `CreditCurvesSnapshot`
- Implement for `InflationCurvesSnapshot`
- Implement for `CorrelationsSnapshot`
- Implement for `VolatilitySnapshot`
- Implement for `ScalarsSnapshot`
- Move current extraction logic into trait methods

**Verification**:
```bash
cargo test --lib attribution::factors
```

**Acceptance**:
- ✅ All snapshot types implement trait correctly
- ✅ Extraction behavior unchanged
- ✅ Generic function works with all types

---

### [ ] Step 4.3: Update call sites and deprecate old functions
**Files**: Multiple attribution files

**Tasks**:
- Update call sites to use `extract::<T>()` or `T::extract()`
- Mark old `extract_*_curves()` functions as `#[deprecated]`
- Add deprecation messages with migration guidance
- Update documentation to recommend new trait-based approach

**Verification**:
```bash
make test-rust
cargo doc --no-deps
```

**Acceptance**:
- ✅ All call sites updated
- ✅ Deprecation warnings compile correctly
- ✅ Documentation explains migration

---

## Phase 5: Waterfall Execution Unification [LOWER PRIORITY]

**Impact**: 200+ duplicate lines → single implementation  
**Estimated Time**: 2-3 days

### [ ] Step 5.1: Implement execute_waterfall_core() (if not done in Phase 3.3)
**File**: `finstack/valuations/src/instruments/structured_credit/pricing/waterfall.rs`

**Tasks**:
- If not already done in Phase 3.3, implement unified core function
- Handle optional workspace parameter with branching logic
- Ensure determinism regardless of workspace usage

**Verification**:
```bash
cargo test --lib instruments::structured_credit::pricing::waterfall
```

**Acceptance**:
- ✅ Core function works with and without workspace
- ✅ Identical results in both cases
- ✅ No code duplication

---

### [ ] Step 5.2: Integration testing and benchmarking
**Files**: Integration tests, benchmarks

**Tasks**:
- Run full structured credit test suite
- Compare outputs with golden files
- Run waterfall benchmarks
- Verify no regression in performance or correctness

**Verification**:
```bash
cargo test --test integration_structured_credit
cargo bench --bench waterfall
```

**Acceptance**:
- ✅ All tests pass
- ✅ Outputs match golden files
- ✅ Performance within 5% of original

---

## Phase 6: JSON Envelope Boilerplate [LOWER PRIORITY]

**Impact**: Eliminate ~30 lines per envelope type (8+ types)  
**Estimated Time**: 1-2 days

### [ ] Step 6.1: Define JsonEnvelope trait
**File**: `finstack/valuations/src/attribution/types.rs` or new `envelope.rs`

**Tasks**:
- Add `JsonEnvelope` trait with default methods
- Include `from_json`, `from_reader`, `to_json` methods
- Define error conversion methods (abstract)
- Add comprehensive documentation with examples

**Verification**:
```bash
cargo build --lib
```

**Acceptance**:
- ✅ Trait compiles and default methods work
- ✅ Documentation is clear with usage examples

---

### [ ] Step 6.2: Implement trait for all envelope types
**Files**: Multiple attribution envelope types

**Tasks**:
- Implement `JsonEnvelope` for `AttributionEnvelope`
- Implement for `PnlAttribution`
- Implement for other envelope types (6-8 total)
- Remove duplicate method definitions
- Keep only error conversion implementations

**Verification**:
```bash
cargo test --lib attribution
```

**Acceptance**:
- ✅ All envelope types implement trait
- ✅ JSON serialization/deserialization works
- ✅ Reduced boilerplate by ~30 lines per type

---

## Final Integration and Release

### [ ] Step: Final verification and documentation
**Files**: Multiple, documentation, CHANGELOG

**Tasks**:
- Run full test suite across all crates
- Run all benchmarks and verify no regressions
- Update main README with refactoring summary
- Update CHANGELOG with all changes
- Review deprecation warnings and migration guides
- Prepare release notes

**Verification**:
```bash
make test-rust
make lint-rust
make test-wasm
make test-python
cd finstack && cargo doc --no-deps --all-features
```

**Acceptance**:
- ✅ All tests pass (Rust + WASM + Python)
- ✅ Zero clippy warnings
- ✅ All benchmarks show <5% regression
- ✅ Documentation is complete and accurate
- ✅ CHANGELOG is updated
- ✅ Migration guides are clear

---

### [ ] Step: Create pull request and review
**Tasks**:
- Create PR with detailed description
- Link to spec.md and this plan
- Request reviews from:
  - Quant team (for Phase 2 Monte Carlo changes)
  - Structuring desk (for Phase 3 waterfall changes)
  - Core maintainers (for overall architecture)
- Address review comments
- Merge when approved

**Acceptance**:
- ✅ PR approved by all reviewers
- ✅ CI/CD passes
- ✅ Merged to main branch

---

## Success Metrics Summary

**After completion, verify**:
- ✅ Reduced duplication by 500+ lines
- ✅ Parameter counts reduced from 15+ to 2-3 in waterfall functions
- ✅ Zero test failures
- ✅ Zero clippy warnings
- ✅ <5% performance regression in any benchmark
- ✅ 100% backward compatibility maintained
- ✅ All deprecated functions have migration guidance
- ✅ Documentation is clear and comprehensive

---

## Rollback Plan

If issues arise in production:

1. **Immediate**: Revert PR and redeploy previous version
2. **Investigate**: Determine root cause using test suite and logs
3. **Fix**: Either fix forward or plan phased rollback
4. **Re-test**: Full test suite + production smoke tests
5. **Re-deploy**: With extra monitoring and gradual rollout

**Rollback triggers**:
- Attribution P&L differs by >1bp from previous version
- Monte Carlo prices outside acceptable tolerances
- Waterfall distributions fail conservation checks
- Performance regression >10%
- Production crashes or errors

