# Quick Migration Reference Card

## 3-Step Migration Process

### Step 1: Update Imports
```diff
+ use crate::valuations::instruments::InstrumentWrapper;
```

### Step 2: Convert Struct
```diff
  #[wasm_bindgen(js_name = InterestRateSwap)]
  #[derive(Clone, Debug)]
- pub struct JsInterestRateSwap {
-     inner: InterestRateSwap,
- }
+ pub struct JsInterestRateSwap(InterestRateSwap);

- impl JsInterestRateSwap {
-     pub(crate) fn from_inner(inner: InterestRateSwap) -> Self {
-         Self { inner }
-     }
-
-     #[allow(dead_code)]
-     pub(crate) fn inner(&self) -> InterestRateSwap {
-         self.inner.clone()
-     }
- }
+ impl InstrumentWrapper for JsInterestRateSwap {
+     type Inner = InterestRateSwap;
+     fn from_inner(inner: InterestRateSwap) -> Self { JsInterestRateSwap(inner) }
+     fn inner(&self) -> InterestRateSwap { self.0.clone() }
+ }
```

### Step 3: Update Field References
```diff
  pub fn instrument_id(&self) -> String {
-     self.inner.id.as_str().to_string()
+     self.0.id.as_str().to_string()
  }

  pub fn notional(&self) -> JsMoney {
-     JsMoney::from_inner(self.inner.notional)
+     JsMoney::from_inner(self.0.notional)
  }
```

## Find & Replace Commands

### VS Code / Cursor
1. Open the instrument file
2. Find: `self.inner`
3. Replace: `self.0`
4. **Important**: Use "Replace in File" to limit scope

### Vim / Neovim
```vim
:%s/self\.inner/self.0/g
```

### sed (command line)
```bash
sed -i '' 's/self\.inner/self.0/g' finstack-wasm/src/valuations/instruments/irs.rs
```

## Verification Checklist

After each migration:

- [ ] Struct is tuple struct: `pub struct JsXxx(Xxx);`
- [ ] Trait is implemented with 3 lines
- [ ] No remaining `self.inner` (except in comments)
- [ ] All `self.inner.field` → `self.0.field`
- [ ] All `SomeType::new()` → `SomeType::from_inner()` if calling wrapper constructor
- [ ] File compiles: `cargo check`
- [ ] No clippy warnings: `cargo clippy`

## Common Gotchas

### ❌ Don't mix syntax styles
```rust
// WRONG - using named field syntax on tuple struct
JsInterestRateSwap { inner: swap }

// CORRECT
JsInterestRateSwap(swap)
```

### ❌ Don't forget deep field access
```rust
// WRONG - missed nested field
self.inner.float_spec.as_ref().map(|f| f.margin)

// CORRECT
self.0.float_spec.as_ref().map(|f| f.margin)
```

### ✅ DO update ALL references
Use global find-replace in the file to ensure you catch all references.

## Time Estimate

Per instrument:
- Simple (like Deposit): **3-5 minutes**
- Medium (like InterestRateSwap): **5-7 minutes**  
- Complex (like Bond): **7-10 minutes**

Total for 23 remaining: **2-4 hours**

## Order of Migration

Suggested order (simple → complex):

**Phase 1: Simple (5 instruments, ~20 min)**
1. `equity.rs` - Very simple
2. `repo.rs` - Simple
3. `variance_swap.rs` - Simple
4. `convertible.rs` - Simple
5. `ir_future.rs` - Simple

**Phase 2: Medium (10 instruments, ~60 min)**
6. `irs.rs` - Standard swap
7. `fra.rs` - Simple forward
8. `basis_swap.rs` - Similar to IRS
9. `cap_floor.rs` - Options on rates
10. `swaption.rs` - Options on swaps
11. `equity_option.rs` - Equity options
12. `cds.rs` - Credit default swap
13. `cds_index.rs` - CDS index
14. `inflation_swap.rs` - Inflation swap
15. `inflation_linked_bond.rs` - Inflation bond

**Phase 3: Complex (8 instruments, ~90 min)**
16. `fx.rs` - 3 types (FxSpot, FxOption, FxSwap)
17. `cds_tranche.rs` - Structured credit
18. `cds_option.rs` - CDS options
19. `structured.rs` - 5 types (Basket, Abs, Clo, Cmbs, Rmbs)
20. `private_markets_fund.rs` - Complex fund structures
21. `trs.rs` - 2 types (Equity TRS, FI Index TRS)

## Batch Commands

Check all at once:
```bash
# Find any remaining "inner" references
rg "self\.inner" finstack-wasm/src/valuations/instruments/ --type rust

# Count migrated instruments
rg "impl InstrumentWrapper for" finstack-wasm/src/valuations/instruments/ | wc -l

# Should show 2 (bond + deposit) now, 25 when complete
```

## Commit Strategy

Commit after each instrument or small batch:
```bash
git add finstack-wasm/src/valuations/instruments/irs.rs
git commit -m "refactor(wasm): migrate InterestRateSwap to InstrumentWrapper trait"
```

Or batch by phase:
```bash
git add finstack-wasm/src/valuations/instruments/{equity,repo,variance_swap,convertible,ir_future}.rs
git commit -m "refactor(wasm): migrate phase 1 instruments to InstrumentWrapper (5 simple)"
```

## Testing Strategy

After each migration:
```bash
# Quick compile check
cargo check

# Full lint + compile
cargo clippy --all-targets

# Run existing tests (if any)
cargo test --lib

# Build WASM (slower, but catches wasm-bindgen issues)
wasm-pack build --target web
```

## Done!

When complete:
- [ ] All 25 instruments use `InstrumentWrapper`
- [ ] No `self.inner` references remain
- [ ] `cargo check` passes
- [ ] `cargo clippy` passes
- [ ] Update `WRAPPER_CONSOLIDATION_COMPLETE.md` with completion date

