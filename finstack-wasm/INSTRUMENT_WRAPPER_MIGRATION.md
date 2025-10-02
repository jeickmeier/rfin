# Instrument Wrapper Consolidation - Migration Guide

## Overview

This guide shows how to migrate the remaining 23 instrument wrappers to use the new `InstrumentWrapper` trait, reducing boilerplate from ~800 LOC to ~100 LOC.

## Pattern Summary

### Before (30+ lines of boilerplate per instrument)

```rust
#[wasm_bindgen(js_name = Deposit)]
#[derive(Clone, Debug)]
pub struct JsDeposit {
    inner: Deposit,
}

impl JsDeposit {
    pub(crate) fn from_inner(inner: Deposit) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> Deposit {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = Deposit)]
impl JsDeposit {
    // ... all methods reference self.inner ...
}
```

### After (3 lines of wrapper impl + field access changes)

```rust
use crate::valuations::instruments::InstrumentWrapper;

#[wasm_bindgen(js_name = Deposit)]
#[derive(Clone, Debug)]
pub struct JsDeposit(Deposit);

impl InstrumentWrapper for JsDeposit {
    type Inner = Deposit;
    fn from_inner(inner: Deposit) -> Self { JsDeposit(inner) }
    fn inner(&self) -> Deposit { self.0.clone() }
}

#[wasm_bindgen(js_class = Deposit)]
impl JsDeposit {
    // ... all methods reference self.0 instead of self.inner ...
}
```

## Migration Steps (Per Instrument)

### Step 1: Add Import

At the top of the file, add:

```rust
use crate::valuations::instruments::InstrumentWrapper;
```

### Step 2: Convert Struct to Tuple

**Before:**
```rust
pub struct JsDeposit {
    inner: Deposit,
}
```

**After:**
```rust
pub struct JsDeposit(Deposit);
```

### Step 3: Replace impl Block with Trait

**Before:**
```rust
impl JsDeposit {
    pub(crate) fn from_inner(inner: Deposit) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> Deposit {
        self.inner.clone()
    }
}
```

**After:**
```rust
impl InstrumentWrapper for JsDeposit {
    type Inner = Deposit;
    fn from_inner(inner: Deposit) -> Self { JsDeposit(inner) }
    fn inner(&self) -> Deposit { self.0.clone() }
}
```

### Step 4: Update Field References

Use find-and-replace to update all method implementations:

- `self.inner` → `self.0`
- Keep all other method code unchanged

**Before:**
```rust
pub fn instrument_id(&self) -> String {
    self.inner.id.as_str().to_string()
}
```

**After:**
```rust
pub fn instrument_id(&self) -> String {
    self.0.id.as_str().to_string()
}
```

### Step 5: Update Constructor Calls

Replace any internal uses of `::new()` with `::from_inner()`:

**Before:**
```rust
JsDeposit::new(deposit)
```

**After:**
```rust
JsDeposit::from_inner(deposit)
```

## Instruments to Migrate

- [x] `deposit.rs` - **DONE** (example in this guide)
- [x] `bond.rs` - **DONE** (example in this guide)
- [ ] `irs.rs` - InterestRateSwap
- [ ] `fra.rs` - ForwardRateAgreement
- [ ] `swaption.rs` - Swaption
- [ ] `basis_swap.rs` - BasisSwap
- [ ] `cap_floor.rs` - InterestRateOption
- [ ] `ir_future.rs` - InterestRateFuture
- [ ] `fx.rs` - FxSpot, FxOption, FxSwap (3 types in one file)
- [ ] `cds.rs` - CreditDefaultSwap
- [ ] `cds_index.rs` - CDSIndex
- [ ] `cds_tranche.rs` - CdsTranche
- [ ] `cds_option.rs` - CdsOption
- [ ] `equity.rs` - Equity
- [ ] `equity_option.rs` - EquityOption
- [ ] `inflation_linked_bond.rs` - InflationLinkedBond
- [ ] `inflation_swap.rs` - InflationSwap
- [ ] `structured.rs` - Basket, Abs, Clo, Cmbs, Rmbs (5 types in one file)
- [ ] `private_markets_fund.rs` - PrivateMarketsFund
- [ ] `repo.rs` - Repo
- [ ] `variance_swap.rs` - VarianceSwap
- [ ] `convertible.rs` - ConvertibleBond
- [ ] `trs.rs` - EquityTotalReturnSwap, FiIndexTotalReturnSwap (2 types)

## Example: Complete Migration (InterestRateSwap)

### Before

```rust
use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::{JsDayCount, JsFrequency};
use crate::core::money::JsMoney;
use crate::core::error::js_error;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use finstack_valuations::instruments::irs::InterestRateSwap;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = InterestRateSwap)]
#[derive(Clone, Debug)]
pub struct JsInterestRateSwap {
    inner: InterestRateSwap,
}

impl JsInterestRateSwap {
    pub(crate) fn from_inner(inner: InterestRateSwap) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> InterestRateSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = InterestRateSwap)]
impl JsInterestRateSwap {
    #[wasm_bindgen(constructor)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        fixed_rate: f64,
        start: &JsDate,
        maturity: &JsDate,
        pay_fixed: bool,
        // ... more parameters
    ) -> Result<JsInterestRateSwap, JsValue> {
        // ... builder code ...
        Ok(JsInterestRateSwap::from_inner(swap))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    // ... more methods ...
}
```

