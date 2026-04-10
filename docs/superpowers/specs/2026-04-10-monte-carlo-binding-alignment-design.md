# Monte Carlo Binding Alignment Design Spec

**Date**: 2026-04-10
**Status**: Draft
**Phase**: Binding/core alignment for Monte Carlo surfaces

## Problem

The Monte Carlo Python bindings currently do not mirror the Rust crate structure cleanly. The largest issue is architectural: some Monte Carlo logic lives in `finstack-py` instead of `finstack/monte_carlo`. That creates drift risk across Rust, Python, and eventual WASM bindings, and it makes it harder for users to move between Rust and Python because similar concepts are exposed from different module shapes and with slightly different semantics.

The biggest current examples are:

- standalone path generation logic implemented directly in `finstack-py`
- binding-side recomputation of interval/statistical helpers
- binding-side duplication of deterministic RNG seed derivation
- Python-side hard-coded path-state labels that do not fully match core semantics
- Monte Carlo bindings living under `valuations.common.monte_carlo` instead of a top-level `monte_carlo` module that mirrors the Rust crate

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Canonical Python module | `finstack.monte_carlo` | Mirrors `finstack/monte_carlo` and makes cross-language navigation obvious |
| Binding crate layout | Add `finstack-py/src/monte_carlo/` | Aligns binding source structure with Rust crate structure |
| Backward compatibility | Keep `finstack.valuations.common.monte_carlo` as a compatibility re-export path for now | Avoids abrupt user breakage while establishing the new canonical path |
| Core ownership | Move MC semantics into `finstack/monte_carlo` | Keeps bindings thin and preserves WASM parity |
| Binding ownership | Python bindings do conversion, wrapper construction, registration, and error mapping only | Matches repo-wide binding policy |
| Result helper policy | If Python exposes a statistical helper, implement it in Rust core first and call it from bindings | Avoids duplicated math and drift |
| Path metadata policy | Python state labels should derive from core metadata where possible | Prevents process-specific mislabeling |

## Architecture

### Layer 1: Rust Core (`finstack/monte_carlo`)

`finstack/monte_carlo` becomes the sole owner of Monte Carlo behavior and semantics. This includes:

- standalone path generation entry points
- path capture behavior and sample-selection semantics
- per-path result semantics such as `SimulatedPath.final_value`
- deterministic RNG construction from string seeds
- any exposed estimate/confidence-interval helpers
- process/path metadata that bindings can surface directly

The goal is that Python and any future WASM bindings can delegate to the same underlying Rust API without reimplementing simulation logic or interpretation rules.

### Layer 2: Python Binding Module (`finstack-py/src/monte_carlo/`)

Add a new top-level binding module:

```text
finstack-py/src/monte_carlo/
  mod.rs
  discretization.rs
  engine.rs
  estimate.rs
  generator.rs
  params.rs
  paths.rs
  payoffs.rs
  processes.rs
  result.rs
  rng.rs
  time_grid.rs
  variance_reduction.rs
```

This module is the Python binding surface for the Rust Monte Carlo crate. Its responsibilities are limited to:

- Python argument extraction
- wrapping and unwrapping Rust types
- module registration, docstrings, and `__all__`
- error mapping via `core_to_py`
- Python-specific ergonomic adapters only when they do not introduce new domain semantics

No simulation loops, statistical calculations, or domain-specific decisions should remain here.

### Layer 3: Compatibility Re-exports

The old path:

```text
finstack.valuations.common.monte_carlo
```

should remain available as a compatibility layer for this pass, but it should stop being the canonical home of MC bindings. It should re-export the new top-level MC module rather than owning an independent implementation tree.

That gives the user-facing structure:

- canonical: `finstack.monte_carlo`
- compatibility: `finstack.valuations.common.monte_carlo`

## Concrete Changes

### 1. Add top-level Monte Carlo binding module

Update `finstack-py/src/lib.rs` to register a top-level `monte_carlo` submodule in the same style as `core`, `valuations`, `statements`, `portfolio`, and the other top-level domains.

Update Python-side package exports and stubs so the canonical import path is also top-level:

- `finstack-py/finstack/monte_carlo/__init__.pyi`
- any supporting `finstack-py/finstack/monte_carlo/*.pyi` files
- package re-export surfaces such as `finstack-py/finstack/__init__.pyi` or binding export helpers as needed

