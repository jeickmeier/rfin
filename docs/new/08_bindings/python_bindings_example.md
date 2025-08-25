## Python Bindings — Usage Examples

This guide demonstrates the main features of the RustFin Python bindings and how to use them in idiomatic Python.

### Installation and setup

```bash
# From PyPI
pip install rfin

# Or, recommended for local dev with uv
curl -LsSf https://astral.sh/uv/install.sh | sh
./scripts/setup-python.sh
# Then build/install the extension in dev mode
uv run maturin develop --manifest-path rfin-python/Cargo.toml
```

Verify installation and version:

```python
import rfin
print("rfin version:", rfin.__version__)
```

---

## Currency and Money

```python
from rfin import Currency, Money

# Create currencies
USD = Currency("USD")
EUR = Currency("EUR")

print(USD.code, USD.numeric_code, USD.decimals)

# Create money amounts
cash = Money(100.0, USD)
fees = Money(7.5, USD)
print("Cash:", cash)                 # 100 USD
print("Currency:", cash.currency)     # Currency('USD')
print("Amount:", cash.amount)         # 100.0

# Currency-safe arithmetic
total = cash + fees                  # OK (same currency)
print("Total:", total)

try:
    bad = cash + Money(10.0, EUR)   # Raises ValueError (currency mismatch)
except ValueError as e:
    print("Expected error:", e)

# Scalar math
print("Doubled:", cash * 2)
print("Half:", cash / 2)

# Tuple extraction
amt, ccy = cash.to_parts()
print(amt, ccy)
```

---

## Dates, Calendars, DayCount, and Schedules

```python
from rfin import Date, DayCount
from rfin.dates import (
    Calendar, BusDayConvention, Frequency,
    generate_schedule, available_calendars,
    StubRule, third_wednesday, next_imm, next_cds_date,
)

trade = Date(2025, 6, 27)
settle = Date(2025, 7, 1)
print("Trade:", trade, "is_weekend=", trade.is_weekend())

# Day count calculations
dc = DayCount.act360()
print("ACT/360 year fraction:", dc.year_fraction(trade, settle))

# Holiday calendars
print("Available calendars (sample):", available_calendars()[:5])
target2 = Calendar.from_id("target2")
adj = target2.adjust(trade, BusDayConvention.Following)
print("TARGET2 following adj:", adj)

# Schedule generation (start, end, frequency)
start = Date(2025, 1, 15)
end   = Date(2030, 1, 15)
semi_annual = generate_schedule(start, end, Frequency.SemiAnnual)
print("Semi-annual dates:", [str(d) for d in semi_annual[:4]], "...")

# IMM / CDS helpers
print("Third Wednesday (Mar 2025):", third_wednesday(3, 2025))
print("Next IMM after trade:", next_imm(trade))
print("Next CDS after trade:", next_cds_date(trade))
```

---

## Cashflows — Fixed Rate Leg

```python
from rfin import Currency, Date, DayCount
from rfin.dates import Frequency
from rfin.cashflow import FixedRateLeg

notional  = 1_000_000.0
currency  = Currency("USD")
rate      = 0.04
start     = Date(2025, 1, 15)
end       = Date(2027, 1, 15)
frequency = Frequency.SemiAnnual
dc        = DayCount.act365f()

leg = FixedRateLeg(notional, currency, rate, start, end, frequency, dc)

print("Flows:")
for cf in leg.flows():
    print(cf)  # CashFlow(date=..., amount=..., currency=USD, kind=Fixed)

print("n_flows:", leg.num_flows)
print("NPV (flat 0%):", leg.npv())
print("Accrued @ 2025-09-30:", leg.accrued(Date(2025, 9, 30)))
```

End-to-end PV with a discount curve (manual integration):

```python
from rfin import Date, DayCount
from rfin.market_data import DiscountCurve, InterpStyle

# Build a simple USD OIS curve
base = Date(2025, 1, 1)
times = [0.0, 0.5, 1.0, 2.0, 5.0]
dfs   = [1.0, 0.99, 0.98, 0.95, 0.88]
curve = DiscountCurve(id="USD-OIS", base_date=base, times=times, discount_factors=dfs,
                      interpolation=InterpStyle.MonotoneConvex)

# Discount each cashflow using ACT/365F year fractions to base date
dc = DayCount.act365f()
pv = 0.0
for cf in leg.flows():
    t  = dc.year_fraction(base, cf.date)
    df = curve.df(t)
    pv += cf.amount * df

print("Manual PV:", pv)
```

---

## Market Data — Curves and Surfaces

### DiscountCurve

