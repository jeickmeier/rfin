# Finstack Python Bindings - Beta Testing Program

Welcome to the Finstack Python bindings beta testing program! This document provides everything you need to participate in testing the v1.0.0-beta.1 release.

## 🎯 Beta Testing Objectives

The primary goals of this beta testing phase are:

1. **API Ergonomics**: Validate that the Python API is intuitive and follows Python best practices
2. **Documentation Quality**: Ensure documentation is clear, complete, and helpful
3. **Performance**: Verify acceptable performance for production workloads
4. **Feature Coverage**: Identify any critical missing features or functionality gaps
5. **Cross-Platform**: Test on different operating systems and Python versions

## 👥 Target User Personas

We're seeking beta testers representing the following personas:

1. **Portfolio Manager** - Managing multi-asset portfolios with risk limits
2. **Credit Analyst** - Modeling structured credit and private debt
3. **Quantitative Analyst** - Building pricing models and risk analytics
4. **Risk Manager** - Calculating VaR, stress testing, scenario analysis
5. **Data Engineer** - Integrating finstack with data pipelines

## 📋 Beta Testing Timeline

- **Beta Start**: Week 13 of development cycle
- **Testing Period**: 2 weeks
- **Feedback Deadline**: End of week 14
- **Analysis & Prioritization**: Week 15
- **Beta 2 (if needed)**: Week 16

## 🚀 Getting Started

### Prerequisites

- Python 3.9+ (3.12 recommended)
- pip or uv package manager
- Basic familiarity with financial concepts

### Installation from Wheel

```bash
# Install the beta wheel
pip install finstack-1.0.0b1-cp312-cp312-macosx_11_0_arm64.whl

# Verify installation
python -c "import finstack; print(finstack.__version__)"
```

### Installation from Source (Advanced)

```bash
# Clone repository
git clone https://github.com/your-org/finstack.git
cd finstack/finstack-py

# Build and install
pip install maturin
maturin develop --release

# Run tests
pytest tests/ -v
```

## 📝 Beta Testing Checklist

### 1. Installation & Setup (15 minutes)

- [ ] Install from wheel on your platform
- [ ] Verify import works: `import finstack`
- [ ] Run basic example from quickstart
- [ ] Check documentation site renders correctly

### 2. API Exploration (1-2 hours)

Select 2-3 modules relevant to your persona and try:

- [ ] **Core**: Currency, Money, Date, MarketContext
- [ ] **Valuations**: Price 5+ different instrument types
- [ ] **Scenarios**: Build and apply scenario
- [ ] **Portfolio**: Construct and value portfolio
- [ ] **Statements**: Build and evaluate statement model

### 3. Real-World Use Case (2-4 hours)

Implement a realistic workflow for your persona:

- **Portfolio Manager**: Build portfolio, apply stress test, generate risk report
- **Credit Analyst**: Model term loan, calculate spreads, analyze covenants
- **Quantitative Analyst**: Price exotic option, compare analytical vs MC
- **Risk Manager**: Calculate VaR, run scenarios, aggregate exposures
- **Data Engineer**: Load data, compute metrics, export to DataFrame

### 4. Documentation Review (30 minutes)

- [ ] Follow a tutorial end-to-end
- [ ] Look up API reference for 5+ classes
- [ ] Review docstring examples
- [ ] Check cookbook for relevant patterns

### 5. Performance Testing (Optional, 30 minutes)

If you have production-scale workloads:

- [ ] Price 1000+ instruments
- [ ] Value large portfolio (100+ positions)
- [ ] Run complex scenario (multi-step)
- [ ] Evaluate statement model (50+ nodes)

## 📊 Feedback Collection

### Feedback Survey

Please complete the [Beta Feedback Survey](./FEEDBACK_SURVEY.md) after testing. Key areas:

1. **API Ergonomics** (1-5 rating):
   - Intuitive vs confusing
   - Python idioms vs Rust patterns
   - Error messages quality
   - Type hints coverage

2. **Documentation Quality** (1-5 rating):
   - Tutorial clarity
   - API reference completeness
   - Example relevance
   - Getting started experience

3. **Performance** (acceptable/needs improvement):
   - Import time
   - Pricing speed
   - Memory usage
   - Large-scale operations

4. **Missing Features** (open text):
   - Critical blockers
   - Nice-to-have additions
   - API inconsistencies

### Issue Reporting

Found a bug or issue? Please use our issue templates:

- [Bug Report](./.github/ISSUE_TEMPLATE/bug_report_beta.md)
- [Feature Request](./.github/ISSUE_TEMPLATE/feature_request_beta.md)
- [Documentation Issue](./.github/ISSUE_TEMPLATE/docs_issue_beta.md)

Label issues with `beta-feedback` for priority review.

## 🏆 Testing Scenarios by Persona

### Portfolio Manager

```python
# Scenario: Build portfolio, stress test, generate DV01 report
from finstack import *

# 1. Build portfolio
portfolio = (
    PortfolioBuilder("global-fixed-income")
    .base_ccy("USD")
    .as_of(Date.parse("2024-01-15"))
    .entity("corp-bonds", "Corporate Bonds")
    .position("AAPL-2029", bond1, 1_000_000, entity_id="corp-bonds")
    .position("TSLA-2028", bond2, 500_000, entity_id="corp-bonds")
    .build()
)

# 2. Stress test
scenario = ScenarioSpec.from_dsl("""
    shift USD.OIS +50bp
    shift equities -10%
""")
engine = ScenarioEngine()
stressed_market, report = engine.apply(scenario, market, config)

# 3. Generate risk report
valuation = value_portfolio(portfolio, stressed_market, config)
df = valuation.to_pandas()
df.to_csv("stress_report.csv")
```