### 2. Collapse duplicate binding implementation

Move the current MC binding files out of `finstack-py/src/valuations/common/monte_carlo/` into `finstack-py/src/monte_carlo/`, or replace the old location with thin re-export wiring.

The new top-level module should be the only real binding implementation. The old valuations path should become aliasing/re-export glue only.

### 3. Move standalone path generation into Rust core

The current Python binding path generator should stop simulating paths directly.

Instead:

- add a Rust-core API in `finstack/monte_carlo` for standalone path generation
- return core-owned `PathDataset` / `SimulatedPath` / `PathPoint` values
- make the Python binding call that API and wrap the returned types

This is the most important architecture correction in the whole change.

### 4. Remove binding-side statistical logic

`PyMonteCarloResult` should not recompute confidence intervals or other statistical outputs in Python binding code.

Two acceptable end states:

1. expose only the statistics already stored by core, or
2. expose a core-owned helper for arbitrary-alpha intervals and call that helper from Python

Recommendation: keep the helper only if it is genuinely useful for both Rust and Python users. Otherwise expose the stored estimate values directly and keep the binding simpler.

### 5. Remove binding-side RNG seed derivation

`PyPhiloxRng::from_string()` should delegate to the Rust core implementation instead of duplicating the FNV-1a logic in binding code.

If the current Rust RNG type does not expose the numeric seed in a binding-friendly way, add a small core helper rather than replicating the hash in `finstack-py`.

### 6. Align path-state labels with core metadata

Python path/state inspection should not hard-code ambiguous labels like `variance` for a slot that can represent different state variables for different processes.

Preferred behavior:

- surface labels from core process metadata (`factor_names`, `state_var_keys`, or equivalent)
- keep convenience accessors only when their meaning is unambiguous
- preserve compatibility helpers carefully, but avoid presenting ambiguous labels as authoritative

## Public Surfaces To Keep In Sync

These surfaces all need to be updated together:

- `finstack-py/src/lib.rs`
- `finstack-py/src/monte_carlo/**`
- `finstack-py/src/valuations/common/**` compatibility wiring
- `finstack-py/finstack/monte_carlo/**` stubs
- `finstack-py/finstack/valuations/**` re-export stubs
- package export helpers such as `finstack-py/finstack/_binding_exports.py` if affected
- tests covering canonical imports and compatibility imports

## API Strategy

### Canonical path

Users should be guided toward:

```python
from finstack.monte_carlo import TimeGrid, PhiloxRng, MonteCarloPathGenerator
```

instead of:

```python
from finstack.valuations.common.monte_carlo import ...
```

### Compatibility policy

Existing `finstack.valuations.common.monte_carlo` imports should continue to work during this pass. The compatibility layer should be simple forwarding only. It should not accumulate its own logic, conversions, or semantics.

### Cleanup policy

Small Python API cleanups are acceptable where they materially improve cross-language alignment and reduce duplicated binding structure. Large behavioral redesigns are out of scope for this pass.

## Testing Strategy

1. Add a Rust-core regression test for standalone path generation so the simulation semantics are owned and verified in `finstack/monte_carlo`.
2. Add or update Python tests to verify:
   - canonical import path `finstack.monte_carlo`
   - compatibility import path still works
   - delegated path generation returns expected `PathDataset` semantics
   - path state labeling reflects core metadata correctly
   - string-seeded RNG behavior matches Rust core behavior
3. Run focused Rust and Python tests for the touched areas before claiming completion.
4. Run lints on edited files and fix any newly introduced issues.

## Migration Notes

- This change is intentionally structural, but it also corrects at least one semantic bug (`SimulatedPath.final_value` meaning in standalone generated paths).
- The compatibility re-export path reduces breakage risk, but examples, docs, and stubs should begin favoring `finstack.monte_carlo` immediately.
- The old valuations MC location should not be expanded further after this refactor; it should only forward to the new top-level module.

## Out Of Scope

- full redesign of the Monte Carlo public API across all languages
- removal of the compatibility import path in the same pass
- broad changes to valuation APIs unrelated to Monte Carlo
- WASM implementation work beyond keeping the architecture compatible
