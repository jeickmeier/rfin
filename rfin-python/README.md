# RustFin Python Bindings

Python bindings for the RustFin financial computation library.

## Installation

```bash
pip install rfin
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
cd rfin-python && maturin develop --release
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

# Run example
uv run python ../examples/python_example.py
```

## Usage

```python
import rfin

# Using dates module
from rfin.dates import Date, Calendar

# Using primitives module  
from rfin.primitives import Money, Currency

# Create a currency
usd = Currency("USD")

# Create money
amount = Money(100.0, usd)
```