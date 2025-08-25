## Finstack docs — PRDs, TDDs, and examples

This index helps you navigate the new documentation set: Product Requirements Documents (PRDs), Technical Design Documents (TDDs), and usage examples.

### Quick links (by module)

- **Overall**: [PRD](01_overall/01_overall_prd.md) • [TDD](01_overall/01_overall_tdd.md)
- **Core (`/core`)**: [PRD](02_core/02_core_prd.md) • [TDD](02_core/02_core_tdd.md)
- **Valuations (`/valuations`)**: [PRD](03_valuations/03_valuations_prd.md) • [TDD](03_valuations/03_valuations_tdd.md)
- **Statements (`/statements`)**: [PRD](04_statements/04_statements_prd.md) • [TDD](04_statements/04_statements_tdd.md)
- **Scenarios (`/scenarios`)**: [PRD](05_scenarios/05_scenarios_prd.md) • [TDD](05_scenarios/05_scenarios_tdd.md)
- **Analysis (`/analysis`)**: [PRD](06_analysis/06_analysis_prd.md) • [TDD](06_analysis/06_analysis_tdd.md)
- **Portfolio (`/portfolio`)**: [PRD](07_portfolio/07_portfolio_prd.md) • [TDD](07_portfolio/07_portfolio_tdd.md)
- **Bindings (Python/WASM)**: [PRD](08_bindings/08_bindings_prd.md) • [TDD](08_bindings/08_bindings_tdd.md)
  - Examples: [Python](08_bindings/python_bindings_example.md) • [JS/WASM](08_bindings/js_bindings_example.md)
- **Structured Credit (feature‑gated)**: [PRD](09_structured_credit/09_structured_credit_prd.md) • [TDD](09_structured_credit/09_structured_credit_tdd.md)
- **Global Config & Rounding**: [PRD](10_config/10_config_prd.md) • [TDD](10_config/10_config_tdd.md)
- **Caching & Hashing**: [PRD](11_caching/11_caching_prd.md) • [TDD](11_caching/11_caching_tdd.md)

### Suggested reading order

1) **Overall** (context, invariants, release plan)
2) **Core** (types, time, FX, expression engine, rounding/config)
3) **Statements** and **Valuations** (primary user features)
4) **Scenarios** and **Portfolio** (composition/orchestration)
5) **Analysis** (plugins, sensitivities, waterfalls)
6) **Bindings** (Python/WASM parity) and **Config/Caching** (cross‑cutting)

### Conventions you’ll see across docs

- **Determinism**: Decimal mode; serial equals parallel.
- **Currency safety**: No implicit cross‑currency math; FX is explicit and stamped in results.
- **Stable wire formats**: Serde names are versioned and strict on inbound.
- **DataFrames first**: Polars is the canonical time‑series surface; Python parity via bindings.
- **Rounding context**: Results include a `RoundingContext` per the global config.


