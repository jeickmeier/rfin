# Configuration

## FinstackConfig

`FinstackConfig` is the global configuration object that controls rounding
behavior, decimal precision, and tolerance settings across all computations.

**Python**

```python
from finstack.core.config import (
    FinstackConfig, RoundingPolicy, RoundingMode, CurrencyScalePolicy,
)

cfg = FinstackConfig()
```

## Rounding

### Rounding Modes

| Mode | Behavior |
|------|----------|
| `Bankers` | Round half to even (default, IEEE 754 standard) |
| `AwayFromZero` | Round halves away from zero |
| `TowardZero` | Truncate (round toward zero) |
| `Floor` | Round toward $-\infty$ |
| `Ceil` | Round toward $+\infty$ |

### Scale Policies

Scale (decimal precision) is determined per currency:

```python
cfg = FinstackConfig()

# Default: ISO minor units (USD=2, JPY=0, KWD=3)
cfg.rounding.mode = RoundingMode.BANKERS

# Override specific currencies
cfg.rounding.output_scale.overrides = {"JPY": 0, "KWD": 3}
cfg.rounding.ingest_scale.overrides = {"JPY": 0}

# Check effective scale
output_decimals = cfg.output_scale(Currency("USD"))  # 2
output_decimals = cfg.output_scale(Currency("JPY"))  # 0
```

### Using Config with Money

```python
from finstack.core.money import Money

# Default rounding (Bankers)
m = Money(123.455, "USD")  # → 123.46

# Custom config
cfg = FinstackConfig()
cfg.rounding.mode = RoundingMode.AWAY_FROM_ZERO
m = Money.from_config(123.455, "USD", cfg)  # → 123.46
```

## Tolerances

Numerical tolerances used for floating-point comparisons:

```rust,no_run
pub const ZERO_TOLERANCE: f64 = 1e-10;  // Near-zero guard
```

The `ToleranceConfig` controls thresholds for convergence checks in
root-finding, calibration, and pricing algorithms.

## Config Extensions

Namespaced configuration for domain-specific settings. Keys follow the pattern
`{crate}.{domain}.v{N}`:

```python
# Extensions are stored as a BTreeMap<String, JsonValue>
# Example keys:
# "valuations.calibration.v2"
# "portfolio.optimization.v1"
```

This design allows crates to add their own configuration sections without
modifying the core config struct.

## ResultsMeta

Every `ValuationResult` includes metadata describing the configuration used:

```python
result = registry.price_with_metrics(bond, "discounting", market, as_of, [])
meta = result.meta
# meta.rounding    → RoundingMode
# meta.numeric_mode → NumericMode (Decimal vs f64)
```
