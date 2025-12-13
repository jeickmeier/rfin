# Calibration Module

**Comprehensive calibration framework for term structures and volatility surfaces**

## Overview

The calibration module provides market-standard methodologies for bootstrapping yield curves, hazard curves, inflation curves, volatility surfaces, and correlation structures from market quotes. It implements both sequential bootstrapping (for term structures) and global optimization (for surfaces and model parameters).

### Key Features

- **Multi-curve framework**: Post-2008 standard with separate discount and forward curves
- **Sequential bootstrapping**: Efficient iterative calibration for term structures
- **Global optimization**: Levenberg-Marquardt and Differential Evolution solvers
- **Volatility surfaces**: SABR model calibration with analytical derivatives
- **Credit structures**: Hazard curves and base correlation from CDS and tranche quotes
- **JSON specifications**: Serializable calibration pipelines with stable schemas
- **Validation**: No-arbitrage and monotonicity checks for all structures
- **Diagnostics**: Rich reporting with residuals, convergence metrics, and explanations

### Design Philosophy

1. **Market standards**: ISDA CDS, SABR volatility, multi-curve interest rates
2. **Instrument-based pricing**: Uses actual instrument pricers, not reimplemented formulas
3. **Deterministic**: Reproducible results with stable serialization
4. **Composable**: Pipeline architecture for complex multi-step calibrations
5. **Observable**: Progress reporting and explanation tracing for transparency

---

## Module Architecture

```
calibration/
├── mod.rs                      # Public API and solver helpers
├── traits.rs                   # Calibrator<Input, Output> trait
├── spec.rs                     # JSON serialization framework
├── quote.rs                    # Market quote types
├── config.rs                   # Solver configuration
├── validation.rs               # No-arbitrage validators
├── report.rs                   # Diagnostic reporting
│
├── methods/                    # Calibration implementations
│   ├── discount.rs             # OIS discount curves
│   ├── forward_curve.rs        # Forward rate curves
│   ├── hazard_curve.rs         # Credit hazard curves
│   ├── inflation_curve.rs      # Inflation (CPI) curves
│   ├── sabr_surface.rs         # SABR volatility surfaces
│   ├── swaption_vol.rs         # Swaption volatility surfaces
│   ├── base_correlation.rs     # CDO base correlation
│   └── convexity.rs            # Futures/CMS convexity adjustments
│
└── derivatives/                # Analytical gradients
    ├── sabr_derivatives.rs     # SABR Jacobian for LM solver
    └── sabr_model_params.rs    # SABR parameter wrapper
```

---

## Core Concepts

### 1. The Calibrator Trait

All calibration processes implement the `Calibrator<Input, Output>` trait:

```rust
pub trait Calibrator<Input, Output> {
    fn calibrate(
        &self,
        quotes: &[Input],
        base_context: &MarketContext,
    ) -> Result<(Output, CalibrationReport)>;
}
```

**Implementations:**

- `DiscountCurveCalibrator`: `Calibrator<RatesQuote, DiscountCurve>`
- `ForwardCurveCalibrator`: `Calibrator<RatesQuote, ForwardCurve>`
- `HazardCurveCalibrator`: `Calibrator<CreditQuote, HazardCurve>`
- `InflationCurveCalibrator`: `Calibrator<InflationQuote, InflationCurve>`
- `VolSurfaceCalibrator`: `Calibrator<VolQuote, VolSurface>`
- `SwaptionVolCalibrator`: `Calibrator<VolQuote, VolSurface>`
- `BaseCorrelationCalibrator`: `Calibrator<CreditQuote, BaseCorrelationCurve>`

### 2. Sequential Bootstrapping

Builds curves iteratively by solving one point at a time. Each new point uses the previously calibrated portion of the curve:

1. **Sort quotes** by maturity
2. **For each quote**:
   - Build partial curve from solved knots
   - Solve for discount factor / hazard rate that reprices instrument to market
   - Add knot to curve
3. **Interpolate** between knots
4. **Validate** no-arbitrage constraints

**Used for:** Discount curves, forward curves, hazard curves, inflation curves

### 3. Global Optimization

Fits all parameters simultaneously to minimize total pricing error across all quotes:

1. **Define objective function**: Sum of squared pricing errors
2. **Initialize parameters**: ATM vol, reasonable bounds
3. **Solve**: Levenberg-Marquardt or Differential Evolution
4. **Build output**: Construct surface/curve from optimal parameters

**Used for:** SABR surfaces, swaption volatility, base correlation

### 4. Multi-Curve Framework

Post-2008 standard practice separates discounting and forward projection:

**Calibration order:**
1. **Discount curve** (OIS): Calibrate first using deposits + OIS swaps
2. **Forward curves** (IBOR/RFR): Calibrate second using FRAs, futures, and tenor swaps

This captures the **basis spread** between different rate indices (e.g., LIBOR vs OIS).

### 5. Pipeline Execution

Define multi-step calibrations in JSON with explicit ordering:

```rust
CalibrationSpec {
    base_date,
    base_currency,
    config,
    steps: [
        CalibrationStep::Discount { calibrator, quotes },  // Step 1: OIS curve
        CalibrationStep::Forward { calibrator, quotes },   // Step 2: 3M-LIBOR
        CalibrationStep::Vol { calibrator, quotes },       // Step 3: Vol surface
    ]
}
```

