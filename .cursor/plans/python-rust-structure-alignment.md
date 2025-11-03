# Python-Rust Structure Alignment Plan

## Executive Summary

The Python bindings (`finstack-py`) currently have a flatter file structure compared to the Rust crates (`finstack/*`), which can create confusion for users navigating between languages. This document proposes a comprehensive plan to align the Python bindings structure with the Rust crates organization while maintaining backward compatibility.

## Current State Analysis

### 1. Core Module

**Rust (`finstack/core/src/`):**
```
cashflow/
  ├── discounting.rs
  ├── primitives.rs
  ├── performance.rs
  └── xirr.rs
dates/
  ├── calendar/
  │   ├── algo.rs
  │   ├── business_days.rs
  │   ├── composite.rs
  │   ├── generated.rs
  │   ├── registry.rs
  │   ├── rule.rs
  │   └── types.rs
  ├── daycount.rs
  ├── imm.rs
  ├── periods.rs
  ├── schedule_iter.rs
  └── utils.rs
market_data/
  ├── scalars/
  ├── surfaces/
  └── term_structures/
      ├── discount_curve.rs
      ├── forward_curve.rs
      ├── hazard_curve.rs
      ├── inflation.rs
      └── ...
math/
  └── interp/
      ├── cubic_hermite.rs
      ├── flat_fwd.rs
      ├── linear.rs
      └── ...
money/
  └── fx/
      └── providers.rs
```

**Python (`finstack-py/finstack/core/`):**
```
cashflow/
  ├── __init__.pyi
  └── primitives.pyi
dates/
  ├── __init__.pyi
  ├── calendar.pyi
  ├── daycount.pyi
  ├── imm.pyi
  ├── periods.pyi
  ├── schedule.pyi
  └── utils.pyi
market_data/
  ├── __init__.pyi
  ├── context.pyi
  ├── dividends.pyi
  ├── fx.pyi
  ├── interp.pyi
  ├── scalars.pyi
  ├── surfaces.pyi
  └── term_structures.pyi
math/
  ├── __init__.pyi
  ├── distributions.pyi
  ├── integration.pyi
  └── solver.pyi
```

**Status:** ✅ **Reasonably well aligned**
- Python roughly mirrors Rust structure
- Could improve by adding submodules for `dates/calendar/`, `market_data/scalars/`, `market_data/surfaces/`, `market_data/term_structures/`, `math/interp/`

---

### 2. Valuations Module

**Rust (`finstack/valuations/src/`):**
```
calibration/
  ├── derivatives/
  │   ├── sabr_derivatives.rs
  │   └── sabr_model_params.rs
  └── methods/
      ├── discount.rs
      ├── forward_curve.rs
      ├── hazard_curve.rs
      ├── sabr_surface.rs
      └── ...
cashflow/
  └── builder/
      ├── compile.rs
      ├── schedule.rs
      ├── schedule_utils.rs
      ├── state.rs
      └── types.rs
instruments/
  ├── bond/
  │   ├── cashflows.rs
  │   ├── metrics/
  │   │   ├── accrued_interest.rs
  │   │   ├── clean_price.rs
  │   │   ├── dv01.rs
  │   │   └── ...
  │   ├── pricing/
  │   │   ├── analytical.rs
  │   │   ├── discounting.rs
  │   │   └── ...
  │   └── types.rs
  ├── cds/
  │   ├── metrics/
  │   ├── pricer.rs
  │   └── types.rs
  ├── structured_credit/
  │   ├── components/
  │   │   ├── waterfall.rs
  │   │   ├── coverage_tests.rs
  │   │   ├── diversion.rs
  │   │   └── ...
  │   ├── metrics/
  │   ├── templates/
  │   │   ├── abs.rs
  │   │   ├── clo.rs
  │   │   ├── cmbs.rs
  │   │   └── rmbs.rs
  │   └── types.rs
  └── common/
      ├── mc/
      │   ├── brownian.rs
      │   ├── gbm.rs
      │   ├── heston.rs
      │   └── ...
      └── models/
          ├── black_scholes.rs
          ├── sabr.rs
          └── ...
```

**Python (`finstack-py/src/valuations/`):**
```
calibration/
  ├── config.rs
  ├── methods.rs          ← FLATTENED (should be methods/)
  ├── quote.rs
  ├── report.rs
  ├── sabr.rs             ← FLATTENED (should be derivatives/)
  ├── simple.rs
  └── validation.rs
cashflow/
  └── builder.rs          ← FLATTENED (should be builder/)
instruments/
  ├── bond.rs             ← FLATTENED (should be bond/)
  ├── cds.rs              ← FLATTENED (should be cds/)
  ├── structured_credit.rs ← FLATTENED (should be structured_credit/)
  └── ...                 ← ALL INSTRUMENTS FLATTENED
common/
  └── parameters.rs       ← MISSING mc/, models/
```

