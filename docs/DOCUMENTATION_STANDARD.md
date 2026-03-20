# Finstack Documentation Standard

This document defines the minimum documentation bar for public APIs in the
Finstack workspace, with special emphasis on finance, market-convention, and
numerical code.

## Public API Requirements

Every public Rust item should document the parts of its contract that a caller
needs in order to use it correctly.

### Required For Most Public Items

- A one-line summary describing what the item represents or does.
- `# Arguments` for functions, builders, and constructors that take inputs.
- `# Returns` for functions returning non-`()` values.
- `# Errors` for fallible APIs.
- `# Examples` with runnable code when practical, or `no_run` / `ignore` when an
  example is intentionally illustrative.

### Required For Finance And Numerical APIs

- Units and quote conventions:
  - decimal vs basis points
  - simple vs compounded vs continuously compounded
  - year fractions vs calendar days
  - strike, delta, log-moneyness, or total variance when relevant
- Market conventions and assumptions:
  - day-count basis
  - business-day adjustment behavior
  - base date vs settlement date vs payment date
  - interpolation and extrapolation policy
- Numerical behavior:
  - convergence expectations
  - tolerance meanings
  - failure modes and edge cases
- `# References` linking to `docs/REFERENCES.md` anchors when the implementation
  follows a canonical source or an industry convention.

## Module-Level Documentation

Every public module should include a `//!` overview that answers:

- What domain the module covers.
- Which types or entry points most users should start with.
- Which conventions or assumptions are important.
- Where to look next for deeper detail.

Facade modules should prefer a small guided overview over an exhaustive changelog
of submodules.

## Intra-Doc Link Rules

- Use explicit paths for links that are easy to mis-resolve, such as items
  re-exported through facades.
- Prefer `[`Self::method`]` or `[`crate::path::Item`]` style links when ambiguity
  is possible.
- Treat `RUSTDOCFLAGS='-D warnings' cargo doc ...` as the source of truth.

## Example Guidelines

- Keep examples small and copy-pasteable.
- Use realistic financial terminology where the API is domain-facing.
- Avoid examples that silently assume a convention without naming it.
- If an example cannot be run cheaply or deterministically, mark it `no_run` and
  explain the missing setup in one sentence.

## Style Notes

- Prefer concise, direct sentences over tutorial prose inside rustdoc.
- Put the most important convention near the top, not buried at the end.
- Keep terminology consistent across modules:
  - `discount factor`
  - `survival probability`
  - `year fraction`
  - `basis points`
  - `forward delta`

## Verification

Documentation work is not complete until all of the following pass:

- `cargo fmt -p finstack-core`
- `cargo clippy -p finstack-core --all-features -- -D warnings`
- `RUSTDOCFLAGS='-D warnings' cargo doc -p finstack-core --no-deps --all-features`
- `cargo test -p finstack-core --doc`
