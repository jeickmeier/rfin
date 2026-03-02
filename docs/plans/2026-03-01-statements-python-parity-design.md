# Statements Python Bindings Parity Design

**Date:** 2026-03-01
**Scope:** Core modules (types, builder, evaluator, forecast) + Analysis module
**Approach:** Bottom-up — fix core gaps first, then add analysis features in dependency order

## Current State

The statements Python bindings are ~90% complete for core modules and ~50% complete for the analysis module. Sensitivity, variance, scenario, dependency tracing, and formula explanation are fully bound. The major gaps are DCF valuation, credit context, covenant analysis, backtesting, Monte Carlo config, and the corporate orchestrator.

---

## Phase 1: Core Type Completeness (5 items)

Surgical additions to existing binding files. No new files needed.

### 1.1 AmountOrScalar — `finstack-py/src/statements/types/value.rs`

| Item | Rust Signature | Python API |
|------|---------------|------------|
| `is_amount` | `pub fn is_amount(&self) -> bool` | `@property is_amount -> bool` |
| `as_money()` | `pub fn as_money(&self) -> Option<Money>` | `def as_money(self) -> Money \| None` |

### 1.2 StatementResult — `finstack-py/src/statements/evaluator/mod.rs`

| Item | Rust Signature | Python API |
|------|---------------|------------|
| `get_money()` | `pub fn get_money(&self, node_id: &str, period_id: &PeriodId) -> Option<Money>` | `def get_money(self, node_id: str, period_id: PeriodId) -> Money \| None` |
| `get_scalar()` | `pub fn get_scalar(&self, node_id: &str, period_id: &PeriodId) -> Option<f64>` | `def get_scalar(self, node_id: str, period_id: PeriodId) -> float \| None` |

### 1.3 Evaluator — `finstack-py/src/statements/evaluator/mod.rs`

| Item | Rust Signature | Python API |
|------|---------------|------------|
| `with_market_context()` | `pub fn with_market_context(market_ctx, as_of) -> EvaluatorWithContext` | `def with_market_context(self, market_ctx: MarketContext, as_of: date) -> EvaluatorWithContext` |

---

## Phase 2: Supporting Types (4 items)

### 2.1 NodeValueType enum — `finstack-py/src/statements/types/node.rs`

New enum exposure:

```python
class NodeValueType:
    MONETARY: NodeValueType  # with currency property
    SCALAR: NodeValueType

    @staticmethod
    def monetary(currency: Currency) -> NodeValueType: ...

    @staticmethod
    def scalar() -> NodeValueType: ...

    @property
    def currency(self) -> Currency | None: ...
```

### 2.2 EvalWarning enum — `finstack-py/src/statements/evaluator/mod.rs`

New enum exposure with variant-specific data:

```python
class EvalWarning:
    @property
    def kind(self) -> str: ...  # "division_by_zero", "nan_propagated", "non_finite_value"

    @property
    def node_id(self) -> str: ...

    @property
    def period(self) -> PeriodId: ...
```

### 2.3 ResultsMeta extensions — `finstack-py/src/statements/evaluator/mod.rs`

Add to existing `ResultsMeta`:

| Property | Type | Description |
|----------|------|-------------|
| `warnings` | `list[EvalWarning]` | Evaluation warnings |
| `parallel` | `bool` | Whether parallel evaluation was used |

### 2.4 StatementResult extensions — `finstack-py/src/statements/evaluator/mod.rs`

Add to existing `StatementResult`:

| Property | Type | Description |
|----------|------|-------------|
| `cs_cashflows` | `CapitalStructureCashflows \| None` | Capital structure cashflows |
| `node_value_types` | `dict[str, NodeValueType]` | Value type per node |

---

## Phase 3: Analysis Foundation (11 items)

Standalone structs and functions with simple dependencies.

### 3.1 Backtesting — new: `finstack-py/src/statements/analysis/backtesting.rs`