**Status:** ❌ **Significant misalignment**
- Most multi-file Rust modules are collapsed into single Python binding files
- Missing submodule structure for instruments
- Missing `components/`, `metrics/`, `pricing/`, `templates/` submodules

---

### 3. Statements Module

**Rust (`finstack/statements/src/`):**
```
builder/
  ├── mod.rs
  └── model_builder.rs
capital_structure/
  ├── builder.rs
  ├── integration.rs
  ├── types.rs
  └── mod.rs
dsl/
  ├── ast.rs
  ├── compiler.rs
  ├── parser.rs
  └── mod.rs
evaluator/
  ├── context.rs
  ├── dag.rs
  ├── engine.rs
  ├── forecast_eval.rs
  ├── formula.rs
  ├── precedence.rs
  └── results.rs
extensions/
  ├── corkscrew.rs
  ├── plugin.rs
  ├── registry.rs
  └── scorecards.rs
forecast/
  ├── deterministic.rs
  ├── override_method.rs
  ├── statistical.rs
  └── timeseries.rs
registry/
  ├── builtins.rs
  ├── dynamic.rs
  ├── schema.rs
  └── validation.rs
```

**Python (`finstack-py/src/statements/`):**
```
builder/
  └── mod.rs              ← FLATTENED (missing model_builder)
evaluator/
  └── mod.rs              ← FLATTENED (missing submodules)
extensions/
  ├── builtins.rs
  └── mod.rs              ← MISSING plugin, registry, scorecards
registry/
  ├── mod.rs
  └── schema.rs           ← MISSING builtins, dynamic, validation
types/
  ├── forecast.rs
  ├── model.rs
  ├── node.rs
  └── value.rs            ← OK
```

**Status:** ⚠️ **Partially misaligned**
- Missing `capital_structure/`, `dsl/`, `forecast/` modules entirely
- Submodules exist but are incomplete

---

### 4. Scenarios Module

**Rust (`finstack/scenarios/src/`):**
```
adapters/
  ├── curves.rs
  ├── fx.rs
  ├── instruments.rs
  ├── scalars.rs
  ├── statements.rs
  └── ...
engine.rs
spec.rs
utils.rs
```

**Python (`finstack-py/src/scenarios/`):**
```
adapters/
  └── mod.rs              ← EXISTS but may be incomplete
engine.rs
spec.rs
```

**Status:** ⚠️ **Partially aligned**
- Basic structure exists but adapters may be incomplete

---

### 5. Portfolio Module

**Rust (`finstack/portfolio/src/`):**
```
builder.rs
dataframe.rs
error.rs
grouping.rs
metrics.rs
portfolio.rs
position.rs
results.rs
scenarios.rs
types.rs
valuation.rs
```

**Python (`finstack-py/src/portfolio/`):**
```
builder.rs
grouping.rs
metrics.rs
portfolio.rs
position.rs
results.rs
scenarios.rs
types.rs
valuation.rs
```

**Status:** ✅ **Well aligned**
- Missing `dataframe.rs` only

---

## Discrepancy Summary

| Module | Alignment | Key Issues |
|--------|-----------|------------|
| **core** | Good (80%) | Missing submodule depth in `dates/calendar/`, `market_data/*`, `math/interp/` |
| **valuations** | Poor (40%) | Instruments flattened, missing `components/`, `metrics/`, `pricing/`, `templates/`, `mc/`, `models/` |
| **statements** | Fair (60%) | Missing `capital_structure/`, `dsl/`, `forecast/` modules |
| **scenarios** | Good (75%) | Adapters may be incomplete |
| **portfolio** | Excellent (95%) | Nearly complete |

---

## Proposed Target Structure

### Valuations Module (Priority: HIGH)

