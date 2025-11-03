# Python Bindings Structure Notes

## Monte Carlo Module Organization

As of 2025-11-03, the Monte Carlo functionality has been reorganized to match the Rust structure:

### File Structure (Correct)
```
finstack-py/src/valuations/
└── common/
    └── mc/                      ← Now correctly nested
        ├── mod.rs
        ├── generator.rs
        ├── params.rs
        ├── paths.rs
        └── result.rs
```

**Previously (INCORRECT):**
```
finstack-py/src/valuations/
├── mc_generator.rs              ← Was at top level (wrong!)
├── mc_params.rs
├── mc_paths.rs
└── mc_result.rs
```

### Import Pattern (Due to PyO3 Limitations)

**In Python code**, use **flat imports** from the top level:
```python
from finstack.valuations import (
    PathPoint,
    PathDataset,
    ProcessParams,
    MonteCarloResult,
    McGenerator
)
```

**Nested imports don't work** due to PyO3/maturin extension module limitations:
```python
# ❌ This doesn't work (PyO3 limitation):
from finstack.valuations.common.mc import PathPoint

# ✅ This works:
from finstack.valuations import PathPoint
```

### Stub Files (.pyi) Structure

The `.pyi` stub files **do reflect the nested structure** for documentation and IDE support:
```
finstack-py/finstack/valuations/
└── common/
    ├── __init__.pyi
    └── mc/
        ├── __init__.pyi
        ├── generator.pyi
        ├── params.pyi
        ├── paths.pyi
        └── result.pyi
```

This provides better organization in IDEs while the runtime imports remain flat.

### WASM Bindings

The same restructuring was applied to WASM:
```
finstack-wasm/src/valuations/
└── common/
    └── mc/
        ├── mod.rs
        ├── generator.rs
        ├── params.rs
        ├── paths.rs
        └── result.rs
```

WASM exports remain at the top level in `lib.rs`:
```typescript
import { PathPoint, PathDataset } from 'finstack-wasm';
```

## Structured Credit Waterfall Module Organization

Similarly, the `structured_credit_waterfall` functionality has been moved from the top level to under `instruments/structured_credit/`:

### File Structure (Correct)
```
finstack-py/src/valuations/instruments/
└── structured_credit/
    ├── mod.rs                ← Main StructuredCredit instrument
    └── waterfall.rs          ← Waterfall-specific bindings
```

**Previously (INCORRECT):**
```
finstack-py/src/valuations/
├── structured_credit_waterfall.rs    ← Was at top level (wrong!)
└── instruments/
    └── structured_credit.rs
```

**Why:** The waterfall functionality is specific to structured credit instruments, so it should live with the StructuredCredit module, not as a separate top-level module.

## Why This Matters

1. **Conceptual Clarity**: 
   - MC is shared infrastructure used by multiple instruments, not a top-level valuations concern
   - Waterfall is specific to structured credit, so it belongs with StructuredCredit
2. **Rust Alignment**: The Python/WASM code now mirrors the Rust organization
3. **Maintainability**: Easier to find and update related code
4. **Discoverability**: Developers familiar with Rust will find Python code in expected locations

## Migration Notes

If any code was directly referencing the old file paths, those would need to be updated. However, **no user-facing Python imports change** because everything was already exported from the top level.

---
**Updated:** 2025-11-03
**Status:** Complete (MC + Structured Credit Waterfall)

