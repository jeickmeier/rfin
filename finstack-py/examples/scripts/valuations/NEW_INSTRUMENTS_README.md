# New Exotic Options Examples

This directory contains example files for newly added exotic option instruments:

- `asian_option_example.py` - Asian options with arithmetic/geometric averaging
- `autocallable_example.py` - Autocallable structured products
- `barrier_option_example.py` - Barrier options (up/down, in/out)
- `cliquet_option_example.py` - Cliquet (ratchet) options
- `cms_option_example.py` - CMS caps and floors
- `lookback_option_example.py` - Lookback options (fixed/floating strike)
- `quanto_option_example.py` - Quanto options (cross-currency)
- `range_accrual_example.py` - Range accrual notes
- `revolving_credit_example.py` - Revolving credit facilities

## Current Status

**Note**: These instruments are currently being finalized and tested. The examples demonstrate
the expected API patterns but may require updates as the instrument implementations are completed.

The instruments have Python bindings and are registered in the module, but full pricing functionality
may require additional configuration or may be in development.

## Expected Usage Pattern

```python
from datetime import date
from finstack import Money
from finstack.core.currency import USD
from finstack.valuations.instruments import AsianOption

# Create instrument
option = AsianOption.builder(
    instrument_id="ASIAN_001",
    ticker="AAPL",
    strike=150.0,
    expiry=date(2026, 6, 30),
    fixing_dates=[date(2026, 3, 31), date(2026, 6, 30)],
    notional=Money(10000.0, USD),
    discount_curve="USD.SOFR",
    spot_id="AAPL",
    vol_surface="AAPL.VOL",
    averaging_method="arithmetic",
    option_type="call",
    dividend_yield_id="AAPL.DIV",
)
```

For working examples of similar instruments, see:
- `equity_capabilities.py` - Equity and equity options
- `fx_capabilities.py` - FX options
- `variance_swap_capabilities.py` - Variance swaps

