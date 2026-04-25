## finstack-py documentation style

`finstack-py` is the PyO3-based Python binding for the `finstack` Rust workspace. The user-facing API is consumed via Python imports and IDE tooltips. Documentation must read naturally as Python docstrings while staying in sync with the Rust source.

This style guide is the Python counterpart to [`finstack-wasm/DOCS_STYLE.md`](../finstack-wasm/DOCS_STYLE.md). Both are governed by [`docs/DOCUMENTATION_STANDARD.md`](../docs/DOCUMENTATION_STANDARD.md).

### Where docs live

- **Source of truth**: Rust `///` doc comments on `#[pyfunction]`, `#[pyclass]`, and `#[pymethods]` items in `finstack-py/src/bindings/**`.
- **PyO3 mapping**: PyO3 forwards `///` doc comments verbatim into Python `__doc__`. Whatever you write in the Rust source is what users see at the Python REPL via `help(thing)`.
- **Type stubs**: `.pyi` files in `finstack-py/finstack/**`. These provide IDE tooltips, mypy typing, and a richer docstring surface than what fits naturally in Rust comments.
- **Notebooks**: `finstack-py/examples/notebooks/` — long-form learning material, indexed by level.
- **Parity contract**: `finstack-py/parity_contract.toml` — the exact Python API surface that parity tests pin.

### Required sections per binding

For every `#[pyfunction]`, `#[pyclass]`, classmethod, instance method, property:

#### 1. Summary

One sentence. Reads as a Python docstring would.

#### 2. Parameters / Returns / Raises

Use NumPy-style sections in `.pyi` files (this is what IDEs render best). In Rust binding `///` comments, use the standard rustdoc sections (`# Arguments`, `# Returns`, `# Errors`) — PyO3 forwards them as-is and they are still readable at `help(thing)`.

```rust
/// Construct a Money amount in the given currency.
///
/// # Arguments
///
/// * `amount` - Numeric amount, finite (no NaN or infinity).
/// * `currency` - Either a [`PyCurrency`] or an ISO-4217 code string.
///
/// # Returns
///
/// A new `Money` value pinned to `currency`.
///
/// # Errors
///
/// Raises `ValueError` if `amount` is non-finite or `currency` is not a
/// valid ISO-4217 code.
```

NumPy style in `.pyi`:

```python
def __init__(self, amount: float, currency: Currency | str) -> None:
    """Construct a Money amount in the given currency.

    Parameters
    ----------
    amount : float
        Numeric amount, finite (no NaN or infinity).
    currency : Currency or str
        Either a Currency or an ISO-4217 code string.

    Raises
    ------
    ValueError
        If amount is non-finite or currency is not a valid ISO-4217 code.

    Examples
    --------
    >>> Money(100.0, "USD")
    Money(100.00, USD)
    """
    ...
```

#### 3. Examples

