# Greenfield Rust-Canonical Binding Rewrite Design Spec

**Date**: 2026-04-10
**Status**: Revised after design change
**Scope**: Python bindings, WASM bindings, parity contract, audit tooling, tests, docs, and package entrypoints

## Decision

We will not refactor the current Python and WASM bindings into shape.

We will instead do a greenfield rewrite with these rules:

- Rust public crate and module structure is the only canonical API design.
- No compatibility paths, forwarding modules, alias exports, or deprecation shims will be added.
- The existing Python and WASM binding code becomes reference material only through git history and `.audit/` artifacts.
- New bindings will be built under clean internal roots and exposed through a clean public root.
- Legacy binding trees will be disconnected from the build as soon as the new root is in place, then deleted once the new domains land.

## Why Greenfield

The current binding trees have three kinds of debt at once:

- wrong public topology
- binding-side logic leakage
- audit and test infrastructure coupled to the wrong topology

Trying to preserve those surfaces would slow the rewrite down more than rebuilding them:

- every rename would need a forwarding layer
- every layout fix would need compatibility rules
- every audit would need to distinguish intended APIs from transitional shims

Because the project is still early, the fastest path to good bindings is to stop preserving the wrong shape.

## Goals

- Ship Python and WASM bindings that mirror the Rust crate structure directly.
- Make the public binding roots small, explicit, and predictable.
- Keep all pricing, model, calibration, margin, and simulation logic in Rust crates.
- Replace dynamic export wiring with explicit contract-driven exports.
- Replace the flat WASM package surface with a namespaced facade that follows Rust crate ownership.
- Make parity tooling compare the new canonical contract instead of the current binding inventory.

## Non-Goals

- Preserve any current Python or WASM import path that is not canonical to Rust.
- Keep root convenience exports like `finstack.Currency` or `finstack.build_periods`.
- Keep `finstack.valuations.common.monte_carlo` or similar legacy placements alive.
- Preserve the current flat `finstack-wasm` package shape.
- Reuse the existing binding module layout if it fights the new structure.

## Canonical Surface

### Rust Umbrella First

The top-level binding roots must mirror the Rust umbrella crate `finstack/src/lib.rs`.

Current umbrella exports already include:

- `core`
- `analytics`
- `margin`
- `valuations`
- `statements`
- `statements_analytics`
- `portfolio`
- `scenarios`

Before the binding rewrite begins, the umbrella crate must also promote:

- `correlation`
- `monte_carlo`

That yields the target top-level domain set:

- `core`
- `analytics`
- `margin`
- `valuations`
- `statements`
- `statements_analytics`
- `portfolio`
- `scenarios`
- `correlation`
- `monte_carlo`

Phase 1 will mirror crate roots and crate modules only. Bridge exports such as `covenants` can be added after crate-level parity is complete.

### Python Public API

The public Python package root `finstack` will export only the umbrella domains:

```python
__all__ = (
    "core",
    "analytics",
    "margin",
    "valuations",
    "statements",
    "statements_analytics",
    "portfolio",
    "scenarios",
    "correlation",
    "monte_carlo",
)
```

Rules:

- no leaf conveniences at `finstack.*` unless Rust exports them at the umbrella root
- no legacy aliases
- every crate gets its own top-level package
- every public Rust module that belongs in bindings is exposed at the matching crate path
- if two different Rust crates both expose similarly named modules, both binding paths exist
- type and function names in Python bindings match their Rust names exactly (e.g. Rust `sharpe` stays `sharpe`, not `sharpe_ratio`; Rust `Date` stays `Date`, not `FsDate`)

Examples:

- `finstack.margin` and `finstack.valuations.margin` both exist if both Rust crates expose them
- `finstack.monte_carlo` and `finstack.valuations.lsmc` both exist because they are different Rust owners
- `finstack.statements_analytics` is first-class and is not folded into `finstack.statements`

### WASM Public API

