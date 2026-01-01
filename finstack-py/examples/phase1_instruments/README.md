# Phase 1 Instruments - Examples

This directory contains comprehensive, runnable examples demonstrating the usage of all Phase 1 instruments added to the Python bindings.

## Examples Included

### Fixed Income Instruments

1. **bond_future_example.py** - Government bond futures with deliverable baskets
   - UST 10-year future (TY contract)
   - Mark-to-market and fair value pricing
   - Cheapest-to-deliver (CTD) bond analysis
   - P&L calculation

2. **xccy_swap_example.py** - Cross-currency floating-for-floating swaps
   - USD/EUR basis swap with notional exchange
   - Dual-curve pricing (SOFR + ESTR)
   - FX risk management
   - NPV decomposition by leg

3. **inflation_capfloor_example.py** - Year-on-year inflation caps/floors
   - US CPI-U cap with Black-76 pricing
   - Caplet valuation and Greeks
   - Sensitivity to volatility

### Equity & FX Instruments

4. **equity_index_future_example.py** - Equity index futures
   - E-mini S&P 500 future (ES)
   - Mark-to-market and cost-of-carry models
   - Basis and carry cost analytics
   - Contract specification handling

5. **ndf_example.py** - Non-deliverable forwards
   - USD/CNY 3-month NDF
   - Pre-fixing and post-fixing modes
   - Covered interest rate parity
   - Settlement cashflow calculation

6. **fx_variance_swap_example.py** - FX variance swaps
   - EUR/USD variance swap with daily observations
   - Realized variance tracking
   - Implied forward variance from vol surface
   - Payoff decomposition

### Commodity & Real Estate

7. **commodity_option_example.py** - Commodity options (calls/puts)
   - WTI crude oil call option (European)
   - WTI put option (American with early exercise)
   - Black-76 and binomial tree pricing
   - Greeks (delta, gamma, vega, theta, rho)

8. **real_estate_example.py** - Real estate valuation
   - Direct capitalization method
   - Discounted cashflow (DCF) method
   - NOI projection and exit value
   - Cap rate sensitivity analysis

## Prerequisites

Ensure you have finstack-py installed:

```bash
# From the finstack-py directory
maturin develop --release
```

## Running the Examples

Each example is standalone and can be run directly:

```bash
cd finstack-py/examples/phase1_instruments

# Run individual examples
python bond_future_example.py
python xccy_swap_example.py
python inflation_capfloor_example.py
python equity_index_future_example.py
python ndf_example.py
python fx_variance_swap_example.py
python commodity_option_example.py
python real_estate_example.py
```

## Example Output

Each example produces formatted output showing:

- Instrument configuration parameters
- Market data inputs
- Valuation results (NPV/PV)
- Risk metrics (DV01, CS01, Greeks where applicable)
- Additional analytics specific to the instrument type

Example output format:

```
======================================================================
Bond Future Valuation Results
======================================================================
Contract ID:        TYH5
Contract Specs:     UST 10Y Future
Position:           Long
Quantity:           10
Entry Price:        125.5000
Quoted Price:       126.2500

Present Value:      23,437.50 USD

Metrics:
  Clean Price:      126.2500
  Dirty Price:      127.1234
  Accrued Interest: 873.44
  CTD Bond:         US912828XG33
  Gross Basis:      0.1234 (32nds)
======================================================================
```

## Key Concepts Demonstrated

### Pricing Methodologies

- **Discounting**: Present value with market discount curves
- **Black-76**: European option pricing for commodities and rates
- **Binomial Trees**: American option pricing with early exercise
- **Cost-of-Carry**: Fair value for futures contracts
- **Variance Replication**: FX variance swap pricing

### Market Data Integration

- Discount curves (OIS, government)
- Forward curves (SOFR, ESTR, commodity forwards)
- FX matrices and spot rates
- Volatility surfaces (equity, FX, commodity, inflation)
- Inflation indices (CPI-U)

### Risk Metrics

- **DV01**: Dollar value of 1bp rate shift
- **CS01**: Credit spread sensitivity
- **Greeks**: Delta, Gamma, Vega, Theta, Rho
- **FX Delta**: Currency exposure
- **Basis**: Fair value vs. quoted price difference

## Modifying the Examples

All examples use realistic but simplified market data. To adapt for production use:

1. **Replace market data sources**:
   - Load curves from Bloomberg, Refinitiv, or internal systems
   - Use actual volatility surfaces
   - Incorporate real-time FX rates

2. **Adjust instrument parameters**:
   - Use actual contract specifications
   - Update notionals, strikes, and dates
   - Modify settlement conventions

3. **Extend metrics**:
   - Add bucketed DV01 (when available in Phase 2)
   - Compute stress scenarios
   - Generate time-series valuations

## Documentation

Each instrument wrapper has comprehensive NumPy-style docstrings accessible via:

```python
from finstack.valuations.instruments import BondFuture
help(BondFuture)
help(BondFuture.builder)
```

For detailed API reference, see the main documentation.

## Testing

Examples can be tested with pytest:

```bash
pytest examples/phase1_instruments/ --doctest-modules
```

## Troubleshooting

### Import Errors

If you get `ModuleNotFoundError: No module named 'finstack'`, rebuild the Python package:

```bash
cd finstack-py
maturin develop --release
```

### Market Data Errors

If pricing fails with "Curve not found" or similar, verify:
- Curve IDs match between instrument and market context
- Discount curves exist for all required currencies
- Forward curves exist for floating legs
- Volatility surfaces cover the required tenors/strikes

### Numerical Issues

If results differ from expected values:
- Check day count conventions (Act/360 vs Act/365F)
- Verify business day conventions (Following, Modified Following)
- Ensure calendar IDs are valid (USNY, GBLO, etc.)
- Confirm interpolation methods match expectations

## Support

For issues or questions:
- Check the main finstack-py documentation
- Review the Rust API documentation (cargo doc)
- Consult the pricing methodology guide
- Open a GitHub issue with reproduction steps

## License

These examples are part of the finstack-py project and are provided under the same license.