Each step produces an updated `MarketContext` that feeds into the next step.

### 6. Configuration via `FinstackConfig.extensions`

Calibration settings can be sourced from the central `FinstackConfig` object via its `extensions` map.
This enables passing a single config object through all calibration entrypoints.

**Extension keys:**
- `valuations.calibration.v1`: General calibration settings (solver, tolerances, rate bounds)
- `valuations.hull_white_calibration.v1`: Hull-White model calibration settings

**Example:**

```json
{
  "extensions": {
    "valuations.calibration.v1": {
      "tolerance": 1e-10,
      "solver_kind": "Brent",
      "rate_bounds_policy": "auto_currency"
    },
    "valuations.hull_white_calibration.v1": {
      "fix_kappa": 0.03,
      "tree_steps": 100
    }
  }
}
```

**Usage in Rust:**

```rust
use finstack_core::config::FinstackConfig;
use finstack_valuations::calibration::methods::DiscountCurveCalibrator;

let mut cfg = FinstackConfig::default();
cfg.extensions.insert(
    "valuations.calibration.v1",
    serde_json::json!({ "tolerance": 1e-8, "solver_kind": "Newton" }),
);

let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
    .with_finstack_config(&cfg)?;
```

All fields in the extension schemas are optional; unspecified fields use deterministic defaults.

---

## Feature Set

### Term Structure Calibration

#### Discount Curves (`methods/discount.rs`)

- **Instruments**: Deposits, OIS swaps, FRAs, futures
- **Bootstrapping**: Sequential solve for discount factors
- **Interpolation**: Linear, monotone convex, cubic spline
- **Validation**: Monotonic DFs, positive forward rates

**Key methods:**
- `DiscountCurveCalibrator::new(curve_id, base_date, currency)`
- `.with_solve_interp(InterpStyle)` - Set interpolation
- `.with_finstack_config(&FinstackConfig)` - Configure solver via extensions
- `.calibrate(&quotes, &context)` - Execute calibration

#### Forward Curves (`methods/forward_curve.rs`)

- **Instruments**: FRAs, futures (with convexity adjustment), basis swaps
- **Multi-curve**: Requires discount curve in context
- **Tenor**: Specify IBOR tenor (e.g., 0.25 for 3M-LIBOR)
- **Bootstrapping**: Solve for forward rates given discount curve

#### Hazard Curves (`methods/hazard_curve.rs`)

- **Instruments**: CDS spreads (par and upfront)
- **ISDA Standard**: Follows ISDA CDS Standard Model 1.8.2
- **Recovery rate**: Configurable, typically 0.40 for senior unsecured
- **Output**: Survival probabilities, hazard rates

#### Inflation Curves (`methods/inflation_curve.rs`)

- **Instruments**: Zero-coupon inflation swaps, year-on-year swaps
- **Output**: CPI index level curve
- **Validation**: Positive CPI, reasonable inflation rates

### Surface Calibration

#### SABR Volatility Surfaces (`methods/sabr_surface.rs`)

- **Model**: Stochastic Alpha Beta Rho (SABR) per expiry
- **Calibration**: Levenberg-Marquardt with analytical Jacobian
- **Parameters**: α (ATM vol), β (CEV exponent), ρ (correlation), ν (vol-of-vol)
- **Output**: Interpolated volatility surface
- **Validation**: Calendar spread arbitrage, butterfly arbitrage

**Gradient modes:**
- **Analytical** (default): Fast approximations treating x(z) as ~constant
- **Finite-difference**: More accurate but slower (`config.use_fd_sabr_gradients = true`)

#### Swaption Volatility (`methods/swaption_vol.rs`)

- **Dimensions**: Expiry × Tenor × Strike
- **Quotes**: ATM, OTM swaption vols
- **Calibration**: SABR per expiry-tenor pair
- **Market conventions**: Broker-normal vs lognormal quote types

#### Base Correlation (`methods/base_correlation.rs`)

- **Instruments**: CDO tranches (attachment/detachment points)
- **Constraint**: Monotonically increasing with detachment
- **Use case**: Synthetic CDO pricing, index tranches

### Validation (`validation.rs`)

**Curve validators:**
- **No-arbitrage**: Forward rates positive, survival probs decreasing
- **Monotonicity**: Discount factors decreasing, hazard rates reasonable
- **Bounds**: Rates in sensible ranges (e.g., -5% to 50%)

**Surface validators:**
- **Calendar spread**: Total variance (σ²T) increasing with time
- **Butterfly spread**: Convexity constraints on strike dimension
- **Vol bounds**: Positive volatility, cap at 500%

**Validation modes:**
- `ValidationMode::Warn` - Log warnings
- `ValidationMode::Error` - Fail on validation issues (default)
- `ValidationMode::Error` - Hard errors
- Feature `strict_validation` - Escalate all to errors

### Configuration (`config.rs`)

#### CalibrationConfig

