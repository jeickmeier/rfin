# Finstack Documentation Standard

This document defines the documentation expectations for every public item in the `finstack` workspace. It is the canonical reference linked from `finstack/core/src/lib.rs` and from each binding crate's lints.

Companion files:

- [`docs/REFERENCES.md`](REFERENCES.md) — anchor list for `# References` citations.
- [`finstack-wasm/DOCS_STYLE.md`](../finstack-wasm/DOCS_STYLE.md) — WASM-specific JSDoc conventions.
- [`finstack-py/DOCS_STYLE.md`](../finstack-py/DOCS_STYLE.md) — Python binding conventions.
- [`AGENTS.md`](../AGENTS.md) — naming-strategy and lints (the binding triplet rules live there).
- [`INVARIANTS.md`](../INVARIANTS.md) — workspace invariants (Decimal-vs-f64, determinism, sign conventions).

## Scope

Applies to:

- All `pub` items in the Rust crates under `finstack/`.
- All `#[pyfunction]`, `#[pyclass]`, `#[pymethods]` exports in `finstack-py/`.
- All `#[wasm_bindgen]` exports in `finstack-wasm/`.
- The hand-written `finstack-wasm/index.d.ts` and `finstack-wasm/exports/*.js` facade files.
- Module-level (`//!`) comments on every `pub mod`.

`pub(crate)` and `pub(super)` items are not formally in scope. A one-line `///` is encouraged for any non-trivial helper but the lint does not enforce it.

## Lints in force

- **Rust:** `#![warn(missing_docs)]` (workspace) plus `-D missing_docs` at CI.
- **Rustdoc:** every crate's CI job builds with `RUSTDOCFLAGS="-D warnings"`. Broken intra-doc links and stale references fail the build.
- **Doctests:** `cargo test -p <crate> --doc` runs in CI. Doctests must be runnable; if a doctest can only compile, mark it `no_run`. Use `ignore` only when the example needs external resources.

If a lint is too aggressive for a specific item, prefer `#[allow(...)]` at the smallest scope possible and add a one-line comment justifying it. Don't disable the lint at the crate level.

## Required sections per item

### 1. Summary

One or two sentences, indicative mood. Describes **what** the item does and **when** to use it. Not a re-statement of the type signature.

### 2. `# Arguments`

For every `pub fn` and method. Each parameter on its own bullet:

- Name in backticks
- Description (one short sentence)
- Units / range / domain when finite (e.g. `decimal (0.05 = 5%)`, `bps`, `years (positive)`, `monotonic non-decreasing`)
- Optional: link to canonical type or convention.

```rust
/// # Arguments
///
/// * `notional` - Face amount in the schedule's settlement currency.
/// * `coupon_rate` - Annualized coupon, decimal (e.g. `0.05` = 5%).
/// * `day_count` - [`DayCount`] used for the accrual factor.
```

Skip this section when the function has zero parameters and a `&self` receiver.

### 3. `# Returns`

For every `pub fn` and method that returns something other than `()`.

- Describe the returned value's meaning (not just its type).
- Include units.
- For `Result<T, E>`, describe the `Ok` value here and the failure modes under `# Errors`.

### 4. `# Errors`

For every fallible function. List each `E` variant or category that can be returned and what triggers it.

```rust
/// # Errors
///
/// Returns [`PricingError::MissingMarketData`] if `market` does not contain
/// the discount curve named in the bond's metadata.
```

### 5. `# Panics`