```
finstack-py/
├── src/valuations/
│   ├── calibration/
│   │   ├── derivatives/        ← NEW
│   │   │   ├── mod.rs
│   │   │   └── sabr.rs
│   │   └── methods/            ← NEW (split from methods.rs)
│   │       ├── discount.rs
│   │       ├── forward_curve.rs
│   │       ├── hazard_curve.rs
│   │       ├── sabr_surface.rs
│   │       └── mod.rs
│   ├── cashflow/
│   │   └── builder/            ← NEW (split from builder.rs)
│   │       ├── schedule.rs
│   │       ├── state.rs
│   │       ├── types.rs
│   │       └── mod.rs
│   ├── instruments/
│   │   ├── bond/               ← NEW (split from bond.rs)
│   │   │   ├── types.rs
│   │   │   ├── cashflows.rs
│   │   │   ├── metrics.rs
│   │   │   ├── pricing.rs
│   │   │   └── mod.rs
│   │   ├── cds/                ← NEW (split from cds.rs)
│   │   ├── structured_credit/  ← NEW (split from structured_credit.rs)
│   │   │   ├── components.rs
│   │   │   ├── templates.rs
│   │   │   ├── waterfall.rs    ← Use existing structured_credit_waterfall.rs
│   │   │   └── mod.rs
│   │   └── ...                 ← Expand all instrument modules
│   └── common/
│       ├── mc/                 ← NEW
│       │   ├── brownian.rs
│       │   ├── gbm.rs
│       │   └── mod.rs
│       └── models/             ← NEW
│           ├── black_scholes.rs
│           └── mod.rs
└── finstack/valuations/
    ├── calibration/
    │   ├── derivatives/        ← NEW .pyi submodule
    │   └── methods/            ← NEW .pyi submodule
    ├── instruments/
    │   ├── bond/               ← NEW .pyi submodule
    │   ├── cds/                ← NEW .pyi submodule
    │   ├── structured_credit/  ← NEW .pyi submodule
    │   │   ├── __init__.pyi
    │   │   ├── components.pyi
    │   │   ├── templates.pyi
    │   │   └── waterfall.pyi
    │   └── ...
    └── common/
        ├── mc/                 ← NEW .pyi submodule
        └── models/             ← NEW .pyi submodule
```

### Statements Module (Priority: MEDIUM)

```
finstack-py/
├── src/statements/
│   ├── capital_structure/      ← NEW
│   │   ├── builder.rs
│   │   ├── integration.rs
│   │   └── mod.rs
│   ├── dsl/                    ← NEW
│   │   ├── ast.rs
│   │   ├── compiler.rs
│   │   ├── parser.rs
│   │   └── mod.rs
│   ├── evaluator/
│   │   ├── context.rs          ← NEW
│   │   ├── dag.rs              ← NEW
│   │   ├── engine.rs           ← SPLIT from mod.rs
│   │   └── mod.rs
│   ├── forecast/               ← NEW
│   │   ├── deterministic.rs
│   │   ├── statistical.rs
│   │   └── mod.rs
│   └── registry/
│       ├── builtins.rs         ← MOVE from extensions/
│       ├── dynamic.rs          ← NEW
│       └── validation.rs       ← NEW
└── finstack/statements/
    ├── capital_structure/      ← NEW .pyi submodule
    ├── dsl/                    ← NEW .pyi submodule
    └── forecast/               ← NEW .pyi submodule
```

### Core Module (Priority: LOW)

```
finstack-py/
├── src/core/
│   ├── dates/
│   │   └── calendar/           ← NEW
│   │       ├── business_days.rs
│   │       ├── registry.rs
│   │       └── mod.rs
│   ├── market_data/
│   │   ├── scalars/            ← NEW
│   │   ├── surfaces/           ← NEW
│   │   └── term_structures/    ← NEW
│   └── math/
│       └── interp/             ← NEW
└── finstack/core/
    ├── dates/
    │   └── calendar/           ← NEW .pyi submodule
    ├── market_data/
    │   ├── scalars/            ← NEW .pyi submodule
    │   ├── surfaces/           ← NEW .pyi submodule
    │   └── term_structures/    ← NEW .pyi submodule
    └── math/
        └── interp/             ← NEW .pyi submodule
```

---

## Implementation Strategy

### Phase 1: Documentation & Planning (Week 1)
1. ✅ Create this alignment plan
2. Document all current exports and public APIs
3. Create migration checklist
4. Design backward compatibility strategy

### Phase 2: Valuations Restructuring (Weeks 2-4)
**Goal:** Align valuations module structure with Rust

1. **Instruments refactoring:**
   - Split each instrument binding file into subdirectories
   - Create `mod.rs` files that re-export types
   - Update `.pyi` stub files with new submodule structure
   - Maintain top-level re-exports for backward compatibility

2. **Calibration refactoring:**
   - Split `methods.rs` → `methods/` directory
   - Move SABR code → `derivatives/` directory
   - Update imports in dependent modules

3. **Cashflow refactoring:**
   - Split `builder.rs` → `builder/` directory
   - Mirror Rust's schedule/state/types organization

