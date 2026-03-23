# Python-Rust Structural Parity Program

## Summary

- Parity target: Python must match Rust in public capability, public package/module structure, and canonical import paths.
- Canonical Python topology mirrors the public Rust umbrella crate and public sub-crates.
- Python-friendly aliases may remain as compatibility shims only. They are not the source of truth.
- Cross-crate surfaces currently flattened into other packages must move to their matching canonical package.

## Canonical Mapping (Current State → Target)

Based on the umbrella crate's `pub use` re-exports in `finstack/src/lib.rs`:

| Rust Crate (umbrella re-export) | Target Python Package | Current Python State | Gap |
|---|---|---|---|
| `finstack_core as core` | `finstack.core` | `finstack.core` exists | Module-level parity needed |
| `finstack_analytics as analytics` | `finstack.analytics` | Nested at `finstack.core.analytics` | **Missing canonical root package** |
| `finstack_margin as margin` | `finstack.margin` | No dedicated package | **Missing canonical root package** |
| `finstack_valuations as valuations` | `finstack.valuations` | `finstack.valuations` exists | Module-level parity needed |
| `finstack_statements as statements` | `finstack.statements` | `finstack.statements` exists | Contains statements_analytics content |
| `finstack_statements_analytics as statements_analytics` | `finstack.statements_analytics` | Flattened into `finstack.statements` | **Missing canonical root package** |
| `finstack_portfolio as portfolio` | `finstack.portfolio` | `finstack.portfolio` exists | Module-level parity needed |
| `finstack_scenarios as scenarios` | `finstack.scenarios` | `finstack.scenarios` exists | Module-level parity needed |

**Excluded from parity** (internal crates, not re-exported from umbrella):
- `finstack-cashflows` — internal to valuations
- `finstack-correlation` — internal to valuations/monte-carlo
- `finstack-monte-carlo` — internal to valuations
- `finstack-valuations-macros` — proc-macro, compile-time only

## Current Infrastructure Issues

1. **Audit scripts write tracked files**: `python_api.json`, `rust_api.json`, `wasm_api.json`, `parity_manifest.json`, `wasm_parity_checklist.json` are all tracked in git.
2. **`__all__` uses `globals()`**: 7 of 18 packages derive `__all__` dynamically — `finstack`, `finstack.core`, `finstack.portfolio`, `finstack.statements`, `finstack.valuations`, `finstack.valuations.calibration`, `finstack.valuations.instruments`.
3. **No topology tests**: Existing parity tests verify behavioral correctness, not structural placement.
4. **Parity manifest is JSON, driven from stubs only**: `generate_parity_manifest.py` parses `.pyi` files via AST. Does not cross-reference Rust source for ownership.
5. **Hybrid module system is fragile**: Root `__init__.py` uses `_setup_hybrid_module()` to re-register Rust submodules in `sys.modules` — works but is implicit.

---

## Implementation Plan

### Wave 1: Parity Contract + Topology Audit + Audit Hygiene

**Goal**: Establish the source-of-truth contract and non-mutating audit infrastructure.

#### Step 1.1: Create parity contract file (`parity_contract.toml`)

Location: `finstack-py/parity_contract.toml`

Content structure:

