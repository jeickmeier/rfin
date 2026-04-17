# finstack Python Notebook Examples

A layered Jupyter notebook curriculum covering the full `finstack` Python library.
Designed for a starting quant to learn every module from core types through
advanced credit portfolio analytics.

## Prerequisites

- Python 3.12+
- `finstack` built and installed (`make python-dev`)
- Project dependencies installed from the repository root `pyproject.toml`

## How to Run

**Interactive** -- open individual notebooks in JupyterLab:

```bash
uv run jupyter lab
```

**Programmatic** -- execute all notebooks and report pass/fail:

```bash
uv run python finstack-py/examples/run_all_notebooks.py
```

Run a single section:

```bash
uv run python finstack-py/examples/run_all_notebooks.py --directory 01_foundations
```

## Curriculum Structure

The curriculum has two tiers: **section overview notebooks** (read in the order
listed below) and **deep-dive sub-directories** (jump to as needed).

### Level 1 -- Foundations (`01_foundations/`)

| Notebook | Topics |
|----------|--------|
| Core Types and Money | Currency, Money, Rate, Bps, Percentage, CreditRating, Attributes |
| Dates, Calendars, Schedules | DayCount, Tenor, PeriodId, HolidayCalendar, ScheduleBuilder |
| Market Data and Curves | DiscountCurve, ForwardCurve, HazardCurve, FxMatrix, MarketContext |
| Math Toolkit | Linear algebra, statistics, special functions, compensated summation |

Deep dives: `market_data/` (8 notebooks), `dates/` (3 notebooks)

### Level 2 -- Instrument Pricing (`02_pricing/`)

| Notebook | Topics |
|----------|--------|
| Pricing Fundamentals | Instrument JSON, ValuationResult, model keys, metrics |
| Pricing Across Asset Classes | Deposit, IRS, CDS, equity option, FX option, exotic |
| PnL Attribution | Attribution workflows and explain |

Deep dives: `instruments/` (12 notebooks)

### Level 3 -- Performance and Risk Analytics (`03_analytics/`)

| Notebook | Topics |
|----------|--------|
| Performance Analytics | Performance class, CAGR, Sharpe, drawdowns, rolling metrics |
| Risk and Factor Analytics | VaR, factor regression, capture ratios, ruin estimation |

### Level 4 -- Financial Statement Modeling (`04_statement_modeling/`)

| Notebook | Topics |
|----------|--------|
| Statement Modeling | ModelBuilder, Evaluator, DSL formulas, Polars export |
| Statement Analytics | Sensitivity, tornado, variance, goal-seek, dependency tracing |

Deep dives: `models/` (7 notebooks)

### Level 5 -- Portfolio and Scenarios (`05_portfolio_and_scenarios/`)

| Notebook | Topics |
|----------|--------|
| Portfolio Construction and Valuation | Portfolio spec, valuation, aggregation, cashflow ladder |
| Scenarios and Stress Testing | Templates, composition, application, revaluation |
| Horizon Total Return | Carry + scenario P&L composition, factor-decomposed total return |
| Historical Replay | Replay portfolio through dated market snapshots, P&L, attribution |

Deep dives: `scenarios/` (4 notebooks)

### Level 6 -- Advanced Quantitative Methods (`06_advanced_quant/`)

| Notebook | Topics |
|----------|--------|
| Monte Carlo Simulation | TimeGrid, McEngine, EuropeanPricer, Black-Scholes benchmarks |
| Correlation and Credit Models | Copulas, recovery models, factor models, correlated Bernoulli |
| Margin, Collateral, and XVA | CSA, VM/IM, XVA, collateral analytics |

Deep dives: `monte_carlo/` (4 notebooks), `correlation/` (3 notebooks)

### Level 7 -- Capstone (`07_capstone/`)

| Notebook | Topics |
|----------|--------|
| End-to-End Credit Portfolio Workflow | Integrates all modules into a realistic workflow |