4. **Common refactoring:**
   - Add `mc/` submodule for Monte Carlo bindings
   - Add `models/` submodule for pricing models
   - Move existing code into appropriate submodules

### Phase 3: Statements Restructuring (Weeks 5-6)
1. Add missing modules: `capital_structure/`, `dsl/`, `forecast/`
2. Expand existing modules with missing submodules
3. Update `.pyi` stubs

### Phase 4: Core & Other Modules (Week 7)
1. Add submodules to `dates/`, `market_data/`, `math/`
2. Update `.pyi` stubs
3. Test import paths

### Phase 5: WASM Alignment (Week 8)
1. Apply same structural changes to `finstack-wasm`
2. Ensure JS/TS exports mirror structure
3. Update documentation

### Phase 6: Documentation & Examples (Week 9)
1. Update all examples to use new import paths
2. Create migration guide for users
3. Update API documentation
4. Add deprecation warnings for old import paths (if removing)

### Phase 7: Testing & Validation (Week 10)
1. Run full test suite
2. Verify parity tests pass
3. Test backward compatibility
4. Performance benchmarks

---

## Backward Compatibility Strategy

### Option A: Dual Exports (Recommended)
- Keep both old flat structure AND new nested structure
- Re-export new modules at old locations
- Add deprecation warnings in documentation
- Plan removal in next major version

**Example:**
```python
# finstack-py/src/valuations/mod.rs

// New structured approach
pub(crate) mod instruments {
    pub mod bond;  // Contains bond/types.rs, bond/cashflows.rs, etc.
    pub mod cds;
    // ... etc
}

// Backward compatibility - re-export at old flat location
pub(crate) mod bond {
    pub use super::instruments::bond::*;
}
```

### Option B: Immediate Breaking Change
- Only support new structure
- Provide automated migration script
- Bump major version
- Requires user code updates

**Recommendation:** Option A for Python, consider Option B for WASM (smaller user base)

---

## Benefits of Alignment

1. **Consistency:** Users can navigate both Rust and Python codebases similarly
2. **Discoverability:** Nested structure makes it clearer where functionality lives
3. **Maintainability:** Easier to keep bindings in sync with Rust updates
4. **Documentation:** Can reuse documentation structure across languages
5. **Scalability:** Easier to add new instrument types or features
6. **Type Safety:** Better stub organization for Python type checkers
7. **Learning Curve:** Understanding Rust code helps with Python and vice versa

---

## Risks & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking existing user code | High | Option A: Maintain backward compat for 1-2 versions |
| Increased complexity | Medium | Clear documentation, gradual migration |
| Stub generation issues | Medium | Automated stub generation tests |
| Import performance | Low | Python caches imports, minimal impact |
| Increased file count | Low | Better organization outweighs this |
| Testing burden | Medium | Incremental changes with tests per phase |

---

## Success Metrics

- [ ] 100% of Rust public modules have corresponding Python submodules
- [ ] All existing tests pass with new structure
- [ ] Parity tests confirm identical behavior
- [ ] Documentation updated with new import patterns
- [ ] Migration guide completed
- [ ] Zero performance regression
- [ ] Community feedback positive

---

## Appendix: Import Path Examples

### Current (Flat)
```python
from finstack.valuations.instruments import Bond
from finstack.valuations.calibration import CalibrationConfig
from finstack.valuations.cashflow import CashflowBuilder
```

### Proposed (Nested)
```python
from finstack.valuations.instruments.bond import Bond, BondMetrics
from finstack.valuations.calibration.config import CalibrationConfig
from finstack.valuations.calibration.methods import DiscountCurveCalibrator
from finstack.valuations.cashflow.builder import CashflowBuilder, ScheduleParams
from finstack.valuations.instruments.structured_credit.templates import CLOTemplate
from finstack.valuations.instruments.structured_credit.waterfall import WaterfallEngine
```

### With Backward Compatibility
```python
# Both work:
from finstack.valuations.instruments.bond import Bond  # New
from finstack.valuations.instruments import Bond       # Old (deprecated)
```

---

## Next Steps

1. **Review this plan** with team/stakeholders
2. **Prototype** one instrument (e.g., Bond) with nested structure
3. **Validate** stub generation and imports work correctly
4. **Decide** on backward compatibility approach
5. **Create** detailed implementation tickets
6. **Begin** Phase 1 execution

---

**Document Version:** 1.0  
**Created:** 2025-11-03  
**Author:** AI Assistant  
**Status:** Draft - Awaiting Review