```toml
[meta]
version = "1.0.0"
description = "Canonical Rust→Python structural parity contract"

# One section per umbrella re-export
[crates.core]
rust_crate = "finstack-core"
python_package = "finstack.core"
visibility = "pub"  # only pub items at crate boundary

  [crates.core.modules]
  # Each pub mod in finstack-core/src/lib.rs
  cashflow = { python = "finstack.core.cashflow" }
  config = { python = "finstack.core.config" }
  currency = { python = "finstack.core.currency" }
  dates = { python = "finstack.core.dates" }
  error = { python = "finstack.core.error" }
  explain = { python = "finstack.core.explain" }
  expr = { python = "finstack.core.expr" }
  factor_model = { python = "finstack.core.factor_model" }
  market_data = { python = "finstack.core.market_data" }
  math = { python = "finstack.core.math" }
  money = { python = "finstack.core.money" }
  prelude = { python = "finstack.core.prelude" }
  types = { python = "finstack.core.types" }
  validation = { python = "finstack.core.validation" }

[crates.analytics]
rust_crate = "finstack-analytics"
python_package = "finstack.analytics"
visibility = "pub"

  [crates.analytics.modules]
  aggregation = { python = "finstack.analytics.aggregation" }
  benchmark = { python = "finstack.analytics.benchmark" }
  consecutive = { python = "finstack.analytics.consecutive" }
  drawdown = { python = "finstack.analytics.drawdown" }
  lookback = { python = "finstack.analytics.lookback" }
  performance = { python = "finstack.analytics.performance" }
  returns = { python = "finstack.analytics.returns" }
  risk_metrics = { python = "finstack.analytics.risk_metrics" }

[crates.margin]
rust_crate = "finstack-margin"
python_package = "finstack.margin"
visibility = "pub"

  [crates.margin.modules]
  calculators = { python = "finstack.margin.calculators" }
  config = { python = "finstack.margin.config" }
  constants = { python = "finstack.margin.constants" }
  metrics = { python = "finstack.margin.metrics" }
  registry = { python = "finstack.margin.registry" }
  traits = { python = "finstack.margin.traits" }
  types = { python = "finstack.margin.types" }
  xva = { python = "finstack.margin.xva" }

[crates.valuations]
rust_crate = "finstack-valuations"
python_package = "finstack.valuations"
visibility = "pub"
# modules TBD — valuations lib.rs is large, needs detailed audit

[crates.statements]
rust_crate = "finstack-statements"
python_package = "finstack.statements"
visibility = "pub"

  [crates.statements.modules]
  adjustments = { python = "finstack.statements.adjustments" }
  builder = { python = "finstack.statements.builder" }
  capital_structure = { python = "finstack.statements.capital_structure" }
  dsl = { python = "finstack.statements.dsl" }
  error = { python = "finstack.statements.error" }
  evaluator = { python = "finstack.statements.evaluator" }
  extensions = { python = "finstack.statements.extensions" }
  forecast = { python = "finstack.statements.forecast" }
  prelude = { python = "finstack.statements.prelude" }
  registry = { python = "finstack.statements.registry" }
  types = { python = "finstack.statements.types" }
  utils = { python = "finstack.statements.utils" }

[crates.statements_analytics]
rust_crate = "finstack-statements-analytics"
python_package = "finstack.statements_analytics"
visibility = "pub"

  [crates.statements_analytics.modules]
  analysis = { python = "finstack.statements_analytics.analysis" }
  extensions = { python = "finstack.statements_analytics.extensions" }
  templates = { python = "finstack.statements_analytics.templates" }
  prelude = { python = "finstack.statements_analytics.prelude" }

[crates.portfolio]
rust_crate = "finstack-portfolio"
python_package = "finstack.portfolio"
visibility = "pub"

  [crates.portfolio.modules]
  attribution = { python = "finstack.portfolio.attribution" }
  book = { python = "finstack.portfolio.book" }
  builder = { python = "finstack.portfolio.builder" }
  cashflows = { python = "finstack.portfolio.cashflows" }
  dependencies = { python = "finstack.portfolio.dependencies" }
  error = { python = "finstack.portfolio.error" }
  factor_model = { python = "finstack.portfolio.factor_model" }
  grouping = { python = "finstack.portfolio.grouping" }
  margin = { python = "finstack.portfolio.margin" }
  metrics = { python = "finstack.portfolio.metrics" }
  optimization = { python = "finstack.portfolio.optimization" }
  portfolio = { python = "finstack.portfolio.portfolio" }
  position = { python = "finstack.portfolio.position" }
  prelude = { python = "finstack.portfolio.prelude" }
  results = { python = "finstack.portfolio.results" }
  types = { python = "finstack.portfolio.types" }
  valuation = { python = "finstack.portfolio.valuation" }

[crates.scenarios]
rust_crate = "finstack-scenarios"
python_package = "finstack.scenarios"
visibility = "pub"

  [crates.scenarios.modules]
  adapters = { python = "finstack.scenarios.adapters" }
  engine = { python = "finstack.scenarios.engine" }
  error = { python = "finstack.scenarios.error" }
  spec = { python = "finstack.scenarios.spec" }
  templates = { python = "finstack.scenarios.templates" }
  utils = { python = "finstack.scenarios.utils" }

# Compatibility aliases — old import path → canonical path
[aliases]
"finstack.core.analytics" = "finstack.analytics"
"finstack.core.analytics.Performance" = "finstack.analytics.performance.Performance"
"finstack.statements.analysis" = "finstack.statements_analytics.analysis"
"finstack.statements.extensions.CorkscrewExtension" = "finstack.statements_analytics.extensions.CorkscrewExtension"
"finstack.statements.extensions.CreditScorecardExtension" = "finstack.statements_analytics.extensions.CreditScorecardExtension"
"finstack.statements.templates" = "finstack.statements_analytics.templates"

# Explicit exclusions — Rust-only items not mapped to Python
[exclusions]
reason_prefix = true  # each key has a reason

  [exclusions.items]
  "finstack_core::golden" = "Test-only module, feature-gated"
  "finstack_core::validation" = "Internal validation helpers"
  "finstack_statements::utils" = "Internal utilities"
```

