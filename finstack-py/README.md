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
mise run python-build
```

That installs the Python dependencies defined in the root `pyproject.toml` and
builds the extension with the Rust **dev** profile (fast compile).

For a release build (slower compile, faster runtime — e.g. large portfolio runs):

```bash
mise run python-build -- --release
```

If you want to build the extension directly:

```bash
cd finstack-py
uv run python -m maturin develop          # dev (default)
uv run python -m maturin develop --release
```

`mise run python-build` builds with the dev profile (fast compile, slower runtime); use `mise run python-build -- --release` for the optimized release build.

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
`finstack-py/finstack/`. Runtime coverage lives under `finstack-py/tests/`;
structural parity checks live under `finstack-py/tests/parity/`.

Useful checks from the repository root:

```bash
uv run pyright
uv run ty check finstack-py/finstack
uv run pytest finstack-py/tests
uv run pytest finstack-py/tests/parity
```

### Parity contract

`finstack-py/parity_contract.toml` is the authoritative spec for the
Python-visible API surface that parity tests pin. When you add or rename a
binding, update the contract in the same change. The structural parity tests
under `finstack-py/tests/parity/` enforce that:

- Every entry in the contract resolves to an importable Python symbol.
- Names match Rust source 1:1 (per `AGENTS.md` naming-strategy rules).
- Public modules marked `exists` or `flattened` import successfully, while
  modules marked `missing` remain absent until the contract changes.

Behavioral parity cases that compare Rust-backed results live alongside the
runtime tests, for example `finstack-py/tests/test_core_parity.py`.

## Type Discovery

Python types live alongside their Rust counterparts. A few common entry
points:

| Concept | Python module | Key types |
|---------|---------------|-----------|
| Money / currency | `finstack.core.money`, `finstack.core.currency` | `Money`, `Currency` |
| Rates and bps | `finstack.core.types` | `Rate`, `Bps`, `Percentage` |
| Credit ratings | `finstack.core.types` | `CreditRating` |
| Dates and calendars | `finstack.core.dates` | `Tenor`, `DayCount`, `Schedule`, `ScheduleBuilder`, `BusinessDayConvention` |
| Configuration | `finstack.core.config` | `FinstackConfig`, `RoundingMode`, `ToleranceConfig` |
| Discount and forward curves | `finstack.core.market_data` | `DiscountCurve`, `ForwardCurve`, `MarketContext` |
| Credit scoring | `finstack.core.credit.scoring` | `altman_z_score`, `ohlson_o_score`, `ScoringResult` |
| Pricers and metrics | `finstack.valuations` | `price_instrument`, instrument types |
| Performance and risk | `finstack.analytics` | `value_at_risk`, drawdown, return-series helpers |

For the full surface, browse `finstack-py/finstack/**/*.pyi` — every public
import has a stub with type annotations and docstrings.

## Common Pitfalls

A few things that surprise users coming from the Rust side or other
financial Python libraries:

### Decimal vs `float`

Per `INVARIANTS.md` §1, the Rust workspace uses `rust_decimal::Decimal` at
the money/accounting boundary (`finstack-core::money`) and `f64` everywhere
else (rates, vols, returns, derivative prices). The Python bindings expose
`f64` for ergonomic interop. If your downstream code needs exact
`decimal.Decimal` arithmetic, convert at your boundary:

```python
from decimal import Decimal
from finstack.core.money import Money

m = Money(123.45, "USD")
d = Decimal(m.format(decimals=2, show_currency=False))
```

### Builder pattern: in-place mutation

Rust builders use fluent self-return (`builder.frequency(x).stub_rule(y).build()`).
Python builders **mutate in place and return `None`** — chaining will fail.
Call setters sequentially on the same instance:

```python
b = ScheduleBuilder(start, end)
b.frequency("3M")               # returns None
b.stub_rule(StubKind.SHORT_FRONT)
schedule = b.build()
```

### Errors are `ValueError`

All fallible bindings raise `ValueError` (with the underlying Rust error
chain flattened into the message). The error-mapping helper lives at
`finstack-py/src/errors.rs`. There are no domain-specific exception
classes; check `str(exc)` if you need to discriminate.

### Naming follows Rust 1:1

Rust `snake_case` ↔ Python `snake_case` — no rename. If you find yourself
hunting for a Python name that "should" exist, search the Rust source first
— odds are the name is identical and the answer is in `finstack/<crate>/src/`.

## Documentation Style

For contributors adding or editing bindings, see
[`finstack-py/DOCS_STYLE.md`](DOCS_STYLE.md). It covers PyO3 docstring
conventions, `.pyi` NumPy-style format, financial conventions, and the
in-place-mutation contract for Python builders.

## Relationship To Rust And WASM

The Python package is one binding surface for the same Rust workspace described
in the repository root `README.md`. The repository also ships
`finstack-wasm`, which follows the same domain layout for browser and Node.js
consumers.

## License

MIT OR Apache-2.0