```rust
CalibrationConfig {
    tolerance: f64,              // Solver convergence tolerance
    max_iterations: usize,       // Iteration limit
    use_parallel: bool,          // Parallel execution (breaks determinism)
    random_seed: Option<u64>,    // For reproducible DE solver
    verbose: bool,               // Logging level
    solver_kind: SolverKind,     // Newton, Brent, Hybrid, LM, DE
    entity_seniority: HashMap,   // Credit entity mappings
    multi_curve: MultiCurveConfig,
    use_fd_sabr_gradients: bool, // FD vs analytical for SABR
    explain: ExplainOpts,        // Explanation tracing
    validation_mode: ValidationMode,
}
```

**Presets:**
- `CalibrationConfig::default()` - Balanced (tol=1e-10, iter=100)
- `.conservative()` - High precision (tol=1e-12, FD gradients)
- `.aggressive()` - Fast convergence (tol=1e-6, iter=1000)
- `.fast()` - Quick approximate (tol=1e-4, iter=50, Brent solver)

#### SolverKind

- **Newton**: Fast if good initial guess, requires derivatives
- **Brent**: Robust bracketing, no derivatives needed
- **Hybrid**: Newton first, falls back to Brent (recommended for 1D)
- **LevenbergMarquardt**: Multi-dimensional least squares
- **DifferentialEvolution**: Global optimization, handles multiple local minima

### Reporting (`report.rs`)

```rust
CalibrationReport {
    success: bool,
    residuals: BTreeMap<String, f64>,    // Per-instrument pricing errors
    iterations: usize,
    objective_value: f64,
    max_residual: f64,
    rmse: f64,                            // Root mean square error
    convergence_reason: String,
    metadata: BTreeMap<String, String>,   // Domain-specific info
    results_meta: ResultsMeta,            // Timestamp, version, rounding
    explanation: Option<ExplanationTrace>,// Opt-in detailed trace
}
```

### JSON Serialization (`spec.rs`)

#### CalibrationEnvelope

```rust
CalibrationEnvelope {
    schema: "finstack.calibration/1",
    calibration: CalibrationSpec,
}
```

**Stability guarantees:**
- `#[serde(deny_unknown_fields)]` - Strict schema enforcement
- Stable field names for long-term pipelines
- Round-trip determinism

#### CalibrationSpec

```rust
CalibrationSpec {
    base_date: Date,
    base_currency: Currency,
    config: CalibrationConfig,
    steps: Vec<CalibrationStep>,
}
```

**Execution:**
```rust
let envelope = CalibrationEnvelope::from_json(&json_string)?;
let result = envelope.execute(None)?;  // Returns CalibrationResultEnvelope
```

---

## Usage Examples

### Example 1: OIS Discount Curve Calibration

```rust
use finstack_valuations::calibration::{
    DiscountCurveCalibrator, CalibrationConfig, RatesQuote, Calibrator
};
use finstack_core::prelude::*;
use finstack_core::dates::{create_date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::types::IndexId;
use time::Month;

let base_date = create_date(2025, Month::January, 15)?;

// Create calibrator
let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
    .with_config(CalibrationConfig::default());

// Define market quotes
let quotes = vec![
    // Overnight deposit
    RatesQuote::Deposit {
        maturity: create_date(2025, Month::January, 16)?,
        rate: 0.0520,  // 5.20%
        day_count: DayCount::Act360,
    },
    // 3-month OIS swap
    RatesQuote::Swap {
        maturity: create_date(2025, Month::April, 15)?,
        rate: 0.0515,
        fixed_freq: Tenor::annual(),
        float_freq: Tenor::annual(),
        fixed_dc: DayCount::Act360,
        float_dc: DayCount::Act360,
        index: IndexId::from("SOFR"),
    },
    // 1-year OIS swap
    RatesQuote::Swap {
        maturity: create_date(2026, Month::January, 15)?,
        rate: 0.0500,
        fixed_freq: Tenor::annual(),
        float_freq: Tenor::annual(),
        fixed_dc: DayCount::Act360,
        float_dc: DayCount::Act360,
        index: IndexId::from("SOFR"),
    },
];

// Calibrate
let context = MarketContext::new();
let (discount_curve, report) = calibrator.calibrate(&quotes, &context)?;

// Inspect results
println!("Calibration success: {}", report.success);
println!("Max residual: {:.2e}", report.max_residual);
println!("RMSE: {:.2e}", report.rmse);
println!("Iterations: {}", report.iterations);

// Use curve
let df_1y = discount_curve.df(1.0);  // Discount factor at 1 year
let zero_1y = discount_curve.zero(1.0);  // Zero rate at 1 year
println!("DF(1Y) = {:.6}, Zero(1Y) = {:.4}%", df_1y, zero_1y * 100.0);
```

### Example 2: Multi-Curve Framework (Discount + Forward)