**Tasks:**
1. Write `finstack-py/parity_contract.toml` with complete crate→module mapping derived from Rust `lib.rs` files.
2. For each crate, enumerate `pub mod` entries from `src/lib.rs` as module entries.
3. For each module, list key `pub` symbols (structs, enums, functions) — this is the symbol-level contract.
4. Record all known compatibility aliases (current import paths that will become shims).
5. Record exclusions with reasons.

#### Step 1.2: Gitignore audit outputs + create `.audit/` directory

**Tasks:**
1. Add to `.gitignore`:

   ```
   # Audit outputs (regenerated by scripts/audits/)
   .audit/
   ```

2. Update all audit scripts to write to `.audit/` instead of `scripts/audits/`:
   - `audit_python_api.py` → `.audit/python_api.json`
   - `audit_rust_api.py` → `.audit/rust_api.json`
   - `audit_wasm_api.py` → `.audit/wasm_api.json`
   - `compare_apis.py` → `.audit/PARITY_AUDIT.md`
   - `generate_parity_manifest.py` → `.audit/parity_manifest.json` + `.audit/wasm_parity_checklist.json`
3. Remove tracked JSON artifacts from `scripts/audits/` (git rm the generated files, keep the scripts).
4. Keep `PARITY_AUDIT.md` at repo root as untracked (already is).

#### Step 1.3: Build topology audit script

New script: `scripts/audits/audit_topology.py`

**What it validates (reads parity_contract.toml as source of truth):**
1. **Root package parity**: For each `[crates.*]` entry, verify a Python package exists at the declared `python_package` path (has `__init__.py` or `__init__.pyi`).
2. **Module tree parity**: For each `[crates.*.modules.*]` entry, verify the Python module exists.
3. **Symbol placement**: For each symbol listed in the contract, verify it is importable from its canonical Python path (not just from somewhere).
4. **Alias correctness**: For each `[aliases]` entry, verify that `import old_path.Symbol` and `import canonical_path.Symbol` resolve to the same object (`is` identity check).
5. **No undeclared exports**: For each Python package, verify that `__all__` contains only symbols declared in the contract (no leaked helpers).

**Output**: Structured JSON to `.audit/topology_report.json` + human-readable summary to stdout. Exit code 0 if clean, 1 if gaps found.

**Tasks:**
1. Write `scripts/audits/audit_topology.py`.
2. Add a `Makefile` target or script entry point: `make audit-topology` / `python -m scripts.audits.audit_topology`.
3. The script must NOT write any tracked file.

#### Step 1.4: Add topology parity pytest

New test: `finstack-py/tests/parity/test_topology_parity.py`

**Tests:**
1. `test_all_crates_have_python_packages()` — Parse contract, assert each crate has a corresponding importable Python package.
2. `test_all_modules_exist()` — For each module in contract, assert it's importable.
3. `test_symbol_canonical_placement()` — For key symbols, assert they live in their canonical module.
4. `test_alias_identity()` — For each alias, assert `old is new` (object identity).
5. `test_no_leaked_helpers_in_all()` — For each package with `__all__`, assert no names starting with `_` or not in contract.

**Tasks:**
1. Write the test file.
2. Mark tests that will initially fail as `@pytest.mark.xfail(reason="Wave 2: pending structural moves")` so CI stays green.
3. Tests progressively pass as Waves 2-4 land.

