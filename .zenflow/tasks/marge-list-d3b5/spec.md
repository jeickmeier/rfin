# Technical Specification: Finstack Code Consolidation ("Marge List")

## Complexity Assessment

**Difficulty**: **HARD**

**Rationale**:

- Significant architectural refactoring across 10+ files and 3 crates
- High risk of introducing regressions in critical pricing/attribution code
- Requires maintaining backward compatibility while consolidating abstractions
- Multiple interdependent refactoring activities (can't be done in isolation)
- Touches sensitive financial computation logic requiring extensive validation
- Need to coordinate changes across Rust core, WASM, and Python bindings

**Estimated Impact**:

- ~600+ lines of duplicated code to consolidate
- 15+ functions to refactor or eliminate
- 6 new traits/abstractions to introduce
- Multiple files in critical paths (valuations, attribution, Monte Carlo)

---

## Technical Context

### Language & Dependencies

- **Primary Language**: Rust (2021 edition)
- **Affected Crates**:
  - `finstack-valuations` (primary target)
  - `finstack-core` (market data abstractions)
  - `finstack-statements` (waterfall integration)
  - `finstack-wasm` (binding updates)
- **Key Dependencies**:
  - `bitflags` = "2.4" (for CurveRestoreFlags)
  - `hashbrown` = "0.14" (existing dependency)
  - `serde` = "1.0" (serialization stability)
  - Standard library traits (Default, Clone, Debug)

### Architecture Patterns

- **Trait-based polymorphism** for extraction/restoration
- **Bitflags** for type-safe combination of restore options
- **Enum-based dispatch** for payoff and allocation strategies
- **Builder pattern** with context structs for parameter reduction
- **Generic implementations** for JSON envelope boilerplate

---

## Implementation Approach

### Phase 1: Market Data Curve Restoration (Highest Priority)

**Impact**: 327 lines → ~80 lines (75% reduction)
**Files**: `finstack/valuations/src/attribution/factors.rs`

#### Current State

Four nearly-identical functions (~80 lines each):

- `restore_rates_curves()`
- `restore_credit_curves()`
- `restore_inflation_curves()`
- `restore_correlations()`

Each manually:

1. Creates new MarketContext
2. Iterates curve_ids() with 4-5 if-let branches
3. Selectively copies non-restored curves
4. Inserts snapshot curves
5. Copies FX, surfaces, scalars

#### Target State

Single unified implementation using bitflags:

```rust
// New abstraction in factors.rs
use bitflags::bitflags;

bitflags! {
    pub struct CurveRestoreFlags: u8 {
        const DISCOUNT    = 0b0000_0001;
        const FORWARD     = 0b0000_0010;
        const HAZARD      = 0b0000_0100;
        const INFLATION   = 0b0000_1000;
        const CORRELATION = 0b0001_0000;

        const RATES  = Self::DISCOUNT.bits() | Self::FORWARD.bits();
        const CREDIT = Self::HAZARD.bits();
    }
}

pub struct MarketSnapshot {
    pub discount_curves: HashMap<CurveId, Arc<DiscountCurve>>,
    pub forward_curves: HashMap<CurveId, Arc<ForwardCurve>>,
    pub hazard_curves: HashMap<CurveId, Arc<HazardCurve>>,
    pub inflation_curves: HashMap<CurveId, Arc<InflationCurve>>,
    pub base_correlation_curves: HashMap<CurveId, Arc<BaseCorrelationCurve>>,
}

impl MarketSnapshot {
    pub fn extract(market: &MarketContext, flags: CurveRestoreFlags) -> Self;
}

pub fn restore_market(
    current_market: &MarketContext,
    snapshot: &MarketSnapshot,
    restore_flags: CurveRestoreFlags,
) -> MarketContext;

// Backward-compatible wrappers
pub fn restore_rates_curves(market: &MarketContext, snapshot: &RatesCurvesSnapshot) -> MarketContext {
    let unified = MarketSnapshot {
        discount_curves: snapshot.discount_curves.clone(),
        forward_curves: snapshot.forward_curves.clone(),
        ..Default::default()
    };
    restore_market(market, &unified, CurveRestoreFlags::RATES)
}
```

**Benefits**:

- Single source of truth for curve restoration logic
- Type-safe combinations (e.g., RATES | CREDIT)
- Extensible for future curve types
- Maintains backward compatibility via wrapper functions

---

### Phase 2: Monte Carlo Payoff Consolidation

**Impact**: ~150 lines → ~50 lines per pair (66% reduction)
**Files**:

- `finstack/valuations/src/instruments/common/models/monte_carlo/payoff/rates.rs`
- `finstack/valuations/src/instruments/common/models/monte_carlo/payoff/lookback.rs`

#### 2.1 Cap/Floor Payoff Merge

**Current**: Two 95% identical structs (CapPayoff, FloorPayoff)

**Target**:

```rust
pub enum RatesPayoffType {
    Cap,
    Floor,
}

pub struct RatesPayoff {
    pub payoff_type: RatesPayoffType,
    pub strike_rate: f64,
    pub notional: f64,
    pub fixing_dates: Vec<f64>,
    pub accrual_fractions: Vec<f64>,
    pub discount_factors: Vec<f64>,
    pub currency: Currency,
    accumulated_pv: f64,
    next_fixing_idx: usize,
}

impl Payoff for RatesPayoff {
    fn on_event(&mut self, state: &mut PathState) {
        let forward_rate = self.compute_forward_rate(state, self.next_fixing_idx);
        let payoff = match self.payoff_type {
            RatesPayoffType::Cap => (forward_rate - self.strike_rate).max(0.0),
            RatesPayoffType::Floor => (self.strike_rate - forward_rate).max(0.0),
        };
        // ... common accumulation logic
    }
}
```

#### 2.2 Lookback Call/Put Merge

**Current**: Two similar structs with opposite min/max tracking

**Target**:

```rust
pub enum LookbackDirection {
    Call,
    Put,
}

pub struct Lookback {
    pub direction: LookbackDirection,
    pub strike: f64,
    pub notional: f64,
    pub maturity_step: usize,
    pub currency: Currency,
    extreme_spot: f64, // max for Call, min for Put
    accumulated_pv: f64,
}

impl Lookback {
    pub fn new(direction: LookbackDirection, ...) -> Self {
        let extreme_spot = match direction {
            LookbackDirection::Call => f64::NEG_INFINITY,
            LookbackDirection::Put => f64::INFINITY,
        };
        Self { extreme_spot, ... }
    }
}

impl Payoff for Lookback {
    fn on_event(&mut self, state: &mut PathState) {
        match self.direction {
            LookbackDirection::Call => {
                self.extreme_spot = self.extreme_spot.max(spot);
            }
            LookbackDirection::Put => {
                self.extreme_spot = self.extreme_spot.min(spot);
            }
        }
        // ... rest identical
    }
}
```

---

### Phase 3: Parameter Reduction via Context Structs

**Impact**: 15-parameter functions → 2-parameter functions
**Files**: `finstack/valuations/src/instruments/structured_credit/pricing/waterfall.rs`

#### 3.1 Waterfall Allocation Context

**Current**: `allocate_pro_rata()` and `allocate_sequential()` take 15 parameters

**Target**:

```rust
pub struct AllocationContext<'a> {
    pub base_currency: Currency,
    pub tier: &'a WaterfallTier,
    pub recipients: &'a [Recipient],
    pub available: Money,
    pub tranches: &'a TrancheStructure,
    pub tranche_index: &'a HashMap<&'a str, usize>,
    pub pool_balance: Money,
    pub period_start: Date,
    pub payment_date: Date,
    pub market: &'a MarketContext,
    pub diverted: bool,
}

pub struct AllocationOutput<'a> {
    pub distributions: &'a mut HashMap<RecipientType, Money>,
    pub payment_records: &'a mut Vec<PaymentRecord>,
    pub trace: &'a mut Option<ExplanationTrace>,
}

fn allocate_pro_rata(
    ctx: &AllocationContext,
    output: &mut AllocationOutput,
    explain: &ExplainOpts,
) -> Result<Money>
```

**Benefits**:

- Self-documenting parameter groups
- Easy to extend without breaking signatures
- Clear separation of inputs vs outputs
- Enables partial application and testing

#### 3.2 Attribution Context

**Current**: `attribute_pnl_parallel()` and similar take 7+ mixed parameters

**Target**:

```rust
pub struct AttributionInput<'a> {
    pub instrument: &'a Arc<dyn Instrument>,
    pub market_t0: &'a MarketContext,
    pub market_t1: &'a MarketContext,
    pub as_of_t0: Date,
    pub as_of_t1: Date,
    pub config: &'a FinstackConfig,
    pub model_params_t0: Option<&'a ModelParamsSnapshot>,
}

pub enum AttributionMethod {
    Parallel,
    Waterfall,
    MetricsBased,
}

pub fn attribute_pnl(
    method: AttributionMethod,
    input: &AttributionInput
) -> Result<PnlAttribution>
```

---

### Phase 4: Trait-Based Market Data Extraction

**Impact**: 6 functions → 1 generic + 6 trait impls
**Files**: `finstack/valuations/src/attribution/factors.rs`

#### Current State

Six similar functions:

- `extract_rates_curves()`
- `extract_credit_curves()`
- `extract_inflation_curves()`
- `extract_correlations()`
- `extract_volatility()`
- `extract_scalars()`

#### Target State

```rust
pub trait MarketExtractable: Sized {
    fn extract(market: &MarketContext) -> Self;
}

impl MarketExtractable for RatesCurvesSnapshot {
    fn extract(market: &MarketContext) -> Self {
        // Current extract_rates_curves logic
    }
}

impl MarketExtractable for CreditCurvesSnapshot { ... }
impl MarketExtractable for InflationCurvesSnapshot { ... }
impl MarketExtractable for CorrelationsSnapshot { ... }
impl MarketExtractable for VolatilitySnapshot { ... }
impl MarketExtractable for ScalarsSnapshot { ... }

// Generic helper
pub fn extract<T: MarketExtractable>(market: &MarketContext) -> T {
    T::extract(market)
}
```

**Benefits**:

- Uniform interface for all extraction types
- Easy to add new snapshot types
- Type inference: `let snapshot = extract::<RatesCurvesSnapshot>(market);`

---

### Phase 5: Waterfall Execution Unification

**Impact**: 200+ duplicate lines → single implementation
**Files**: `finstack/valuations/src/instruments/structured_credit/pricing/waterfall.rs`

#### Current State

Three functions with ~150 lines of identical logic:

- `execute_waterfall()`
- `execute_waterfall_with_explanation()`
- `execute_waterfall_with_workspace()`

#### Target State

```rust
pub fn execute_waterfall_core(
    waterfall: &Waterfall,
    tranches: &TrancheStructure,
    pool: &Pool,
    context: WaterfallContext,
    explain: ExplainOpts,
    workspace: Option<&mut WaterfallWorkspace>,
) -> Result<WaterfallDistribution> {
    // Unified implementation
    // Use workspace if Some(...), else local state
}

// Backward-compatible wrappers
pub fn execute_waterfall(...) -> Result<WaterfallDistribution> {
    execute_waterfall_core(waterfall, tranches, pool, context, ExplainOpts::disabled(), None)
}

pub fn execute_waterfall_with_workspace(...) -> Result<WaterfallDistribution> {
    execute_waterfall_core(waterfall, tranches, pool, context, ExplainOpts::disabled(), Some(workspace))
}
```

---

### Phase 6: JSON Envelope Boilerplate (Lower Priority)

**Impact**: Eliminate ~30 lines per envelope type (8+ types)
**Files**: Multiple envelope types across `finstack/valuations/src/attribution/`

#### Target State

```rust
pub trait JsonEnvelope: Sized + Serialize + DeserializeOwned {
    fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(Self::parse_error)
    }

    fn from_reader<R: std::io::Read>(reader: R) -> Result<Self> {
        serde_json::from_reader(reader).map_err(Self::parse_error)
    }

    fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(Self::serialize_error)
    }

    fn parse_error(e: serde_json::Error) -> finstack_core::Error;
    fn serialize_error(e: serde_json::Error) -> finstack_core::Error;
}

// Usage
impl JsonEnvelope for AttributionEnvelope {
    fn parse_error(e: serde_json::Error) -> finstack_core::Error {
        finstack_core::Error::Serde(format!("Failed to parse AttributionEnvelope: {}", e))
    }

    fn serialize_error(e: serde_json::Error) -> finstack_core::Error {
        finstack_core::Error::Serde(format!("Failed to serialize AttributionEnvelope: {}", e))
    }
}
```

---

## Source Code Structure Changes

### New Files

1. **None** - All changes are refactorings within existing files

### Modified Files (Priority Order)

1. ✅ **HIGH PRIORITY**:
   - `finstack/valuations/src/attribution/factors.rs`
     - Add CurveRestoreFlags bitflags
     - Add MarketSnapshot unified struct
     - Add restore_market() unified function
     - Update existing restore_* functions to use new implementation
     - Add MarketExtractable trait and implementations

2. ✅ **MEDIUM PRIORITY**:
   - `finstack/valuations/src/instruments/common/models/monte_carlo/payoff/rates.rs`
     - Merge CapPayoff + FloorPayoff → RatesPayoff
     - Add RatesPayoffType enum

   - `finstack/valuations/src/instruments/common/models/monte_carlo/payoff/lookback.rs`
     - Merge LookbackCall + LookbackPut → Lookback
     - Add LookbackDirection enum

3. ✅ **MEDIUM PRIORITY**:
   - `finstack/valuations/src/instruments/structured_credit/pricing/waterfall.rs`
     - Add AllocationContext and AllocationOutput structs
     - Refactor allocate_pro_rata() and allocate_sequential()
     - Add execute_waterfall_core() unified implementation
     - Update wrapper functions

4. ⚠️ **LOWER PRIORITY** (nice-to-have):
   - `finstack/valuations/src/attribution/parallel.rs`
   - `finstack/valuations/src/attribution/waterfall.rs`
   - `finstack/valuations/src/attribution/metrics_based.rs`
     - Add AttributionInput context struct
     - Add AttributionMethod enum
     - Refactor attribute_pnl_* functions

   - Multiple envelope types:
     - Add JsonEnvelope trait
     - Implement for all envelope types

### Cargo.toml Changes

Add to `finstack-valuations/Cargo.toml`:

```toml
[dependencies]
bitflags = "2.4"  # For CurveRestoreFlags
```

---

## Data Model / API Changes

### Breaking Changes (Require Major Version Bump)

**NONE** - All refactorings maintain backward compatibility via wrapper functions.

### Backward-Compatible Additions

1. **New Public Types**:
   - `CurveRestoreFlags` (bitflags)
   - `MarketSnapshot` (unified snapshot)
   - `RatesPayoffType`, `LookbackDirection` (enums)
   - `AllocationContext`, `AllocationOutput`, `AttributionInput` (context structs)
   - `MarketExtractable`, `JsonEnvelope` (traits)

2. **New Public Functions**:
   - `restore_market()` (unified restoration)
   - `extract::<T>()` (generic extraction)
   - `execute_waterfall_core()` (unified execution)

3. **Deprecated Functions** (keep for 1-2 versions):
   - Existing `restore_*_curves()` functions (become thin wrappers)
   - Existing standalone extraction functions (become trait methods)

### API Stability Guarantees

- **Old functions remain**: All existing public functions kept as wrappers
- **Semantic equivalence**: Wrappers produce identical results to original implementations
- **Deprecation warnings**: Add `#[deprecated]` attributes with migration guidance
- **Documentation updates**: Update docs to recommend new APIs

---

## Verification Approach

### 1. Unit Tests (Per Phase)

Each refactoring phase must include:

- **Equivalence tests**: Verify new implementation matches old behavior exactly
- **Edge case tests**: Empty markets, missing curves, zero notionals, etc.
- **Performance tests**: Ensure no regression in hot paths

**Example Test Structure**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restore_rates_equivalence() {
        let market = create_test_market();
        let snapshot = extract_rates_curves(&market);

        // Old implementation
        let restored_old = restore_rates_curves(&market, &snapshot);

        // New implementation
        let unified = MarketSnapshot {
            discount_curves: snapshot.discount_curves.clone(),
            forward_curves: snapshot.forward_curves.clone(),
            ..Default::default()
        };
        let restored_new = restore_market(&market, &unified, CurveRestoreFlags::RATES);

        // Assert equivalence
        assert_market_contexts_equal(&restored_old, &restored_new);
    }
}
```

### 2. Integration Tests

- **Attribution tests**: Run full P&L attribution on complex portfolios
- **Waterfall tests**: Execute structured credit waterfalls with actual data
- **Monte Carlo tests**: Price exotic options with both old and new payoffs

Test data sources:

- Existing test fixtures in `finstack/valuations/tests/`
- Golden outputs from production runs
- Edge cases from historical bugs

### 3. Benchmark Verification

Run existing benchmarks to ensure no performance regression:

```bash
cd finstack/valuations
cargo bench --bench attribution
cargo bench --bench monte_carlo
cargo bench --bench waterfall
```

**Acceptance Criteria**: No more than 5% regression in any benchmark.

### 4. Lint and Type Checking

```bash
make lint-rust
make test-rust
cargo clippy -- -D warnings
cargo doc --no-deps --document-private-items
```

**Must pass**:

- Zero clippy warnings
- Zero failing tests
- All documentation builds
- No new unsafe code

### 5. Manual Verification Checklist

- [ ] WASM bindings build successfully
- [ ] Python bindings build and import
- [ ] Example notebooks run unchanged
- [ ] Documentation examples compile
- [ ] Deprecation warnings show correct migration paths

---

## Risk Assessment and Mitigation

### High-Risk Areas

#### 1. Market Data Restoration (Phase 1)

**Risk**: Attribution P&L calculations depend on precise curve restoration
**Mitigation**:

- Extensive unit tests for each curve type combination
- Integration tests with real market data
- Golden file tests comparing old vs new outputs
- Gradual rollout: Keep old functions as non-deprecated fallbacks initially

#### 2. Monte Carlo Payoffs (Phase 2)

**Risk**: Pricing errors could propagate to production valuations
**Mitigation**:

- Property-based tests (quickcheck) for payoff symmetries
- Compare against analytical formulas where available
- Run side-by-side with old implementation for 1 sprint before deprecation
- Extra scrutiny in code review from quant team

#### 3. Waterfall Allocation (Phase 3)

**Risk**: Cash distribution errors in structured credit products
**Mitigation**:

- Test with actual deal structures from production
- Verify sum of distributions equals available amount (conservation)
- Check against external waterfall calculators (Excel, Bloomberg)
- Require sign-off from structuring desk

### Medium-Risk Areas

#### 4. Parameter Context Structs (Phase 3)

**Risk**: Accidentally changing semantics during restructuring
**Mitigation**:

- Keep internal implementation identical initially
- Use compiler to ensure all fields are preserved
- Add #[non_exhaustive] to allow future extensions without breaking changes

#### 5. Trait Abstractions (Phase 4, 6)

**Risk**: Over-engineering or future extensibility issues
**Mitigation**:

- Keep traits simple and focused
- Document intended use cases and extension points
- Don't over-generalize beyond current needs

---

## Rollout Strategy

### Stage 1: Internal Refactoring (Week 1-2)

- Implement unified abstractions
- Keep old functions as primary public API
- New functions marked as `#[doc(hidden)]` or in private submodules
- Full test coverage before exposing

