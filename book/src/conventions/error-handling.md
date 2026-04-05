# Error Handling

Finstack uses a layered error system: Rust `Result` types at the core,
mapped to Python exceptions and WASM `JsError` in bindings.

## Rust Error Types

All core errors derive from `finstack_core::Error`:

```rust,no_run
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    Input(InputError),
    InterpOutOfBounds,
    CurrencyMismatch { expected, actual },
    Calibration { message, category },
    Validation(String),
    UnknownMetric { metric_id },
    MetricNotApplicable { metric_id },
    MetricCalculationFailed { metric_id },
    CircularDependency,
    Internal(String),
}
```

Input errors cover common data issues:

```rust,no_run
pub enum InputError {
    MissingCurve { requested, suggestions },
    NotFound { id },
    AdjustmentFailed { date, convention, max_days },
    UnknownCurrency,
    InvalidDate { year, month, day },
    InvalidDateRange,
    TooFewPoints,
    NonMonotonicKnots,
    NonPositiveValue,
    NegativeValue,
    DimensionMismatch,
    Invalid,
}
```

## Python Exception Hierarchy

```text
FinstackError
├── ConfigurationError
│   ├── MissingCurveError
│   ├── MissingFxRateError
│   └── InvalidConfigError
├── ComputationError
│   ├── ConvergenceError
│   ├── CalibrationError
│   └── PricingError
├── ValidationError
│   ├── CurrencyMismatchError
│   ├── DateError
│   └── ParameterError
│       ├── ConstraintValidationError
│       └── CholeskyError
└── InternalError
```

Catch specific exceptions:

```python
from finstack import MissingCurveError, CalibrationError

try:
    result = pricer.price(instrument, market, as_of)
except MissingCurveError as e:
    print(f"Curve not found: {e}")
except CalibrationError as e:
    print(f"Calibration failed: {e}")
```

## Binding Layer Error Mapping

Centralized in `finstack-py/src/errors.rs`:

```rust,no_run
/// Map a Rust core error to a Python exception.
pub fn map_error(err: CoreError) -> PyErr {
    match err {
        CoreError::Input(input_err)   => map_input_error(input_err),
        CoreError::CurrencyMismatch { .. } => CurrencyMismatchError::new_err(..),
        CoreError::Calibration { .. }      => CalibrationError::new_err(..),
        CoreError::Validation(msg)         => ValidationError::new_err(msg),
        CoreError::Internal(msg)           => InternalError::new_err(msg),
        other => FinstackError::new_err(other.to_string()),
    }
}

/// Alias used in binding code.
pub(crate) fn core_to_py(err: CoreError) -> PyErr {
    map_error(err)
}
```

## Rules

1. **Never** use `.unwrap()` or `.expect()` in binding code
2. Always propagate errors with `?` or `map_err(map_error)`
3. Rust tests may use `.unwrap()` (guarded with `#[allow(clippy::unwrap_used)]`)
4. WASM errors map to `JsError::new(&e.to_string())`