```python
class ForecastMetrics:
    @property
    def mae(self) -> float: ...      # Mean Absolute Error
    @property
    def mape(self) -> float: ...     # Mean Absolute Percentage Error
    @property
    def rmse(self) -> float: ...     # Root Mean Squared Error
    @property
    def n(self) -> int: ...          # Sample size

    def summary(self) -> str: ...    # "MAE: 2.50, MAPE: 3.70%, RMSE: 3.20 (n=10)"

def backtest_forecast(actual: list[float], forecast: list[float]) -> ForecastMetrics: ...
```

### 3.2 Credit Context — new: `finstack-py/src/statements/analysis/credit_context.rs`

```python
class CreditContextMetrics:
    @property
    def dscr(self) -> list[tuple[PeriodId, float]]: ...
    @property
    def interest_coverage(self) -> list[tuple[PeriodId, float]]: ...
    @property
    def ltv(self) -> list[tuple[PeriodId, float]]: ...
    @property
    def dscr_min(self) -> float | None: ...
    @property
    def interest_coverage_min(self) -> float | None: ...

def compute_credit_context(
    statement: StatementResult,
    cs_cashflows: CapitalStructureCashflows,
    instrument_id: str,
    coverage_node: str,
    periods: list[Period],
    reference_value: float | None = None,
) -> CreditContextMetrics: ...
```

### 3.3 DCF Valuation — new: `finstack-py/src/statements/analysis/corporate.rs`

```python
class DcfOptions:
    def __init__(
        self,
        *,
        mid_year_convention: bool = False,
        shares_outstanding: float | None = None,
    ) -> None: ...

class CorporateValuationResult:
    @property
    def equity_value(self) -> Money: ...
    @property
    def enterprise_value(self) -> Money: ...
    @property
    def net_debt(self) -> Money: ...
    @property
    def terminal_value_pv(self) -> Money: ...
    @property
    def equity_value_per_share(self) -> float | None: ...
    @property
    def diluted_shares(self) -> float | None: ...

def evaluate_dcf(
    model: FinancialModelSpec,
    wacc: float,
    terminal_value: TerminalValueSpec,
    ufcf_node: str = "ufcf",
    net_debt_override: float | None = None,
) -> CorporateValuationResult: ...

def evaluate_dcf_with_options(
    model: FinancialModelSpec,
    wacc: float,
    terminal_value: TerminalValueSpec,
    ufcf_node: str = "ufcf",
    net_debt_override: float | None = None,
    options: DcfOptions | None = None,
) -> CorporateValuationResult: ...

def evaluate_dcf_with_market(
    model: FinancialModelSpec,
    wacc: float,
    terminal_value: TerminalValueSpec,
    ufcf_node: str = "ufcf",
    net_debt_override: float | None = None,
    options: DcfOptions | None = None,
    market: MarketContext | None = None,
) -> CorporateValuationResult: ...
```

**Note:** `TerminalValueSpec` already has Python bindings in `finstack.valuations.instruments`. Reuse it directly.

---

## Phase 4: Analysis Advanced (17 items)

### 4.1 Monte Carlo Config — enhance: `finstack-py/src/statements/analysis/monte_carlo.rs`

```python
class MonteCarloConfig:
    def __init__(self, n_paths: int, seed: int) -> None: ...
    def with_percentiles(self, percentiles: list[float]) -> MonteCarloConfig: ...

class PercentileSeries:
    @property
    def metric(self) -> str: ...
    @property
    def values(self) -> dict[PeriodId, list[tuple[float, float]]]: ...  # period -> [(percentile, value)]
```

Enhance existing `MonteCarloResults`:

| Method | Signature | Description |
|--------|-----------|-------------|
| `get_percentile_series()` | `(metric: str, percentile: float) -> dict[PeriodId, float] \| None` | Time series at specific percentile |
| `breach_probability()` | `(metric: str, threshold: float) -> float \| None` | P(breach) across forecast periods |
| `forecast_periods` | `list[PeriodId]` (property) | Periods in the simulation |

Also update `Evaluator.evaluate_monte_carlo()` to accept `MonteCarloConfig` as an alternative to individual parameters.

