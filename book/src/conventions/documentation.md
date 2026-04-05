# Documentation

For the full documentation standard, see
[DOCUMENTATION_STANDARD.md](https://github.com/your-org/finstack/blob/main/docs/DOCUMENTATION_STANDARD.md).

## Rust Doc Comments

All public items require doc comments (`-D missing_docs` is enabled):

```rust,no_run
/// A discount curve for present-value calculations.
///
/// Stores (time, discount_factor) knot points and interpolates between them
/// using the configured interpolation method.
///
/// # References
///
/// - Ametrano & Bianchetti (2013), "Everything You Always Wanted to Know
///   About Multiple Interest Rate Curve Bootstrapping but Were Afraid to Ask"
pub struct DiscountCurve {
    /// Unique curve identifier (e.g., `"USD-OIS"`).
    id: String,
    /// Base date for time calculations.
    base_date: Date,
}
```

### Doc Comment Rules

1. First line: concise summary sentence
2. Blank line, then detailed description if needed
3. `# Examples` section with compilable code
4. `# References` for academic/financial references
5. All public struct fields need doc comments
6. List continuations use 2-space indent (not aligned to text)

## List Indentation

Clippy enforces `doc_overindented_list_items`:

```rust,no_run
/// Features:
///
/// - First item
///   continuation with 2-space indent
/// - Second item
///   also 2-space indent
```

## RUSTDOCFLAGS

CI verifies docs build without warnings:

```bash
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

## Python Stubs

`.pyi` files in `finstack-py/finstack/` are manually maintained and must
stay in sync with the Rust bindings:

```python
class DiscountCurve:
    """A discount curve for present-value calculations."""

    @staticmethod
    def builder(id: str) -> DiscountCurveBuilder:
        """Create a new builder."""
        ...

    def discount_factor(self, t: float) -> float:
        """Get the discount factor at time t."""
        ...
```

## mdBook (This Guide)

- Source in `book/src/`, built with mdBook v0.5.2
- Parallel Rust/Python/WASM code in fenced blocks
- Table of contents in `SUMMARY.md`

## mkdocs (Python API Reference)

- Config in `mkdocs.yml`, built with mkdocs-material
- Auto-generated from `.pyi` stubs via mkdocstrings
- 243 pages covering all public Python types
