# finstack-statements-analytics

`finstack-statements-analytics` is the high-level analysis layer for `finstack-statements`. It adds scenario management, variance and sensitivity tooling, DCF-oriented corporate analysis, covenant and credit-context helpers, reporting, dependency introspection, runtime extensions, and build-time modeling templates.

This crate is designed for workflows where you already have, or are building, a `finstack-statements` model and want richer analysis on top of it rather than a lower-level evaluator API.

## Overview

Core capabilities:

- Unified corporate analysis that combines statement evaluation, DCF equity value, and per-instrument credit context.
- Scenario sets with parent chaining, deterministic overrides, and variance-style diffs.
- Sensitivity, tornado, variance, Monte Carlo result handling, goal seek, and forecast backtesting.
- Credit helpers for covenant forecasting and lender-style coverage metrics.
- Runtime extensions for corkscrew validation and credit scorecards.
- Build-time templates for roll-forward, vintage/cohort, and real-estate operating-statement models.
- Reporting and explainability utilities for tables, P&L summaries, credit assessment, and dependency tracing.

## Where It Fits

| Need | Crate |
|------|-------|
| Build and evaluate financial statement models | `finstack-statements` |
| Add high-level analysis, templates, reports, scenarios, and extensions to statement models | `finstack-statements-analytics` |
| Price instruments, run covenant engines, and perform market-based valuation work | `finstack-valuations` |
| Use dates, money, curves, and other foundational types | `finstack-core` |

## Feature Flags

| Feature | Effect | Operational note |
|---------|--------|------------------|
| `default` | Core analytics runtime | Suitable for most statement-analysis flows |
| `parallel` | Forwards parallel Monte Carlo support from `finstack-statements` | Useful when scenario or Monte Carlo workflows need Rayon-backed execution |

Recommended verification matrix:

```bash
cargo test -p finstack-statements-analytics
```

## Installation

Within the Finstack workspace:

```toml
[dependencies]
finstack-core = { path = "../core" }
finstack-statements = { path = "../statements" }
finstack-statements-analytics = { path = "../statements-analytics" }
finstack-valuations = { path = "../valuations" }
```

Import path:

```rust
use finstack_statements_analytics::analysis::*;
use finstack_statements_analytics::extensions::*;
use finstack_statements_analytics::templates::*;
```

For a narrower surface, prefer importing specific types:

```rust
use finstack_statements_analytics::analysis::{CorporateAnalysis, CorporateAnalysisBuilder};
```

## Quick Start

The highest-level workflow is `CorporateAnalysisBuilder`, which evaluates a statement model once and optionally layers in DCF equity valuation plus credit-context metrics.

```rust
use finstack_core::dates::PeriodId;
use finstack_statements::builder::ModelBuilder;
use finstack_statements::types::AmountOrScalar;
use finstack_statements_analytics::analysis::CorporateAnalysisBuilder;
use finstack_valuations::instruments::equity::dcf_equity::TerminalValueSpec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = ModelBuilder::new("lbo-demo")
        .periods("2025Q1..Q4", None)?
        .value(
            "revenue",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(10_000_000.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(10_500_000.0)),
                (PeriodId::quarter(2025, 3), AmountOrScalar::scalar(11_000_000.0)),
                (PeriodId::quarter(2025, 4), AmountOrScalar::scalar(11_500_000.0)),
            ],
        )
        .compute("ebitda", "revenue * 0.25")?
        .compute("ufcf", "ebitda * 0.6")?
        .with_meta("currency", serde_json::json!("USD"))
        .build()?;

    let analysis = CorporateAnalysisBuilder::new(model)
        .dcf(0.10, TerminalValueSpec::GordonGrowth { growth_rate: 0.02 })
        .net_debt_override(20_000_000.0)
        .coverage_node("ebitda")
        .analyze()?;

    println!(
        "EBITDA Q1: {:?}",
        analysis.statement.get("ebitda", &PeriodId::quarter(2025, 1))
    );

    if let Some(equity) = &analysis.equity {
        println!("Equity value: {}", equity.equity_value);
    }

    for (instrument_id, credit) in &analysis.credit {
        println!(
            "{instrument_id} min DSCR: {:?}",
            credit.coverage.dscr_min
        );
    }

    Ok(())
}
```

## Core Workflows

### 1. Scenario Sets and Variance

`ScenarioSet` gives you a named registry of cases, optional parent inheritance, and deterministic model overrides. This is useful for management/base/downside/stress workflows and for lender-vs-sponsor comparisons.

