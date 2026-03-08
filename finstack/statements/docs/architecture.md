# Architecture

## Runtime Layers

1. `builder` and `types` define model authoring and serialized model structure.
2. `dsl` parses and compiles formulas.
3. `evaluator` owns the execution runtime, Monte Carlo types, and capital-structure runtime helpers.
4. `analysis` is now a consumer of evaluator outputs rather than a dependency of the evaluator core.

## Capital Structure Boundary

- `evaluator/engine.rs` handles core model evaluation flow.
- `evaluator/capital_structure_runtime.rs` contains the capital-structure-specific orchestration used by the evaluator.
- `capital_structure/*` contains domain logic for instrument construction, contractual flow extraction, and waterfall mechanics.

## Monte Carlo Boundary

- `evaluator/monte_carlo.rs` owns Monte Carlo config, result types, and aggregation helpers.
- `analysis/monte_carlo.rs` is a compatibility facade that re-exports evaluator-owned Monte Carlo types.

## Design Rules

- Fail closed at system boundaries instead of warning and defaulting for material valuation inputs.
- Keep optional domain logic out of the evaluator core unless the evaluator is the only coherent owner.
- Prefer compile-time embedded assets for built-ins over runtime source-tree discovery.
