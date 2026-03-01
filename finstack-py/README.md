# finstack Python bindings

Python-friendly access to the [finstack](https://github.com/finstacklabs/rfin) Rust crates. The
package provides comprehensive Python bindings for all finstack modules:

- **`finstack.core`**: Financial primitives (currencies, money, dates, market data, math)
- **`finstack.valuations`**: Instrument pricing, cashflow modeling, and risk metrics
- **`finstack.statements`**: Financial statement modeling with forecasting and extensions
- **`finstack.scenarios`**: Reproducible scenario analysis and stress testing with stable ordering
- **`finstack.portfolio`**: Portfolio management, aggregation, and multi-entity analysis

All business logic remains in the Rust library, with Python bindings providing ergonomic interfaces
for analytics, prototyping, and production workflows.

## Installation

Use [maturin](https://www.maturin.rs/) (or `uv`/`pip`) to build and install the extension:

```bash
uv run maturin develop --release
```

This compiles the Rust crate and exposes the `finstack` module to your active Python environment.

### Features

The package includes several optional features:

- **`scenarios`** (default): Enables scenario analysis capabilities
- **`polars_export`**: Enables Polars DataFrame exports for statements

### Dependencies

The core extension has no required Python dependencies. Install the `analytics` extra if you plan to
work with numpy/pandas/polars alongside the bindings:

```bash
pip install finstack[analytics]
```

## Quick start

```python
from datetime import date
from finstack.core.config import FinstackConfig
from finstack.core.currency import Currency
from finstack.core.dates import (
    BusinessDayConvention,
    DayCount,
    DayCountContext,
    Frequency,
    FiscalConfig,
    ScheduleBuilder,
    add_months,
    adjust,
    build_fiscal_periods,
    build_periods,
    get_calendar,
)
from finstack.core.market_data import (
    BaseCorrelationCurve,
    DiscountCurve,
    DividendScheduleBuilder,
    FxConversionPolicy,
    FxMatrix,
    HazardCurve,
    MarketContext,
    MarketScalar,
    ScalarTimeSeries,
    SeriesInterpolation,
    VolSurface,
)
from finstack.core.money import Money

usd = Currency("USD")
amount = Money(1_000_000, usd)
print(amount.format())  # "USD 1000000.00"

calendar = get_calendar("usny")
adjusted = adjust(date(2025, 1, 4), BusinessDayConvention.FOLLOWING, calendar)
print(adjusted)  # date(2025, 1, 6)

ctx = DayCountContext(calendar=calendar, frequency=Frequency.SEMI_ANNUAL)
print(
    "Act/Act ISMA year fraction:",
    DayCount.ACT_ACT_ISMA.year_fraction(date(2025, 1, 4), adjusted, ctx),
)

schedule = (
    ScheduleBuilder.new(date(2025, 1, 15), date(2025, 7, 15))
    .frequency(Frequency.MONTHLY)
    .adjust_with(BusinessDayConvention.MODIFIED_FOLLOWING, calendar)
    .end_of_month(True)
    .build()
)
print(list(schedule.dates))

periods = build_periods("2024Q1..Q2", actuals_until="2024Q1")
print([p.id.code for p in periods.periods])

fiscal = build_fiscal_periods("2025Q1..Q2", FiscalConfig.US_FEDERAL, None)
print([p.id.code for p in fiscal.periods])

print(add_months(date(2025, 1, 31), 1))
print(FiscalConfig.US_FEDERAL.start_month)

# Market data
discount = DiscountCurve(
    "USD-OIS",
    date(2024, 1, 2),
    [(0.0, 1.0), (1.0, 0.97), (5.0, 0.85)],
    day_count=DayCount.ACT_365F,
    interp="monotone_convex",
)
hazard = HazardCurve(
    "CDX-IG",
    date(2024, 1, 2),
    [(0.0, 0.01), (5.0, 0.015), (10.0, 0.02)],
    recovery_rate=0.4,
    currency=Currency("USD"),
)
surface = VolSurface(
    "EQ-FLAT",
    expiries=[1.0, 2.0],
    strikes=[90.0, 100.0, 110.0],
    grid=[[0.2, 0.21, 0.22], [0.19, 0.2, 0.21]],
)
fx = FxMatrix()
fx.set_quote(Currency("EUR"), Currency("USD"), 1.1)

ctx = MarketContext()
ctx.insert_discount(discount)
ctx.insert_hazard(hazard)
ctx.insert_base_correlation(BaseCorrelationCurve("CDX-IG", [(3.0, 0.25), (7.0, 0.45)]))
ctx.insert_surface(surface)
ctx.insert_fx(fx)
ctx.insert_price("AAPL", MarketScalar.price(Money(188.25, Currency("USD"))))
ctx.insert_series(
    ScalarTimeSeries(
        "US-CPI",
        [(date(2023, 12, 31), 300.0), (date(2024, 1, 31), 301.5)],
        interpolation=SeriesInterpolation.LINEAR,
    )
)

builder = DividendScheduleBuilder("AAPL-DIVS")
builder.underlying("AAPL")
builder.cash(date(2024, 2, 15), Money(0.24, Currency("USD")))
dividends = builder.build()
ctx.insert_dividends(dividends)

print(ctx.stats())
print(fx.rate(Currency("EUR"), Currency("USD"), date(2024, 1, 2), FxConversionPolicy.CASHFLOW_DATE))
```

## Valuations

The `finstack.valuations` module provides comprehensive instrument pricing, cashflow modeling, and risk metrics:

```python
from finstack.valuations.instruments import Bond, EquityOption, IRS
from finstack.valuations.pricer import Pricer
from finstack.valuations.cashflow import CashFlowBuilder
from finstack.core.market_data import MarketContext

# Create instruments
bond = Bond.builder("BOND_1")
    .notional(1_000_000)
    .currency("USD")
    .coupon_rate(0.05)
    .frequency("semiannual")
    .maturity(date(2029, 6, 15))
    .build()

# Price with market context
market = MarketContext()
# ... add curves, surfaces, etc. ...
pricer = Pricer()
results = pricer.price(bond, market)

print(f"Bond PV: ${results.present_value:,.2f}")
print(f"Duration: {results.duration:.2f}")
print(f"Convexity: {results.convexity:.2f}")
```

### Available Instruments

- **Fixed Income**: Bonds, FRAs, IRS, Swaptions, Cap/Floor
- **Equity**: Equity options, variance swaps, TRS
- **Credit**: CDS, CDS Index, CDS Tranche, CDS Options
- **FX**: FX options, FX forwards
- **Inflation**: Inflation-linked bonds, inflation swaps
- **Structured**: Convertible bonds, structured credit, private markets funds
- **Other**: Repos, basis swaps, baskets

### Key Features

- **Comprehensive instrument coverage** with 20+ instrument types
- **Cashflow modeling** with flexible builder patterns
- **Risk metrics** (duration, convexity, Greeks, DV01, CS01)
- **Calibration framework** for curve and surface fitting
- **Currency-safe pricing** with explicit FX handling
- **JSON serialization** for instrument persistence

### Required Inputs and Defaults

For reproducible pricing, provide explicit market identifiers rather than relying on implicit defaults:

- **Cap/Floor, Swaption**: Pass an explicit `vol_surface` identifier. No hard-coded default is used.
- **FX Option**: Prefer `FxOption.builder(...)` where you must provide `domestic_curve`, `foreign_curve`, and `vol_surface`.
- **Equity Option**: Prefer `EquityOption.builder(...)` requiring `discount_curve`, `spot_id`, and `vol_surface`.
- **JSON-defined instruments**: Python dicts are serialized via `json.dumps` before parsing.
- **Frequencies and stubs**: Bindings accept common synonyms (e.g., `"q"`, `"3m"`, `"semiannual"`, `"6m"`).

When in doubt, construct a `MarketContext` with the exact curves/surfaces required by an instrument and use the instrument's builder taking explicit IDs.

## Financial Statements Modeling

The `finstack.statements` module provides a complete financial statement modeling engine:

```python
from finstack.statements.builder import ModelBuilder
from finstack.statements.types import AmountOrScalar, ForecastSpec
from finstack.statements.evaluator import Evaluator
from finstack.statements.registry import Registry
from finstack.core.dates import PeriodId

# Build a P&L model
builder = ModelBuilder.new("Acme Corp P&L")
builder.periods("2025Q1..Q4", "2025Q2")  # Q1-Q2 actuals, Q3-Q4 forecasts

# Add revenue with actuals and growth forecast
builder.value(
    "revenue",
    [
        (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(1_000_000.0)),
        (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(1_100_000.0)),
    ],
)
builder.forecast("revenue", ForecastSpec.growth(0.05))  # 5% quarterly growth

# Add calculated metrics
builder.compute("cogs", "revenue * 0.6")
builder.compute("gross_profit", "revenue - cogs")
builder.compute("gross_margin", "gross_profit / revenue")

model = builder.build()

# Evaluate the model
evaluator = Evaluator.new()
results = evaluator.evaluate(model)

# Access results
q1 = PeriodId.quarter(2025, 1)
print(f"Q1 Revenue: ${results.get('revenue', q1):,.0f}")
print(f"Q1 Gross Profit: ${results.get('gross_profit', q1):,.0f}")
print(f"Q1 Gross Margin: {results.get('gross_margin', q1):.1%}")

# Use the metric registry
registry = Registry.new()
registry.load_builtins()  # Load fin.* metrics
print(f"Available metrics: {registry.list_metrics('fin')}")
```

### Key Features

- **Declarative modeling** with rich DSL for formulas
- **Time-series forecasting** (forward fill, growth, curve, normal, log-normal, seasonal)
- **Currency-safe arithmetic** with explicit FX handling
- **Deterministic evaluation** with precedence rules (Value > Forecast > Formula)
- **Dynamic metric registry** for reusable financial metrics
- **Extension system** for custom analysis (corkscrew, scorecards)
- **JSON serialization** for model persistence

## Portfolio Management

The `finstack.portfolio` module provides comprehensive portfolio management capabilities:

```python
from finstack.portfolio import Portfolio, PortfolioBuilder, Entity, Position
from finstack.core.market_data import MarketContext
from finstack.valuations.instruments import Bond

# Create a portfolio with entities and positions
portfolio = (
    PortfolioBuilder("FUND_A")
    .name("Alpha Fund")
    .base_ccy("USD")
    .as_of(date(2024, 1, 1))
    .entity(Entity("ACME", name="Acme Corp"))
    .position(Position("POS_1", "ACME", "BOND_1", 1_000_000.0))
    .build()
)

# Value the portfolio
market = MarketContext()
# ... add market data ...
results = portfolio.value(market)

# Group and aggregate by attributes
grouped = portfolio.group_by_attribute("sector")
print(f"Portfolio value: ${results.total_value:,.2f}")
print(f"Positions by sector: {grouped}")
```

### Key Features

- **Entity and position management** with validation
- **Fluent builder API** for portfolio construction
- **Multi-entity portfolios** with base currency aggregation
- **Attribute-based grouping** (sector, rating, etc.)
- **Scenario integration** for stress testing
- **DataFrame exports** for analysis and reporting

## Scenario Analysis

The `finstack.scenarios` module provides reproducible scenario analysis for stress testing and what-if analysis:

```python
from finstack.scenarios import ScenarioSpec, OperationSpec, ScenarioEngine, ExecutionContext
from finstack.scenarios import CurveKind, VolSurfaceKind

# Create scenario operations
operations = [
    OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD-OIS", 50.0),  # +50bp parallel shift
    OperationSpec.equity_shock("AAPL", 0.1),  # +10% equity shock
    OperationSpec.vol_surface_shift(VolSurfaceKind.Equity, "EQ-FLAT", 0.05),  # +5% vol shift
]

# Build scenario specification
scenario = ScenarioSpec(
    "stress_test_q1",
    operations,
    name="Q1 Stress Test",
    description="Parallel rate shock with equity and vol adjustments",
    priority=1
)

# Apply scenario to execution context
engine = ScenarioEngine()
context = ExecutionContext(market=market, model=model, as_of=date(2024, 1, 1))
report = engine.apply(scenario, context)

print(f"Scenario applied: {report.applied_operations} operations")
print(f"Market context updated: {len(context.market.discount_curves)} curves")
```

### Key Features

- **Stable composition** with priority-based conflict resolution
- **Market data shocks** (curves, surfaces, FX, equities, base correlation)
- **Statement forecast adjustments** with percentage and assignment operations
- **Instrument pricing updates** by type and attributes
- **Time roll operations** with carry and theta calculations
- **JSON serialization** for scenario persistence and sharing

## Type Stubs

Type stubs (`.pyi` files) are **manually maintained** for all modules. We don't use automated stub generation because tools like `pyo3-stubgen` only work for functions, not PyO3 classes (which make up most of our API).

**When to update stubs:**

- After adding/changing classes, methods, or functions in Rust
- When method signatures change
- After user reports of missing type information

**Testing stubs:**

```bash
# Run type checker on examples
uv run mypy finstack-py/examples/

# Verify runtime imports still work
uv run pytest finstack-py/tests/
```

See `finstack-py/STUB_GENERATION.md` for detailed guidelines on writing and maintaining stubs.

## Examples

The package includes comprehensive examples demonstrating all major features:

### Core Examples

- **`core_basics.py`**: Currency, money, dates, and market data fundamentals
- **`cashflow_basics.py`**: Cashflow modeling and scheduling
- **`math_core_showcase.py`**: Mathematical utilities and distributions

### Valuations Examples

- **`bond_capabilities.py`**: Fixed income instrument pricing
- **`equity_capabilities.py`**: Equity options and derivatives
- **`credit_capabilities.py`**: CDS and credit derivatives
- **`fx_capabilities.py`**: FX options and forwards
- **`irs_capabilities.py`**: Interest rate swaps and swaptions
- **`inflation_capabilities.py`**: Inflation-linked instruments
- **`structured_credit_capabilities.py`**: Structured credit products
- **`private_markets_capabilities.py`**: Private markets fund modeling
- **`calibration_capabilities.py`**: Curve and surface calibration
- **`cashflow_builder_capabilities.py`**: Advanced cashflow modeling

### Statements Examples

- **`statements_example.py`**: Financial statement modeling with forecasting

### Scenarios Examples

- **`scenarios_example.py`**: Scenario analysis and stress testing

### Portfolio Examples

- **`portfolio_example.py`**: Portfolio management and aggregation

### Running Examples

```bash
# Run all examples
uv run python finstack-py/examples/scripts/run_all_examples.py

# Run specific examples
uv run python finstack-py/examples/scripts/core/core_basics.py
uv run python finstack-py/examples/scripts/valuations/bond_capabilities.py
uv run python finstack-py/examples/scripts/portfolio/portfolio_example.py
```

### Jupyter Notebooks

Interactive notebooks are available in `finstack-py/examples/notebooks/`:

- **`core_basics.ipynb`**: Interactive core functionality walkthrough

## Using with TypeScript/WASM

The finstack library also provides WebAssembly bindings for browser and Node.js environments. The WASM bindings have **100% feature parity** with the Python bindings, enabling seamless code migration between languages.

### Quick Links

- **WASM Bindings:** See [`finstack-wasm/README.md`](../finstack-wasm/README.md)
- **API Reference:** Complete Python ↔ TypeScript mapping in [`book/src/bindings/api-reference.md`](../book/src/bindings/api-reference.md)
- **Migration Guide:** Detailed migration patterns in [`book/src/bindings/migration-guide.md`](../book/src/bindings/migration-guide.md)
- **Naming Conventions:** Function name mappings in [`NAMING_CONVENTIONS.md`](../NAMING_CONVENTIONS.md)
- **Side-by-Side Examples:** Code comparisons in [`book/src/bindings/examples.md`](../book/src/bindings/examples.md)

### Example: Same Code, Different Language

**Python:**

```python
from finstack.valuations.instruments import Bond
from finstack.valuations.calibration import DiscountCurveCalibrator

bond = Bond.treasury("US-10Y", 1_000_000, "USD", 0.0375, maturity, issue)
calibrator = DiscountCurveCalibrator("USD-OIS", date, "USD")
curve, report = calibrator.calibrate(quotes, market)
```

**TypeScript:**

```typescript
import { Bond, DiscountCurveCalibrator } from 'finstack-wasm';

const bond = Bond.treasury("US-10Y", 1_000_000, "USD", 0.0375, maturity, issue);
const calibrator = new DiscountCurveCalibrator("USD-OIS", date, "USD");
const [curve, report] = calibrator.calibrate(quotes, market);
```

**Key Differences:** Method names use camelCase in TypeScript vs snake_case in Python. See the [Naming Conventions](../NAMING_CONVENTIONS.md) guide for complete mappings.