The public npm API will no longer be the raw flat `wasm-bindgen` output.

Instead:

- `pkg/finstack_wasm.js` remains an internal generated artifact
- the published package entrypoint becomes a hand-written JS/TS facade
- the facade exports namespaced crate domains mirroring Rust umbrella exports

Target shape:

```ts
import init, { core, analytics, margin, valuations, statements, statements_analytics, portfolio, scenarios, correlation, monte_carlo } from "finstack-wasm";
```

Rules:

- no public flat-root export surface
- namespace objects are lower snake or lower camel at the package level only for JS ergonomics; ownership must still mirror Rust crates exactly
- types stay `PascalCase`
- functions stay `camelCase`
- the facade owns package shape; Rust owns semantics
- binding-specific name exceptions are allowed only to avoid host-language collisions (e.g. WASM exports `FsDate` instead of `Date` because JavaScript has a built-in `Date`); each exception must be documented in the contract

## Binding Architecture

### Python Internal Architecture

Create a clean internal binding tree:

```text
finstack-py/src/bindings/
  mod.rs
  core/
  analytics/
  margin/
  valuations/
  statements/
  statements_analytics/
  portfolio/
  scenarios/
  correlation/
  monte_carlo/
```

Design rules:

- `finstack-py/src/lib.rs` becomes thin and delegates to `bindings::register_root`
- each crate binding root is responsible only for that crate
- explicit `__all__` export lists only
- no `dir()`-driven export discovery
- no package wiring based on `globals().update()` over arbitrary Rust module contents
- shared helper code is allowed only for explicit registration and error mapping, not for implicit export discovery

The Python package tree under `finstack-py/finstack/` will be rebuilt to mirror the same crate roots. Static typing stubs are derived from the new contract and binding code, not treated as the source of truth.

### WASM Internal Architecture

Create a clean Rust-side wrapper tree:

```text
finstack-wasm/src/api/
  mod.rs
  core/
  analytics/
  margin/
  valuations/
  statements/
  statements_analytics/
  portfolio/
  scenarios/
  correlation/
  monte_carlo/
```

And a clean JS/TS facade:

```text
finstack-wasm/
  index.js
  index.d.ts
  exports/
    core.js
    analytics.js
    margin.js
    valuations.js
    statements.js
    statements_analytics.js
    portfolio.js
    scenarios.js
    correlation.js
    monte_carlo.js
```

Design rules:

- `finstack-wasm/src/lib.rs` reexports only the new `api` tree
- `package.json` points `main`, `types`, and `exports["."]` to the facade, not `pkg/finstack_wasm.js`
- the facade groups raw bindgen exports into crate namespaces
- the raw generated package is internal and not part of the supported public API

## Legacy Code Policy

The old binding trees are not part of the target design.

Operational rules:

- once the new public root is wired, old modules stop being publicly registered
- old source files may temporarily remain on disk while their replacements are being built
- old source files are deleted as soon as the replacement crate domain passes tests
- no new code may be added to the legacy trees
- no new tests may target legacy paths

This is not a migration program. It is a replacement program.

## Contract and Tooling

### Tracked Contract

The tracked contract moves to repo root:

`parity_contract.toml`

It defines only the canonical surface:

- umbrella exports
- crate names
- crate public modules in scope for bindings
- canonical Python package/module paths
- canonical WASM namespace paths

It does not define:

- aliases
- compatibility paths
- deprecation metadata

Illustrative shape:

```toml
[meta]
version = "3.0.0"
canonical_language = "rust"
umbrella_crate = "finstack"
umbrella_lib = "finstack/src/lib.rs"

[crates.monte_carlo]
rust_crate = "finstack-monte-carlo"
rust_umbrella = "finstack::monte_carlo"
python_package = "finstack.monte_carlo"
wasm_namespace = "monte_carlo"
status = "exists"

[crates.statements_analytics]
rust_crate = "finstack-statements-analytics"
rust_umbrella = "finstack::statements_analytics"
python_package = "finstack.statements_analytics"
wasm_namespace = "statements_analytics"
status = "exists"
```