#### Wave 1 Exit Criteria

- `parity_contract.toml` checked in and complete for all 8 crate mappings.
- All audit scripts write to `.audit/` only. Zero tracked JSON artifacts in `scripts/audits/`.
- `audit_topology.py` runs and produces a report (expected failures for missing packages are documented).
- Topology parity tests exist and xfail where structural moves haven't happened yet.

---

### Wave 2: Root Package Structure + Cross-Crate Canonical Paths

**Goal**: Create missing canonical packages and move misplaced APIs. Every commit keeps all existing imports working via shims.

#### Step 2.1: Create `finstack.analytics` canonical package

**Current state**: `finstack.core.analytics` exists with `Performance` class and `expr` submodule. The umbrella crate re-exports `finstack_analytics as analytics` at the root level.

**Tasks:**
1. **Rust binding layer** (`finstack-py/src/`):
   - Register `analytics` as a top-level submodule in `lib.rs` (alongside `core`, `valuations`, etc.).
   - Create `finstack-py/src/analytics/mod.rs` that exposes the analytics classes/functions directly from `finstack_analytics`.
   - Sub-register modules matching the crate's `pub mod` structure: `aggregation`, `benchmark`, `consecutive`, `drawdown`, `lookback`, `performance`, `returns`, `risk_metrics`.

2. **Python runtime layer** (`finstack-py/finstack/`):
   - Create `finstack-py/finstack/analytics/__init__.py` that imports from the Rust `_finstack.analytics` module.
   - Use explicit `__all__` (not `globals()`).
   - Create submodule `__init__.py` files for `performance/`, `returns/`, etc. as needed.

3. **Stub layer**:
   - Create `finstack-py/finstack/analytics/__init__.pyi` with type stubs.

4. **Compatibility shim** (atomic with step 2):
   - Update `finstack-py/finstack/core/analytics/__init__.py` to become a re-export shim:

     ```python
     # Compatibility shim — canonical path is finstack.analytics
     from finstack.analytics import *
     from finstack.analytics import __all__
     ```

   - Add `warnings.warn("finstack.core.analytics is deprecated, use finstack.analytics", DeprecationWarning)` if desired (Wave 4).

5. **Update root `__init__.py`**:
   - Add `analytics` to the list of submodules registered in the hybrid module system.

#### Step 2.2: Create `finstack.margin` canonical package

**Current state**: No `finstack.margin` Python package. Margin types are accessed through `finstack.portfolio.margin` or `finstack.valuations.margin`.

**Tasks:**
1. **Rust binding layer**:
   - Register `margin` as a top-level submodule in `lib.rs`.
   - Create `finstack-py/src/margin/mod.rs` exposing `finstack_margin` types.
   - Sub-register: `calculators`, `config`, `constants`, `metrics`, `types`, `xva`.

2. **Python runtime layer**:
   - Create `finstack-py/finstack/margin/__init__.py` with explicit `__all__`.

3. **Stub layer**:
   - Create `finstack-py/finstack/margin/__init__.pyi`.

4. **Compatibility**:
   - `finstack.portfolio.margin` and `finstack.valuations.margin` remain as shims that re-export from `finstack.margin`.

#### Step 2.3: Create `finstack.statements_analytics` canonical package

**Current state**: `finstack-statements-analytics` owns `analysis`, `extensions` (CorkscrewExtension, CreditScorecardExtension), `templates`, and `prelude`. These are currently flattened into `finstack.statements.analysis`, `finstack.statements.extensions`, and `finstack.statements.templates`.

**This is the highest-risk move. Shims must land atomically.**

**Tasks:**
1. **Rust binding layer**:
   - Register `statements_analytics` as a top-level submodule in `lib.rs`.
   - Create `finstack-py/src/statements_analytics/mod.rs` exposing `finstack_statements_analytics` types.
   - Sub-register: `analysis`, `extensions`, `templates`.
   - The `analysis` module includes: sensitivity, scenario_set, variance, dcf, goal_seek, covenants, backtesting, monte_carlo, introspection, corporate, credit_context, reports.
   - The `extensions` module includes: CorkscrewExtension, CreditScorecardExtension, and their config types.

2. **Python runtime layer**:
   - Create `finstack-py/finstack/statements_analytics/__init__.py` with explicit `__all__`.
   - Create subpackage `__init__.py` files for `analysis/`, `extensions/`, `templates/`.

