# Finstack UI Kit: Domain Designs

## 3.5 Detailed Domain Designs

This document captures the domain-specific UI designs for **Valuations**, **Portfolio**, **Statements**, **Market**, and **Scenarios**.

---

### A. Domain: Valuations (`finstack-wasm/valuations`)

Focus: Visualizing the output of `pricer.rs`, `metrics/`, and `attribution/` across all instruments, with full access to calibration, cashflows, margin, covenants, and result envelopes.

1. **`RiskHeatmap`**
    * **Purpose:** Display Greeks (Delta, Gamma, Vega) across a portfolio.
    * **Tech:** TanStack Table with dynamic cell coloring.
    * **Features:** Grouping by Currency/Sector; Drill-down to individual trades.
    * **Accessibility:** Color-blind safe palette, keyboard navigation.

2. **`CashflowWaterfall`**
    * **Purpose:** Inspect intermediate cashflows of a trade (e.g., Swaps).
    * **Tech:** TanStack Table + TanStack Virtual (Virtualization required for 30Y swaps with 120+ rows).
    * **Columns:** Period | Fix/Float | Rate | Notional | Discount Factor | PV.

3. **`PnLAttributionBridge`**
    * **Purpose:** Explain PnL changes (e.g., "Why did we lose money?").
    * **Tech:** ECharts Waterfall.
    * **Data:** Visualizes the `AttributionResult` struct from `attribution.rs`.

4. **Instrument Panels (1:1 with `valuations::instruments`)**
    * **Purpose:** Provide instrument-specific pricing and inspection surfaces that mirror each WASM instrument binding (e.g., `Bond`, `InterestRateSwap`, `FxOption`, `ConvertibleBond`, `VarianceSwap`).
    * **Components:** For every exported instrument in `finstack-wasm/src/valuations/instruments` there will be:
        * **Inputs section:** A typed form component (e.g., `BondPanel`, `InterestRateSwapPanel`, `FxOptionPanel`) that wraps form controls for instrument attributes and calls `useValuation` under the hood.
        * **Cashflows section (discountable instruments only):** A reusable `CashflowWaterfall` (or related cashflow table component) embedded inside the instrument UI to show projected cashflows and discount factors.
        * **Market data section:** Read-only viewers for the relevant curves/surfaces used in pricing (e.g., `CurveChart`, `VolSurfaceViewer`, quote tickers) so users and LLMs can see exactly which market data is driving the valuation.
        * **Outputs section:** A metrics table showing PV and risk measures (DV01/CS01, bucketed risk, Greeks, etc.) in a consistent, schema-backed layout.
        * **Viewer variant:** A read-only viewer (e.g., `BondViewer`, `SwaptionViewer`) for displaying key terms, cashflows and metrics without editing.
    * **Invariants:** Adding a new instrument to `valuations::instruments` requires adding/auto-generating the corresponding panel + viewer components and their Zod schemas, so LLMs can reliably target them via the GenUI bridge.

5. **Calibration Views (1:1 with `valuations::calibration`)**
    * **Purpose:** Provide dedicated UIs for curve and surface calibration that match each WASM calibrator.
    * **Components:**
        * `DiscountCurveCalibration` - Quote grid, config editor, curve visualization
        * `ForwardCurveCalibration` - Deposits/FRAs/Swaps quote entry, tenor selection
        * `HazardCurveCalibration` - CDS quote entry, recovery rate config
        * `InflationCurveCalibration` - ZC inflation swap quotes, seasonality
        * `VolSurfaceCalibration` - Strike/expiry grid, SABR/SVI params
        * `BaseCorrelationCalibration` - Detachment points, tranche quotes
        * `SabrCalibration` - α, β, ρ, ν parameter fitting
        * `HullWhiteCalibration` - a, σ parameter calibration from swaptions
    * **Behavior:** Each view owns quote entry grids (typed to `RatesQuote`, `CreditQuote`, `VolQuote`, `InflationQuote`), configuration editors (`CalibrationConfig`, `SolverKind`), and visualization of calibration reports (error diagnostics, convergence plots), with state fully serializable for LLM control.

6. **Metrics & Risk Registry**
    * **Components:** `InstrumentRiskTable`, `BucketedRiskGrid`, and `MetricsRegistryBrowser` mapped to `metrics/` and the global metrics registry.
    * **Features:** Per-instrument and per-bucket risk tables (DV01/CS01/Delta/Gamma/Vega/Theta), bucketed risk grids (by tenor/strike), and a browser for discovering which metrics are available for each instrument type.

7. **Margin Analytics**
    * **Components:** `InstrumentMarginPanel`, `SimmBreakdownView` backed by the valuations `margin` module (IM/VM calculators, CSA specs).
    * **Features:** Per-instrument and CSA-level margin details (IM, VM, total), SIMM risk-class breakdowns, and links into the portfolio-level margin views for aggregated context.