```rust
use finstack_core::dates::PeriodId;
use finstack_statements::builder::ModelBuilder;
use finstack_statements::types::{AmountOrScalar, FinancialModelSpec};
use finstack_statements_analytics::analysis::{ScenarioDefinition, ScenarioSet};
use indexmap::IndexMap;

fn build_model() -> Result<FinancialModelSpec, Box<dyn std::error::Error>> {
    Ok(ModelBuilder::new("scenario-demo")
        .periods("2025Q1..Q2", None)?
        .value(
            "revenue",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100_000.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(100_000.0)),
            ],
        )
        .compute("cogs", "revenue * 0.4")?
        .compute("ebitda", "revenue - cogs")?
        .build()?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = build_model()?;
    let mut scenarios = IndexMap::new();

    scenarios.insert(
        "base".to_string(),
        ScenarioDefinition {
            model_id: Some(model.id.clone()),
            parent: None,
            overrides: IndexMap::new(),
        },
    );

    let mut downside_overrides = IndexMap::new();
    downside_overrides.insert("revenue".to_string(), 90_000.0);
    scenarios.insert(
        "downside".to_string(),
        ScenarioDefinition {
            model_id: Some(model.id.clone()),
            parent: Some("base".to_string()),
            overrides: downside_overrides,
        },
    );

    let set = ScenarioSet { scenarios };
    let results = set.evaluate_all(&model)?;

    let diff = set.diff(
        &results,
        "base",
        "downside",
        &["revenue".to_string(), "ebitda".to_string()],
        &[PeriodId::quarter(2025, 1)],
    )?;

    println!("Variance rows: {}", diff.variance.rows.len());
    Ok(())
}
```

Related APIs:

- `VarianceAnalyzer` for direct baseline-vs-comparison analysis on two `StatementResult`.
- `SensitivityAnalyzer`, `SensitivityConfig`, and `generate_tornado_entries` for single-driver sweeps and tornado outputs.
- `MonteCarloConfig`, `MonteCarloResults`, and `PercentileSeries` for Monte Carlo result handling aligned with the statements engine.

### 2. Credit and Covenant Analysis

The credit surface is split into two complementary layers:

- `compute_credit_context()` derives lender-style coverage and leverage metrics from statement results plus capital-structure cashflows.
- `forecast_breaches()` bridges evaluated statement results into the covenant engine from `finstack-valuations`.

Useful types and entry points:

- `CreditContextMetrics`
- `compute_credit_context()`
- `forecast_breaches()`
- `analysis::credit::covenants::forecast_covenant()`
- `analysis::credit::covenants::forecast_covenants()`

Covenant forecast outputs can also be exported through `analysis::credit::covenants::to_table()`.

### 3. Build-Time Templates

Templates extend `ModelBuilder` with higher-level construction helpers. They mutate the graph at build time; they do not add bespoke runtime evaluator behavior.

```rust
use finstack_core::dates::PeriodId;
use finstack_statements::prelude::*;
use finstack_statements_analytics::templates::{TemplatesExtension, VintageExtension};

fn main() -> Result<()> {
    let decay_curve = vec![1.0, 0.8, 0.5, 0.0];

    let model = ModelBuilder::new("saas-model")
        .periods("2025Q1..2025Q4", None)?
        .value_scalar(
            "new_arr",
            &[
                (PeriodId::quarter(2025, 1), 100.0),
                (PeriodId::quarter(2025, 2), 120.0),
                (PeriodId::quarter(2025, 3), 140.0),
                (PeriodId::quarter(2025, 4), 160.0),
            ],
        )
        .value_scalar(
            "churn_arr",
            &[
                (PeriodId::quarter(2025, 1), 10.0),
                (PeriodId::quarter(2025, 2), 12.0),
                (PeriodId::quarter(2025, 3), 14.0),
                (PeriodId::quarter(2025, 4), 16.0),
            ],
        )
        .add_roll_forward("arr", &["new_arr"], &["churn_arr"])?
        .add_vintage_buildup("cohort_revenue", "new_arr", &decay_curve)?
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model)?;
    println!("{:?}", results.get("arr_end", &PeriodId::quarter(2025, 2)));
    Ok(())
}
```

Available template traits:

- `TemplatesExtension` for generic roll-forward structures.
- `VintageExtension` for cohort/vintage buildup models.
- `RealEstateExtension` for NOI, NCF, rent-roll, and full property-operating-statement builders.

The real-estate template surface is the richest in the crate and includes:

- `add_noi_buildup()`
- `add_ncf_buildup()`
- `add_rent_roll()`
- `add_property_operating_statement()`

### 4. Runtime Extensions

The crate ships with two production-oriented analytics extensions, each callable directly via inherent methods:

- `CorkscrewExtension` for roll-forward validation and balance-sheet articulation checks.
- `CreditScorecardExtension` for weighted metric scoring and rating assignment.

Example with direct method dispatch:

