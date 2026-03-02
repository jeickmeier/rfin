# Portfolio Python Binding 100% Parity Design

**Date:** 2026-03-01
**Scope:** Close all remaining gaps between the Rust `finstack-portfolio` public API and `finstack-py/src/portfolio/` Python bindings (P0-P2 items).

## Current State

The portfolio Python bindings cover ~85% of the Rust public API surface. All 14 submodules are registered and the core workflow (build portfolio → value → aggregate metrics → optimize) is fully operational. The gaps fall into five categories: dropped return values, missing properties on existing types, missing methods, unbound types, and a missing free function.

## Architecture: 5 Independent Work Streams

Each stream touches non-overlapping files, enabling parallel development via subagents.

---

## Stream 1: Scenario Return Values (P0)

**Problem:** `apply_scenario` and `apply_and_revalue` silently drop data from the Rust return tuples.

| Function | Rust Returns | Python Returns | Dropped |
|---|---|---|---|
| `apply_scenario` | `(Portfolio, MarketContext, ApplicationReport)` | `Portfolio` | Stressed market + report |
| `apply_and_revalue` | `(PortfolioValuation, ApplicationReport)` | `PortfolioValuation` | Report |

**Design:** Breaking change — return tuples matching Rust.

**New Python signatures:**

```python
def apply_scenario(
    portfolio: Portfolio,
    scenario: ScenarioSpec,
    market_context: MarketContext,
) -> tuple[Portfolio, MarketContext, ApplicationReport]: ...

def apply_and_revalue(
    portfolio: Portfolio,
    scenario: ScenarioSpec,
    market_context: MarketContext,
    config: FinstackConfig | None = None,
) -> tuple[PortfolioValuation, ApplicationReport]: ...
```

**New type: `ApplicationReport`**

```python
class ApplicationReport:
    """Diagnostic report from scenario application."""
    @property
    def operations_applied(self) -> int: ...
    @property
    def warnings(self) -> list[str]: ...
    @property
    def rounding_context(self) -> str | None: ...
    def __repr__(self) -> str: ...
```

**Files modified:**
- `finstack-py/src/portfolio/scenarios.rs` — return tuples, bind `ApplicationReport`
- `finstack-py/finstack/portfolio/scenarios.pyi` — update stubs
- `finstack-py/finstack/portfolio/__init__.pyi` — re-export `ApplicationReport`

---

## Stream 2: Valuation & Metrics Enhancement (P0-P1)

**Problem:** `PortfolioValuationOptions` hardcodes `additional_metrics=None` and `replace_standard_metrics=false`. `PositionValue` doesn't expose the underlying `ValuationResult`.

### 2a. PortfolioValuationOptions (P0)

**New Python constructor:**

```python
class PortfolioValuationOptions:
    def __init__(
        self,
        *,
        strict_risk: bool = False,
        additional_metrics: list[MetricId] | None = None,
        replace_standard_metrics: bool = False,
    ) -> None: ...

    @property
    def strict_risk(self) -> bool: ...
    @property
    def additional_metrics(self) -> list[MetricId] | None: ...
    @property
    def replace_standard_metrics(self) -> bool: ...
```

### 2b. PositionValue.valuation_result (P1)

Expose the optional `ValuationResult` that contains per-position metrics, cashflow schedule, and covenant reports.

```python
class PositionValue:
    # ... existing properties ...
    @property
    def valuation_result(self) -> ValuationResult | None:
        """Full valuation result if available (includes metrics, cashflows, covenants)."""
        ...
```

**Note:** `ValuationResult` must already be bound in the valuations module. If not, this becomes a dependency on the valuations parity work.

**Files modified:**
- `finstack-py/src/portfolio/valuation.rs` — add constructor params, getters, expose `valuation_result`
- `finstack-py/finstack/portfolio/valuation.pyi` — update stubs

---

## Stream 3: Optimization Diagnostics (P1-P2)

**Problem:** `OptimizationResult` exposes only 5 of 11 public fields. `MaxYieldWithCccLimitResult` is returned as untyped `dict`. `DefaultLpOptimizer` is not exposed. `OptimizationStatus` variant data is inaccessible.

### 3a. OptimizationResult missing fields (P1)

Add getters for all missing fields:

```python
class OptimizationResult:
    # ... existing properties ...

    @property
    def implied_quantities(self) -> dict[str, float]:
        """Implied position quantities at optimal weights."""
        ...
    @property
    def metric_values(self) -> dict[str, float]:
        """Metric values at the optimal solution."""
        ...
    @property
    def dual_values(self) -> dict[str, float]:
        """Dual values (shadow prices) for constraints."""
        ...
    @property
    def constraint_slacks(self) -> dict[str, float]:
        """Constraint slack values at optimal solution."""
        ...
    @property
    def meta(self) -> dict[str, Any]:
        """Calculation metadata (numeric mode, rounding, timing)."""
        ...
    @property
    def problem(self) -> PortfolioOptimizationProblem:
        """The optimization problem that produced this result."""
        ...
```