### Stage 2: Soft Launch (Week 3)

- Expose new APIs in public docs
- Mark old functions as "soft deprecated" (no compiler warning yet)
- Update examples to use new APIs
- Monitor usage in internal projects

### Stage 3: Deprecation (Week 4-5)

- Add `#[deprecated]` attributes with migration guidance
- Update all internal code to use new APIs
- Release as minor version bump (backward compatible)

### Stage 4: Removal (Future Major Version)

- Remove deprecated wrapper functions
- Clean up any transitional code
- Major version bump

---

## Success Criteria

### Quantitative Metrics

- ✅ Reduce duplication by 500+ lines (from ~600 duplicate lines)
- ✅ Reduce parameter counts from 15+ to 2-3 in waterfall functions
- ✅ Zero test failures
- ✅ Zero clippy warnings
- ✅ <5% performance regression in any benchmark
- ✅ 100% backward compatibility (all old APIs still work)

### Qualitative Goals

- ✅ Code is easier to understand and maintain
- ✅ Adding new curve types requires minimal boilerplate
- ✅ Parameter context structs improve readability
- ✅ Trait abstractions provide clear extension points
- ✅ Documentation is clearer and more consistent

---

## Dependencies and Blockers

### Internal Dependencies

- None - this is a pure refactoring within finstack-valuations