```rust
use finstack_valuations::calibration::{
    DiscountCurveCalibrator, ForwardCurveCalibrator, 
    CalibrationConfig, RatesQuote, Calibrator
};
use finstack_core::prelude::*;
use finstack_core::dates::{create_date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::types::IndexId;
use time::Month;

let base_date = create_date(2025, Month::January, 15)?;
let config = CalibrationConfig::default();

// Step 1: Calibrate OIS discount curve
let ois_calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
    .with_config(config.clone());

let ois_quotes = vec![
    RatesQuote::Deposit {
        maturity: create_date(2025, Month::January, 16)?,
        rate: 0.0520,
        day_count: DayCount::Act360,
    },
    RatesQuote::Swap {
        maturity: create_date(2026, Month::January, 15)?,
        rate: 0.0500,
        fixed_freq: Tenor::annual(),
        float_freq: Tenor::annual(),
        fixed_dc: DayCount::Act360,
        float_dc: DayCount::Act360,
        index: IndexId::from("SOFR"),
    },
];

let mut context = MarketContext::new();
let (discount_curve, ois_report) = ois_calibrator.calibrate(&ois_quotes, &context)?;
context = context.insert_discount(discount_curve);

println!("OIS calibration: max_residual={:.2e}", ois_report.max_residual);

// Step 2: Calibrate 3M-LIBOR forward curve (requires discount curve)
let fwd_calibrator = ForwardCurveCalibrator::new(
    "USD-LIBOR-3M",
    0.25,  // 3-month tenor
    base_date,
    Currency::USD,
    "USD-OIS",  // Discount curve ID
).with_config(config);

let libor_quotes = vec![
    RatesQuote::FRA {
        start: create_date(2025, Month::April, 15)?,
        end: create_date(2025, Month::July, 15)?,
        rate: 0.0525,
        day_count: DayCount::Act360,
    },
    RatesQuote::Swap {
        maturity: create_date(2026, Month::January, 15)?,
        rate: 0.0510,
        fixed_freq: Tenor::quarterly(),
        float_freq: Tenor::quarterly(),
        fixed_dc: DayCount::Thirty360,
        float_dc: DayCount::Act360,
        index: IndexId::from("USD-LIBOR-3M"),
    },
];

let (forward_curve, fwd_report) = fwd_calibrator.calibrate(&libor_quotes, &context)?;
context = context.insert_forward(forward_curve);

println!("Forward calibration: max_residual={:.2e}", fwd_report.max_residual);

// Now context contains both discount and forward curves
// Can be used for pricing swaps, FRAs, etc.
```

### Example 3: SABR Volatility Surface

```rust
use finstack_valuations::calibration::{
    VolSurfaceCalibrator, CalibrationConfig, VolQuote, Calibrator
};
use finstack_core::prelude::*;
use finstack_core::dates::create_date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::types::UnderlyingId;
use time::Month;

let base_date = create_date(2025, Month::January, 15)?;

// Define target grid
let target_expiries = vec![0.25, 0.5, 1.0, 2.0];  // 3M, 6M, 1Y, 2Y
let target_strikes = vec![90.0, 95.0, 100.0, 105.0, 110.0];  // Strike prices

// Create calibrator
let calibrator = VolSurfaceCalibrator::new(
    "SPX-VOL",
    0.5,  // Beta = 0.5 (typical for equity)
    target_expiries.clone(),
    target_strikes.clone(),
)
.with_base_date(base_date)
.with_config(CalibrationConfig::default());

// Market quotes (ATM and wings)
let quotes = vec![
    VolQuote::OptionVol {
        underlying: UnderlyingId::from("SPX"),
        expiry: create_date(2025, Month::April, 15)?,  // 3M
        strike: 100.0,  // ATM
        vol: 0.18,      // 18% implied vol
        option_type: "Call".to_string(),
    },
    VolQuote::OptionVol {
        underlying: UnderlyingId::from("SPX"),
        expiry: create_date(2025, Month::April, 15)?,
        strike: 95.0,   // 95 put
        vol: 0.22,      // 22% (higher due to skew)
        option_type: "Put".to_string(),
    },
    VolQuote::OptionVol {
        underlying: UnderlyingId::from("SPX"),
        expiry: create_date(2025, Month::April, 15)?,
        strike: 105.0,  // 105 call
        vol: 0.16,      // 16% (lower due to skew)
        option_type: "Call".to_string(),
    },
    // Additional expiries...
];

let context = MarketContext::new();
let (vol_surface, report) = calibrator.calibrate(&quotes, &context)?;

println!("SABR calibration: success={}, RMSE={:.4}%", 
    report.success, report.rmse * 100.0);

// Use surface
let vol_95_put_3m = vol_surface.value(0.25, 95.0);
println!("Implied vol for 95 put 3M: {:.2}%", vol_95_put_3m * 100.0);
```

### Example 4: JSON Pipeline Execution

```json
{
  "schema": "finstack.calibration/1",
  "calibration": {
    "base_date": "2025-01-15",
    "base_currency": "USD",
    "config": {
      "tolerance": 1e-10,
      "max_iterations": 100,
      "use_parallel": false,
      "solver_kind": "Hybrid",
      "validation_mode": "Warn"
    },
    "steps": [
      {
        "kind": "discount",
        "calibrator": {
          "curve_id": "USD-OIS",
          "base_date": "2025-01-15",
          "currency": "USD",
          "solve_interp": "MonotoneConvex"
        },
        "quotes": [
          {
            "deposit": {
              "maturity": "2025-01-16",
              "rate": 0.0520,
              "day_count": "Act360"
            }
          },
          {
            "swap": {
              "maturity": "2026-01-15",
              "rate": 0.0500,
              "fixed_freq": "Annual",
              "float_freq": "Annual",
              "fixed_dc": "Act360",
              "float_dc": "Act360",
              "index": "SOFR"
            }
          }
        ]
      },
      {
        "kind": "forward",
        "calibrator": {
          "curve_id": "USD-LIBOR-3M",
          "tenor": 0.25,
          "base_date": "2025-01-15",
          "currency": "USD",
          "discount_id": "USD-OIS"
        },
        "quotes": [
          {
            "fra": {
              "start": "2025-04-15",
              "end": "2025-07-15",
              "rate": 0.0525,
              "day_count": "Act360"
            }
          }
        ]
      }
    ]
  }
}
```

