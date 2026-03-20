# Finstack Core Documentation Audit

This audit tracks the highest-value documentation gaps identified while bringing
`finstack/core` up to the workspace documentation standard.

## Severity Guide

- `blocker`: incorrect or missing documentation that can mislead users or break
  rustdoc navigation
- `major`: public API documentation is present but incomplete for practical use
- `minor`: style, consistency, or reference gaps that should be cleaned up in
  the final pass

## Infrastructure

### Blockers

- `finstack/core/src/expr/context.rs`
  - `SimpleContext`
  - Broken intra-doc link to `CompiledExpr::eval`
- `finstack/core/src/market_data/term_structures/discount_curve_builder.rs`
  - `DiscountCurveBuilder::min_forward_tenor`
  - Broken intra-doc link to `DEFAULT_MIN_FORWARD_TENOR`
- `finstack/core/src/math/stats.rs`
  - `realized_variance`, `realized_variance_ohlc`
  - Broken intra-doc links to `Error::Validation`

### Majors

- Missing shared references file for stable rustdoc citations.
- Missing shared documentation standard for consistent rustdoc headings and
  finance-specific requirements.

## Module Facades

### Majors

- `finstack/core/src/factor_model/mod.rs`
  - Missing `//!` overview for the public module facade.
- `finstack/core/src/market_data/term_structures/mod.rs`
  - Trait names in the overview table drift from the real public API.
- `finstack/core/src/market_data/surfaces/mod.rs`
  - Facade overview under-describes FX-delta volatility surfaces.

### Minors

- Normalize `# Example` / `# Examples` and `# Reference` / `# References`.

## Dates

### Blockers

- `finstack/core/src/dates/schedule_iter.rs`
  - Broken or misleading references to `Frequency`; the public type is `Tenor`.
- `finstack/core/src/dates/calendar/mod.rs`
  - References `get_calendar`, which is not the public lookup API.

### Majors

- `finstack/core/src/dates/periods.rs`
  - `build_periods`, `build_fiscal_periods`, and `PeriodPlan` need fuller public
    contract docs and examples.
- `finstack/core/src/dates/calendar/registry.rs`
  - `CalendarId` and `CalendarRegistry` need clearer identifier and lifecycle
    semantics.
- `finstack/core/src/dates/fx.rs`
  - Module overview is too thin for joint-calendar FX settlement logic.
- `finstack/core/src/dates/daycount.rs`
  - `act_act_isma_year_fraction_with_reference_period` needs stronger input and
    convention guidance.

## Market Data

### Majors

- `finstack/core/src/market_data/mod.rs`
  - Re-exported `validate_knots` lacks public rustdoc.
- `finstack/core/src/market_data/context/getters.rs`
  - Typed getters are too terse about return types and error semantics.
- `finstack/core/src/market_data/bumps.rs`
  - Cross-cutting bump semantics and units need a clearer module narrative.
- `finstack/core/src/market_data/surfaces/vol_surface.rs`
  - `VolInterpolationMode` needs more explicit finance interpretation.
- `finstack/core/src/market_data/README.md`
  - Narrative drift from the implemented API (`apply_bumps` vs `bump`).

### Minors

- `finstack/core/src/market_data/mod.rs`
  - “Industry standards” section should use more specific references.
- `finstack/core/src/market_data/context/ops_bump.rs`
  - Duplicate rustdoc text suggests a cleanup pass is needed.

## Math, Cashflow, Expression, And Factor Model

### Blockers

- `finstack/core/src/cashflow/discounting.rs`
  - References `npv_constant`, which does not exist in the public API.

### Majors

- `finstack/core/src/factor_model/config.rs`
  - `FactorModelConfig`, `RiskMeasure`, and `PricingMode` need stronger
    behavioral and convention docs.
- `finstack/core/src/factor_model/definition.rs`
  - `MarketMapping::CurveBucketed` needs explicit weighting semantics.
- `finstack/core/src/factor_model/covariance.rs`
  - `FactorCovarianceMatrix` needs clearer units, validation, and unchecked
    constructor guidance.
- `finstack/core/src/factor_model/error.rs`
  - `UnmatchedPolicy` variants need precise runtime behavior.
- `finstack/core/src/expr/mod.rs`
  - Public expression semantics need a clearer overview of evaluation contracts.
- `finstack/core/src/expr/ast.rs`
  - `EvaluationResult::metadata` needs a public explanation.
- `finstack/core/src/math/integration.rs`
  - `adaptive_simpson` error docs point readers at the enum instead of the
    specific error variant.
- `finstack/core/src/cashflow/mod.rs`
  - Discounting conventions need a cleaner bridge between curve-based discounting
    and IRR/XIRR-style rate calculations.

## References To Add

- `docs/REFERENCES.md#isda-2006-definitions`
- `docs/REFERENCES.md#icma-rule-book`
- `docs/REFERENCES.md#andersen-piterbarg-interest-rate-modeling`
- `docs/REFERENCES.md#hagan-west-monotone-convex`
- `docs/REFERENCES.md#gatheral-volatility-surface`
- `docs/REFERENCES.md#hagan-2002-sabr`
- `docs/REFERENCES.md#heston-1993`
- `docs/REFERENCES.md#meucci-risk-and-asset-allocation`
- `docs/REFERENCES.md#mcneil-frey-embrechts-qrm`
