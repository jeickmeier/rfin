# Python-Rust Feature Gap Analysis

## Executive Summary

This document identifies **missing features** in the Python bindings compared to the Rust crates. Structural flattening (e.g., `bond/` directory → `bond.rs` file) is **acceptable**, but entire missing modules represent functional gaps that confuse users.

---

## Critical Feature Gaps

### 1. Statements Module - Missing Core Features ⚠️

**Rust Modules (`finstack/statements/src/`):**
```
✓ builder/
✗ capital_structure/      ← MISSING ENTIRELY
✗ dsl/                    ← MISSING ENTIRELY  
✓ error
✓ evaluator/
✓ extensions/
✗ forecast/               ← MISSING ENTIRELY
✓ registry/
✓ results/
✓ types/
```

**Python Modules (`finstack-py/src/statements/`):**
```
✓ builder/
✓ error
✓ evaluator/
✓ extensions/
✓ registry/
✓ types/
✓ utils
```

**Coverage:** 6/9 modules (67%)

#### Missing Feature: `capital_structure/`

**Impact: HIGH**

- **What it does in Rust:** Integration between financial statements and capital structure modeling (debt tranches, equity, preferred stock)
- **User impact:** Cannot model leveraged companies, LBOs, or capital structure-dependent metrics from Python
- **Example Rust API:**
  ```rust
  use finstack_statements::capital_structure::CapitalStructure;
  
  let cap_structure = CapitalStructure::builder()
      .add_tranche("senior_debt", Money::new(100_000_000, USD))
      .add_tranche("mezzanine", Money::new(50_000_000, USD))
      .build()?;
  
  // Integrate with statement model
  model.integrate_capital_structure(cap_structure)?;
  ```
- **Python workaround:** **None** - functionality completely unavailable

#### Missing Feature: `dsl/`

**Impact: MEDIUM**

- **What it does in Rust:** DSL for defining statement models in text format (parser, compiler, AST)
- **User impact:** Cannot use text-based model definitions, must build everything programmatically
- **Example Rust API:**
  ```rust
  use finstack_statements::dsl::parse_model;
  
  let model_text = r#"
      Revenue = @Value
      COGS = Revenue * 0.6
      Gross_Profit = Revenue - COGS
  "#;
  
  let model = parse_model(model_text)?;
  ```
- **Python workaround:** Must use builder API only (more verbose, but functional)

#### Missing Feature: `forecast/`

**Impact: HIGH**

- **What it does in Rust:** Forecasting methods (deterministic, statistical, time-series)
- **User impact:** Cannot configure advanced forecast methods for model nodes
- **Example Rust API:**
  ```rust
  use finstack_statements::forecast::{ForecastMethod, TimeSeriesMethod};
  
  let forecast = ForecastMethod::TimeSeries(TimeSeriesMethod::LinearRegression {
      periods: 4,
  });
  
  node.set_forecast_method(forecast);
  ```
- **Python workaround:** Limited - basic forecasts may work through node types, but advanced statistical/time-series methods unavailable

---

### 2. Scenarios Module - Missing Adapters ⚠️

**Rust Modules (`finstack/scenarios/src/`):**
```
✗ adapters/               ← MISSING ENTIRELY  
✓ engine
✓ error
✓ spec
✗ utils                   ← MISSING (likely internal-only)
```

**Python Modules (`finstack-py/src/scenarios/`):**
```
✓ engine
✓ enums
✓ error
✓ reports
✓ spec
```

**Coverage:** 3/4 public modules (75%) - `utils` is likely internal

#### Missing Feature: `adapters/`

**Impact: MEDIUM**

- **What it does in Rust:** Adapters for applying scenarios to different data types
  - `adapters/curves.rs` - Apply shocks to discount/forward/hazard curves
  - `adapters/fx.rs` - Apply FX rate shocks
  - `adapters/scalars.rs` - Apply scalar shocks
  - `adapters/instruments.rs` - Apply instrument-level shocks
  - `adapters/statements.rs` - Apply shocks to statement forecasts

- **User impact:** May not be able to apply scenarios to all data types, or adapter logic is embedded in engine
- **Example Rust API:**
  ```rust
  use finstack_scenarios::adapters::apply_curve_shock;
  
  let shocked_curve = apply_curve_shock(
      &base_curve,
      &scenario_spec,
  )?;
  ```
- **Investigation needed:** ✓ Check if adapter functionality is embedded in the engine bindings

---

### 3. Valuations Module - Good Coverage ✅

**Monte Carlo Module - Now Properly Located:** ✅

The Monte Carlo functionality has been **moved** from the top level (`mc_generator.rs`, `mc_params.rs`, etc.) to under `common/mc/` in both Python and WASM bindings, matching the Rust structure:

```
Rust:  valuations/src/instruments/common/mc/
Python: valuations/src/common/mc/  
WASM:  valuations/src/common/mc/
```

**Note:** Due to PyO3/maturin limitations, Python imports are flat (`from finstack.valuations import PathPoint`) even though the code is properly nested. The `.pyi` stub files reflect the nested structure for documentation.

**Instrument Coverage:**