8. **Covenant Engine Surfaces**
    * **Components:** `InstrumentCovenantPanel`, `CovenantTimelineView` wired to the valuations `covenants` engine.
    * **Features:** Configure and visualize covenants attached to instruments (e.g., coverage ratios, leverage tests), see pass/fail status over time, and expose covenant breach timelines that can be reused by portfolio and statements domains.

9. **Attribution Module**
    * **Components:** `PnLAttributionBridge`, `FactorBreakdownPanel`, `BucketedAttributionGrid`.
    * **Features:** Waterfall visualization of PnL attribution factors (rates, credit, vol, FX, carry, residual), drill-down to factor-level detail (e.g., rates bucket by tenor), percentage breakdown display.

10. **Valuation Runs & Result Envelopes**
    * **Components:** `ValuationRunViewer` for inspecting `results/` envelopes (PV, risk vectors, metadata, FX policy, numeric mode).
    * **Features:** Show raw and FX-collapsed results, period aggregation details, and provide export hooks for downstream consumers (DataFrame/CSV) in a way that stays aligned with the valuations `results` module.

---

### B. Domain: Portfolio (`finstack-wasm/portfolio`)

Focus: Entity-based position tracking, cross-currency aggregation, portfolio metrics, P&L attribution, margin, optimization, and scenario integration on top of valuations.

1. **Entity & Position Management**
    * **Components:** `TradeEntryForm`, `EntityTreeView`, and `PositionGrid` backed by `Portfolio`, `Entity`, `Position`, and `PositionUnit`.
    * **Features:** Entity-centric views (including dummy entity for standalone instruments), unit-aware quantity handling, tag editing (rating, sector, strategy), and per-position drill-down to the underlying instrument panels in the Valuations domain.

2. **Valuation & Metrics Dashboards**
    * **Components:** `PortfolioSummaryPanel` and `PortfolioMetricsPanel` mapped to `value_portfolio`, `PortfolioValuation`, and `PortfolioMetrics`.
    * **Features:** Totals by portfolio and entity, cross-currency aggregation to base currency with explicit FX policies, and display of aggregated DV01/CS01/Greeks alongside non-summable metrics at position level.

3. **Attribute-Based Grouping & Pivots**
    * **Components:** `TagPivotGrid` using grouping and aggregation helpers from `grouping.rs`.
    * **Features:** Interactive rollups by arbitrary tags (e.g., asset_class, rating, desk, strategy), with pivot-like controls and exportable grouped views.

4. **Scenario & What-If at Portfolio Level**
    * **Components:** `PortfolioScenarioImpactView` under `domains/portfolio/scenarios`, layered on top of the Scenarios domain and portfolio `apply_and_revalue` helpers.
    * **Features:** Before/after portfolio value and metric deltas by scenario, entity, and tag, reusing `ScenarioExecutionPanel` runs but focused on portfolio-level impacts.

5. **P&L Attribution**
    * **Components:** `PortfolioAttributionView` mapped to `PortfolioAttribution` and `attribute_portfolio_pnl`.
    * **Features:** Factor breakdown (rates, credit, inflation, vol, FX, carry, residual) with percentages of total, plus drill-down to per-position attribution and optional detailed factor views (e.g., rates curve buckets).

6. **Margin Aggregation**
    * **Components:** `MarginSummaryPanel`, `NettingSetView`, `SensitivityBreakdown` mirroring `margin::PortfolioMarginResult` and `NettingSetMargin`.
    * **Features:** IM/VM/Total margin by netting set, cleared vs bilateral splits, SIMM risk-class breakdowns, and optional views of aggregated sensitivities; designed to sit alongside `PortfolioSummaryPanel` for funding/risk oversight.

7. **Optimization Surface**
    * **Components:**
        * `PortfolioOptimizerView` - Main optimization interface
        * `ConstraintEditor` - Define constraints (position limits, sector caps, etc.)
        * `ConstraintViolationsPanel` - Show which constraints bind
        * `EfficientFrontierChart` - Risk/return scatter plot (Recharts)
        * `TradeProposalGrid` - Proposed buy/sell actions
        * `OptimizationResultView` - Pre/post comparison
    * **Backed by:** `PortfolioOptimizationProblem`, `PortfolioOptimizer`, `PortfolioOptimizationResult`, and `TradeSpec`.
    * **Features:** Express objectives and constraints over portfolio metrics (e.g., max yield with CCC limit), visualize proposed trades (buy/sell, size, direction), and compare pre/post-optimization portfolios.

8. **Cashflow Aggregation**
    * **Components:** `PortfolioCashflowAggregate` displaying aggregated cashflows across all positions.
    * **Features:** Timeline view of all portfolio cashflows, grouped by date/type, with currency aggregation.