```rust
use finstack_valuations::calibration::CalibrationEnvelope;

// Parse from JSON
let envelope = CalibrationEnvelope::from_json(&json_string)?;

// Execute pipeline
let result_envelope = envelope.execute(None)?;

// Inspect results
println!("Pipeline success: {}", result_envelope.result.report.success);
println!("Final market context has {} discount curves", 
    result_envelope.result.final_market.discount_curves.len());

// Per-step diagnostics
for (step_name, step_report) in &result_envelope.result.step_reports {
    println!("{}: max_residual={:.2e}, iters={}", 
        step_name, step_report.max_residual, step_report.iterations);
}

// Serialize result to JSON
let result_json = result_envelope.to_string()?;
```

### Example 5: Hazard Curve from CDS Spreads

```rust
use finstack_valuations::calibration::{
    HazardCurveCalibrator, CalibrationConfig, CreditQuote, Calibrator
};
use finstack_core::prelude::*;
use finstack_core::dates::create_date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::Seniority;
use time::Month;

let base_date = create_date(2025, Month::January, 15)?;

// Create calibrator
let calibrator = HazardCurveCalibrator::new(
    "ACME-CORP",          // Entity ID
    base_date,
    Currency::USD,
    0.40,                 // 40% recovery rate (senior unsecured)
    Seniority::Senior,
    "USD-OIS",            // Discount curve ID
).with_config(CalibrationConfig::default());

// CDS market quotes
let quotes = vec![
    CreditQuote::CDS {
        entity: "ACME-CORP".to_string(),
        maturity: create_date(2026, Month::January, 15)?,  // 1Y
        spread_bp: 120.0,  // 120 bps
        recovery_rate: 0.40,
        currency: Currency::USD,
    },
    CreditQuote::CDS {
        entity: "ACME-CORP".to_string(),
        maturity: create_date(2030, Month::January, 15)?,  // 5Y
        spread_bp: 150.0,  // 150 bps
        recovery_rate: 0.40,
        currency: Currency::USD,
    },
];

// Need discount curve in context
let mut context = MarketContext::new();
// ... (assume discount curve already calibrated and inserted)

let (hazard_curve, report) = calibrator.calibrate(&quotes, &context)?;

println!("Hazard calibration: success={}, max_residual={:.2e}", 
    report.success, report.max_residual);

// Use curve
let survival_prob_1y = hazard_curve.sp(1.0);  // Survival probability
let hazard_rate_1y = -survival_prob_1y.ln();  // Integrated hazard rate
println!("1Y survival prob: {:.4} ({:.2}% default prob)", 
    survival_prob_1y, (1.0 - survival_prob_1y) * 100.0);
```

### Example 6: Validation and Diagnostics

```rust
use finstack_valuations::calibration::{
    DiscountCurveCalibrator, CalibrationConfig, Calibrator,
    CurveValidator, ValidationConfig
};
use finstack_core::market_data::context::MarketContext;

let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
    .with_config(CalibrationConfig::conservative());  // Strict settings

// ... define quotes ...

let (discount_curve, report) = calibrator.calibrate(&quotes, &MarketContext::new())?;

// Validate curve
match discount_curve.validate() {
    Ok(_) => println!("Curve passes all validations"),
    Err(e) => {
        eprintln!("Validation failed: {}", e);
        // Decide whether to accept or reject
    }
}

// Check specific constraints
discount_curve.validate_no_arbitrage()?;  // Forward rates positive
discount_curve.validate_monotonicity()?;  // DFs decreasing
discount_curve.validate_bounds()?;        // Rates in reasonable range

// Inspect residuals
for (instrument, residual) in &report.residuals {
    if residual.abs() > 1e-6 {
        println!("Warning: {} has residual {:.2e}", instrument, residual);
    }
}

// Check convergence
if report.iterations >= report.results_meta.max_iterations {
    eprintln!("Warning: Calibration hit iteration limit");
}
```

### Example 7: Explanation Tracing

```rust
use finstack_valuations::calibration::{
    DiscountCurveCalibrator, CalibrationConfig, Calibrator
};
use finstack_core::explain::ExplainOpts;

// Enable explanation tracing
let config = CalibrationConfig::default()
    .with_explain();  // Enable detailed trace

let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
    .with_config(config);

// ... define quotes ...

let (curve, report) = calibrator.calibrate(&quotes, &MarketContext::new())?;

// Inspect explanation trace
if let Some(trace) = &report.explanation {
    println!("Calibration steps:");
    for entry in &trace.entries {
        println!("  - {}: {}", entry.level, entry.message);
    }
}
```

---

## How to Add New Features

### Adding a New Calibrator

Follow these steps to add a calibrator for a new structure type:

#### 1. Define Quote Type (in `quote.rs`)

