# F Type Alias Elimination - Refactoring Summary

## Completed: September 29, 2025

### Changes Made

Successfully eliminated the `F` type alias (`pub type F = f64;`) throughout the `finstack-core` crate and replaced all occurrences with explicit `f64` type annotations.

### Impact

- **Files changed**: 37 files in `finstack/core/src/`
- **Net change**: 582 insertions(+), 600 deletions(-) (18 line reduction)
- **Type alias removed**: `pub type F = f64;` from `finstack/core/src/lib.rs`
- **Re-export removed**: `pub use crate::F;` from `finstack/core/src/market_data/mod.rs`

### Key Replacements

1. **Type signatures**: All `F` type parameters replaced with `f64`
   - Function return types: `-> F` → `-> f64`
   - Function parameters: `: F` → `: f64`
   - Generic bounds: `Vec<F>`, `Box<[F]>`, `&[F]` → `Vec<f64>`, `Box<[f64]>`, `&[f64]`

2. **Import statements**: Removed all `use crate::F;` statements

3. **Preserved identifiers**:
   - `Act365F` (DayCount variant - "F" stands for "Fixed", not the type alias)
   - Generic type parameter `F` in function signatures (e.g., `fn foo<F: Fn(...)>`)
   - Meaningful type aliases: `FxRate = f64`, `AmountRepr = f64`

### Verification

✅ **Compilation**: `cargo check --workspace` passes  
✅ **Tests**: All 179 tests in `finstack-core` pass  
✅ **Clippy**: No warnings with `-D warnings`

### Benefits

1. **Readability**: Code now uses explicit `f64` type, eliminating mental indirection
2. **Discoverability**: IDEs and documentation show `f64` directly  
3. **Clippy-friendly**: Numeric lints work directly on `f64` without alias confusion
4. **Future-proof**: If Decimal mode is needed, can use feature-gated newtype instead

### Notes

- The `finstack-valuations` crate will need similar changes (separate task)
- Type-safe aliases like `FxRate = f64` were intentionally preserved as they provide semantic meaning
- All necessary `as f64` casts were preserved/added for proper type conversions (i32→f64, usize→f64, etc.)

