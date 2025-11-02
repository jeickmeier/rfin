# Finstack Calibration Framework

A comprehensive calibration system for financial market data structures, implementing market-standard methodologies for curve and surface construction.

## Overview

The calibration framework provides a unified approach to calibrating:

- **Interest Rate Curves**: Discount curves (OIS) and forward curves (IBOR/RFR) from deposits, FRAs, futures, and swaps
- **Credit Curves**: Survival probability and hazard rate curves from CDS spreads
- **Inflation Curves**: Real CPI level curves from zero-coupon inflation swaps
- **Volatility Surfaces**: Implied volatility surfaces using SABR models per expiry
- **Base Correlation Curves**: Credit correlation curves from CDS tranche quotes

## Architecture

### Core Components

1. **`Calibrator` Trait**: Unified interface for all calibration processes
2. **Solver Framework**: 1D root finding (Newton, Brent, Hybrid) and multi-dimensional optimization  
3. **Bootstrap Modules**: Sequential bootstrapping for term structures
4. **Surface Fitting**: SABR-based volatility surface construction
5. **Simple Calibration**: End-to-end market environment calibration

### Market Standards Compliance

- **Interest Rates**: Post-2008 multi-curve framework with OIS discounting
- **Credit**: ISDA 2014 standard model with accrual-on-default
- **Inflation**: Proper lag handling and seasonality support
- **Volatility**: SABR model with appropriate beta by asset class
- **Base Correlation**: One-factor Gaussian Copula with equity tranche decomposition

## Usage

### Basic Calibration

```rust
use finstack_valuations::calibration::{
    SimpleCalibration, 
    MarketQuote, RatesQuote
};

// Create calibration builder
let calibration = SimpleCalibration::new(base_date, Currency::USD);

// Prepare market quotes
let quotes = vec![
    MarketQuote::Rates(RatesQuote::Deposit { 
        maturity: base_date + Duration::days(30),
        rate: 0.045,
        day_count: DayCount::Act360,
    }),
    MarketQuote::Rates(RatesQuote::Swap {
        maturity: base_date + Duration::days(365*2), 
        rate: 0.047,
        fixed_freq: Frequency::semi_annual(),
        float_freq: Frequency::quarterly(),
        fixed_dc: DayCount::Thirty360,
        float_dc: DayCount::Act360,
        index: "USD-SOFR-3M".to_string(),
    },
    // ... more quotes
];

// Calibrate complete market environment
let (market_context, report) = orchestrator.calibrate_market(&quotes)?;
```

### Individual Curve Calibration

```rust
use finstack_valuations::calibration::methods::DiscountCurveCalibrator;

// Calibrate discount curve only
let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
    .with_solve_interp(InterpStyle::MonotoneConvex);

let (discount_curve, report) = calibrator.calibrate(&quotes, &base_context)?;
```

### Volatility Surface Calibration

```rust
use finstack_valuations::calibration::methods::sabr_surface::VolSurfaceCalibrator;

// Set up SABR calibration for equity volatility
let calibrator = VolSurfaceCalibrator::new(
    "SPY-VOL",
    1.0, // Lognormal beta for equity
    vec![0.25, 0.5, 1.0, 2.0], // Expiry grid
    vec![80.0, 90.0, 100.0, 110.0, 120.0], // Strike grid
);

// Create market context with appropriate forward curve data
// (e.g., equity spots, dividends, discount curves)
let market_context = MarketContext::new()
    .insert_spot("SPY", Money::new(100.0, Currency::USD))
    .insert_discount(discount_curve);

let (vol_surface, report) = calibrator.calibrate(&vol_quotes, &market_context)?;
```

## Implementation Status

### ✅ Completed
- Core calibration framework (`Calibrator` trait, `CalibrationReport`, error handling)
- Solver infrastructure (Newton, Brent, Hybrid, Levenberg-Marquardt)
- Market quote primitives and hashable float utilities
- Calibration orchestrator for sequenced calibration
- FRA and Interest Rate Future instruments for short-end calibration
- Framework structure for all curve types

### 🚧 Simplified Implementations  
The current implementations provide working stubs that demonstrate the framework:
- **Discount/Forward Curves**: Framework in place, simplified bootstrap logic
- **Credit Curves**: ISDA-compliant structure, simplified spread mapping
- **Inflation Curves**: CPI level framework, simplified growth assumptions
- **Volatility Surfaces**: SABR model integration, basic grid construction
- **Base Correlation**: Gaussian Copula integration, simplified correlation mapping

### 🔄 Next Steps
1. **Full Bootstrap Logic**: Implement complete sequential solving with proper objective functions
2. **Multi-Curve Solver**: Add coupled OIS+IBOR calibration with basis optimization
3. **Advanced Features**: Convexity adjustments, smile interpolation, no-arbitrage constraints
4. **Performance**: Parallel processing, analytical Jacobians, caching
5. **Validation**: Comprehensive market data validation and stress testing

## Supported Volatility Instruments

The calibration framework supports multiple volatility instrument types:

- **OptionVol quotes**: Supported by `VolSurfaceCalibrator` for various underlying assets (requires explicit forward curve specification).
- **SwaptionVol quotes**: ✅ **Now Supported** by `SwaptionVolCalibrator` which properly handles:
  - Normal and lognormal volatility quoting conventions
  - Various ATM strike conventions (swap rate, par rate, delta neutral)
  - SABR model calibration per expiry-tenor combination
  - Accurate forward swap rate and annuity calculations
  - Integration with `SimpleCalibration` workflow

The `SwaptionVolCalibrator` estimates forward swap rates and annuities from appropriate discount curves and swap conventions, making it suitable for interest rate volatility surfaces.

## Key Features

### Deterministic & Parallel Ready
- All calibrations use deterministic algorithms with optional parallelization
- Reproducible results with consistent rounding and ordering
- Full precision preservation in discount factors and other market data

### Extensible Design
- New instruments can be added by implementing simple pricing interfaces
- Custom solvers can be plugged in via the `Solver` trait
- Calibration constraints and objectives are fully customizable

### Market Conventions
- Proper day count handling across all asset classes
- Business day adjustments with holiday calendar support
- Reset lags, settlement conventions, and market-standard interpolation

### New in this release (market-standards updates)
- strict_validation feature flag to escalate calendar/butterfly arbitrage checks to hard errors (`--features strict_validation`)
- Multi-curve separation enforcement via `CalibrationConfig.multi_curve.enforce_separation`
- Explicit discount curve selection for equity/FX vol surfaces (require `discount_id` unless unambiguous)
- Optional `calendar_id` on calibrators for schedule generation
- Newtype IDs: `IndexId` (rates index) and `UnderlyingId` (option underlyings) for safer APIs

## Error Handling & Diagnostics

The framework provides comprehensive diagnostics:
- **Residual Analysis**: Per-instrument pricing errors after calibration
- **Convergence Metrics**: Iteration counts, final objective values, gradient norms
- **Validation Reports**: No-arbitrage checks, curve monotonicity, reasonableness tests
- **Metadata Tracking**: Calibration parameters, market conventions, data sources

## Integration

The calibration framework integrates seamlessly with:
- **Instruments**: All finstack instruments can be used as calibration targets
- **Market Data**: Outputs directly to `MarketContext` for immediate use
- **Metrics**: Calibrated curves work with the existing metrics framework
- **Scenarios**: Calibrated environments support scenario analysis and stress testing

---

*This framework provides the foundation for institutional-grade market data calibration with the flexibility to handle complex instruments and market conditions.*