```rust
/// New instrument quote type
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum NewQuote {
    InstrumentA {
        maturity: Date,
        rate: f64,
        // ... other fields
    },
    InstrumentB {
        // ...
    },
}
```

#### 2. Create Calibrator Module

Create `methods/new_structure.rs`:

```rust
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator, NewQuote};
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Calibrator for NewStructure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewStructureCalibrator {
    pub structure_id: String,
    pub base_date: Date,
    pub config: CalibrationConfig,
    // ... other fields
}

impl NewStructureCalibrator {
    pub fn new(structure_id: impl Into<String>, base_date: Date) -> Self {
        Self {
            structure_id: structure_id.into(),
            base_date,
            config: CalibrationConfig::default(),
        }
    }
    
    pub fn with_config(mut self, config: CalibrationConfig) -> Self {
        self.config = config;
        self
    }
    
    fn validate_quotes(&self, quotes: &[NewQuote]) -> Result<()> {
        // Check quote ordering, no duplicates, etc.
        Ok(())
    }
}

impl Calibrator<NewQuote, NewStructure> for NewStructureCalibrator {
    fn calibrate(
        &self,
        quotes: &[NewQuote],
        base_context: &MarketContext,
    ) -> Result<(NewStructure, CalibrationReport)> {
        // 1. Validate quotes
        self.validate_quotes(quotes)?;
        
        // 2. Build structure iteratively or via optimization
        let mut residuals = BTreeMap::new();
        let mut iterations = 0;
        
        // ... calibration logic ...
        
        // 3. Create output structure
        let structure = NewStructure::builder(self.structure_id.clone())
            // ... configure from calibrated parameters
            .build()?;
        
        // 4. Generate report
        let report = CalibrationReport::new(
            residuals,
            iterations,
            true,
            "NewStructure calibration completed"
        );
        
        Ok((structure, report))
    }
}
```

#### 3. Add Validation (in `validation.rs`)

```rust
impl CurveValidator for NewStructure {
    fn validate_no_arbitrage(&self) -> Result<()> {
        // Check no-arbitrage conditions
        Ok(())
    }
    
    fn validate_monotonicity(&self) -> Result<()> {
        // Check monotonicity constraints
        Ok(())
    }
    
    fn validate_bounds(&self) -> Result<()> {
        // Check values in reasonable ranges
        Ok(())
    }
}
```

#### 4. Add to Pipeline (in `spec.rs`)

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CalibrationStep {
    // ... existing steps ...
    
    NewStructure {
        calibrator: NewStructureCalibrator,
        quotes: Vec<NewQuote>,
    },
}

impl CalibrationStep {
    pub fn execute(&self, context: &MarketContext) -> Result<(MarketContext, CalibrationReport)> {
        match self {
            // ... existing cases ...
            
            CalibrationStep::NewStructure { calibrator, quotes } => {
                let (structure, report) = calibrator.calibrate(quotes, context)?;
                Ok((context.clone().insert_new_structure(structure), report))
            }
        }
    }
}
```

#### 5. Export from Module

In `methods/mod.rs`:

```rust
pub mod new_structure;
pub use new_structure::*;
```

In `calibration/mod.rs`:

```rust
pub use quote::NewQuote;
```

#### 6. Add Tests

Create `calibration/methods/new_structure.rs` tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_new_structure_calibration() {
        let calibrator = NewStructureCalibrator::new("TEST", base_date);
        let quotes = vec![/* test quotes */];
        let context = MarketContext::new();
        
        let (structure, report) = calibrator.calibrate(&quotes, &context).unwrap();
        
        assert!(report.success);
        assert!(report.max_residual < 1e-6);
        
        // Validate output
        structure.validate().unwrap();
    }
    
    #[test]
    fn test_new_structure_validation() {
        // Test validation logic
    }
    
    #[test]
    fn test_new_structure_json_roundtrip() {
        // Test serialization
    }
}
```

### Adding Analytical Derivatives

For global optimization calibrators (like SABR), analytical gradients significantly improve convergence.

#### 1. Create Derivatives Module

Create `derivatives/new_model_derivatives.rs`:

```rust
use finstack_core::Result;

/// Analytical Jacobian for NewModel calibration
pub struct NewModelDerivatives {
    pub expiry: f64,
    pub forward: f64,
    // ... other market data
}

impl NewModelDerivatives {
    pub fn new(expiry: f64, forward: f64) -> Self {
        Self { expiry, forward }
    }
    
    /// Compute Jacobian: dVol/dParam for each parameter
    pub fn jacobian(
        &self,
        strike: f64,
        params: &NewModelParams,
    ) -> Result<Vec<f64>> {
        let mut jac = Vec::with_capacity(params.len());
        
        // Analytical derivatives: ∂σ/∂param_i
        jac.push(self.d_vol_d_param1(strike, params));
        jac.push(self.d_vol_d_param2(strike, params));
        // ...
        
        Ok(jac)
    }
    
    fn d_vol_d_param1(&self, strike: f64, params: &NewModelParams) -> f64 {
        // Analytical derivative computation
        unimplemented!()
    }
}
```

#### 2. Use in Calibrator

