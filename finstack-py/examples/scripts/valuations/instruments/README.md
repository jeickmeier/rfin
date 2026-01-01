# Valuations Instruments - Examples

This directory contains comprehensive, runnable examples demonstrating the usage of all instruments available in the Finstack Python bindings.

## Examples by Instrument Category

### Fixed Income

| Example | Instruments Covered |
|---------|---------------------|
| `bond_capabilities.py` | Bond (fixed, floating, zero-coupon, callable, PIK) |
| `irs_capabilities.py` | InterestRateSwap |
| `rates_capabilities.py` | Deposit, ForwardRateAgreement, InterestRateFuture, InterestRateOption (caps/floors), BasisSwap, Swaption |
| `floating_rate_with_curves_example.py` | Floating rate note pricing with curves |
| `inflation_capabilities.py` | InflationLinkedBond, InflationSwap |
| `term_loan_example.py` | TermLoan (fixed rate, PIK, serialization) |
| `repo_capabilities.py` | Repo |

### Credit

| Example | Instruments Covered |
|---------|---------------------|
| `credit_capabilities.py` | CreditDefaultSwap, CdsIndex, CdsOption, CdsTranche |
| `structured_credit_capabilities.py` | StructuredCredit (ABS, CLO, CMBS, RMBS) |
| `cms_option_example.py` | CmsOption |

### Equity

| Example | Instruments Covered |
|---------|---------------------|
| `equity_capabilities.py` | Equity, EquityOption |
| `basket_capabilities.py` | Basket |
| `barrier_option_example.py` | BarrierOption |
| `asian_option_example.py` | AsianOption |
| `lookback_option_example.py` | LookbackOption |
| `cliquet_option_example.py` | CliquetOption |
| `quanto_option_example.py` | QuantoOption |
| `autocallable_example.py` | Autocallable |
| `range_accrual_example.py` | RangeAccrual |
| `convertible_capabilities.py` | ConvertibleBond |
| `variance_swap_capabilities.py` | VarianceSwap |
| `trs_capabilities.py` | EquityTotalReturnSwap, FiIndexTotalReturnSwap |

### FX

| Example | Instruments Covered |
|---------|---------------------|
| `fx_capabilities.py` | FxSpot, FxOption, FxSwap, FxBarrierOption |

### Private Markets

| Example | Instruments Covered |
|---------|---------------------|
| `private_markets_capabilities.py` | PrivateMarketsFund |
| `revolving_credit/` | RevolvingCredit (deterministic and stochastic pricing) |

### Monte Carlo Demonstrations

| Example | Description |
|---------|-------------|
| `mc_path_capture_example.py` | Monte Carlo path capture and analysis |
| `mc_visualization_demo.py` | Monte Carlo visualization techniques |

## Prerequisites

Ensure you have finstack-py installed:

```bash
# From the finstack-py directory
maturin develop --release

# Or using uv
uv pip install -e .
```

## Running the Examples

Each example is standalone and can be run directly:

```bash
cd finstack-py/examples/scripts/valuations/instruments

# Run individual examples
uv run python bond_capabilities.py
uv run python rates_capabilities.py
uv run python term_loan_example.py
uv run python credit_capabilities.py
# etc.

# Or run all examples via the runner script
cd finstack-py/examples/scripts
uv run python run_all_examples.py
```

## Key Concepts Demonstrated

### Pricing Methodologies

- **Discounting**: Present value with market discount curves
- **Black-Scholes/Black-76**: Option pricing
- **Monte Carlo**: Path-dependent derivative pricing
- **Binomial Trees**: American option pricing with early exercise

### Market Data Integration

- Discount curves (OIS, government)
- Forward curves (SOFR, ESTR, etc.)
- FX matrices and spot rates
- Volatility surfaces (equity, FX, rates)
- Hazard curves (credit)
- Inflation curves (CPI)

### Risk Metrics

- **DV01**: Dollar value of 1bp rate shift
- **CS01**: Credit spread sensitivity
- **Greeks**: Delta, Gamma, Vega, Theta, Rho
- **Duration**: Macaulay and Modified duration
- **Convexity**: Interest rate convexity

## Instrument Coverage Summary

All instruments from `finstack.valuations.instruments` are covered:

- ✅ Bond, Deposit, InterestRateSwap, ForwardRateAgreement
- ✅ InterestRateOption, InterestRateFuture, BasisSwap, Swaption
- ✅ InflationLinkedBond, InflationSwap
- ✅ FxSpot, FxOption, FxSwap, FxBarrierOption
- ✅ Equity, EquityOption, LookbackOption, CliquetOption
- ✅ ConvertibleBond, QuantoOption, RangeAccrual
- ✅ CreditDefaultSwap, CdsIndex, CdsOption, CdsTranche, CmsOption
- ✅ StructuredCredit, Repo
- ✅ RevolvingCredit, TermLoan
- ✅ EquityTotalReturnSwap, FiIndexTotalReturnSwap
- ✅ VarianceSwap, AsianOption, Autocallable
- ✅ Basket, BarrierOption, PrivateMarketsFund

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

## License

These examples are part of the finstack-py project and are provided under the same license.