```python
import numpy as np
from rfin import Date
from rfin.market_data import DiscountCurve, InterpStyle

base = Date(2025, 1, 1)
curve = DiscountCurve(
    id="USD-OIS",
    base_date=base,
    times=[0.0, 0.25, 0.5, 1.0, 2.0, 5.0],
    discount_factors=[1.0, 0.9925, 0.985, 0.97, 0.94, 0.85],
    interpolation=InterpStyle.MonotoneConvex,
)

print("DF(1.5):", curve.df(1.5))
print("Zero(1.5):", curve.zero(1.5))
print("Fwd(1,2):", curve.forward(1.0, 2.0))

# Batch evaluation
ts  = np.linspace(0, 5, 6)
dfs = curve.df_batch(ts)
print("Batch DFs:", dfs.tolist())
```

### ForwardCurve

```python
from rfin import Date, DayCount
from rfin.market_data import ForwardCurve, InterpStyle

fc = ForwardCurve(
    id="USD-SOFR3M",
    tenor=0.25,
    base_date=Date(2025, 1, 1),
    times=[0.0, 1.0, 2.0, 5.0],
    forward_rates=[0.035, 0.04, 0.042, 0.045],
    interpolation=InterpStyle.Linear,
    reset_lag=2,
    day_count=DayCount.act360(),
)
print("rate(1.5):", fc.rate(1.5))
print("avg 1.0→2.0:", fc.rate_period(1.0, 2.0))
```

### HazardCurve

```python
from rfin import Date
from rfin.market_data import HazardCurve

hc = HazardCurve(
    id="CORP-A-USD",
    base_date=Date(2025, 1, 1),
    times=[0.0, 1.0, 3.0, 5.0, 10.0],
    hazard_rates=[0.01, 0.015, 0.02, 0.025, 0.03],
)
print("SP(2.0):", hc.sp(2.0))
print("DP(1→3):", hc.default_probability(1.0, 3.0))
```

### InflationCurve

```python
from rfin.market_data import InflationCurve, InterpStyle

ic = InflationCurve(
    id="US-CPI",
    base_cpi=300.0,
    times=[0.0, 1.0, 2.0, 5.0],
    cpi_levels=[300.0, 306.0, 312.24, 331.5],
    interpolation=InterpStyle.LogLinear,
)
print("CPI(3.0):", ic.cpi(3.0))
print("Inflation 0→5:", ic.inflation_rate(0.0, 5.0))
```

### VolSurface

```python
from rfin.market_data import VolSurface

expiries = [0.25, 0.5, 1.0, 2.0]
strikes  = [80, 90, 100, 110, 120]
values = [
    [0.25, 0.22, 0.20, 0.22, 0.25],
    [0.24, 0.21, 0.19, 0.21, 0.24],
    [0.23, 0.20, 0.18, 0.20, 0.23],
    [0.22, 0.19, 0.17, 0.19, 0.22],
]
vs = VolSurface(id="SPX-IV", expiries=expiries, strikes=strikes, values=values)
print("Vol(0.75, 95):", vs.value(0.75, 95.0))
print("Smile@1Y:", vs.get_expiry_slice(2).tolist())
```

### CurveSet container

```python
from rfin import Date
from rfin.market_data import CurveSet, DiscountCurve, ForwardCurve

base = Date(2025, 1, 1)
usd = DiscountCurve(id="USD-OIS", base_date=base, times=[0.0,1.0,5.0], discount_factors=[1.0,0.97,0.85])
eur = DiscountCurve(id="EUR-OIS", base_date=base, times=[0.0,1.0,5.0], discount_factors=[1.0,0.98,0.88])
sofr3m = ForwardCurve(id="USD-SOFR3M", tenor=0.25, base_date=base, times=[0.0,1.0,5.0], forward_rates=[0.035,0.04,0.045])

curves = CurveSet()
curves["USD-OIS"] = usd
curves["EUR-OIS"] = eur
curves["USD-SOFR3M"] = sofr3m
curves.map_collateral("CSA-USD", "USD-OIS")

print("IDs:", curves.keys())
print("USD df@1:", curves.discount_curve("USD-OIS").df(1.0))
print("SOFR3M@1:", curves.forward_curve("USD-SOFR3M").rate(1.0))
print("CSA df@1:", curves.collateral_curve("CSA-USD").df(1.0))
```

---

## Error handling and best practices

- Always catch `ValueError` for user-input validation issues (e.g., invalid currency codes, calendar IDs, or currency-mismatched arithmetic).
- Prefer vectorized methods for performance when working with arrays (`df_batch`, NumPy arrays supported).
- Use `DayCount` to convert dates into year fractions relative to a curve base date before calling time-based curve methods.

---

## Running snippets with uv

```bash
# Run any snippet file
uv run python path/to/snippet.py
```

---

### Statements — build and evaluate a financial model

