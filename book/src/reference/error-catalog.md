# Error Catalog

Finstack uses a structured exception hierarchy. All errors derive from
`FinstackError`.

## Exception Hierarchy

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

## Error Details

### MissingCurveError

| | |
|---|---|
| **Cause** | Requested curve ID not found in `MarketContext` |
| **Rust variant** | `InputError::MissingCurve { requested, suggestions }` |
| **Message** | `"Curve 'USD-OIS' not found. Did you mean: 'USD_OIS'?"` |
| **Fix** | Check curve ID spelling; add the curve to `MarketContext` |

### MissingFxRateError

| | |
|---|---|
| **Cause** | FX rate pair not available for currency conversion |
| **Fix** | Add the FX rate to `MarketContext` or `FxMatrix` |

### InvalidConfigError

| | |
|---|---|
| **Cause** | Invalid configuration parameters |
| **Fix** | Check builder parameters for valid ranges |

### ConvergenceError

| | |
|---|---|
| **Cause** | Newton/Brent solver did not converge within max iterations |
| **Fix** | Increase `max_iter`, widen bounds, or improve initial guess |

### CalibrationError

| | |
|---|---|
| **Cause** | Curve or model calibration failed |
| **Rust variant** | `Error::Calibration { message, category }` |
| **Fix** | Check input market quotes for consistency; verify bootstrap order |

### PricingError

| | |
|---|---|
| **Cause** | Pricing computation failed (e.g., no pricer registered) |
| **Fix** | Verify instrument type is registered in the pricer registry |

### CurrencyMismatchError

| | |
|---|---|
| **Cause** | Operation on `Money` with different currencies |
| **Rust variant** | `Error::CurrencyMismatch { expected, actual }` |
| **Fix** | Convert to same currency before arithmetic; use `FxProvider` |

### DateError

| | |
|---|---|
| **Cause** | Invalid date or date range |
| **Rust variants** | `InputError::InvalidDate`, `InputError::InvalidDateRange` |
| **Fix** | Check date validity; ensure start < end |

### ParameterError

| | |
|---|---|
| **Cause** | Invalid parameter value (negative vol, non-monotonic knots) |
| **Rust variants** | `InputError::NonPositiveValue`, `InputError::NonMonotonicKnots` |
| **Fix** | Validate inputs before construction |

### ConstraintValidationError

| | |
|---|---|
| **Cause** | Statement constraint or covenant violated |
| **Fix** | Check constraint definitions match expected data |

### CholeskyError

| | |
|---|---|
| **Cause** | Correlation matrix is not positive definite |
| **Fix** | Ensure matrix is symmetric with all eigenvalues > 0 |

### InternalError

| | |
|---|---|
| **Cause** | Unexpected internal state (should not occur in normal usage) |
| **Fix** | Report as a bug with full stack trace |