### 3b. OptimizationStatus variant access (P1)

```python
class OptimizationStatus:
    def is_feasible(self) -> bool: ...
    @property
    def conflicting_constraints(self) -> list[str] | None:
        """Constraint labels when status is Infeasible, else None."""
        ...
    @property
    def error_message(self) -> str | None:
        """Error message when status is Error, else None."""
        ...
    @property
    def status_name(self) -> str:
        """Variant name: 'Optimal', 'FeasibleButSuboptimal', 'Infeasible', 'Unbounded', 'Error'."""
        ...
```

### 3c. MaxYieldWithCccLimitResult as typed class (P1)

Replace dict return with proper class:

```python
class MaxYieldWithCccLimitResult:
    """Result from optimize_max_yield_with_ccc_limit convenience function."""
    @property
    def label(self) -> str | None: ...
    @property
    def status(self) -> OptimizationStatus: ...
    @property
    def status_label(self) -> str: ...
    @property
    def objective_value(self) -> float: ...
    @property
    def ccc_weight(self) -> float: ...
    @property
    def optimal_weights(self) -> dict[str, float]: ...
    @property
    def current_weights(self) -> dict[str, float]: ...
    @property
    def weight_deltas(self) -> dict[str, float]: ...
    def __repr__(self) -> str: ...
```

### 3d. PortfolioOptimizationProblem readable getters (P2)

```python
class PortfolioOptimizationProblem:
    # ... existing setters and methods ...
    @property
    def portfolio(self) -> Portfolio: ...
    @property
    def objective(self) -> Objective: ...
    @property
    def constraints(self) -> list[Constraint]: ...
    @property
    def trade_universe(self) -> TradeUniverse: ...
```

### 3e. DefaultLpOptimizer (P2)

```python
class DefaultLpOptimizer:
    """LP-based portfolio optimizer with configurable solver parameters."""
    def __init__(self, *, tolerance: float = 1e-8, max_iterations: int = 10000) -> None: ...
    @property
    def tolerance(self) -> float: ...
    @property
    def max_iterations(self) -> int: ...
    def optimize(
        self,
        problem: PortfolioOptimizationProblem,
        market_context: MarketContext,
        config: FinstackConfig | None = None,
    ) -> OptimizationResult: ...
    def __repr__(self) -> str: ...
```

**Files modified:**
- `finstack-py/src/portfolio/optimization.rs` — add getters, bind new types
- `finstack-py/finstack/portfolio/optimization.pyi` — update stubs

---

## Stream 4: Core Types & Grouping (P1-P2)

### 4a. Position.instrument property (P1)

Expose the instrument back from a position. The instrument is stored as `Arc<dyn Instrument>` in Rust — the Python binding should return it as the appropriate instrument wrapper type.

```python
class Position:
    # ... existing properties ...
    @property
    def instrument(self) -> Any:
        """The instrument held by this position."""
        ...
```

Implementation: use the existing `instrument_to_py()` conversion (or similar) that wraps `Arc<dyn Instrument>` into the correct `PyBond` / `PyEquity` / etc.

### 4b. Book methods (P1-P2)

```python
class Book:
    # ... existing properties ...
    def is_root(self) -> bool:
        """True if this book has no parent."""
        ...
    def contains_position(self, position_id: str) -> bool:
        """Check if position is directly assigned (non-recursive)."""
        ...
    def contains_child(self, child_id: str) -> bool:
        """Check if child book exists."""
        ...
    def add_position(self, position_id: str) -> None:
        """Add a position to this book."""
        ...
    def add_child(self, child_id: str) -> None:
        """Add a child book."""
        ...
    def remove_position(self, position_id: str) -> None:
        """Remove a position from this book."""
        ...
    def remove_child(self, child_id: str) -> None:
        """Remove a child book."""
        ...
```

### 4c. aggregate_by_multiple_attributes (P1)

```python
def aggregate_by_multiple_attributes(
    valuation: PortfolioValuation,
    portfolio: Portfolio,
    attribute_keys: list[str],
) -> dict[tuple[str, ...], Money]:
    """Aggregate values by multiple attribute keys simultaneously.

    Returns dict keyed by tuples of attribute values.
    """
    ...
```

### 4d. PortfolioSpec / PositionSpec + to_spec/from_spec (P2)

Bind the serializable spec types and round-trip methods:

