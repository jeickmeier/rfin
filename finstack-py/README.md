# finstack Python bindings

Python-friendly access to the [finstack](https://github.com/finstacklabs/rfin) Rust crates. The
package wraps the `finstack-core` primitives (currencies, configuration, money, and holiday
calendars) without introducing new business logic, making it easy to drive analytics and
prototyping directly from Python notebooks.

## Installation

Use [maturin](https://www.maturin.rs/) (or `uv`/`pip`) to build and install the extension:

```bash
uv run maturin develop --release
```

This compiles the Rust crate and exposes the `finstack` module to your active Python environment.

## Quick start

```python
from datetime import date
from finstack import Currency, Money, BusinessDayConvention, adjust, get_calendar

usd = Currency("USD")
amount = Money(1_000_000, usd)
print(amount.format())  # "USD 1000000.00"

calendar = get_calendar("usny")
adjusted = adjust(date(2025, 1, 4), BusinessDayConvention.FOLLOWING, calendar)
print(adjusted)  # date(2025, 1, 6)
```

## Optional Python dependencies

The core extension has no required Python dependencies. Install the `analytics` extra if you plan to
work with numpy/pandas/polars alongside the bindings:

```bash
pip install finstack[analytics]
```

## Generating type stubs

The bindings are compiled with PyO3's docstrings and signatures. To generate `.pyi` stub files once
the API settles, run:

```bash
uv run pyo3-stubgen finstack
```

Place the generated files under `finstack-py/finstack/` and add them to the `tool.maturin.include`
list if you want to ship them in wheels.
