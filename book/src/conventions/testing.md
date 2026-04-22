# Testing

Finstack has four test layers: unit tests, doctests, parity tests, and
integration tests.

## Rust Unit Tests

Every module has a `#[cfg(test)]` block. Test modules allow `.unwrap()`:

```rust,no_run
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_construction() {
        let scale = RatingScale::standard();
        assert_eq!(scale.n_states(), 10);
        assert_eq!(scale.default_state(), Some(9));
    }

    #[test]
    fn test_error_case() {
        let err = RatingScale::custom(vec!["A".to_string()]);
        assert!(matches!(err, Err(MigrationError::InsufficientStates)));
    }
}
```

Run Rust tests:

```bash
cargo test                 # all tests
cargo test -p finstack-core  # single crate
```

## Doctests

All public types should have doc examples that compile and run:

```rust,no_run
/// Create a discount curve.
///
/// # Examples
///
/// ```
/// use finstack_core::market_data::DiscountCurve;
/// let curve = DiscountCurve::builder("USD-OIS")
///     .base_date(date(2025, 1, 15))
///     .add_knot(1.0, 0.99)
///     .build()
///     .unwrap();
/// ```
pub fn builder(id: impl Into<String>) -> DiscountCurveBuilder { ... }
```

## Python Parity Tests

Parity tests verify Python bindings produce identical results to Rust.
Located in `finstack-py/tests/parity/`:

```python
class TestBondPricingParity:
    """Test bond pricing matches Rust implementation."""

    def test_bond_construction(self) -> None:
        bond = (Bond.builder("BOND-001")
            .notional(1_000_000.0)
            .currency("USD")
            .issue(date(2024, 1, 1))
            .maturity(date(2029, 1, 1))
            .coupon_rate(0.05)
            .frequency(Frequency.SEMI_ANNUAL)
            .day_count(DayCount.THIRTY_360)
            .disc_id("USD-OIS")
            .build())
        assert bond.id() == "BOND-001"
```

Run parity tests:

```bash
uv run pytest finstack-py/tests/parity/ -v
```

## Coverage Gate

Rust coverage must meet **80% line coverage**:

```bash
python scripts/check_rust_coverage_gate.py
```

The gate reads `DEFAULT_THRESHOLD = 80.0` and fails CI if coverage drops below.

## Makefile Targets

```bash
mise run all-test          # cargo test + pytest
mise run all-lint          # clippy -D warnings + format check
mise run all-fmt           # cargo fmt + ruff format
```