```rust
impl Calibrator<VolQuote, VolSurface> for NewModelCalibrator {
    fn calibrate(
        &self,
        quotes: &[VolQuote],
        base_context: &MarketContext,
    ) -> Result<(VolSurface, CalibrationReport)> {
        // ... group quotes by expiry ...
        
        for (expiry, slice_quotes) in quotes_by_expiry {
            let derivatives = NewModelDerivatives::new(expiry, forward);
            
            // Build objective with analytical Jacobian
            let objective = |params: &[f64]| {
                let model_params = NewModelParams::from_slice(params);
                let mut residuals = Vec::new();
                let mut jacobian_rows = Vec::new();
                
                for quote in slice_quotes {
                    let model_vol = model_params.vol(quote.strike);
                    let market_vol = quote.vol;
                    
                    residuals.push(model_vol - market_vol);
                    jacobian_rows.push(derivatives.jacobian(quote.strike, &model_params)?);
                }
                
                Ok((residuals, jacobian_rows))
            };
            
            // Solve with LM using analytical Jacobian
            let solver = self.config.create_lm_solver();
            let optimal_params = solver.solve_with_jacobian(objective, &initial_guess)?;
            
            // ... store calibrated params ...
        }
        
        // ... build surface from all expiry slices ...
    }
}
```

---

## Performance Notes

### Bootstrapping Performance

Sequential bootstrapping is O(n) in number of quotes, with each solve taking 5-20 iterations typically:

- **Deposits**: 1-3 iterations (linear in rate)
- **Swaps**: 5-10 iterations (requires annuity calculation)
- **Futures**: 10-20 iterations (convexity adjustment complicates)

**Optimization tips:**
1. Use `MonotoneConvex` interpolation during solve (faster than cubic spline)
2. Provide good initial guesses (previous DF, par rate from nearest quote)
3. Disable parallel mode for determinism (minimal performance impact for bootstrapping)

### Surface Calibration Performance

SABR calibration is O(k × m × n) where:
- k = number of expiries
- m = number of strikes per expiry
- n = LM iterations (typically 10-50)

**Optimization tips:**
1. Use analytical derivatives (10x faster than finite differences)
2. Calibrate per expiry in parallel if determinism not required
3. Use good initial guesses: α = ATM vol, β = 0.5, ρ = -0.3, ν = 0.3
4. Limit strike range to reduce m (use extrapolation for far wings)

### Validation Performance

Validation checks are typically <1ms per curve/surface. To minimize overhead:

1. Use `ValidationMode::Error` in production (fail-fast; market standard)
2. Sample test points instead of full grid for large surfaces
3. Skip validation in hot pricing loops (validate once after calibration)

### Memory Usage

Typical memory footprint:
- **Discount curve**: ~1 KB (100 knots)
- **SABR surface**: ~10 KB (20 expiries × 20 strikes)
- **CalibrationReport**: ~1-5 KB (depends on residuals count)
- **Explanation trace**: ~10-100 KB if enabled (disabled by default)

---

## Testing Standards

### Unit Tests

Each calibrator should have:

1. **Smoke test**: Calibrate from minimal valid quotes
2. **Accuracy test**: Known quotes → known curve (golden values)
3. **Validation test**: Invalid input → proper error
4. **Round-trip test**: Curve → quotes → calibrate → same curve
5. **JSON test**: Serialize → deserialize → identical

### Integration Tests

- **Multi-curve pipeline**: OIS → forward → pricing consistency
- **Surface+Curve**: Vol surface + discount curve → option pricing
- **Credit**: Hazard + discount → CDS pricing matches market

### Property Tests

Use `proptest` for:
- Monotonicity preservation across random quote sets
- No-arbitrage constraints hold for all valid inputs
- Solver convergence within iteration limit

---

## Mathematical References

### Interest Rate Curve Bootstrapping

- **Hagan & West (2006)**: "Interpolation Methods for Curve Construction"
  - Monotone convex interpolation for arbitrage-free curves
  - Hermite splines preserving forward rate positivity

- **Ametrano & Bianchetti (2013)**: "Multiple Interest Rate Curve Bootstrapping"
  - Multi-curve framework: OIS discounting, IBOR projection
  - Basis spread modeling and calibration order

### Credit Curve Calibration

- **O'Kane & Turnbull (2003)**: "Valuation of Credit Default Swaps"
  - Hazard rate bootstrapping from CDS spreads
  - Recovery rate conventions and seniority hierarchies

- **ISDA CDS Standard Model**: Version 1.8.2
  - Day count conventions (Act/360 for CDS)
  - Accrual-on-default treatment
  - Standard coupon and upfront quote conversion

### Volatility Surface Calibration

- **Hagan et al. (2002)**: "Managing Smile Risk" (SABR model)
  - Stochastic-alpha-beta-rho model for smile dynamics
  - Closed-form approximations for implied volatility
  - Calibration to market swaption/cap volatilities

- **Gatheral (2004)**: "A Parsimonious Arbitrage-Free Implied Volatility Parameterization"
  - SVI (Stochastic Volatility Inspired) model
  - No-arbitrage conditions: calendar spread, butterfly spread

- **Andersen & Brotherton-Ratcliffe (2005)**: "Extended LIBOR Market Models"
  - Swaption volatility cube: expiry × tenor × strike
  - Correlation structure and smile interpolation

### Numerical Methods

