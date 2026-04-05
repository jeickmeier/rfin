# Naming Conventions

Consistent naming across Rust, Python, and WASM layers.

## Accessor Methods

All getter methods use the `get_*` prefix:

```rust,no_run
// Market data
ctx.get_discount("USD-OIS")          // -> Arc<DiscountCurve>
ctx.get_forward("USD-SOFR-3M")       // -> Arc<ForwardCurve>
ctx.get_hazard("ACME-HZD")           // -> Arc<HazardCurve>
ctx.get_vol_surface("AAPL-VOL")      // -> Arc<VolSurface>
ctx.get_price("AAPL")                // -> &MarketScalar

// Registry
registry.get_pricer(key)              // -> Option<&dyn Pricer>
```

Python mirrors the same names:

```python
market.get_discount("USD-OIS")
market.get_forward("USD-SOFR-3M")
registry.get_pricer(key)
```

## Metric Key Format

Fully qualified, `::` separated:

```text
metric_type::curve_or_entity::tenor_or_qualifier
```

| Pattern | Example |
|---------|--------|
| Scalar | `dv01`, `cs01`, `vega` |
| Per-curve | `pv01::usd_ois` |
| Bucketed | `bucketed_dv01::USD-OIS::10y` |
| Per-entity | `cs01::ACME-HZD` |
| Z-spread | `cs01::BOND_A` |
| Vega bucket | `vega::AAPL::6m` |

## Module Layout

### Rust

```text
finstack/core/src/
  currency/          # Currency, Money
  dates/             # Date, Calendar, DayCount
  market_data/       # Curves, surfaces, FX, context
  math/              # Interpolation, optimization, solvers
  credit/            # Migration, recovery, copula
```

### Python Binding Modules

```text
finstack-py/src/
  core/              # Mirrors finstack-core public API
  valuations/        # Instruments, pricing, calibration
  portfolio/         # Portfolio, positions, valuation
  statements/        # Statement modeling, waterfalls
  scenarios/         # Scenario specification, application
```

Each module has a `register()` function:

```rust,no_run
pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "my_module")?;
    m.add_class::<MyClass>()?;
    parent.add_submodule(&m)?;
    Ok(())
}
```

## Rust vs Python Naming

| Rust | Python | Notes |
|------|--------|-------|
| `snake_case` functions | `snake_case` functions | Same |
| `CamelCase` types | `CamelCase` classes | Same |
| `SCREAMING_SNAKE` consts | `SCREAMING_SNAKE` | Same |
| `disc_id` parameter | `disc_id` parameter | Preserved |
| `fwd_id` parameter | `fwd_id` parameter | Preserved |
| `std::result::Result<T, E>` | Raises exception | See [Error Handling](error-handling.md) |

## Curve ID Conventions

| Pattern | Example | Use |
|---------|---------|-----|
| `CCY-BENCHMARK` | `USD-OIS` | Discount curves |
| `CCY-INDEX-TENOR` | `USD-SOFR-3M` | Forward curves |
| `ENTITY-HZD` | `ACME-HZD` | Hazard curves |
| `TICKER-VOL` | `AAPL-VOL` | Vol surfaces |
