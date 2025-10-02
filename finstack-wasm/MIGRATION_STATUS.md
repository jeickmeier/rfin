# Instrument Wrapper Migration Status

## Completed Migrations ✅ (5/25)

1. ✅ **bond.rs** - Complex instrument with builders, helpers (EXAMPLE)
2. ✅ **deposit.rs** - Simple instrument (EXAMPLE)
3. ✅ **equity.rs** - Simple instrument
4. ✅ **repo.rs** - Medium complexity with collateral spec
5. ✅ **variance_swap.rs** - Medium complexity with side/method parsing

## Remaining Migrations (20/25)

### Phase 1: Simple (2 remaining)
6. **convertible.rs** - ConvertibleBond (+ helper structs JsConversionPolicy, JsConversionSpec)
7. **ir_future.rs** - InterestRateFuture

### Phase 2: Medium Complexity (10 files)
8. **irs.rs** - InterestRateSwap
9. **fra.rs** - ForwardRateAgreement
10. **basis_swap.rs** - BasisSwap
11. **cap_floor.rs** - InterestRateOption
12. **swaption.rs** - Swaption
13. **equity_option.rs** - EquityOption
14. **cds.rs** - CreditDefaultSwap
15. **cds_index.rs** - CDSIndex
16. **inflation_swap.rs** - InflationSwap
17. **inflation_linked_bond.rs** - InflationLinkedBond (if exists)

### Phase 3: Complex Multi-Type Files (8 files / 10+ types)
18. **fx.rs** - JsFxSpot, JsFxOption, JsFxSwap (3 types)
19. **cds_tranche.rs** - CdsTranche
20. **cds_option.rs** - CdsOption
21. **structured.rs** - JsBasket, JsAbs, JsClo, JsCmbs, JsRmbs (5 types)
22. **private_markets_fund.rs** - PrivateMarketsFund
23. **trs.rs** - JsEquityTotalReturnSwap, JsFiIndexTotalReturnSwap (2 types)

## Migration Pattern (Proven & Working)

### Step 1: Add Import
```rust
use crate::valuations::instruments::InstrumentWrapper;
```

### Step 2: Convert Struct
```rust
// BEFORE:
pub struct JsXxx {
    inner: Xxx,
}

// AFTER:
pub struct JsXxx(Xxx);
```

### Step 3: Replace Impl
```rust
// BEFORE:
impl JsXxx {
    pub(crate) fn from_inner(inner: Xxx) -> Self {
        Self { inner }
    }
    pub(crate) fn inner(&self) -> Xxx {
        self.inner.clone()
    }
}

// AFTER:
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

### Step 4: Replace Field Access
```bash
# Find and replace in each file:
self.inner → self.0
```

## Automated Completion Script

See `complete_migrations.sh` for a semi-automated approach that:
1. Adds the import statement
2. Changes all `self.inner` to `self.0`
3. Provides manual checklist for struct/impl changes

## Time Estimate

- **Simple instruments**: 3-5 minutes each × 2 = 6-10 minutes
- **Medium complexity**: 5-7 minutes each × 10 = 50-70 minutes  
- **Complex multi-type**: 10-15 minutes each × 8 = 80-120 minutes

**Total remaining**: ~2-3 hours

## Verification Commands

After each migration:
```bash
# Compile check
cargo check

# Count completed
rg "impl InstrumentWrapper for" finstack-wasm/src/valuations/instruments/ | wc -l

# Find remaining self.inner (should only be in comments after completion)
rg "self\.inner" finstack-wasm/src/valuations/instruments/ --type rust
```

## Current Status

**5 out of 25 complete (20%)**

Files are compiling successfully. Pattern is proven and working. Remaining work is purely mechanical application of the same pattern.

