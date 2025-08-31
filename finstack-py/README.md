# Finstack Python Bindings

Python bindings for the Finstack financial computation library.

## Installation

```bash
pip install finstack
```

## Development

### Prerequisites

- Rust 1.78+
- Python 3.8+
- [uv](https://github.com/astral-sh/uv) (recommended)

### Quick Setup with uv

From the project root:

```bash
# Install uv if not already installed
curl -LsSf https://astral.sh/uv/install.sh | sh

# Run the setup script
./scripts/setup-python.sh

# Or manually:
uv venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate
cd finstack-py && maturin develop --release
```

### Traditional Setup

```bash
# Install maturin
pip install maturin

# Build the wheel
maturin build

# Install in development mode
maturin develop
```

### Running tests

```bash
# With uv
uv run pytest

# Traditional
pytest
```

### Development workflow

```bash
# Format and lint
uv run black .
uv run ruff check .

# Type checking
uv run mypy .

# Run examples
uv run python ../examples/python/dates_python_example.py
uv run python ../examples/python/primitives_python_example.py
uv run python ../examples/python/cashflow_leg_example.py
uv run python ../examples/python/market_data_example.py
```

## Usage

```python
import finstack

# Using dates module
from finstack.dates import Date, Calendar

# Using primitives module  
from finstack.money import Money
from finstack.currency import Currency

# Create a currency
usd = Currency("USD")

# Create money
amount = Money(100.0, usd)

# Using market data module
from finstack.market_data import DiscountCurve, InterpStyle

# Create a discount curve
curve = DiscountCurve(
    id="USD-OIS",
    base_date=Date.from_ymd(2025, 1, 1),
    times=[0.0, 1.0, 5.0],
    discount_factors=[1.0, 0.97, 0.85],
    interpolation=InterpStyle.MonotoneConvex,
)

# Get discount factor
df = curve.df(2.5)
```

## Modules

### Core Modules

- **dates**: Date arithmetic, calendars, day count conventions
- **primitives**: Currency and Money types
- **cashflow**: Cash flow generation and NPV calculations
- **market_data**: Financial curves, surfaces, and interpolation

### Market Data Features

The `market_data` module provides:

- **Discount Curves**: For pricing with various interpolation methods
- **Forward Curves**: For modeling forward rates (e.g., SOFR, EURIBOR)
- **Hazard Curves**: For credit risk modeling
- **Inflation Curves**: For real/nominal calculations
- **Volatility Surfaces**: For option pricing
- **MarketContext**: Container for managing multiple curves

#### Interpolation Methods

All curves support multiple interpolation styles:
- `Linear`: Simple linear interpolation
- `LogLinear`: Linear in log space (constant rates)
- `MonotoneConvex`: Hagan-West method (shape-preserving)
- `CubicHermite`: Smooth cubic spline
- `FlatFwd`: Piecewise constant forward rates

### Examples

See the `examples/python/` directory for complete examples:
- `dates_python_example.py`: Working with dates and calendars
- `primitives_python_example.py`: Currency and money operations
- `cashflow_leg_example.py`: Cash flow generation
- `market_data_example.py`: Comprehensive market data usage