```python
from rfin import Currency
from rfin.statements import FinancialModel, NodeType

# Define periods and a minimal model
model = (
    FinancialModel.builder("Acme Corp")
    .periods("2025Q1..2026Q4", actuals="2025Q1..Q2")
    # Known actuals
    .value("revenue", {"2025Q1": 1_000_000, "2025Q2": 1_050_000})
    .value("gross_profit", {"2025Q1": 550_000, "2025Q2": 600_000})
    # Forecast: extend revenue beyond actuals using GrowthPct (g = 5%)
    .forecast("revenue", {"method": "GrowthPct", "params": {"g": 0.05}})
    # Custom formula node
    .compute("gross_margin", "gross_profit / revenue")
    # Load built-in metrics registry and add a metric from it
    .register_metrics("fin.basic")
    .compute("gross_margin_builtin", "fin.gross_margin(gross_profit, revenue)")
    .build()
)

results = model.evaluate(parallel=False)

# Built-in and custom metric
print("gross_margin Q2:", results.values["gross_margin"]["2025Q2"])            # 600k/1.05m
print("gross_margin_builtin Q2:", results.values["gross_margin_builtin"]["2025Q2"])  # via registry metric

# Forecasted value (first non-actual period Q3)
print("revenue Q3 (forecast):", results.values["revenue"]["2025Q3"])  # ~1.05m * 1.05
```

### Valuations — price a bond and a swap

```python
from rfin import Date, Currency
from rfin.market_data import DiscountCurve, InterpStyle
from rfin.valuations import MarketData, Bond, InterestRateSwap

# Curves for USD OIS
base = Date(2025, 1, 1)
ois = DiscountCurve(
    id="USD-OIS",
    base_date=base,
    times=[0.0, 0.5, 1.0, 2.0, 5.0],
    discount_factors=[1.0, 0.995, 0.99, 0.975, 0.92],
    interpolation=InterpStyle.MonotoneConvex,
)

market = MarketData(as_of="2025-01-01", curves={"USD-OIS": ois}, fx={})

# Bond
bond = (
    Bond.builder("AAPL-5Y")
    .currency(Currency("USD"))
    .coupon(0.04)
    .maturity("2030-01-25")
    .build()
)
bond_px = bond.price(market)
print("Bond clean price:", bond_px.clean_price)

# Vanilla IRS (pay fixed, receive float)
swap = (
    InterestRateSwap.builder("IRS-USD")
    .pay_fixed(rate=0.038, currency=Currency("USD"))
    .receive_float(index="SOFR-3M")
    .effective("2025-01-15")
    .maturity("2030-01-15")
    .build()
)
swap_res = swap.price(market)
print("Swap NPV:", swap_res.npv_base)
```

### Scenarios — deterministic shocks via DSL

```python
from rfin.scenarios import Scenario
from rfin.portfolio import PortfolioRunner

# Scenario DSL: bump FX and a position quantity
scenario = Scenario.parse(
    """
    market.fx.USD/EUR:+%2
    portfolio.positions."TLB-1".quantity:+%10
    """
)

# Run a portfolio under a scenario (see Portfolio section for building inputs)
runner = PortfolioRunner(parallel=True)
out = runner.run(portfolio, market_data, scenario=scenario)
print(out.valuation.portfolio_total_base)
```

### Portfolio — build and evaluate

```python
from rfin import Currency
from rfin.statements import FinancialModel
from rfin.valuations import MarketData, Bond
from rfin.portfolio import Portfolio, Position, PortfolioRunner

# Statements model for an entity
model = (
    FinancialModel.builder("OpCo")
    .periods("2025Q1..2026Q4")
    .value("revenue", {"2025Q1": 1_000_000})
    .build()
)

# Instrument master (could be shared)
bond = Bond.builder("AAPL-5Y").coupon(0.04).maturity("2030-01-25").build()

# Market data (see Valuations for creating curves)
market = MarketData(as_of="2025-01-01", curves={"USD-OIS": ois}, fx={})

# Portfolio definition
portfolio = (
    Portfolio.builder("Fund A")
    .plan(Currency("USD"), "2025-01-01", FinancialModel.periods_of(model))
    .entity("OpCo", {"model": model})
    .position(
        Position(
            id="B1",
            entity="OpCo",
            instrument="AAPL-5Y",
            quantity=1_000_000,
            unit="face_value",
            open_date="2025-01-01",
        )
    )
    .build()
)

results = PortfolioRunner(parallel=False).run(portfolio, market)
print(results.valuation.portfolio_total_base)
```

### Analysis — custom analyzer plugin