3. **Stub layer**:
   - Create `finstack-py/finstack/statements_analytics/__init__.pyi`.
   - Move the large `statements/analysis/__init__.pyi` (39KB) content to `statements_analytics/analysis/__init__.pyi`.
   - Move `statements/extensions/extensions.pyi` content that belongs to `statements_analytics`.
   - Move `statements/templates.pyi` content to `statements_analytics/templates/__init__.pyi`.

4. **Compatibility shims** (ATOMIC with the move):
   - `finstack.statements.analysis` → re-export shim importing from `finstack.statements_analytics.analysis`.
   - `finstack.statements.extensions` → split: types owned by `finstack-statements` stay; types owned by `finstack-statements-analytics` become re-exports.
   - `finstack.statements.templates` → re-export shim importing from `finstack.statements_analytics.templates`.
   - These shims must be committed in the **same commit** as the canonical modules.

5. **Cross-crate bridge**:
   - The umbrella crate has: `pub use finstack_statements_analytics::analysis::covenants` (gated on `valuations + statements`). This maps to `finstack.statements_analytics.analysis.covenants` canonically, with a shim at `finstack.valuations.covenants` if it currently lives there.

#### Step 2.4: Fix `__all__` in existing packages

**Packages that currently use `globals()` pattern:**
- `finstack/__init__.py`
- `finstack/core/__init__.py`
- `finstack/portfolio/__init__.py`
- `finstack/statements/__init__.py`
- `finstack/valuations/__init__.py`
- `finstack/valuations/calibration/__init__.py`
- `finstack/valuations/instruments/__init__.py`

**Tasks:**
For each package:
1. Read the parity contract to get the curated export list for that module.
2. Replace `__all__ = [name for name in globals() if not name.startswith("_")]` with an explicit list.
3. Verify no helper names (`_setup_hybrid_module`, `_rust_core`, `_finstack`, etc.) leak into `__all__`.

#### Wave 2 Exit Criteria

- `finstack.analytics`, `finstack.margin`, `finstack.statements_analytics` exist as importable packages.
- Every symbol in these packages is importable from its canonical path.
- Old import paths (`finstack.core.analytics`, `finstack.statements.analysis`, etc.) still work via shims.
- All `__all__` lists are explicit (zero `globals()` derivation).
- Topology parity tests for Wave 2 packages pass (remove xfail markers).
- All existing behavioral parity tests still pass.

---

### Wave 3: Per-Domain Export Registration + Stub Alignment

**Goal**: Achieve symbol-level parity within each canonical module. Align Rust wrappers, Python runtime, and stubs.

#### Step 3.1: Audit symbol coverage per canonical module

For each module in the parity contract:
1. List all `pub` symbols from the Rust crate source.
2. List all symbols exposed in the PyO3 binding layer (`#[pyclass]`, `#[pyfunction]`, etc.).
3. List all symbols in the Python `__init__.py` `__all__`.
4. List all symbols in the `.pyi` stub.
5. Produce a diff: what's in Rust but not Python, what's in Python but not contract, what's in stub but not runtime.

**Tasks:**
1. Extend `audit_topology.py` to do per-module symbol diffing.
2. Generate a symbol gap report to `.audit/symbol_gaps.json`.

#### Step 3.2: Register missing symbols in Rust binding layer

For each symbol gap (in Rust, not in Python bindings):
1. Determine if it should be exposed (check contract — is it listed, or is it excluded?).
2. If listed: add `#[pyclass]`/`#[pyfunction]` wrapper in the appropriate `finstack-py/src/` module.
3. If excluded: verify exclusion is documented in contract.

**Priority order:**
1. Types that are in the contract but missing from Python.
2. Functions that are in the contract but missing from Python.
3. Enum variants and constants.

#### Step 3.3: Align `__all__` with contract

For each canonical Python module:
1. Set `__all__` to exactly match the contract's symbol list for that module.
2. Remove any symbols from `__all__` that are not in the contract.
3. Add any symbols to `__all__` that are in the contract but missing.

#### Step 3.4: Align stubs with runtime