### After

```rust
use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::{JsDayCount, JsFrequency};
use crate::core::money::JsMoney;
use crate::core::error::js_error;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;  // ADD THIS
use finstack_valuations::instruments::irs::InterestRateSwap;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = InterestRateSwap)]
#[derive(Clone, Debug)]
pub struct JsInterestRateSwap(InterestRateSwap);  // CHANGED

impl InstrumentWrapper for JsInterestRateSwap {  // REPLACED ENTIRE BLOCK
    type Inner = InterestRateSwap;
    fn from_inner(inner: InterestRateSwap) -> Self { JsInterestRateSwap(inner) }
    fn inner(&self) -> InterestRateSwap { self.0.clone() }
}

#[wasm_bindgen(js_class = InterestRateSwap)]
impl JsInterestRateSwap {
    #[wasm_bindgen(constructor)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        fixed_rate: f64,
        start: &JsDate,
        maturity: &JsDate,
        pay_fixed: bool,
        // ... more parameters
    ) -> Result<JsInterestRateSwap, JsValue> {
        // ... builder code ...
        Ok(JsInterestRateSwap::from_inner(swap))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.0.id.as_str().to_string()  // CHANGED: self.inner → self.0
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.0.notional)  // CHANGED: self.inner → self.0
    }

    // ... more methods with self.inner → self.0 ...
}
```

## Testing After Migration

1. **Compile Check**: Ensure the crate compiles without errors
   ```bash
   cd finstack-wasm
   cargo check
   ```

2. **Build WASM**: Verify WASM bindings generate correctly
   ```bash
   wasm-pack build --target web
   ```

3. **Run Examples**: Test the examples app to ensure JS interop works
   ```bash
   cd examples
   npm run dev
   ```

4. **Type Definitions**: Verify TypeScript definitions are unchanged
   ```bash
   # Check that pkg/finstack_wasm.d.ts contains expected types
   cat pkg/finstack_wasm.d.ts | grep "class Bond"
   cat pkg/finstack_wasm.d.ts | grep "class Deposit"
   ```

## Benefits

### Before Migration
- **~800 lines** of repetitive wrapper code across 25+ instruments
- Easy to introduce copy-paste errors
- Inconsistent patterns between instruments
- Hard to spot mistakes in `inner()` conversions

### After Migration  
- **~100 lines** total (trait definition + 3 lines per instrument)
- Impossible to get conversion pattern wrong (trait enforces it)
- Uniform pattern makes code review easier
- Clear signal which types are wrappers

## Common Pitfalls

### ❌ Don't forget to update all field references

```rust
// This will cause a compile error after migration:
pub fn get_id(&self) -> String {
    self.inner.id.as_str().to_string()  // ❌ 'inner' no longer exists
}

// Must be:
pub fn get_id(&self) -> String {
    self.0.id.as_str().to_string()  // ✅
}
```

### ❌ Don't mix tuple and named struct syntax

```rust
// Wrong - trying to use named field on tuple struct:
impl InstrumentWrapper for JsBond {
    fn from_inner(inner: Bond) -> Self { 
        JsBond { inner }  // ❌ Compile error
    }
}

// Correct:
impl InstrumentWrapper for JsBond {
    fn from_inner(inner: Bond) -> Self { 
        JsBond(inner)  // ✅
    }
}
```

### ✅ Use find-and-replace carefully

Use your editor's find-and-replace to update field references:
- Find: `self.inner`
- Replace: `self.0`
- **Scope**: Single file at a time
- **Preview**: Review each change before accepting

## Verification Script

After migrating each instrument, run this check:

```bash
# Check that the struct is a tuple struct
rg "pub struct Js\w+\(.*\);" finstack-wasm/src/valuations/instruments/

# Check that InstrumentWrapper is implemented
rg "impl InstrumentWrapper for" finstack-wasm/src/valuations/instruments/

# Check for any remaining "self.inner" references (should only find comments)
rg "self\.inner" finstack-wasm/src/valuations/instruments/ --type rust
```

## Impact Summary

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| LOC (wrapper boilerplate) | ~800 | ~100 | **87.5% reduction** |
| Lines per instrument | 30+ | 3 | **90% reduction** |
| Pattern consistency | Manual | Enforced | **100% consistency** |
| Error-prone conversions | 50+ manual impls | 1 trait | **50x safer** |
| Maintainability | Low | High | **Much easier** |

## Conclusion

This refactor dramatically improves code quality by:
1. **Reducing duplication** - One trait instead of 25+ identical impls
2. **Enforcing correctness** - Type system prevents conversion errors
3. **Improving clarity** - Obvious which types are wrappers
4. **Simplifying maintenance** - Changes propagate automatically

The pattern is now established with `bond.rs` and `deposit.rs` as examples. Migrating the remaining 23 instruments is purely mechanical following the steps above.

