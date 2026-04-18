# finstack Python bindings

`finstack-py` builds the Python package named `finstack`, providing Python
access to the Rust Finstack workspace without moving pricing or analytics logic
out of Rust. The public package mirrors the Rust umbrella crate structure, and
the package-level modules are loaded lazily so importing `finstack` does not
eagerly import every domain.

## Available Modules

The current top-level Python package exposes these subpackages:

- `finstack.analytics`
- `finstack.core`
- `finstack.margin`
- `finstack.monte_carlo`
- `finstack.portfolio`
- `finstack.scenarios`
- `finstack.statements`
- `finstack.statements_analytics`
- `finstack.valuations` (includes `finstack.valuations.correlation`)

Each subpackage is a thin wrapper over the corresponding Rust crate domain.

## Build And Install

From the repository root, the recommended path is:

```bash
make python-dev
```

That installs the Python dependencies defined in the root `pyproject.toml` and
builds the extension with the Rust **dev** profile (fast compile).

For a release build (slower compile, faster runtime — e.g. large portfolio runs):

```bash
make python-dev-release
```

If you want to build the extension directly:

```bash
cd finstack-py
uv run python -m maturin develop          # dev (default)
uv run python -m maturin develop --release
```

`make python-dev-debug` is an alias for `make python-dev`.

## Quick Start

```python
from datetime import date

from finstack.core.currency import Currency
from finstack.core.dates import BusinessDayConvention, adjust, get_calendar
from finstack.core.money import Money

usd = Currency("USD")
amount = Money(1_000_000, usd)

calendar = get_calendar("usny")
settle = adjust(date(2025, 1, 4), BusinessDayConvention.FOLLOWING, calendar)

print(amount.format())
print(settle)
```

## Package Structure

The package tree under `finstack-py/finstack/` follows the Rust domain layout:

- `analytics/`
- `core/`
- `correlation/`
- `margin/`
- `monte_carlo/`
- `portfolio/`
- `scenarios/`
- `statements/`
- `statements_analytics/`
- `valuations/`

The Rust-side bindings live under `finstack-py/src/bindings/`, again split by
domain. Public type stubs live alongside the Python package as `.pyi` files.

## Examples And Notebooks

The live Python examples are notebook-first and live under
`finstack-py/examples/`. The curriculum currently includes:

- `01_foundations`
- `02_pricing`
- `03_analytics`
- `04_statement_modeling`
- `05_portfolio_and_scenarios`
- `06_advanced_quant`
- `07_capstone`

Start with the examples index:

- `finstack-py/examples/README.md`

Run the notebook suite from the repository root:

```bash
uv run python finstack-py/examples/run_all_notebooks.py
```

Run a single section:

```bash
uv run python finstack-py/examples/run_all_notebooks.py --directory 05_portfolio_and_scenarios
```

## Stubs, Parity, And Testing

The Python package ships manually maintained `.pyi` stubs in
`finstack-py/finstack/`. Runtime and parity coverage lives under
`finstack-py/tests/`, including `finstack-py/tests/parity/`.

Useful checks from the repository root:

```bash
uv run pyright
uv run ty check finstack-py/finstack
uv run pytest finstack-py/tests
```

## Relationship To Rust And WASM

The Python package is one binding surface for the same Rust workspace described
in the repository root `README.md`. The repository also ships
`finstack-wasm`, which follows the same domain layout for browser and Node.js
consumers.

## License

MIT OR Apache-2.0
