# `finstack-statements` Documentation Checklist

This checklist captures the public documentation surface for `finstack-statements`
and highlights the areas that need the strongest ongoing documentation review.

## Canonical Entry Points

- Crate root: `src/lib.rs`
- Common imports: `src/prelude.rs`
- Public data model: `src/types/`
- Builder API: `src/builder/`
- Evaluation/runtime API: `src/evaluator/`
- Analysis API: `src/analysis/`
- Capital structure API: `src/capital_structure/`
- Templates API: `src/templates/`
- Forecast API: `src/forecast/`
- Registry API: `src/registry/`
- Extension API: `src/extensions/`
- DSL API: `src/dsl/`
- Error API: `src/error.rs`

## Public Module Checklist

| Module | Primary entry points | Review status |
| --- | --- | --- |
| `adjustments` | `NormalizationEngine`, `NormalizationConfig`, `NormalizationResult` | High priority: module overview and conventions |
| `analysis` | `evaluate_dcf_with_market`, `compute_credit_context`, `forecast_breaches`, `ScenarioSet`, `CorporateAnalysisBuilder` | High priority: finance-facing assumptions and references |
| `builder` | `ModelBuilder`, `MixedNodeBuilder` | Mostly strong; maintain examples |
| `capital_structure` | `calculate_period_flows`, `execute_waterfall`, `CapitalStructureCashflows`, `WaterfallSpec` | High priority: sign conventions, timing, and market-practice references |
| `dsl` | `parse_formula`, `compile`, `parse_and_compile` | Moderate priority: keep examples and grammar expectations clear |
| `error` | `Error`, `Result` | Low priority: already documented |
| `evaluator` | `Evaluator`, `StatementResult`, `to_polars_*` | Medium priority: facade guidance and feature-gated exports |
| `extensions` | `Extension`, `ExtensionRegistry`, corkscrew and scorecard types | Moderate priority |
| `forecast` | `apply_forecast`, `normal_forecast`, `lognormal_forecast`, deterministic/time-series helpers | Medium priority: parameter conventions and references |
| `prelude` | ergonomic re-exports | Medium priority: import-boundary clarity |
| `registry` | `Registry`, `MetricRegistry`, `MetricDefinition` | Mostly strong |
| `templates` | `add_roll_forward`, `add_vintage_buildup`, `add_rent_roll`, `add_property_operating_statement` | High priority: real-estate templates and examples |
| `types` | `FinancialModelSpec`, `NodeSpec`, `ForecastSpec`, `AmountOrScalar` | Mostly strong |

## High-Priority Symbols

- `analysis::evaluate_dcf_with_market`
- `analysis::compute_credit_context`
- `analysis::forecast_covenant`
- `analysis::forecast_covenants`
- `analysis::forecast_breaches`
- `analysis::ScenarioSet`
- `analysis::CorporateAnalysisBuilder`
- `capital_structure::calculate_period_flows`
- `capital_structure::execute_waterfall`
- `capital_structure::CapitalStructureCashflows`
- `capital_structure::WaterfallSpec`
- `forecast::normal_forecast`
- `forecast::lognormal_forecast`
- `templates::add_rent_roll`
- `templates::add_property_operating_statement`
- `templates::LeaseSpecV2`

## Documentation Expectations

- Every public module should explain what it covers, where users should start,
  and any important conventions.
- Finance-facing APIs should state units, sign conventions, timing assumptions,
  and model-period behavior explicitly.
- Numerical and market-practice APIs should include `# References` links into
  `docs/REFERENCES.md` when a canonical source exists.
- Feature-gated APIs should document the required feature and output schema.
