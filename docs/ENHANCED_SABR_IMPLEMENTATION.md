# Enhanced SABR Model Implementation

## Overview

This document describes the enhanced SABR (Stochastic Alpha Beta Rho) model implementation in Finstack, providing robust handling of negative interest rates and improved numerical stability for near-the-money cases.

## Key Enhancements

### 1. Standard Hagan et al. Approximation with Numerical Stability

The implementation follows the standard SABR approximation formulas from Hagan et al. (2002) with enhanced numerical stability:

#### Robust ATM Detection
```rust
// Enhanced ATM detection with absolute and relative tolerance
let abs_diff = (effective_forward - effective_strike).abs();
let relative_diff = abs_diff / effective_forward.max(effective_strike);
if abs_diff < 1e-8 || relative_diff < 1e-8 {
    return self.atm_volatility(effective_forward, time_to_expiry);
}
```

#### Enhanced Chi Function
- Series expansion for small z values to avoid numerical issues
- Special handling for ρ ≈ ±1 cases
- Robust discriminant calculation with protection against negative values

### 2. Negative Rate Support

#### Shifted SABR Model
Handles negative forward rates by applying a positive shift:

```rust
// Create shifted SABR parameters
let params = SABRParameters::new_with_shift(alpha, beta, nu, rho, shift)?;

// Automatic shift detection
let min_rate = forward.min(*strikes.iter().min().unwrap());
if min_rate < 0.0 {
    let shift = (-min_rate + 0.001).max(0.001); // At least 10bps buffer
    // Use shifted calibration...
}
```

The shifted model transforms: `F_t → F_t + shift` where shift > 0 ensures positive dynamics.

#### Free-Boundary SABR Model  
Alternative approach using absolute value dynamics: `F_t → |F_t|^β`

```rust
// Free-boundary SABR for cross-zero scenarios
let abs_forward = forward.abs();
let abs_strike = strike.abs();

// Apply cross-zero correction if forward and strike have different signs
if forward.signum() != strike.signum() {
    let cross_correction = 1.0 + 0.1 * (forward - strike).abs() / (abs_forward + abs_strike);
    vol *= cross_correction;
}
```

### 3. Numerical Stability Improvements

#### ATM Formula Enhancement
- **Normal Model (β=0)**: Direct formula without power operations
- **Lognormal Model (β=1)**: Optimized calculation avoiding numerical issues
- **General β**: Special handling for β=0.5 (square root case)

#### Robust Parameter Validation
```rust
impl SABRParameters {
    pub fn new(alpha: F, beta: F, nu: F, rho: F) -> Result<Self> {
        if alpha <= 0.0 { return Err(Error::Internal); }
        if !(0.0..=1.0).contains(&beta) { return Err(Error::Internal); }
        if nu < 0.0 { return Err(Error::Internal); }
        if !(-1.0..=1.0).contains(&rho) { return Err(Error::Internal); }
        // ... validation logic
    }
}
```

## API Usage

### Standard SABR Usage
```rust
use finstack::valuations::instruments::options::models::{SABRParameters, SABRModel};

// Create standard SABR parameters
let params = SABRParameters::new(0.2, 0.5, 0.3, -0.1)?;
let model = SABRModel::new(params);

// Calculate implied volatility
let vol = model.implied_volatility(forward, strike, time_to_expiry)?;
```

### Shifted SABR for Negative Rates
```rust
// Create shifted SABR for negative rate environment
let shift = 0.02; // 200bps shift
let params = SABRParameters::new_with_shift(0.2, 0.0, 0.3, -0.1, shift)?;
let model = SABRModel::new(params);

// Works with negative forwards and strikes
let negative_forward = -0.005;
let negative_strike = -0.01;
let vol = model.implied_volatility(negative_forward, negative_strike, time_to_expiry)?;
```

### Automatic Shift Detection
```rust
// Calibrator automatically detects if shift is needed
let calibrator = SABRCalibrator::new();
let params = calibrator.calibrate_auto_shift(
    negative_forward,
    &negative_strikes,
    &market_vols,
    time_to_expiry,
    beta
)?;

// Returns shifted parameters if negative rates detected
assert!(params.is_shifted());
```

### Free-Boundary SABR
```rust
// For cross-zero scenarios (forward and strike have different signs)
let vol = model.implied_volatility_free_boundary(
    negative_forward,
    positive_strike,
    time_to_expiry
)?;
```

### VolSurfaceCalibrator Integration
The `VolSurfaceCalibrator` automatically uses the enhanced SABR features:

```rust
let calibrator = VolSurfaceCalibrator::new(
    "EUR-SWAPTION-VOL",
    0.5, // beta
    expiry_grid,
    strike_grid,
);

// Automatically handles negative rates if present in market data
let (surface, report) = calibrator.calibrate(option_quotes, &[], &market_context)?;
```

## Implementation Benefits

### 1. Numerical Robustness
- **ATM Stability**: Enhanced detection prevents numerical instabilities
- **Extreme Parameters**: Robust handling of boundary cases (ρ ≈ ±1, β ≈ 0/1)  
- **Long Maturities**: Improved accuracy for extended time horizons

### 2. Negative Rate Support
- **Shifted SABR**: Industry-standard approach with automatic shift detection
- **Free-Boundary**: Alternative for complex cross-zero scenarios
- **Validation**: Input validation ensures model consistency

### 3. Market Integration
- **VolSurfaceCalibrator**: Seamless integration with volatility surface construction
- **Automatic Detection**: Smart detection of negative rate environments
- **Backward Compatibility**: Standard SABR behavior preserved for positive rates

## Testing Coverage

The implementation includes comprehensive tests covering:

- **Parameter Validation**: Boundary cases and invalid inputs
- **ATM Stability**: Numerical precision for near-ATM scenarios  
- **Negative Rates**: Both shifted and free-boundary approaches
- **Extreme Cases**: Very low rates, long maturities, boundary parameters
- **Calibration**: Market data fitting with automatic shift detection
- **Smile Generation**: Volatility surface construction and interpolation

## References

1. **Hagan, P.S., Kumar, D., Lesniewski, A.S., and Woodward, D.E.** (2002). "Managing Smile Risk." *Wilmott Magazine*, pp. 84-108.

2. **West, G.** (2005). "Calibration of the SABR Model in Illiquid Markets." *Applied Mathematical Finance*, 12(4), pp. 371-385.

3. **Antonov, A., Konikov, M., and Spector, M.** (2015). "Free Boundary SABR." *Risk Magazine*.

4. **Chibane, M. and Sheldon, D.** (2009). "Building a Volatility Surface for the SABR Model." *Journal of Computational Finance*, 13(4).
