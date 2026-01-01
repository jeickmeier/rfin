# Finstack Python Examples

This directory contains comprehensive examples and tutorials for using the finstack Python bindings.

## Directory Structure

### `phase1_instruments/`

Comprehensive examples demonstrating all Phase 1 instruments added to the Python bindings:

- **Fixed Income**: Bond futures, cross-currency swaps, inflation caps/floors
- **Equity & FX**: Equity index futures, NDFs, FX variance swaps
- **Commodity & Real Estate**: Commodity options, real estate valuation

Each example includes:
- Complete instrument construction
- Market data setup
- Pricing and valuation
- Risk metrics computation
- Practical output interpretation

[View Phase 1 Instruments README](phase1_instruments/README.md)

### `notebooks/`

Jupyter notebooks for interactive exploration (TBD - to be populated in future phases).

### `scripts/`

Utility scripts for data generation and testing (existing).

### `outputs/`

Generated outputs and reports from examples (existing).

## Quick Start

1. **Install finstack-py**:
   ```bash
   cd finstack-py
   maturin develop --release
   ```

2. **Run an example**:
   ```bash
   python examples/phase1_instruments/bond_future_example.py
   ```

3. **Explore instrument types**:
   - Fixed income: `bond_future_example.py`, `xccy_swap_example.py`, `inflation_capfloor_example.py`
   - Equity/FX: `equity_index_future_example.py`, `ndf_example.py`, `fx_variance_swap_example.py`
   - Alternatives: `commodity_option_example.py`, `real_estate_example.py`

## Documentation

Each instrument has NumPy-style docstrings with examples:

```python
from finstack.valuations.instruments import BondFuture
help(BondFuture)
```

For API reference, see the main finstack-py documentation.

## Contributing

When adding new examples:

1. Place in appropriate subdirectory
2. Include comprehensive docstrings
3. Use realistic market data
4. Add output interpretation
5. Update relevant README files

## Support

For questions or issues with examples:
- Check instrument docstrings: `help(InstrumentClass)`
- Review the phase1_instruments README
- Consult the main documentation
- Open a GitHub issue

## License

Examples are part of the finstack-py project and are provided under the same license.
