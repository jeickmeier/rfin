# PR-08: Breadth Expansion – Analysis & Scenarios

## Summary

Expand analytical surfaces and scenario tooling across Statements, Portfolio, and Valuations. Implement Monte Carlo views, variance analysis, portfolio attribution and optimization surfaces, and the complete Scenarios domain (builder, library, execution, horizon grid).

## Background & Motivation

- Implements **Phase 8: Breadth Expansion – Analysis & Scenarios** from `UI_KIT_ROADMAP.md`.
- Leverages designs from `UI_KIT_DOMAINS.md` (Sections A–E: Valuations, Portfolio, Statements, Market, Scenarios).
- Builds on domain slices from PR-03–PR-05 and editor infrastructure from PR-06.

## Scope

### In Scope

- Statements analysis surfaces:
  - `MonteCarloConfigEditor` and `MonteCarloResultsView`.
  - `VarianceBridgeView` and `VarianceBridgeChart`.
  - `DependencyTreeViewer` and `FormulaExplainPanel` (full implementations).
  - `CapitalStructurePanel` with schedule and waterfall views.
- Portfolio analysis surfaces:
  - `PortfolioAttributionView` and supporting factor breakdown components.
  - `PortfolioOptimizerView`, `ConstraintViolationsPanel`, `EfficientFrontierChart`, `TradeProposalGrid`, `OptimizationResultView`.
  - `RiskHeatmap` with canvas/virtualization-aware rendering.
- Scenarios domain:
  - `ScenarioBuilder`, `ScenarioLibrary`, `ScenarioExecutionPanel`, `HorizonScenarioGrid`.
  - Operation editors for key scenario operations.

### Out of Scope

- Final performance tuning and accessibility passes (PR-10).

## Design & Implementation Details

### 1. Statements Analysis

- Implement Monte Carlo and variance analysis components under `domains/statements/analysis/`:
  - `MonteCarloConfigEditor` for configuring distributions per parameter.
  - `MonteCarloResultsView` for histograms, fan charts, and percentile tables.
  - `VarianceBridgeView` and `VarianceBridgeChart` for scenario-to-scenario variance analysis.
- Integrate with unified worker APIs for Monte Carlo and variance analysis as defined in the statements crate.
- Implement `DependencyTreeViewer` and `FormulaExplainPanel` using data from `DependencyTracer`, `FormulaExplainer`, etc. (per `UI_KIT_DOMAINS.md`).

### 2. Portfolio Analysis & Optimization

- Implement `PortfolioAttributionView`:
  - Visualizes portfolio-level P&L attribution by factor (rates, credit, inflation, vol, FX, carry, residual).
  - Integrates with valuations attribution APIs.
- Implement optimization surfaces under `domains/portfolio/optimization/`:
  - `PortfolioOptimizerView` – central optimization UI.
  - `ConstraintViolationsPanel` – shows which constraints bind.
  - `EfficientFrontierChart` – Recharts-based risk/return scatter.
  - `TradeProposalGrid` – lists proposed trades from optimization results.
  - `OptimizationResultView` – compares pre/post portfolio metrics.
- Implement `RiskHeatmap` using TanStack Table for smaller grids and canvas for large ones, following thresholds from `UI_KIT_PERFORMANCE_AND_VIRTUALIZATION.md`.

### 3. Scenarios Domain

- Under `src/domains/scenarios/` add:
  - `ScenarioBuilder.tsx` – editor for `ScenarioSpec` and ordered `OperationSpec` list.
  - `ScenarioLibrary.tsx` – CRUD for named scenarios with tagging and priority.
  - `ScenarioExecutionPanel.tsx` – applies scenarios to current engine state and displays `ApplicationReport`.
  - `HorizonScenarioGrid.tsx` – matrix of horizons vs metrics for horizon scenarios.
  - Operation editors for key types:
    - `CurveShockEditor`.
    - `EquityShockEditor`.
    - `VolShockEditor`.
    - `FxShockEditor`.
    - `StatementShockEditor`.
    - `TimeRollEditor`.
- Integrate Scenarios with unified worker APIs based on `finstack-scenarios` crate types.

### 4. GenUI & Data Binding

- Register new analysis and scenarios components in `ComponentRegistry` with clear descriptions and example props.
- Extend `DataBindingsSchema` usage to support scenario-related bindings (e.g., metric paths for scenario comparisons).
- Provide example dashboards combining:
  - Portfolio attribution + optimization.
  - Statements variance analysis + Monte Carlo.
  - Cross-domain scenario impact dashboards.

### 5. Testing & Validation

- Unit tests:
  - Scenario builder validation logic and JSON serialization.
  - Optimization result parsing and constraint violation detection.
- Integration tests:
  - Scenario creation, execution, and impact visualization on demo portfolios/statements.
  - Portfolio optimization run with visible pre/post comparison.
- Regression tests:
  - Golden result snapshots for selected scenarios and optimization problems.

## Dependencies

- Depends on domain slices (PR-03–PR-05) and editor infrastructure (PR-06).
- Requires scenarios Rust crate and associated WASM APIs to be wired into the unified worker.

## Acceptance Criteria

- Users can:
  - Configure and run Monte Carlo and variance analyses from the UI.
  - View portfolio attribution and optimization proposals.
  - Create, store, and execute scenarios that affect market, portfolio, and statements, with clear impact views.
- LLMs can generate dashboards that include these analysis and scenario components using documented schemas.
