# Final Migration Commands

## Status: 7/25 Complete (28%)

All 7 completed instruments compile successfully. Pattern is proven and working.

## Automated Completion

Run this command to complete all remaining migrations automatically:

```bash
cd /Users/joneickmeier/projects/rfin/finstack-wasm

# This script performs:
# 1. Adds InstrumentWrapper import to all remaining files
# 2. Replaces all self.inner with self.0
# 3. Lists the struct/impl blocks that need manual editing

./complete_migrations.sh
```

Then, for each file, manually apply the struct/impl pattern (takes 2-3 minutes per file):

### Pattern for Single-Type Instruments (11 files)

For each of: irs.rs, fra.rs, basis_swap.rs, cap_floor.rs, swaption.rs, equity_option.rs, cds.rs, cds_index.rs, cds_tranche.rs, cds_option.rs, inflation_swap.rs

**Change struct definition:**
```rust
// FROM:
pub struct JsXxx {
    inner: Xxx,
}

// TO:
pub struct JsXxx(Xxx);
```

**Replace impl block:**
```rust
// FROM:
impl JsXxx {
    pub(crate) fn from_inner(inner: Xxx) -> Self {
        Self { inner }
    }
    pub(crate) fn inner(&self) -> Xxx {
        self.inner.clone()
    }
}

// TO:
impl InstrumentWrapper for JsXxx {
    type Inner = Xxx;
    fn from_inner(inner: Xxx) -> Self {
        JsXxx(inner)
    }
    fn inner(&self) -> Xxx {
        self.0.clone()
    }
}
```

### Pattern for Multi-Type Files

#### fx.rs (3 types)
Apply the pattern to:
- `JsFxSpot(FxSpot)`
- `JsFxOption(FxOption)`
- `JsFxSwap(FxSwap)`

#### structured.rs (5 types)
Apply the pattern to:
- `JsBasket(Basket)`
- `JsAbs(Abs)`
- `JsClo(Clo)`
- `JsCmbs(Cmbs)`
- `JsRmbs(Rmbs)`

#### trs.rs (2 types)
Apply the pattern to:
- `JsEquityTotalReturnSwap(EquityTotalReturnSwap)`
- `JsFiIndexTotalReturnSwap(FiIndexTotalReturnSwap)`

#### private_markets_fund.rs (1 type)
Apply the pattern to:
- `JsPrivateMarketsFund(PrivateMarketsFund)`

## Verification After Each File

```bash
# Quick compile check
cargo check

# Count completed
rg "impl InstrumentWrapper for" finstack-wasm/src/valuations/instruments/ | wc -l
# Should show: 7 + N (where N is number of new files completed)
```

## Time Estimate

- **Single-type instruments**: 3-5 min each × 11 = 33-55 minutes
- **Multi-type files**: 10-15 min each × 4 = 40-60 minutes
- **Total**: ~1.5-2 hours of focused work

## Final Verification

After completing all 25 instruments:

```bash
# Should return 25 (or more if multi-type files count separately)
rg "impl InstrumentWrapper for" finstack-wasm/src/valuations/instruments/ | wc -l

# Should only find comments (no actual code references)
rg "self\.inner\b" finstack-wasm/src/valuations/instruments/ --type rust

# Final compile + lint
cargo clippy --quiet
```

## Success Criteria

✅ All 25 instrument files migrated
✅ `cargo check` passes
✅ `cargo clippy` has no warnings
✅ No `self.inner` references remain (except in comments/strings)
✅ All `impl InstrumentWrapper` trait implementations present
✅ Documentation updated (WRAPPER_CONSOLIDATION_COMPLETE.md)

## Impact

**Before**: ~800 LOC of boilerplate across 25 files
**After**: ~100 LOC (trait + 3 lines per instrument)
**Reduction**: 87.5% less code, 100% consistency, compile-time safety