9. **Data Exports**
    * **Components:** `DataFramePreview` with export actions backed by `positions_to_dataframe` and `entities_to_dataframe`.
    * **Features:** Preview before export, one-click DataFrame/CSV/Parquet exports of position- and entity-level results for external analysis, preserving stable schema from the portfolio crate.

---

### C. Domain: Statements (`finstack-wasm/statements`)

Focus: Visualizing the `evaluator.rs` DAG, `registry.rs` assumptions, capital structure integration, templates, adjustments, and all analysis tooling under `analysis/`.

1. **`StatementViewer`**
    * **Purpose:** Interactive Financial Statement (Income Statement, Balance Sheet, Cashflow).
    * **Tech:** TanStack Table (Matrix View) backed by the evaluator `Results`.
    * **Interaction:** "Corkscrew tracing" powered by the `CorkscrewExtension` – clicking a cell highlights its precedent cells and roll-forward partners (e.g., *Ending Cash* highlights *Opening Cash* + *Net Income*).
    * **Accessibility:** Keyboard navigation between cells, ARIA labels for values.

2. **`ForecastEditor`**
    * **Purpose:** Node-level editor for deterministic/statistical/seasonal/time-series forecasts (`ForecastMethod`) and explicit values.
    * **Logic:** Edits the `StatementModel` JSON (value vs forecast vs formula), triggers WASM re-calc, and refreshes `StatementViewer` plus any open analysis views.

3. **`FormulaBar`**
    * **Purpose:** Excel-like formula editing experience.
    * **Features:** Syntax highlighting, autocomplete for node references, error indicators.

4. **Templates & Roll-Forwards**
    * **Components:** `RollForwardBuilder`, `VintageAnalysisView`, `VintageWaterfallView` mirror `templates::TemplatesExtension` and `VintageExtension`.
    * **Behavior:** Wizard-like builders that add connected nodes (inventory roll-forwards, vintage waterfalls) to a model, with inline validation hooks to `CorkscrewExtension`.

5. **Capital Structure Panel**
    * **Components:**
        * `CapitalStructurePanel` - Main overview with debt schedule
        * `DebtScheduleGrid` - TanStack Table for debt instruments
        * `WaterfallViewer` - Payment priority waterfall
        * `BondInstrumentForm`, `LoanInstrumentForm`, `SwapInstrumentForm` - Instrument editors
    * **Features:** Debt schedule viewer, cashflow breakdown (interest/principal/fees), and read-only views of capital-structure-driven metrics used inside statements (the `cs.*` DSL namespace).

6. **Scenario Management & Comparison**
    * **Components:** `ScenarioSetManager` and `ScenarioComparisonView`, mirroring `analysis::ScenarioSet`, `ScenarioResults`, and `ScenarioDiff`.
    * **Features:** CRUD for named scenarios with parent/override chains, evaluation controls, and wide comparison tables/variance bridges across scenarios (including baseline vs downside/stress).

7. **Sensitivity, Goal Seek & Monte Carlo**
    * **Components:**
        * `SensitivityAnalyzer` - Parameter selection and sweep config
        * `TornadoChart` - Sensitivity tornado visualization
        * `GoalSeekPanel` - Target metric, driver selection, solve button
        * `MonteCarloConfigEditor` - Distribution config per parameter
        * `MonteCarloResultsView` - Histograms, fan charts, percentile tables
    * **Backed by:** `SensitivityAnalyzer`, `goal_seek`, `MonteCarloConfig`, `MonteCarloResults`, `PercentileSeries`.
    * **Features:** Parameter grids & tornado charts, "solve-for-X" panels (targeting a metric by varying drivers), and distribution views for Monte Carlo runs.

8. **Forecast Backtesting & Variance Analysis**
    * **Components:**
        * `BacktestDashboard` - Forecast vs actual comparison
        * `BacktestAccuracyView` - `ForecastMetrics` visualization (MAE, MAPE, etc.)
        * `VarianceBridgeView` - Two-scenario variance analysis
        * `VarianceBridgeChart` - Renders `BridgeStep[]` as waterfall
    * **Backed by:** `backtest_forecast`, `ForecastMetrics`, `VarianceAnalyzer`, `VarianceReport`, `BridgeChart`.
    * **Features:** Forecast vs actual accuracy dashboards, variance/two-scenario bridge charts, and drill-down into period/metric-level deltas.

9. **Covenants & Credit Scorecards**
    * **Components:** `CovenantMonitor` and `CreditScorecardViewer`.
    * **Features:** Visualization of covenant tests and breaches (from `analysis::covenants`), credit scorecard outputs from `CreditScorecardExtension`.