### External Dependencies

- `bitflags = "2.4"` - Well-established, stable crate
- No breaking changes to finstack-core required

### Potential Blockers

1. **Test data availability**: Need real market scenarios for validation
   - **Mitigation**: Use existing test fixtures, generate synthetic data if needed

2. **Review bandwidth**: Large refactoring requires careful review
   - **Mitigation**: Split into 6 phases with separate PRs

3. **Production freeze windows**: Can't deploy during critical periods
   - **Mitigation**: Plan rollout during low-volatility periods

---

## Open Questions for Clarification

1. **Backward Compatibility Timeline**:
   - How long should we keep deprecated wrapper functions? (Suggest: 2 minor versions)
   - When is the next major version bump planned?

2. **Scope Adjustments**:
   - Should we include all 6 phases in this task, or split into separate tasks?
   - Priority order: Phases 1-3 are high value, Phases 4-6 are nice-to-have?

3. **Testing Resources**:
   - Do we have access to production market data for validation?
   - Can we get quant team review for Monte Carlo changes?

4. **Performance Requirements**:
   - Is 5% regression acceptable, or do we need zero regression?
   - Should we add new benchmarks for the unified implementations?

5. **Documentation**:
   - Should we update the main README with refactoring rationale?
   - Do we need migration guides for external users?

---

## Next Steps

After spec approval:

1. **Create detailed implementation plan** in `plan.md` breaking down each phase into incremental, testable steps
2. **Set up feature branch**: `feature/code-consolidation-marge-list`
3. **Implement Phase 1** (curve restoration) as first PR
4. **Iterate based on feedback** before proceeding to subsequent phases

**Estimated Timeline** (with 1 engineer, full-time):

- Phase 1 (Curve Restoration): 3-4 days
- Phase 2 (Monte Carlo Payoffs): 2-3 days
- Phase 3 (Parameter Contexts): 3-4 days
- Phase 4 (Trait Abstractions): 2 days
- Phase 5 (Waterfall Unification): 2-3 days
- Phase 6 (JSON Envelopes): 1-2 days
- **Total**: ~2.5 weeks

Add 50% buffer for testing, review, and iteration: **~3.5-4 weeks total**.