### Derived Artifacts

Everything derived stays under `.audit/`:

- Rust public export manifests
- Python export manifests
- WASM namespace manifests
- parity reports

No audit script writes tracked repo files except when explicitly regenerating contract-driven source artifacts under a dedicated generation step.

### Audit Strategy

Audit layers become:

1. **Contract validation**
   - does the umbrella crate expose the required domains?
   - do Python and WASM expose the required namespaces?

2. **Symbol validation**
   - do crate roots and public modules expose the required symbols?

3. **Method validation**
   - do shared types expose the required constructors, methods, and serialization hooks?

4. **Behavior validation**
   - do Python and WASM produce the same results as Rust for the selected golden cases?

Raw "count of types only in X" reports are no longer treated as the implementation backlog.

## Rewrite Sequencing

### Stage 0: Contract and Umbrella

- move the contract to repo root
- promote `correlation` and `monte_carlo` into the umbrella crate
- make the contract match the real umbrella surface exactly

### Stage 1: New Public Roots

- Python root exports only the new crate domains
- WASM package exports only the new facade namespaces
- legacy roots are disconnected from public registration

### Stage 2: Foundation Crates

Rebuild these first because the rest depend on them:

- `core`
- `analytics`
- `correlation`

### Stage 3: Model and Infrastructure Crates

Rebuild these next:

- `monte_carlo`
- `margin`

### Stage 4: Product Crates

Rebuild these last:

- `valuations`
- `statements`
- `statements_analytics`
- `portfolio`
- `scenarios`

### Stage 5: Cleanup

- delete all legacy wrapper trees
- delete legacy tests that target old paths
- delete or rewrite tooling that assumes the old topology
- update docs and examples to the new package shape

## Domain-Specific Decisions

### `statements_analytics`

In the greenfield rewrite, `statements_analytics` is its own top-level binding domain because Rust already exposes it that way.

### `margin` vs `valuations.margin`

Both paths are valid if both Rust crates expose them. The binding rewrite does not collapse them into one domain.

### `monte_carlo` vs `valuations.lsmc`

Both are valid and separate. `monte_carlo` belongs to the Monte Carlo crate. `valuations.lsmc` belongs to the valuations crate.

### Binding Name Exceptions

Bindings use Rust names by default. Exceptions are allowed only when the Rust name collides with a host-language built-in:

| Rust type | Python binding | WASM/JS binding | Reason |
|-----------|---------------|-----------------|--------|
| `Date` | `Date` | `FsDate` | JS built-in `Date` collision |

No other renames are permitted. In particular, `sharpe` stays `sharpe` (not `sharpe_ratio`), `max_drawdown` stays `max_drawdown`, and `VmCalculator` stays `VmCalculator`.

### Root Convenience Exports

The rewrite removes them. If a root convenience is desirable later, it must first exist at the Rust umbrella root.

## Testing Strategy

### Contract Tests

- top-level namespace tests for Python and WASM
- umbrella export tests in Rust

### Crate Smoke Tests

For each crate domain:

- one import/namespace test
- one representative constructor/build test
- one representative runtime behavior test

### Golden Behavior Tests

Selected cross-language cases must exist for:

- core dates/money/market data
- analytics
- monte_carlo
- margin
- valuations
- statements

No test coverage budget will be spent preserving old paths.

## Success Criteria

The rewrite is successful when all of the following are true:

- the public Python root mirrors the Rust umbrella domains exactly
- the public npm root mirrors the Rust umbrella domains exactly through the facade
- no compatibility modules remain
- no legacy root conveniences remain
- `statements_analytics` is top-level in both Python and WASM
- old binding trees are deleted from the repository
- parity tooling compares the new contract instead of the old inventory
- CI enforces the greenfield contract and selected behavior parity