**Test Focus**: Portfolio construction, scenario application, DataFrame export

### Credit Analyst

```python
# Scenario: Model term loan with step-up coupon
from finstack import *

# 1. Build term loan
builder = CashflowBuilder()
schedule = (
    builder
    .notional_and_currency(10_000_000, "USD")
    .dates(issue, maturity)
    .tenor("quarterly")
    .day_count(DayCount.Act360())
    .step_up_coupon([(0, 0.05), (24, 0.055), (48, 0.06)])  # Months, rate
    .amortization_percent_per_period([0.05] * 20)  # 5% per quarter
    .build()
)

# 2. Price and analyze
result = pricer_registry.price_with_metrics(
    loan, "discounting", market, 
    ["clean_price", "ytm", "duration_mod", "dv01"]
)

# 3. Covenant monitoring
extension = CorkscrewExtension.with_config(corkscrew_config)
results = evaluator.evaluate_with_extensions(model, [extension])
```

**Test Focus**: Cashflow builder, amortization, step-up coupons, extensions

### Quantitative Analyst

```python
# Scenario: Compare barrier option pricing methods
from finstack import *

# 1. Create barrier call option
option = BarrierOption.up_and_out_call(
    "AAPL-BARRIER",
    notional,
    strike=150.0,
    barrier=180.0,
    issue,
    expiry,
    "USD"
)

# 2. Analytical pricing
result_analytical = pricer_registry.price_bond_with_metrics(
    option, "BarrierBSContinuous", market, metrics
)

# 3. Monte Carlo pricing
result_mc = pricer_registry.price_bond_with_metrics(
    option, "MonteCarloGBM", market, metrics
)

# 4. Compare
print(f"Analytical: {result_analytical.present_value.amount:.2f}")
print(f"Monte Carlo: {result_mc.present_value.amount:.2f}")
print(f"Difference: {abs(result_analytical - result_mc):.4f}")
```

**Test Focus**: Analytical vs MC pricing, model key selection, Greeks

### Risk Manager

```python
# Scenario: Multi-asset VaR calculation
from finstack import *

# 1. Define scenarios
scenarios = [
    ScenarioSpec.from_dsl("shift discount USD.OIS +50bp"),
    ScenarioSpec.from_dsl("shift discount USD.OIS -50bp"),
    ScenarioSpec.from_dsl("shift equities +10%"),
    ScenarioSpec.from_dsl("shift equities -10%"),
    # ... 20+ scenarios
]

# 2. Run scenario analysis
engine = ScenarioEngine()
base_value = value_portfolio(portfolio, market, config).total.amount

pnl_scenarios = []
for scenario in scenarios:
    stressed_market, _ = engine.apply(scenario, market, config)
    stressed_value = value_portfolio(portfolio, stressed_market, config).total.amount
    pnl = stressed_value - base_value
    pnl_scenarios.append(pnl)

# 3. Calculate VaR
import numpy as np
var_95 = np.percentile(pnl_scenarios, 5)
print(f"95% VaR: {var_95:,.2f} USD")
```

**Test Focus**: Scenario batching, portfolio revaluation, P&L calculation

### Data Engineer

```python
# Scenario: ETL pipeline with finstack
from finstack import *
import polars as pl

# 1. Load portfolio from CSV
df_positions = pl.read_csv("positions.csv")

# 2. Build instruments and portfolio
builder = PortfolioBuilder("production-portfolio")
for row in df_positions.iter_rows(named=True):
    instrument = create_instrument_from_row(row)
    builder.position(row["id"], instrument, row["quantity"])

portfolio = builder.build()

# 3. Value portfolio
valuation = value_portfolio(portfolio, market, config)

# 4. Export to multiple formats
df_results = valuation.to_polars()
df_results.write_parquet("valuation_results.parquet")
df_results.write_csv("valuation_results.csv")

# 5. Aggregate by sector
sectors = aggregate_by_attribute(valuation, portfolio.positions, "sector", "USD")
df_sectors = pl.DataFrame({"sector": sectors.keys(), "value": sectors.values()})
```

**Test Focus**: DataFrame I/O, bulk operations, data format conversions

## 📈 Success Criteria

For beta to be considered successful:

1. **Response Rate**: ≥5 completed feedback surveys
2. **Satisfaction**: Average rating ≥4.0/5.0 across all categories
3. **Blocker Bugs**: Zero critical bugs preventing real-world usage
4. **Documentation**: ≥80% of testers rate docs as "clear" or "very clear"
5. **Performance**: ≥80% of testers rate performance as "acceptable"

## 🐛 Known Issues

Current known limitations in beta 1:

1. **Bucketed Metrics**: DV01/CS01 bucketing not yet exposed (scalar only)
2. **Custom Pricers**: Custom pricer registration not exposed (use built-ins)
3. **Some Exotic Instruments**: A few specialized instruments pending
4. **Windows Binary**: Windows wheel may require manual compilation

## 💬 Communication Channels

- **Slack**: #finstack-beta-testing (private channel)
- **Email**: beta-testing@finstack.io
- **GitHub Issues**: Use `beta-feedback` label
- **Office Hours**: Tuesdays 2-3pm EST (Zoom link in Slack)

## 📅 Next Steps After Beta

Based on feedback, we will:

1. **Week 15**: Analyze feedback, prioritize issues
2. **Week 16**: Fix critical blockers, release beta 2 (if needed)
3. **Week 17**: Final polish, prepare v1.0.0 GA
4. **Week 18**: GA release with full documentation

## 🙏 Thank You!

Your participation in beta testing is invaluable. Thank you for helping make finstack better for the entire quantitative finance community!

---

**Questions?** Contact beta-testing@finstack.io or ping @finstack-team in Slack.