All 35 Rust instruments have Python bindings:
```
✓ asian_option          ✓ autocallable          ✓ barrier_option
✓ basis_swap            ✓ basket                ✓ bond
✓ cap_floor             ✓ cds                   ✓ cds_index
✓ cds_option            ✓ cds_tranche           ✓ cliquet_option
✓ cms_option            ✓ convertible           ✓ deposit
✓ equity                ✓ equity_option         ✓ fra
✓ fx (spot/option/swap) ✓ fx_barrier_option     ✓ inflation_linked_bond
✓ inflation_swap        ✓ ir_future             ✓ irs
✓ lookback_option       ✓ private_markets_fund  ✓ quanto_option
✓ range_accrual         ✓ repo                  ✓ revolving_credit
✓ structured_credit     ✓ swaption              ✓ term_loan
✓ trs (equity/fi)       ✓ variance_swap
```

**Coverage:** 35/35 instruments (100%)

**Note:** While each instrument is flattened into a single `.rs` file rather than a directory (e.g., `bond.rs` vs `bond/types.rs` + `bond/cashflows.rs`), this is **acceptable per user requirements** - all functionality is present.

---

### 4. Core Module - Acceptable Coverage ✅

**Module Coverage:**
```
✓ cashflow/
✓ config
✓ currency
✓ dates/ (calendar flattened, but API complete)
✓ market_data/ (submodules flattened, but API complete)
✓ math/ (interpolation flattened, but API complete)
✓ money/
⚠️ expr/ (partially exposed - basic expression evaluation available)
```

**Coverage:** ~95% of public API

**Note:** While internal submodule organization differs (e.g., `dates/calendar/` flattened), the Python API exposes all necessary functionality through high-level types like `Calendar`, `DayCount`, etc.

---

### 5. Portfolio Module - Complete ✅

**Module Coverage:**
```
✓ builder
✓ grouping
✓ metrics
✓ portfolio
✓ position
✓ results
✓ scenarios
✓ types
✓ valuation
```

**Coverage:** 9/9 modules (100%)

---

## Summary Matrix

| Module       | Rust Modules | Python Modules | Missing Features | Coverage | Priority |
|--------------|--------------|----------------|------------------|----------|----------|
| **core**     | ~20          | ~15            | expr (partial)   | 95%      | LOW      |
| **valuations** | 35 instruments | 35 instruments | None            | 100%     | ✅       |
| **statements** | 9            | 6              | capital_structure, dsl, forecast | 67% | **HIGH** |
| **scenarios** | 4            | 4              | adapters (maybe embedded) | 75%     | MEDIUM   |
| **portfolio** | 9            | 9              | None             | 100%     | ✅       |

---

## Recommended Actions

### Priority 1: Statements Module (HIGH Impact)

1. **Add `capital_structure/` bindings**
   - Create `finstack-py/src/statements/capital_structure/`
   - Expose `CapitalStructure`, `Tranche`, `CapitalStructureBuilder`
   - Add integration methods for statement models
   - **User benefit:** Enable LBO/leveraged finance modeling

2. **Add `forecast/` bindings**
   - Create `finstack-py/src/statements/forecast/`
   - Expose `ForecastMethod`, time-series methods, statistical methods
   - **User benefit:** Enable advanced forecasting capabilities

3. **Add `dsl/` bindings (Optional)**
   - Create `finstack-py/src/statements/dsl/`
   - Expose `parse_model()`, `compile_model()`
   - **User benefit:** Allow text-based model definitions (nice-to-have)

### Priority 2: Scenarios Module (MEDIUM Impact)

1. **Investigate `adapters/` module**
   - Check if adapter logic is already embedded in `engine.rs` bindings
   - If missing, add explicit adapter bindings
   - Document how to apply shocks to each data type
   - **User benefit:** Clarify scenario application patterns

### Priority 3: Documentation (HIGH Impact, Low Effort)

1. **Document structural differences**
   - Add note to README: "Python bindings flatten Rust submodules into single files (e.g., `bond.rs` vs `bond/`). All functionality is preserved."
   - Create import mapping guide for users switching between Rust and Python
   - **User benefit:** Reduce confusion

2. **Document missing features**
   - Clearly document which Rust features are unavailable in Python
   - Provide workarounds where possible
   - Add to FAQ section
   - **User benefit:** Set clear expectations

---

## Non-Issues (Explicitly Acceptable)

The following are **NOT gaps** per user requirements:

1. ✅ **Flattened instrument modules** - Python has `bond.rs` instead of `bond/types.rs` + `bond/cashflows.rs` etc. **This is fine.**
2. ✅ **Flattened calibration modules** - Python has `methods.rs` instead of `methods/discount.rs` etc. **This is fine.**
3. ✅ **Flattened date/market_data modules** - Internal organization differs but API is complete. **This is fine.**

---

## WASM Alignment

**Question for user:** Should WASM bindings follow the same pattern as Python (functional parity > structural mirroring)?

**Current WASM status:** Similar to Python - instruments flattened, core modules reasonably aligned.

---

## Next Steps

1. ✅ Review this analysis with team
2. [ ] Prioritize which missing features to implement first
3. [ ] Create implementation tickets for:
   - `statements/capital_structure` bindings
   - `statements/forecast` bindings
   - `scenarios/adapters` investigation
4. [ ] Update documentation to clarify structural differences
5. [ ] Create import mapping guide for Rust ↔ Python users

---

**Document Version:** 2.0  
**Created:** 2025-11-03  
**Status:** Ready for Review