Required for every public class, classmethod, and free function. Examples should be runnable as `>>>` doctests in `.pyi` (we don't run them automatically yet, but write them so we can opt into pytest doctest later).

For `#[pymethods]` accessor patterns where the example is identical to a sibling, you may reference the class-level example instead of duplicating.

#### 4. Conventions (when applicable)

State explicitly:

- **Rates**: decimal (`0.05` = 5%) vs basis points (`500.0` = 5%) vs continuously compounded.
- **Dates**: role of each (`as_of` vs `issue` vs `maturity` vs `accrual_*`).
- **Curves**: required IDs in `MarketContext` (e.g. `"USD-OIS"`).
- **Quote convention**: clean vs dirty, percent-of-par vs absolute.
- **Decimal vs float**: per [`INVARIANTS.md`](../INVARIANTS.md) §1, money values that flow to accounting / settlement / regulatory capital MUST be `Decimal` at the Rust boundary; bindings expose `f64`. Document if a Python user needs to convert back to `decimal.Decimal` for downstream work.

### Financial documentation rules (non-negotiable)

Mirror exactly the language in `finstack-wasm/DOCS_STYLE.md` so the triplets read identically across Rust / Python / WASM:

- **Rates**: always state whether inputs are decimal (e.g. `0.05`) or bps (e.g. `120.0`).
- **Dates**: clarify the role of each date (`as_of` valuation date vs `issue`/`start` vs `maturity`).
- **Curves**: document expected IDs and required market data (what must exist in `MarketContext`).
- **Prices**: clarify quote convention (clean vs dirty, percent-of-par vs absolute).
- **Sign conventions**: see [`INVARIANTS.md`](../INVARIANTS.md) §3 for cashflow sign convention by context.

### Builder pattern: in-place mutation, not fluent self-return

This is **the** Python-vs-Rust quirk:

- **Rust**: builders use fluent self-return (`builder.frequency(x).stub_rule(y).build()`).
- **Python (PyO3)**: PyO3 method bindings cannot return `&mut Self` cleanly, so Python builders are exposed with **in-place mutation** — methods return `None`, you call them sequentially on the same object, then call `.build()`.

Document this on every builder class in both the Rust binding source and the `.pyi`:

```python
class ScheduleBuilder:
    """Fluent builder for constructing date schedules.

    Note
    ----
    Methods on this class mutate the builder in place and return ``None``.
    Call them sequentially rather than chaining.

    Examples
    --------
    >>> from finstack.core.dates import ScheduleBuilder, BusinessDayConvention
    >>> from finstack.core.dates import StubKind
    >>> b = ScheduleBuilder(start_date, end_date)
    >>> b.frequency("3M")
    >>> b.stub_rule(StubKind.SHORT_FRONT)
    >>> b.adjust_with(BusinessDayConvention.MODIFIED_FOLLOWING, "usny")
    >>> schedule = b.build()
    """
```

### Dunder methods

Every PyO3-exposed class will eventually surface `__repr__`, `__str__`, `__hash__`, and rich-comparison dunders. Pick one rule and apply it consistently:

**Rule**: every dunder gets a one-line `///` doc comment in the Rust source, even if obvious. This costs 8-16 characters per method and prevents inconsistency drift across files.

Examples:

```rust
/// Return ``repr(self)``.
fn __repr__(&self) -> String { ... }

/// Return ``str(self)``.
fn __str__(&self) -> String { ... }

/// Hash by canonical key components.
fn __hash__(&self) -> u64 { ... }

/// Equality and ordering by canonical key.
fn __richcmp__(&self, other: &Self, op: CompareOp) -> bool { ... }
```

In `.pyi` stubs, dunders generally don't need a docstring (`...` is fine) — the IDE behaviour is intuitive and Python convention is to not document them.

### Naming consistency

Per [`AGENTS.md`](../AGENTS.md):

- Rust `snake_case` ↔ Python `snake_case` — **identical**, no rename.
- Use `#[pyo3(name = "…")]` only when forced by a Python collision (none in core today).
- Type names (Rust `Money` ↔ Python `Money`) are identical.

When you find yourself wanting to rename in the binding, rename the Rust source instead. See AGENTS.md §"Naming Strategy".

### Error conversion contract

Every fallible binding goes through a centralized helper in `finstack-py/src/errors.rs`:

- `core_to_py(err: finstack_core::Error)` — flattens the full source chain.
- `display_to_py(err: impl Display)` — for non-`finstack_core::Error` types whose `Display` is sufficient.
- `error_to_py(err: impl Error)` — escape hatch for full `std::error::Error` chains.

Document errors in the binding `///` comment using `# Errors` or `Raises ------`. Do **not** use `.unwrap()` or `.expect()` in non-test binding code — the workspace lints deny it.

### `.pyi` stub minimum bar

Every public binding needs a `.pyi` entry with:

- Full type annotations on every parameter and return.
- A docstring matching the binding's `///` comment (NumPy-style is preferred).
- Inclusion in the module's `__all__` list.
- Consistency with `finstack-py/parity_contract.toml`.

### Templates

#### Free function

```python
def altman_z_score(input: AltmanZScoreInput) -> ScoringResult:
    """Compute the original Altman Z-Score (1968) for public manufacturing firms.

    Z = 1.2 X1 + 1.4 X2 + 3.3 X3 + 0.6 X4 + 1.0 X5

    Parameters
    ----------
    input : AltmanZScoreInput
        Five financial ratios (working capital, retained earnings, EBIT,
        market equity, sales, all scaled to total assets).

    Returns
    -------
    ScoringResult
        Raw Z score, zone classification (Safe/Grey/Distress), and an
        empirically-mapped implied probability of default.

    Raises
    ------
    ValueError
        If any input ratio is NaN or infinite.

    References
    ----------
    Altman, E. I. (1968). "Financial Ratios, Discriminant Analysis and the
    Prediction of Corporate Bankruptcy." Journal of Finance, 23(4), 589-609.

    Examples
    --------
    >>> from finstack.core.credit.scoring import (
    ...     AltmanZScoreInput, altman_z_score, ScoringZone,
    ... )
    >>> healthy = AltmanZScoreInput(
    ...     working_capital_to_total_assets=0.20,
    ...     retained_earnings_to_total_assets=0.30,
    ...     ebit_to_total_assets=0.15,
    ...     market_equity_to_total_liabilities=1.50,
    ...     sales_to_total_assets=1.00,
    ... )
    >>> result = altman_z_score(healthy)
    >>> result.zone == ScoringZone.SAFE
    True
    """
    ...
```

#### Class with builder

```python
class ScheduleBuilder:
    """Fluent builder for date schedules.

    Methods mutate in place and return ``None``; call them sequentially.

    Parameters
    ----------
    start : Date
        Schedule effective date.
    end : Date
        Schedule terminal date.

    Examples
    --------
    >>> from finstack.core.dates import (
    ...     ScheduleBuilder, BusinessDayConvention, StubKind,
    ... )
    >>> b = ScheduleBuilder(start, end)
    >>> b.frequency("3M")
    >>> b.stub_rule(StubKind.SHORT_FRONT)
    >>> b.adjust_with(BusinessDayConvention.MODIFIED_FOLLOWING, "usny")
    >>> schedule = b.build()
    """

    def __init__(self, start: Date, end: Date) -> None: ...

    def frequency(self, freq: str) -> None:
        """Set the period frequency (e.g. "3M", "6M", "1Y").

        Parameters
        ----------
        freq : str
            Tenor string parseable by :class:`Tenor`.
        """
        ...

    def build(self) -> Schedule:
        """Finalize and return the constructed schedule.

        Raises
        ------
        ValueError
            If the configured parameters are inconsistent (e.g. frequency
            not set, or stub rule incompatible with the date range).
        """
        ...
```

### Workflow

When adding a new binding:

1. Write the Rust binding `///` comment first. Match the Rust source it wraps in semantics.
2. Add the `.pyi` stub with NumPy-style docstring + type annotations.
3. Update `__all__` in the `.pyi` and the `register()` function.
4. Update `parity_contract.toml` if the new binding is in the parity-tested surface.
5. Run `mise run python-build` and `mise run all-test`.

When changing an existing binding:

1. Update the `///` comment.
2. Update the `.pyi` docstring and stubs.
3. Re-run parity tests.

When the binding's behaviour matches the Rust API exactly (the common case), keep the docstrings short — the canonical reference is the Rust source. When the Python surface diverges (in-place builders, dunder-method conventions, error type mapping), document the difference loudly so users don't bounce off it.