```rust
use finstack_core::dates::PeriodId;
use finstack_statements::builder::ModelBuilder;
use finstack_statements::evaluator::Evaluator;
use finstack_statements::types::AmountOrScalar;
use finstack_statements_analytics::extensions::{
    AccountType, CorkscrewAccount, CorkscrewConfig, CorkscrewExtension,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let q1 = PeriodId::quarter(2025, 1);
    let q2 = PeriodId::quarter(2025, 2);

    let model = ModelBuilder::new("corkscrew-demo")
        .periods("2025Q1..Q2", None)?
        .value(
            "cash",
            &[
                (q1, AmountOrScalar::scalar(100.0)),
                (q2, AmountOrScalar::scalar(125.0)),
            ],
        )
        .value("cash_change", &[(q2, AmountOrScalar::scalar(25.0))])
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model)?;

    let config = CorkscrewConfig {
        accounts: vec![CorkscrewAccount {
            node_id: "cash".into(),
            account_type: AccountType::Asset,
            changes: vec!["cash_change".into()],
            beginning_balance_node: None,
        }],
        tolerance: 0.01,
        fail_on_error: false,
    };

    let mut extension = CorkscrewExtension::with_config(config);
    let report = extension.execute(&model, &results)?;
    println!("{}", report.message);
    Ok(())
}
```

### 5. Reporting, Explainability, and Utilities

Useful non-orchestrator APIs include:

- `goal_seek()` to solve for a model driver that hits a target value in a specific period.
- `backtest_forecast()` and `ForecastMetrics` for forecast-accuracy measurement.
- `DependencyTracer`, `DependencyTree`, `FormulaExplainer`, `render_tree_ascii()`, and `render_tree_detailed()` for explainability and dependency inspection.
- `PLSummaryReport`, `CreditAssessmentReport`, `TableBuilder`, and the `Report` trait for human-readable outputs.

Small example:

```rust
use finstack_statements_analytics::analysis::backtest_forecast;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metrics = backtest_forecast(
        &[100.0, 110.0, 105.0, 115.0],
        &[98.0, 112.0, 104.0, 116.0],
    )?;

    println!("{}", metrics.summary());
    Ok(())
}
```

## Module Guide

| Module | Purpose | Key Types / Functions |
|--------|---------|------------------------|
| `analysis::valuation` | DCF valuation and unified corporate pipeline | `evaluate_dcf_with_market`, `CorporateAnalysisBuilder`, `CorporateAnalysis` |
| `analysis::credit` | Coverage, leverage, and covenant forecasting | `compute_credit_context`, `CreditContextMetrics`, `forecast_breaches` |
| `analysis::scenarios` | Scenario sets, sensitivities, variance, Monte Carlo envelopes | `ScenarioSet`, `SensitivityAnalyzer`, `VarianceAnalyzer`, `MonteCarloResults` |
| `analysis::goal_seek` | Root-finding on model drivers | `goal_seek` |
| `analysis::backtesting` | Forecast accuracy metrics | `backtest_forecast`, `ForecastMetrics` |
| `analysis::introspection` | Dependency tracing and formula explanation | `DependencyTracer`, `DependencyTree`, `FormulaExplainer` |
| `analysis::reports` | Human-readable tables and report formatting | `TableBuilder`, `PLSummaryReport`, `CreditAssessmentReport` |
| `extensions` | Runtime extension implementations | `CorkscrewExtension`, `CreditScorecardExtension` |
| `templates` | Build-time model-construction helpers | `TemplatesExtension`, `VintageExtension`, `RealEstateExtension` |
| `prelude` | Common re-exports | Corporate/scenario/template/extension convenience imports |

## Operational Notes

- Ratios are returned as plain scalars. For example, `2.0` means `2.0x`, while `0.40` means `40%`.
- Percentage-style inputs follow the crate-wide decimal convention. For example, `0.10` means `10%`.
- `ScenarioDefinition.overrides` is interpreted as `node_id -> scalar`, broadcast across model periods as explicit values. In models with an actual-history cutoff, historical actuals are preserved while forecast periods are overridden.
- Template helpers are build-time graph builders. For runtime validation of roll-forward structures, use `CorkscrewExtension`.
- `CreditScorecardExtension` supports S&P, Moody's, and Fitch aliases. The shipped rating scales are embedded at compile time; callers do not need a runtime ratings data directory.
- The `parallel` feature enables parallel statement Monte Carlo workflows via the underlying statements crate; outputs remain compatible with the analytics APIs here.

## Common Import Patterns

Most users start with one of these:

```rust
use finstack_statements_analytics::analysis::{
    CorporateAnalysisBuilder, ScenarioDefinition, ScenarioSet, VarianceAnalyzer,
};
```

```rust
use finstack_statements_analytics::templates::{
    RealEstateExtension, TemplatesExtension, VintageExtension,
};
```

## Verification

Primary crate verification:

```bash
cargo test -p finstack-statements-analytics
cargo doc -p finstack-statements-analytics --open
```

## See Also

- `finstack/statements/README.md`
- `finstack/valuations/README.md`
- `finstack/core/README.md`