```python
from rfin.analysis import Analyzer, register_analyzer

@register_analyzer("exposure_by_currency")
class ExposureByCcy(Analyzer):
    def analyze(self, portfolio, args):
        # Return a simple exposure summary; real implementations would inspect results
        return {"USD": 1_234_567.89, "EUR": 0.0}

# Usage is framework-integrated (e.g., runner hooks) or direct:
analyzer = ExposureByCcy()
report = analyzer.analyze(portfolio, {})
print(report)
```

### Structured Credit — build, project, and run a waterfall

```python
from rfin import Currency, Money, Date
from rfin.valuations import MarketData
from rfin.structured_credit import (
    StructuredProduct, CollateralPool, Tranche, TrancheClass,
    Waterfall, WaterfallStep, PaymentRecipient, WaterfallAmountType,
    Trigger, TriggerType,
    StructuredProductAssumptions, DefaultCurve, DefaultCurveType,
    PrepaymentCurve, PrepaymentCurveType,
)

# Collateral pool (empty for sketch; real models add PooledAsset entries)
pool = CollateralPool(assets=[])

# Tranches
senior = Tranche(
    id="A",
    class_=TrancheClass.Senior,
    seniority=1,
    original_balance=Money(100_000_000, Currency("USD")),
    current_balance=Money(100_000_000, Currency("USD")),
    coupon={"type": "Fixed", "rate": 0.06},
    credit_enhancement={"subordination": 0.30},
)

equity = Tranche(
    id="E",
    class_=TrancheClass.Equity,
    seniority=4,
    original_balance=Money(20_000_000, Currency("USD")),
    current_balance=Money(20_000_000, Currency("USD")),
    coupon={"type": "Fixed", "rate": 0.00},
    credit_enhancement={"subordination": 0.00},
)

# Waterfall
waterfall = Waterfall(
    payment_dates=[Date(2025, 3, 31), Date(2025, 6, 30)],
    interest=[
        WaterfallStep(
            priority=1,
            description="Senior interest",
            recipient=PaymentRecipient.tranche("A"),
            amount=WaterfallAmountType.CurrentInterest,
        )
    ],
    principal=[
        WaterfallStep(
            priority=1,
            description="Senior principal",
            recipient=PaymentRecipient.tranche("A"),
            amount=WaterfallAmountType.Principal,
        )
    ],
)

# Triggers
triggers = [
    Trigger(id="OC_A", kind=TriggerType.oc(tranche="A"), threshold=1.30),
]

# Assemble the deal
deal = StructuredProduct(
    id="CLO-1",
    collateral_pool=pool,
    tranches=[senior, equity],
    waterfall=waterfall,
    triggers=triggers,
    reserve_accounts=[],
    fees={"management_fee": {"bps": 50}, "servicing_fee": {"bps": 25}, "trustee_fee": {"bps": 10}, "other_fees": []},
)

# Assumptions for projections
assumptions = StructuredProductAssumptions(
    default_curve=DefaultCurve(curve_type=DefaultCurveType.Constant, parameters=[0.02]),
    prepayment_curve=PrepaymentCurve(curve_type=PrepaymentCurveType.PSA, parameters=[100]),
    recovery_rates={"BB": 0.40, "B": 0.35},
    recovery_lag=6,
)

# Market data (reuse discount curves from earlier examples)
market = MarketData(as_of="2025-01-01", curves={"USD-OIS": ois}, fx={})

# Project tranche cashflows
proj = deal.project_cashflows(market, assumptions)
print("Tranche IDs:", list(proj.tranche_cashflows.keys()))

# Run a single waterfall period with collections
wf = deal.run_waterfall(Money(5_000_000, Currency("USD")), Date(2025, 3, 31))
print("Distributions:", wf.distributions)
```

#### Scenario integration (structured credit)

```python
from rfin.scenarios import Scenario

sc = Scenario.parse(
    """
    structured.credit.triggers."OC_A".threshold:=1.35
    structured.credit.fees.management_fee:+bp10
    """
)

# Preview/apply via the host runner (integration depends on enabled features)
preview = sc.preview(structured_product=deal)
print("Scenario operations:", preview.operations)
```

#### Tranche NPV from projected cashflows

```python
from rfin import DayCount

# Assume `ois` discount curve and `proj` from above
dc = DayCount.act365f()
base = ois.base_date

npv_A = 0.0
for cf in proj.tranche_cashflows["A"]:
    t = dc.year_fraction(base, cf.date)
    df = ois.df(t)
    npv_A += cf.amount * df  # amount is numeric; if Money, use cf.amount.amount

print("Tranche A NPV (base curve):", npv_A)
```

#### Coverage ratios (OC/IC)

```python
ratios = deal.calculate_coverage_ratios(Date(2025, 6, 30))
print("OC A:", ratios.oc["A"])  # Overcollateralization ratio for tranche A
print("IC A:", ratios.ic.get("A"))  # Interest coverage ratio if defined
```


