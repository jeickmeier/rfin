# Finstack Python Cookbook

**Comprehensive workflow examples for quantitative finance professionals**

This cookbook provides end-to-end examples demonstrating how to use Finstack for common quantitative finance workflows. Each example is designed to be copy-paste ready and demonstrates production-quality patterns.

---

## Table of Contents

### Portfolio Management (Examples 01-05)

- **01_build_portfolio.py** - Multi-asset portfolio construction with entities and books
- **02_stress_test.py** - Scenario-based stress testing with market shocks
- **03_risk_report.py** - Comprehensive DV01/CS01/Greeks risk reporting
- **04_portfolio_optimization.py** - Constrained portfolio construction with ratings limits
- **05_margin_and_netting.py** - Margin calculation with CSA terms and netting sets

### Fixed Income & Credit (Examples 06-10)

- **06_calibrate_curves.py** - Bootstrap discount, forward, and credit curves
- **07_term_loan_model.py** - Term loan modeling with covenants and waterfalls
- **08_bond_analytics.py** - Bond pricing, yields, duration, and convexity
- **09_credit_analysis.py** - CDS pricing, hazard curves, and default probabilities
- **10_revolving_credit.py** - Revolving credit facility pricing and analytics

### Derivatives Pricing (Examples 11-15)

- **11_exotic_options.py** - Price barrier, Asian, lookback, and quanto options
- **12_mc_pricing.py** - Monte Carlo vs analytical pricing comparison
- **13_swaptions_caps_floors.py** - Interest rate options with SABR vol
- **14_commodity_options.py** - Commodity futures options and calendar spreads
- **15_fx_derivatives.py** - FX options, NDF, and variance swaps

### Advanced Analytics (Examples 16-20)

- **16_pnl_attribution.py** - Daily P&L attribution with carry, theta, and market moves
- **17_real_estate_dcf.py** - Real estate DCF with rent rolls and cap rates
- **18_private_equity_fund.py** - PE fund modeling with J-curve and waterfall
- **19_cross_currency_swap.py** - Cross-currency swap pricing and FX risk
- **20_inflation_linked.py** - Inflation-linked bonds and swaps

### Integrated Workflows (Examples 21-25)

- **21_full_portfolio_workflow.py** - Complete workflow: build, price, stress, optimize
- **22_statement_modeling.py** - Financial statement modeling with forecasts and scenarios
- **23_covenant_monitoring.py** - Automated covenant testing and alerts
- **24_multi_curve_framework.py** - Post-2008 multi-curve discount/forward setup
- **25_var_calculation.py** - Historical and parametric VaR with confidence intervals

---

## User Personas Covered

### 1. **Equity Analyst**

- Examples: 11 (equity options), 16 (P&L attribution), 18 (PE fund)
- **Focus**: Equity derivatives, fund modeling, performance attribution

### 2. **Credit Analyst**

- Examples: 07 (term loan), 09 (credit analysis), 10 (revolving credit), 23 (covenants)
- **Focus**: Credit instruments, default risk, covenant monitoring

### 3. **Quantitative Researcher**

- Examples: 06 (calibration), 11 (exotic options), 12 (MC pricing), 24 (multi-curve)
- **Focus**: Pricing models, numerical methods, market data calibration

### 4. **Portfolio Manager**

- Examples: 01 (portfolio construction), 02 (stress test), 04 (optimization), 21 (full workflow)
- **Focus**: Portfolio management, risk limits, optimization

### 5. **Risk Analyst**

- Examples: 03 (risk report), 05 (margin), 16 (P&L attribution), 25 (VaR)
- **Focus**: Market risk, credit risk, counterparty risk, VaR

---

## Running the Examples

### Prerequisites

```bash
# Install finstack-py with all optional dependencies
pip install finstack[polars,pandas]

# Or build from source
cd finstack-py
maturin develop --release
pip install polars pandas matplotlib
```

### Run Individual Examples

```bash
# Run single example
python finstack-py/examples/cookbook/01_build_portfolio.py

# Run with verbose output
python finstack-py/examples/cookbook/06_calibrate_curves.py --verbose
```

### Run All Examples (Test Suite)

```bash
# Run all cookbook examples
pytest finstack-py/examples/cookbook/ --doctest-modules -v

# Run specific category
pytest finstack-py/examples/cookbook/ -k "portfolio" -v
pytest finstack-py/examples/cookbook/ -k "option" -v
```

---

## Example Structure

Each example follows a consistent structure:

```python
"""
Title: Brief Description
Persona: [Equity Analyst | Credit Analyst | ...]
Complexity: [Beginner | Intermediate | Advanced]
Runtime: ~X seconds

Description:
Multi-line description of what the example demonstrates.

Key Concepts:
- Concept 1
- Concept 2
- Concept 3

Prerequisites:
- Knowledge area 1
- Knowledge area 2
"""

def main():
    """Main workflow implementation."""
    # 1. Setup
    # 2. Data preparation
    # 3. Computation
    # 4. Analysis
    # 5. Output
    pass

if __name__ == "__main__":
    main()
```