- **Brent (1973)**: "Algorithms for Minimization Without Derivatives"
  - Robust bracketing method for 1D root-finding
  - Guaranteed convergence if bracket contains root

- **Marquardt (1963)**: "An Algorithm for Least-Squares Estimation of Nonlinear Parameters"
  - Levenberg-Marquardt for nonlinear least squares
  - Combines gradient descent and Gauss-Newton

- **Storn & Price (1997)**: "Differential Evolution – A Simple and Efficient Heuristic for Global Optimization"
  - Population-based global optimizer
  - Good for multimodal objective functions (correlation calibration)

---

## Common Pitfalls and Solutions

### 1. Insufficient Quotes

**Problem**: Too few quotes to construct curve
**Solution**: 
- Require minimum 2 quotes for linear interpolation
- Use longer extrapolation beyond last quote
- Add synthetic quotes from model/historical data

### 2. Non-Monotonic Market Quotes

**Problem**: CDS spreads decrease with maturity
**Solution**:
- Smooth quotes before calibration
- Use regularization in objective function
- Flag data quality issues in report

### 3. Solver Divergence

**Problem**: Newton solver fails, residuals explode
**Solution**:
- Use `SolverKind::Hybrid` (falls back to Brent)
- Provide better initial guess
- Check for data errors (e.g., rate = 50% instead of 0.50)

### 4. Calendar Arbitrage in Vol Surface

**Problem**: Total variance decreases with expiry
**Solution**:
- Use SVI or another arbitrage-free parameterization
- Constrain SABR fit with variance monotonicity penalty
- Post-process surface to enforce arbitrage-free constraints

### 5. Multi-Curve Order Dependency

**Problem**: Forward curve calibration fails without discount curve
**Solution**:
- Always calibrate discount curve first
- Use `CalibrationSpec` pipeline to enforce ordering
- Check `base_context` has required curves before calibration

### 6. JSON Deserialization Failures

**Problem**: `deny_unknown_fields` rejects valid JSON
**Solution**:
- Check schema version matches (`finstack.calibration/1`)
- Verify all required fields present
- Use exact field names (snake_case, not camelCase)

---

## Design Decisions

### Why Instrument-Based Pricing?

**Decision**: Use actual instrument pricers instead of closed-form formulas in calibrators.

**Rationale**:
1. **Consistency**: Pricing logic in one place, shared by calibration and valuation
2. **Maintainability**: Changes to instrument pricing automatically propagate
3. **Accuracy**: Handles all nuances (holidays, day counts, compounding)

**Trade-off**: Slightly slower than specialized calibration formulas, but negligible for typical curve sizes.

### Why Sequential Bootstrapping for Curves?

**Decision**: Bootstrap term structures iteratively rather than global solve.

**Rationale**:
1. **Efficiency**: O(n) vs O(n³) for global solve
2. **Stability**: Each knot depends only on earlier knots (no simultaneity)
3. **Interpretability**: Clear 1-1 mapping from quote to knot

**Trade-off**: Cannot handle cross-dependencies (e.g., calendar spreads). Use global solve for those.

### Why Per-Expiry SABR vs Global Surface?

**Decision**: Calibrate SABR parameters independently per expiry slice.

**Rationale**:
1. **Tractability**: 4 parameters per expiry vs 4×k parameters globally
2. **Stability**: Localized fitting reduces overfitting risk
3. **Market practice**: Traders typically manage smile per expiry

**Trade-off**: No smoothness constraint across expiries. Add post-processing if needed.

### Why JSON Schema Versioning?

**Decision**: Explicit schema version in all serialized envelopes.

**Rationale**:
1. **Long-term stability**: Pipelines survive code changes
2. **Migration path**: Can support multiple schema versions
3. **Validation**: Reject incompatible data early

**Trade-off**: Slight verbosity in JSON. Acceptable for clarity.

---

## Future Enhancements

### Planned Features

1. **Alternative surface models**: SVI, SSVI, local volatility
2. **Curve smoothing**: Tension splines, regularization penalties
3. **Cross-curve constraints**: No calendar spread arbitrage across surfaces
4. **Parallel calibration**: Per-expiry or per-instrument parallelism (optional)
5. **Incremental updates**: Recalibrate subset of quotes without full rebuild
6. **Model risk**: Multiple calibrations with different settings, ensemble average

### Research Directions

1. **Machine learning**: Neural network calibration for speed
2. **Uncertainty quantification**: Bootstrap confidence intervals
3. **Real-time calibration**: Streaming quote updates, online solvers
4. **Multi-asset**: FX triangles, basis spread surfaces

---

## Summary

The calibration module provides production-ready implementations of market-standard calibration methodologies:

- **Sequential bootstrapping** for term structures (discount, forward, hazard, inflation)
- **Global optimization** for surfaces and models (SABR, swaption vol, base correlation)
- **Multi-curve framework** following post-2008 conventions
- **JSON pipelines** for reproducible multi-step calibrations
- **Validation** ensuring no-arbitrage and reasonable outputs
- **Diagnostics** with residuals, convergence metrics, and explanation tracing

All calibrators implement the `Calibrator<Input, Output>` trait, providing a uniform interface for integration with pricing, risk, and scenario analysis workflows.

For questions or contributions, refer to the main Finstack documentation and the `.cursor/rules/rust/` development standards.
