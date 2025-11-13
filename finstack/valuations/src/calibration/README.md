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

### Pipeline Calibration

The calibration framework uses an explicit pipeline approach where you define ordered calibration steps:

```rust
use finstack_valuations::calibration::{
    CalibrationSpec, CalibrationStep,
    methods::DiscountCurveCalibrator,
    RatesQuote
};

// Define calibration steps
let spec = CalibrationSpec {
    base_date,
    base_currency: Currency::USD,
    config: CalibrationConfig::default(),
    steps: vec![
        CalibrationStep::Discount {
            calibrator: DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD),
            quotes: vec![
                RatesQuote::Deposit { 
                    maturity: base_date + Duration::days(30),
                    rate: 0.045,
                    day_count: DayCount::Act360,
                },
                RatesQuote::Swap {
                    maturity: base_date + Duration::days(365*2), 
                    rate: 0.047,
                    fixed_freq: Frequency::semi_annual(),
                    float_freq: Frequency::quarterly(),
                    fixed_dc: DayCount::Thirty360,
                    float_dc: DayCount::Act360,
                    index: "USD-OIS".to_string(),
                },
            ],
        },
        // ... more steps
    ],
    schema_version: 1,
};

// Execute calibration pipeline
let result = spec.execute(None)?;
let market_context = MarketContext::try_from(result.final_market)?;
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
- Pipeline-based calibration execution with explicit step ordering
- FRA and Interest Rate Future instruments for short-end calibration
- Framework structure for all curve types
- JSON-driven calibration specifications with stable schemas

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
  - Integration with pipeline workflow

The `SwaptionVolCalibrator` estimates forward swap rates and annuities from appropriate discount curves and swap conventions, making it suitable for interest rate volatility surfaces.

## Key Features

### Explicit Pipeline Mode
- Users define exact calibration order and dependencies
- Each step specifies its calibrator configuration and quotes
- Full control over multi-curve and cross-asset calibration workflows

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
- **Discount Curves**: Default MonotoneConvex interpolation with FlatForward extrapolation for no-arbitrage tails
- **Forward Curves**: Default Linear interpolation with FlatForward extrapolation for stable tail rates
- **DF→FWD Conversion**: Preserves negative forwards (no clamping); errors on malformed data instead of silent fallbacks

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

## JSON Serialization

### Overview

The calibration framework supports 100% JSON-driven calibration specifications with stable, versioned schemas. You can define complete calibration runs in JSON, execute them, and serialize the results deterministically.

### Schema Version

Current version: `finstack.calibration/1`

All JSON files use a top-level envelope pattern:
```json
{
  "schema": "finstack.calibration/1",
  "calibration": { ... }
}
```

### Pipeline Mode

Explicit ordered steps with per-step calibrators and quotes:

```json
{
  "schema": "finstack.calibration/1",
  "calibration": {
    "base_date": "2025-01-01",
    "base_currency": "USD",
    "config": { "tolerance": 1e-10, "max_iterations": 100, ... },
    "steps": [
      {
        "kind": "discount",
        "calibrator": {
          "curve_id": "USD-OIS",
          "base_date": "2025-01-01",
          "currency": "USD",
          "solve_interp": "monotone_convex",
          "config": { ... }
        },
        "quotes": [ ... ]
      },
      {
        "kind": "forward",
        "calibrator": {
          "fwd_curve_id": "USD-SOFR-3M-FWD",
          "tenor_years": 0.25,
          "base_date": "2025-01-01",
          "currency": "USD",
          "discount_curve_id": "USD-OIS",
          "time_dc": "Act360",
          ...
        },
        "quotes": [ ... ]
      }
    ],
    "schema_version": 1
  }
}
```

### Curve Identity and Settings

Each calibrator spec defines the structure of its output curve/surface:

**Discount Curves:**
- `curve_id`: Identifier (e.g., "USD-OIS")
- `base_date`: Valuation date
- `currency`: ISO 4217 code
- `solve_interp`: Interpolation style (Linear, LogLinear, MonotoneConvex, FlatFwd)
- `calendar_id`: Optional calendar for instrument schedules

**Forward Curves:**
- `fwd_curve_id`: Identifier (e.g., "USD-SOFR-3M-FWD")
- `tenor_years`: Index tenor (0.25 for 3M)
- `discount_curve_id`: Reference discount curve
- `time_dc`: Day count for time axis (defaults by currency: USD/EUR → Act360, GBP/JPY → Act365F)

**Hazard Curves:**
- `entity`: Reference entity name
- `seniority`: senior_secured | senior | subordinated | junior
- `recovery_rate`: 0.0 to 1.0 (typically 0.40)
- `discount_curve_id`: Collateral discount curve
- Output ID: "{entity}-{seniority}" (e.g., "AAPL-Senior")

**Inflation Curves:**
- `curve_id`: Index identifier (e.g., "US-CPI-U")
- `base_cpi`: Initial CPI level
- `discount_id`: Discount curve for PV
- `time_dc`, `accrual_dc`: Day counts (default ActAct)
- `solve_interp`: Interpolation (default LogLinear)
- `inflation_lag`: Lag specification (e.g., "months:3" for TIPS)

**Volatility Surfaces:**
- `surface_id`: Identifier (e.g., "SPY-VOL")
- `beta`: SABR beta (1.0 for equity, 0.5 for rates)
- `target_expiries`: Grid of expiry times (years)
- `target_strikes`: Grid of strike levels
- `time_dc`: Day count for expiries (default Act365F)
- `discount_id`: Required when multiple discount curves exist

### Default Naming Conventions

- **Discount**: `{CCY}-OIS`
- **Forward**: `{CCY}-{INDEX}-{TENOR}-FWD` (e.g., "USD-SOFR-3M-FWD")
- **Hazard**: `{ENTITY}-{Seniority}` (e.g., "AAPL-Senior")
- **Inflation**: `{INDEX}` (e.g., "US-CPI-U")
- **Vol**: `{UNDERLYING}-VOL` or `{CCY}-SWPT-VOL`

### Examples

See `tests/calibration/json_examples/` for canonical examples:
- `rates_only_pipeline.json` - OIS discount and forward curves
- `credit_pipeline.json` - Discount + multiple hazard curves
- `vol_pipeline.json` - Discount + SABR equity vol surface
- `full_market_pipeline.json` - Complete multi-step pipeline (discount → hazard)

### Schemas

JSON schemas are available under `schemas/calibration/1/`:
- `calibration.schema.json` - Top-level envelope and spec
- `calibration_step.schema.json` - Pipeline step definitions
- `calibration_result.schema.json` - Result envelope
- `quotes.schema.json` - Market quote types
- `config.schema.json` - Calibration configuration

### Execution

```rust
use finstack_valuations::calibration::CalibrationEnvelope;

// Load from JSON
let envelope = CalibrationEnvelope::from_str(&json_string)?;

// Execute calibration
let result_envelope = envelope.execute(None)?;

// Serialize result
let result_json = result_envelope.to_string()?;
```

### Result Structure

Calibration results include:
- **final_market**: Complete `MarketContextState` with all calibrated curves/surfaces
- **report**: Merged calibration report with residuals and convergence metrics
- **step_reports**: Per-step diagnostics (for pipeline mode)
- **results_meta**: Timestamp, version, rounding context

### Round-Trip Guarantees

All calibration specs and results support deterministic round-trips:
1. JSON → Rust struct → JSON (preserves structure)
2. Execute → Serialize → Deserialize → Re-execute (deterministic)

Tests: `tests/calibration_roundtrip.rs`, `tests/calibration_state_roundtrip.rs`

---

*This framework provides the foundation for institutional-grade market data calibration with the flexibility to handle complex instruments and market conditions.*