For each canonical Python module:
1. Compare `.pyi` stub exports against `__all__`.
2. Add missing stub entries.
3. Remove stub entries for symbols not in `__all__`.
4. Verify signatures match (parameter names, types, return types).

**Tasks:**
1. Run `pyright --verifytypes finstack` and fix all reported mismatches.
2. Add stub files for new canonical packages (`analytics/`, `margin/`, `statements_analytics/`).

#### Step 3.5: Three-layer consistency test

New test: `finstack-py/tests/parity/test_three_layer_parity.py`

For each canonical module in the contract:
1. Assert the Rust binding layer exposes the symbol (via `finstack.finstack` raw module).
2. Assert the Python runtime `__all__` contains the symbol.
3. Assert the `.pyi` stub declares the symbol.
4. Assert all three agree on the symbol's type (class vs function vs constant).

#### Wave 3 Exit Criteria

- Symbol gap report shows zero missing symbols for contracted items.
- `__all__` matches contract for every canonical module.
- Stubs match runtime for every canonical module.
- `pyright --verifytypes finstack` passes with zero errors related to structural mismatches.
- Three-layer consistency test passes.

---

### Wave 4: Behavioral + Documentation Cleanup

**Goal**: Complete the transition. Wire deprecation warnings, update all references, verify end-to-end.

#### Step 4.1: Add deprecation warnings to compatibility shims

For each alias in `[aliases]`:
1. Add `warnings.warn(f"Importing from {old_path} is deprecated, use {canonical_path}", DeprecationWarning, stacklevel=2)` to the shim module's import path.
2. Ensure the warning fires on first import, not on every attribute access.

#### Step 4.2: Update tests to canonical imports

1. Grep all test files for imports from deprecated paths.
2. Update to canonical imports.
3. Add a test that imports each shim path and asserts a `DeprecationWarning` is raised.

#### Step 4.3: Update examples and docs

1. Grep examples/ and docs/ for deprecated import paths.
2. Update all to canonical paths.
3. Add migration notes documenting old → new paths.

#### Step 4.4: Re-anchor behavioral parity tests

1. Move tests currently in `tests/parity/test_statements_parity.py` that test `statements_analytics` functionality to a new `test_statements_analytics_parity.py`.
2. Update import paths in all parity tests to canonical modules.
3. Ensure each parity test file maps to one canonical crate.

#### Step 4.5: Final verification sweep

1. Run full test suite: `pytest finstack-py/tests/`.
2. Run `pyright --verifytypes finstack`.
3. Run `python scripts/audits/audit_topology.py` — must exit 0.
4. Grep for any remaining TODOs referencing canonicalization.
5. Verify zero `globals()`-based `__all__` definitions remain.

#### Wave 4 Exit Criteria

- All examples and docs reference canonical imports (zero legacy-path usage outside shim modules).
- `pyright --verifytypes finstack` passes clean.
- Deprecation warnings fire on all compatibility shim imports.
- No open TODOs referencing canonicalization remain in the codebase.
- All parity tests pass and are anchored to canonical crate ownership.
- Topology audit exits 0 with zero gaps.

---

## Risk Register

| Risk | Mitigation |
|---|---|
| `statements` → `statements_analytics` move breaks users | Atomic shim rule: old path always works. Deprecation warning only, no removal. |
| Stub files get out of sync during moves | Wave 3 three-layer test catches this. Run after every structural commit. |
| `globals()` removal breaks lazy module loading | Test each package's `__all__` against actual importability before committing. |
| Rust binding registration order matters for PyO3 | Register parent modules before children. Test import order in CI. |
| Valuations crate is very large, module enumeration is incomplete | Defer detailed valuations module mapping to Wave 3 audit. Contract marks it as "TBD" until then. |
| Margin types are currently split across portfolio.margin and valuations.margin | New `finstack.margin` becomes canonical; both old paths become shims. |

## Key Principles

- **No commit may break existing imports.** Shims land atomically with structural moves.
- **Contract is the source of truth.** All tooling reads `parity_contract.toml`, not source code heuristics.
- **Business logic stays in Rust.** Python adds wrappers, shims, stubs, and error translation only.
- **Visibility boundary is `pub` at crate boundary.** `pub(crate)` and `pub(super)` are excluded.
- **Audit scripts never write tracked files.** Output goes to `.audit/` (gitignored) or stdout.