---

## Common Patterns

### 1. **Market Data Setup**

Most examples use this pattern for market data:

```python
from finstack import MarketContext, DiscountCurve, FxMatrix, Date

# Create market context
market = MarketContext()
market.set_as_of(Date(2024, 1, 15))

# Add discount curve
curve = DiscountCurve.flat(
    id="USD.OIS",
    base_date=Date(2024, 1, 15),
    rate=0.05,
    day_count="Act360"
)
market.insert_discount(curve)

# Add FX rates
fx_matrix = FxMatrix()
fx_matrix.set_spot("USD", "EUR", 0.92)
market.set_fx_matrix(fx_matrix)
```

### 2. **Instrument Construction**

```python
from finstack import Bond, Money

# Create bond
bond = Bond.fixed_semiannual(
    id="ACME.5Y",
    notional=Money.from_code(1_000_000, "USD"),
    coupon_rate=0.05,
    issue_date=Date(2024, 1, 15),
    maturity_date=Date(2029, 1, 15),
    discount_curve_id="USD.OIS"
)
```

### 3. **Pricing and Metrics**

```python
from finstack import create_standard_registry

# Create pricer registry
registry = create_standard_registry()

# Price with metrics
result = registry.price_bond_with_metrics(
    bond,
    model="discounting",
    market=market,
    metrics=["clean_price", "accrued", "duration_mod", "dv01", "convexity"]
)

# Extract results
pv = result.present_value.amount
clean_price = result.metric("clean_price")
dv01 = result.metric("dv01")
```

### 4. **DataFrame Export**

```python
# Export to Polars/Pandas for analysis
df = results.to_polars()
df.write_csv("results.csv")
df.write_parquet("results.parquet")

# Or via pandas
df_pd = results.to_pandas()
df_pd.to_excel("results.xlsx")
```

---

## Learning Path

### Beginner (Start Here)

1. **01_build_portfolio.py** - Learn portfolio basics
2. **08_bond_analytics.py** - Understand bond pricing
3. **06_calibrate_curves.py** - Build market data

### Intermediate

4. **02_stress_test.py** - Scenario analysis
5. **03_risk_report.py** - Risk metrics
6. **11_exotic_options.py** - Options pricing

### Advanced

7. **12_mc_pricing.py** - Numerical methods
8. **21_full_portfolio_workflow.py** - Complete workflows
9. **24_multi_curve_framework.py** - Advanced market data

---

## Performance Tips

### 1. **Batch Operations**

```python
# Instead of pricing one-by-one
for bond in bonds:
    result = registry.price_bond(bond, market)

# Use bulk pricing (when available)
results = registry.price_bonds_bulk(bonds, market)
```

### 2. **Reuse Market Context**

```python
# Create once, reuse many times
market = create_market_context()

# Price multiple instruments
for instrument in portfolio:
    result = registry.price(instrument, market)
```

### 3. **DataFrame Operations**

```python
# Use Polars for fast operations
df = results.to_polars()
summary = df.group_by("currency").agg([
    pl.col("present_value").sum(),
    pl.col("dv01").sum()
])
```

---

## Troubleshooting

### Common Issues

**1. Missing Market Data**

```python
# Always check market context has required data
if not market.has_discount("USD.OIS"):
    raise ValueError("Missing discount curve: USD.OIS")
```

**2. Currency Mismatch**

```python
# Ensure all instruments use consistent currencies
# Or provide FX conversion via FxMatrix
```

**3. Date Ordering**

```python
# Ensure dates are properly ordered
assert issue_date <= settlement_date <= maturity_date
```

---

## Contributing

To add a new cookbook example:

1. Follow the standard structure (see above)
2. Include comprehensive docstring
3. Add to this README in appropriate section
4. Ensure example runs successfully: `python your_example.py`
5. Add test: `pytest your_example.py --doctest-modules`

---

## References

- **API Documentation**: [finstack-py API docs](https://finstack.readthedocs.io)
- **Rust Core**: `finstack/` (Rust implementation)
- **Python Bindings**: `finstack-py/src/` (PyO3 wrappers)
- **Additional Examples**: `finstack-py/examples/scripts/` (component examples)
- **Notebooks**: `finstack-py/examples/notebooks/` (Jupyter tutorials)

---

## Support

For questions or issues:
- **GitHub Issues**: <https://github.com/your-org/finstack/issues>
- **Discussions**: <https://github.com/your-org/finstack/discussions>
- **Documentation**: <https://finstack.readthedocs.io>

---

**Last Updated**: January 2025
**Examples Count**: 25
**Total Lines**: ~10,000+