```python
class PositionSpec:
    """Serializable position specification (for JSON round-trip)."""
    @property
    def position_id(self) -> str: ...
    @property
    def entity_id(self) -> str: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def quantity(self) -> float: ...
    @property
    def unit(self) -> PositionUnit: ...
    def to_json(self) -> str: ...
    @staticmethod
    def from_json(json: str) -> PositionSpec: ...

class PortfolioSpec:
    """Serializable portfolio specification (for JSON round-trip)."""
    @property
    def id(self) -> str: ...
    @property
    def positions(self) -> list[PositionSpec]: ...
    def to_json(self) -> str: ...
    @staticmethod
    def from_json(json: str) -> PortfolioSpec: ...

# Methods on existing types:
class Position:
    def to_spec(self) -> PositionSpec: ...
    @staticmethod
    def from_spec(spec: PositionSpec) -> Position: ...

class Portfolio:
    def to_spec(self) -> PortfolioSpec: ...
    @staticmethod
    def from_spec(spec: PortfolioSpec) -> Portfolio: ...
```

**Files modified:**
- `finstack-py/src/portfolio/types.rs` — add `Position.instrument`, book methods
- `finstack-py/src/portfolio/book.rs` — add book mutation methods
- `finstack-py/src/portfolio/positions.rs` — add `to_spec`/`from_spec`
- `finstack-py/src/portfolio/grouping.rs` — bind `aggregate_by_multiple_attributes`
- `finstack-py/finstack/portfolio/types.pyi` — update stubs
- `finstack-py/finstack/portfolio/portfolio.pyi` — update stubs
- `finstack-py/finstack/portfolio/grouping.pyi` — update stubs

---

## Stream 5: Margin Module Enhancement (P1-P2)

### 5a. NettingSetManager iterator and methods (P1-P2)

```python
class NettingSetManager:
    # ... existing methods ...
    def __iter__(self) -> Iterator[tuple[NettingSetId, NettingSet]]:
        """Iterate over (id, netting_set) pairs."""
        ...
    def __len__(self) -> int:
        """Number of netting sets (alias for count())."""
        ...
    def get_or_create(self, id: NettingSetId) -> NettingSet:
        """Get existing or create new netting set."""
        ...
    def merge_sensitivities(self, netting_set_id: NettingSetId, sensitivities: Any) -> None:
        """Merge SIMM sensitivities into a specific netting set."""
        ...
```

### 5b. PortfolioMarginResult methods (P1-P2)

```python
class PortfolioMarginResult:
    # ... existing properties ...
    def netting_set_count(self) -> int:
        """Number of netting sets in result."""
        ...
    def __iter__(self) -> Iterator[tuple[str, NettingSetMargin]]:
        """Iterate over (id, margin) pairs."""
        ...
    def __len__(self) -> int:
        """Number of netting sets."""
        ...
```

### 5c. NettingSetMargin constructor (P2)

```python
class NettingSetMargin:
    # ... existing properties ...
    def __init__(
        self,
        netting_set_id: NettingSetId,
        as_of: date,
        initial_margin: Money,
        variation_margin: Money,
        position_count: int,
        im_methodology: str,
    ) -> None: ...
    def with_simm_breakdown(self, sensitivities: Any, breakdown: dict[str, Money]) -> NettingSetMargin: ...
```

### 5d. PortfolioMarginAggregator.netting_set_count (P2)

```python
class PortfolioMarginAggregator:
    # ... existing methods ...
    def netting_set_count(self) -> int:
        """Number of netting sets tracked."""
        ...
```

**Files modified:**
- `finstack-py/src/portfolio/margin.rs` — add methods and constructors
- `finstack-py/finstack/portfolio/margin.pyi` — update stubs

---

## Summary

| Stream | Priority | Items | Files |
|---|---|---|---|
| S1: Scenario Return Values | P0 | 3 (2 functions + 1 new type) | 3 |
| S2: Valuation Enhancement | P0-P1 | 4 (constructor params + getters) | 2 |
| S3: Optimization Diagnostics | P1-P2 | 15+ (getters, new types, status access) | 2 |
| S4: Core Types & Grouping | P1-P2 | 15+ (methods, properties, new types, function) | 7 |
| S5: Margin Enhancement | P1-P2 | 10+ (iterators, constructors, methods) | 2 |

**Total new binding work:** ~47 items across 16 files.

## Items Intentionally Excluded (P3)

- `EntityId`/`PositionId` newtypes — Python uses `str` idiomatically
- `Error` enum variants — mapped to `ValueError`/`RuntimeError`
- `ConstraintValidationError` — mapped to `ValueError`
- `CurrencyMismatchError` — mapped to generic error
- `PortfolioOptimizer` trait — no Python extensibility needed; users use `DefaultLpOptimizer` directly
- `test_utils` module — test-only utilities
- `Portfolio.rebuild_index()` — internal bookkeeping, not user-facing