Library code MUST NOT panic in the happy path. The workspace Clippy profile denies `unwrap_used`, `expect_used`, `panic`, `unreachable`, and `indexing_slicing` outside test code (see [`INVARIANTS.md`](../INVARIANTS.md) §5). If a `pub fn` can panic at all (e.g. an `unsafe fn` debug-assert path, an internal precondition that the documentation sells as the caller's contract), document the panic conditions explicitly.

Most public functions will not have this section.

### 6. `# Examples`

Required for any item whose intended use is not obvious from its signature. Code blocks MUST be runnable as a doctest (or marked `no_run` / `ignore` with a one-line reason).

```rust
/// # Examples
///
/// ```
/// use finstack_core::dates::{adjust, BusinessDayConvention, Date};
///
/// let saturday = Date::from_calendar_date(2025, time::Month::January, 4)?;
/// let monday = adjust(saturday, BusinessDayConvention::Following, &cal);
/// assert_eq!(monday.weekday(), time::Weekday::Monday);
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
```

Examples should:

- Use realistic inputs (real market values, real dates, not `42`).
- Show the canonical, common-case usage. Edge cases belong in unit tests.
- Use `?` propagation rather than `.unwrap()` whenever possible.

### 7. `# References`

Required for any item that encodes a market convention, pricing model, numerical method, or risk calculation with a standard reference. Cite via [`docs/REFERENCES.md`](REFERENCES.md) anchors:

```rust
/// # References
///
/// - [Hull, *Options, Futures, and Other Derivatives*](docs/REFERENCES.md#hull-options-futures)
/// - [Brent (1973)](docs/REFERENCES.md#brent-1973)
```

If a reference is missing from `docs/REFERENCES.md`, add it there in the same change.

What requires a `# References` section:

- Day-count and business-day conventions (cite ISDA / ICMA).
- Pricing models (Heston, Hull-White, Black-Scholes-Merton, etc.).
- Greeks (cite Hull or the model paper).
- Calibration methods (Brent, Newton-Raphson, Levenberg-Marquardt).
- Curve construction (Hagan-West monotone-convex, etc.).
- Risk metrics (Jorion for VaR/ES, BCBS for Basel methods, ISDA SIMM).
- Credit scoring (Altman 1968, Ohlson 1980, etc.).
- Performance metrics (CFA GIPS for TWRR/MWRR).

What does **not** require references: utility helpers, type wrappers, validation functions, error variants.

## Module-level `//!` comments

Every `pub mod` MUST have a module-level doc that:

1. States the module's purpose in one sentence.
2. Lists the major types or functions, with one-line descriptions.
3. (When applicable) Names the canonical reference for the algorithms it implements.
4. (When applicable) Includes a short example showing the typical entry point.

Re-export-only modules (`pub use ...`) may have a thinner module doc but should still state which crate / sub-area the re-exports come from and why.

## Financial conventions (non-negotiable)

These rules apply across Rust, Python, and WASM bindings. Mirror exactly the language in `finstack-wasm/DOCS_STYLE.md` so triplets read identically.

### Rates

Always state the unit:

- **Decimal:** `0.05` = 5%.
- **Basis points (bps):** `500.0` = 5%.
- **Continuously compounded:** flag explicitly when used (typical for zero curves and forward rates pulled from a `DiscountCurve`).
- **Annualized:** assume yes unless the doc says otherwise.

### Dates

Clarify the role of every date parameter:

- `as_of` / `valuation_date` — when the calculation is anchored.
- `issue` / `effective` / `start` — when the instrument's life begins.
- `maturity` / `terminal` — when the instrument terminates.
- `accrual_start` / `accrual_end` — coupon-period boundaries.
- `payment_date` — actual cash settlement date (after BDC adjustment).

Where a function takes a `Date`, prefer `Date` from `time` crate. Where it takes year/month/day, document ISO-8601 ordering.

### Curves

For any function that consumes market data, document:

- Required curve IDs (e.g. `"USD-OIS"`, `"USD-3M-LIBOR"`).
- Required surfaces (FX, vol, hazard).
- The `MarketContext` field that is read.

Curve IDs are case-sensitive and use SCREAMING-KEBAB-CASE (`USD-OIS`, `EUR-ESTR`).

### Quote conventions

For pricing APIs, state:

- Clean vs dirty (bonds, CDS).
- Percent-of-par vs absolute (loans).
- Bid / mid / ask (calibration inputs).
- Sign convention for cashflows (see [`INVARIANTS.md`](../INVARIANTS.md) §3).

### Decimal vs f64

Per [`INVARIANTS.md`](../INVARIANTS.md) §1, money values that flow to accounting / settlement / regulatory capital MUST be `Decimal` at the boundary; everything else is `f64`. Document the choice for any new public API:

- `Money` and methods that return `Money` are `Decimal`-backed.
- Greeks, vols, rates, correlations, returns, derivative prices are `f64`.

## Templates

### Function template

```rust
/// One-sentence summary.
///
/// Optional second sentence with extra context (when applicable, conventions,
/// invariants).
///
/// # Arguments
///
/// * `param_a` - Description, units, range.
/// * `param_b` - Description.
///
/// # Returns
///
/// What the value means and its units.
///
/// # Errors
///
/// Returns [`SomeError::Variant`] when X.
///
/// # Examples
///
/// ```
/// // realistic example here
/// ```
///
/// # References
///
/// - [Citation](docs/REFERENCES.md#anchor)
pub fn example(param_a: f64, param_b: &str) -> Result<f64, SomeError> { ... }
```

### Struct/enum template

```rust
/// One-sentence summary of what this type represents.
///
/// Multi-paragraph context as needed: when to construct, what it pairs with,
/// known invariants the type guarantees.
///
/// # Examples
///
/// ```
/// // typical construction + use
/// ```
///
/// # References
///
/// - [Citation](docs/REFERENCES.md#anchor)
pub struct Example { ... }
```

### Module template (`mod.rs` or top of single-file module)

```rust
//! One-sentence module purpose.
//!
//! # Components
//!
//! - [`SubType`] — short description.
//! - [`sub_function`] — short description.
//!
//! # Example
//!
//! ```
//! // one realistic entry-point usage
//! ```
//!
//! # References
//!
//! - [Citation](docs/REFERENCES.md#anchor)
```

## Crate-specific conventions

### `finstack-core`

- The crate-level doc in `lib.rs` lists API layers (Core / Extended). Keep the list in sync when promoting / demoting modules.
- `prelude` is the recommended import for downstream code; new convenience re-exports go there.

### `finstack-py`

See [`finstack-py/DOCS_STYLE.md`](../finstack-py/DOCS_STYLE.md). Highlights:

- PyO3 `///` comments map to Python `__doc__`. Write them so they read naturally as Python docstrings.
- `.pyi` stubs use NumPy-style docstrings (Parameters / Returns / Raises / Examples sections).
- Document the in-place-mutation contract on Python builders (Python builders return `None`, not `Self`).
- Decimal-vs-f64 boundary: bindings expose `f64`; users converting to Decimal in Python code is their responsibility.

### `finstack-wasm`

See [`finstack-wasm/DOCS_STYLE.md`](../finstack-wasm/DOCS_STYLE.md). Highlights:

- JSDoc tags (`@param`, `@returns`, `@throws`, `@example`) instead of `# Arguments` / `# Returns` sections in any `#[wasm_bindgen]` doc comment.
- Every export must have at least one runnable `@example` block.
- The hand-written `index.d.ts` and `exports/*.js` mirror the same JSDoc requirements as the Rust source.

## Workflow

When adding a new public API:

1. Write the docstring **before** the implementation. The docstring is part of the API contract.
2. Run `cargo doc -p <crate> --no-deps` locally. Check that links resolve.
3. Run `cargo test -p <crate> --doc` if you added doctests.
4. If the API has financial semantics, add or reuse a `docs/REFERENCES.md` anchor.
5. Update binding crates (`finstack-py`, `finstack-wasm`) in the same commit per [`AGENTS.md`](../AGENTS.md) §"Binding updates".

When changing an existing public API:

1. Re-read the docstring before the diff. If the contract changes, update the docstring in the same commit.
2. Update the bindings.
3. If the change is breaking, prefer `#[deprecated]` over removal — see [`INVARIANTS.md`](../INVARIANTS.md) §7.

## CI gates (current and target)

| Gate | Status | Where |
|------|--------|-------|
| `-D missing_docs` (Rust) | Active | All workspace crates |
| `RUSTDOCFLAGS="-D warnings"` | Active for `finstack-core`, `finstack-py`, `finstack-wasm` per local verification | Should be promoted to CI for the full workspace |
| Doctests run | Active per crate | `cargo test -p <crate> --doc` |
| JSDoc `@example` density | Not yet enforced | Target: post-WASM-backfill, add a script that fails if a `#[wasm_bindgen]` export lacks `@example` |
| `# References` density on financial code | Not yet enforced | Target: a heuristic check on `pricer/`, `metrics/`, `credit/`, `dates/daycount`, `risk_metrics/` |

## Review heuristics

When reviewing a docstring, check that:

- The summary line could replace the function in a documentation index without losing meaning.
- A reader who has not seen the implementation can call the API correctly using only the docstring.
- Units appear next to every numeric parameter.
- Date roles are explicit.
- The example actually runs.
- References (when required) point at a `docs/REFERENCES.md` anchor that exists.
- Cross-references (`[`Type`]`) resolve in the local crate's rustdoc.