10. **EBITDA Adjustments**
    * **Components:** `EbitdaAdjustmentsPanel` backed by `adjustments` module.
    * **Features:** Normalized EBITDA calculations, adjustments bridge waterfall, add-back/deduction categorization.

11. **Formula Explain & Dependency Tracing**
    * **Components:**
        * `DependencyTreeViewer` - Tree/table view of dependencies
        * `DependencyGraphViewer` - Interactive DAG (using vis-network or reactflow)
        * `FormulaExplainPanel` - Step-by-step formula breakdown
    * **Wired to:** `DependencyTracer`, `DependencyTree`, `FormulaExplainer`, `Explanation`, `ExplanationStep`.
    * **Features:** Interactive dependency graphs with per-period values, and step-by-step formula breakdowns usable for debugging and LLM explanations.

12. **Metric Registry & Results Export**
    * **Components:** `RegistryBrowser` and export actions on `StatementViewer` / `ScenarioComparisonView`.
    * **Features:** Browse/search metric namespaces (from the `registry` module, `fin.*` and custom), inspect definitions and dependencies, and export Polars-backed tables (DataFrame/CSV) from `results::export` and `ScenarioResults::to_comparison_df()`.

13. **Extensions Console**
    * **Components:** `ExtensionsConsole` surface that lists registered extensions from `extensions::ExtensionRegistry` (including `CorkscrewExtension` and `CreditScorecardExtension`).
    * **Features:** Run extensions on current models/results, inspect status/metadata, and show diagnostics panels alongside `StatementViewer`.

---

### D. Domain: Market (`finstack-wasm/core`)

1. **`CurveEditor`**
    * **Purpose:** visualize and shock yield curves.
    * **Tech:** Recharts (Line) + Drag-and-Drop handles.
    * **Interaction:** Dragging a point updates the `MarketContext` and triggers a global re-price.

2. **`VolSurfaceViewer`**
    * **Purpose:** Display volatility surface as 2D heatmap.
    * **Tech:** ECharts heatmap.

3. **`VolSurface3D`**
    * **Purpose:** Interactive 3D volatility surface.
    * **Tech:** ECharts WebGL surface with Canvas 2D fallback.
    * **Fallback:** Detect WebGL support; fall back to 2D heatmap on unsupported devices.

4. **`FxMatrixViewer`**
    * **Purpose:** Display FX rate matrix.
    * **Tech:** TanStack Table with editable cells.

5. **`InflationIndexViewer`**
    * **Purpose:** Display CPI index series.
    * **Tech:** Recharts time series.

---

### E. Domain: Scenarios (`finstack-wasm/scenarios`)

Focus: Providing a cross-domain scenario engine UI that mirrors `ScenarioSpec`, `OperationSpec`, `ScenarioEngine`, and `ExecutionContext` from the `finstack-scenarios` crate.

1. **`ScenarioBuilder`**
    * **Purpose:** JSON-backed editor for individual `ScenarioSpec` objects and their ordered list of `OperationSpec` variants (FX, curves, vols, statements, instruments, time roll-forward).
    * **Features:** Operation-type pickers, parameter forms (e.g., curve IDs, bp/pct shocks, tenors), and live validation against the active `MarketContext`/statement model.

2. **`ScenarioLibrary`**
    * **Purpose:** Manage a library of named scenarios with priorities, ready for composition via `ScenarioEngine::compose`.
    * **Features:** Tagging and grouping (e.g., "Q1 Stress", "Horizon 1M"), priority sliders, and JSON import/export that matches the serde-stable wire format.

3. **`ScenarioExecutionPanel`**
    * **Purpose:** Apply a single scenario or composed scenario set to the current `ExecutionContext` (Market + Statements + Valuations) and surface the resulting `ApplicationReport`.
    * **Features:** Phase-by-phase execution view (time roll → market → rate bindings → statements → re-eval), operation counts, and a structured warnings/error console aligned with `ApplicationReport::warnings`.

4. **`HorizonScenarioGrid`**
    * **Purpose:** Specialized viewer for horizon scenarios (e.g., 1W/1M/3M `TimeRollForward` plus shocks) across key metrics.
    * **Features:** Matrix layout with columns for horizons and rows for selected metrics (PV, DV01, statement KPIs); integrates with `ScenarioExecutionPanel` to re-use underlying `ScenarioEngine` runs.

5. **Operation Editors**
    * **Components:** Specialized editors for each operation type:
        * `CurveShockEditor` - Parallel/node shifts, curve selection
        * `EquityShockEditor` - Ticker selection, percentage shocks
        * `VolShockEditor` - Surface selection, parallel/bucket shocks
        * `FxShockEditor` - Currency pair selection, rate shocks
        * `StatementShockEditor` - Node selection, forecast adjustments
        * `TimeRollEditor` - Period selection, carry calculation options
