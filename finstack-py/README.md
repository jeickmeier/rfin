# finstack Python bindings

Python-friendly access to the [finstack](https://github.com/finstacklabs/rfin) Rust crates. The
package wraps the `finstack-core` primitives (currencies, configuration, money, and holiday
calendars) without introducing new business logic, making it easy to drive analytics and
prototyping directly from Python notebooks.

## Installation

Use [maturin](https://www.maturin.rs/) (or `uv`/`pip`) to build and install the extension:

```bash
uv run maturin develop --release
```

This compiles the Rust crate and exposes the `finstack` module to your active Python environment.

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

## Valuations: required inputs and defaults

For reproducible pricing, provide explicit market identifiers rather than relying on implicit defaults:

- Cap/Floor, Swaption: pass an explicit `vol_surface` identifier. No hard-coded default is used.
- FX Option: prefer `FxOption.builder(...)` where you must provide `domestic_curve`, `foreign_curve`, and `vol_surface`. The simple `european_call/put` helpers are demo-oriented and infer curves for a few majors.
- Equity Option: prefer `EquityOption.builder(...)` requiring `discount_curve`, `spot_id`, and `vol_surface` (optional `dividend_yield_id`).
- JSON-defined instruments (Basket, StructuredCredit, PrivateMarketsFund): Python dicts are serialized via `json.dumps` before parsing; pass either a JSON string or a dict-like object.
- Frequencies and stubs: bindings accept common synonyms (e.g., `"q"`, `"3m"`, `"semiannual"`, `"6m"`). Unknown values raise `ValueError`.
- Business-day conventions: getters reflect core enum values. Unknown variants map to the closest standard label for stability.

When in doubt, construct a `MarketContext` with the exact curves/surfaces required by an instrument and use the instrument’s builder taking explicit IDs. This guarantees parity with Rust pricers.

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

## Optional Python dependencies

The core extension has no required Python dependencies. Install the `analytics` extra if you plan to
work with numpy/pandas/polars alongside the bindings:

```bash
pip install finstack[analytics]
```

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