### 4.2 Covenant Analysis — new: `finstack-py/src/statements/analysis/covenants.rs`

```python
def forecast_covenant(
    covenant: CovenantSpec,
    model: FinancialModelSpec,
    base_case: StatementResult,
    periods: list[PeriodId],
    config: CovenantForecastConfig | None = None,
) -> CovenantForecast: ...

def forecast_covenants(
    covenants: list[CovenantSpec],
    model: FinancialModelSpec,
    base_case: StatementResult,
    periods: list[PeriodId],
    config: CovenantForecastConfig | None = None,
) -> list[CovenantForecast]: ...

def forecast_breaches(
    results: StatementResult,
    covenants: CovenantEngine,
    model: FinancialModelSpec | None = None,
    config: CovenantForecastConfig | None = None,
) -> list[FutureBreach]: ...
```

**Note:** `CovenantSpec`, `CovenantForecastConfig`, `CovenantEngine`, `FutureBreach`, `CovenantForecast` already have Python bindings in `finstack.valuations.covenants`. Reuse them.

### 4.3 Corporate Orchestrator — new: `finstack-py/src/statements/analysis/orchestrator.rs`

```python
class CreditInstrumentAnalysis:
    @property
    def coverage(self) -> CreditContextMetrics: ...

class CorporateAnalysis:
    @property
    def statement(self) -> StatementResult: ...
    @property
    def equity(self) -> CorporateValuationResult | None: ...
    @property
    def credit(self) -> dict[str, CreditInstrumentAnalysis]: ...

class CorporateAnalysisBuilder:
    def __init__(self, model: FinancialModelSpec) -> None: ...
    def market(self, ctx: MarketContext) -> CorporateAnalysisBuilder: ...
    def as_of(self, date: date) -> CorporateAnalysisBuilder: ...
    def dcf(self, wacc: float, terminal_value: TerminalValueSpec) -> CorporateAnalysisBuilder: ...
    def dcf_with_options(self, wacc: float, terminal_value: TerminalValueSpec, options: DcfOptions) -> CorporateAnalysisBuilder: ...
    def dcf_node(self, node: str) -> CorporateAnalysisBuilder: ...
    def net_debt_override(self, net_debt: float) -> CorporateAnalysisBuilder: ...
    def coverage_node(self, node: str) -> CorporateAnalysisBuilder: ...
    def analyze(self) -> CorporateAnalysis: ...
```

---

## Phase 5: Stubs & Polish

- Update `.pyi` stub files for all additions in Phases 1-4
- Add missing `.pyi` stubs for existing builder methods (real estate templates: `add_roll_forward`, `add_vintage_buildup`, `add_noi_buildup`, `add_ncf_buildup`, `add_rent_roll_rental_revenue`, `add_rent_roll_rental_revenue_v2`, `add_property_operating_statement`, `where_clause`, `add_registry_metrics`)
- Update `__init__.py` / `__init__.pyi` re-exports for analysis submodule
- Verify all new types work with existing serialization patterns (to_json/from_json where applicable)

---

## Dependencies Between Phases

```
Phase 1 (core completeness) ─┐
                              ├─> Phase 3 (analysis foundation)
Phase 2 (supporting types) ──┘            │
                                          v
                              Phase 4 (analysis advanced)
                                          │
                                          v
                              Phase 5 (stubs & polish)
```

- Phase 3.2 (credit context) requires Phase 2.4 (`cs_cashflows` on `StatementResult`)
- Phase 4.2 (covenants) requires Phase 3 types
- Phase 4.3 (orchestrator) requires Phase 3.2 and 3.3

## Estimated Scope

| Phase | New Files | Modified Files | Items |
|-------|-----------|----------------|-------|
| 1 | 0 | 2 | 5 |
| 2 | 0 | 2 | 4 |
| 3 | 3 | 0 | 11 |
| 4 | 3 | 1 | 17 |
| 5 | 0 | ~10 | stubs |
| **Total** | **6** | **~15** | **37+** |
