# Covenant Evaluation and Management System

## Overview

The covenants module provides a deterministic, extensible framework for evaluating financial and non-financial covenants, managing grace/cure periods, applying consequences for breaches, and forecasting covenant compliance with headroom analytics.

**Key capabilities:**

- Evaluate financial covenants (leverage, coverage, custom metrics) against thresholds
- Support non-financial covenants (affirmative/negative) with custom evaluators
- Track breach history with cure period management
- Apply configurable consequences (default, rate increase, cash sweep, distribution blocks, etc.)
- Forward-project covenant compliance with deterministic and stochastic headroom analysis
- Define covenant testing windows for time-based activation
- Integrate with metric systems and time-series models

**Design principles:**

- Determinism: Same inputs → same outputs, stable breach detection
- Extensibility: Custom covenant types, metrics, evaluators, and consequences
- Separation of concerns: Evaluation logic decoupled from instrument mutation
- Generic forecasting: Time-series adapter pattern avoids crate cycles

---

## Architecture

### Module Structure

```
covenants/
├── mod.rs              → Public exports
├── engine.rs           → Covenant evaluation engine, consequence application
├── forward.rs          → Forward projection with headroom analytics
├── schedule.rs         → Piecewise-constant threshold schedules
└── mod_types.rs        → CovenantReport result type
```

### Core Components

#### 1. **CovenantEngine** (`engine.rs`)

Central orchestrator for covenant evaluation and consequence application.

- **CovenantSpec**: Links a `Covenant` to either a `MetricId` or custom evaluator
- **CovenantTestSpec**: Serialization helper used by higher-level planners (not consumed directly by `CovenantEngine`)
- **CovenantWindow**: Time-based covenant activation windows (the `is_grace_period` flag is advisory; cure tracking uses `CovenantBreach`)
- **CovenantBreach**: Breach tracking with cure deadlines and applied consequences
- **InstrumentMutator**: Trait for instruments that can be mutated by consequences

#### 2. **Forward Projection** (`forward.rs`)

Generic covenant forecasting with headroom analytics, optional MC simulation.

- **ModelTimeSeries** trait: Adapter for any time-series model (e.g., statements)
- **GenericCovenantForecast**: Projected values, thresholds, headroom, breach probabilities
- **CovenantForecastConfig**: Deterministic or stochastic (MC) configuration

#### 3. **Threshold Schedules** (`schedule.rs`)

Piecewise-constant threshold lookup for time-varying covenant limits.

- **ThresholdSchedule**: Sorted `(Date, f64)` schedule
- **threshold_for_date**: Resolves threshold for a given test date

#### 4. **Reporting** (`mod_types.rs`)

Covenant evaluation results.

- **CovenantReport**: Pass/fail status, actual value, threshold, details

---

## Feature Set

### Covenant Types

#### Financial Covenants

Built-in support for common credit covenants:

- **MaxDebtToEBITDA**: Total debt / EBITDA <= threshold
- **MinInterestCoverage**: EBITDA / interest expense ≥ threshold
- **MinFixedChargeCoverage**: (EBITDA - CapEx) / fixed charges ≥ threshold
- **MaxTotalLeverage**: Total debt / equity <= threshold
- **MaxSeniorLeverage**: Senior debt / equity <= threshold
- **MinAssetCoverage**: Total assets / total debt ≥ threshold
- **Custom**: User-defined metric with minimum or maximum test

#### Non-Financial Covenants

- **Negative**: Restrictions (e.g., "No additional debt without consent")
- **Affirmative**: Requirements (e.g., "Provide quarterly reporting")

### Covenant Consequences

When a covenant is breached and the cure period expires, the engine can apply:

| Consequence               | Effect                                      |
|---------------------------|---------------------------------------------|
| `Default`                 | Mark instrument as defaulted                |
| `RateIncrease`            | Increase interest rate by basis points      |
| `CashSweep`               | Activate cash sweep (percentage of excess)  |
| `BlockDistributions`      | Prevent dividends/distributions             |
| `RequireCollateral`       | Flag collateral requirement (informational) |
| `AccelerateMaturity`      | Shorten maturity date                       |

### Cure Period Management

- **Grace period**: Optional cure window (e.g., 30 days) after initial breach
- **Breach tracking**: Historical breaches with cure status
- **Consequence deferral**: Consequences only apply after cure deadline passes

### Forward Projection & Headroom

Forecast covenant compliance over future periods:

- **Deterministic**: Project metric values from time-series model
- **Stochastic (MC)**: Lognormal shocks to metric values with breach probability estimation
- **Headroom**: Percentage cushion relative to threshold
- **Summary analytics**: First breach date, minimum headroom date/value
- **Variance reduction**: Antithetic variates support

### Custom Evaluation

Two extension points:

1. **Custom metrics**: Register metric calculators via `CovenantEngine::register_metric`
2. **Custom evaluators**: Provide arbitrary `Fn(&MetricContext) -> Result<bool>` closures

### Covenant Windows

Time-based covenant activation:

- Define windows with `start`, `end`, and active covenants
- Overrides base specs during window periods
- Support grace period windows

---

## Usage Examples

### 1. Basic Covenant Evaluation

```rust
use finstack_valuations::covenants::{
    Covenant, CovenantEngine, CovenantSpec, CovenantType,
};
use finstack_core::dates::{Date, Tenor};
use finstack_valuations::metrics::{MetricContext, MetricId};

// Create covenant
let leverage_covenant = Covenant::new(
    CovenantType::MaxTotalLeverage { threshold: 5.0 },
    Tenor::quarterly(),
)
.with_cure_period(Some(30));

// Build engine and add spec
let mut engine = CovenantEngine::new();
engine.add_spec(CovenantSpec::with_metric(
    leverage_covenant,
    MetricId::custom("total_leverage"),
));

// Prepare metric context (simplified)
let mut context = MetricContext::new(
    Arc::new(instrument),
    Arc::new(market),
    as_of,
    base_value,
    MetricContext::default_config(),
);
context.computed.insert(MetricId::custom("total_leverage"), 4.2);

// Evaluate
let reports = engine.evaluate(&mut context, test_date)?;
let leverage_report = reports.get("Total Leverage <= 5.00x").unwrap();
assert!(leverage_report.passed);
assert_eq!(leverage_report.actual_value, Some(4.2));
assert_eq!(leverage_report.threshold, Some(5.0));
```

### 2. Covenant with Multiple Consequences

```rust
use finstack_valuations::covenants::{
    Covenant, CovenantConsequence, CovenantType,
};

let covenant = Covenant::new(
    CovenantType::MinInterestCoverage { threshold: 1.5 },
    Tenor::quarterly(),
)
.with_cure_period(Some(30))
.with_consequence(CovenantConsequence::RateIncrease { bp_increase: 150.0 })
.with_consequence(CovenantConsequence::CashSweep { sweep_percentage: 0.5 })
.with_consequence(CovenantConsequence::BlockDistributions);

// Add to engine, evaluate, then apply consequences if breached
```

### 3. Applying Consequences

```rust
use finstack_valuations::covenants::{
    CovenantBreach, CovenantEngine, InstrumentMutator,
};

// Track breach
let breach = CovenantBreach {
    covenant_type: "Interest Coverage >= 1.50x".to_string(),
    breach_date: breach_date,
    actual_value: Some(1.1),
    threshold: Some(1.5),
    cure_deadline: Some(breach_date + Duration::days(30)),
    is_cured: false,
    applied_consequences: vec![],
};

// Apply consequences (after cure deadline)
let applications = engine.apply_consequences(
    &mut instrument,
    &[breach],
    as_of,
)?;

for app in applications {
    println!("{}: {}", app.consequence_type, app.details);
}
```

### 4. Custom Covenant Evaluator

```rust
use finstack_valuations::covenants::{Covenant, CovenantSpec, CovenantType};

let reporting_covenant = Covenant::new(
    CovenantType::Affirmative {
        requirement: "Provide quarterly reporting".to_string(),
    },
    Tenor::quarterly(),
);

// Custom evaluator checks if reporting is compliant
let spec = CovenantSpec::with_evaluator(reporting_covenant, |ctx| {
    // Access instrument attributes, check reporting status
    let reporting_complete = ctx.instrument
        .attributes()
        .get_bool("quarterly_reporting_complete")?;
    Ok(reporting_complete)
});

engine.add_spec(spec);
```

### 5. Custom Metric Registration

```rust
// Register custom metric calculator
engine.register_metric("liquidity_ratio", |ctx| {
    let cash = ctx.computed.get(&MetricId::custom("cash"))
        .ok_or(InputError::NotFound { id: "cash".to_string() })?;
    let current_liabilities = ctx.computed.get(&MetricId::custom("current_liabilities"))
        .ok_or(InputError::NotFound { id: "current_liabilities".to_string() })?;
    Ok(cash / current_liabilities)
});

// Use in covenant
let liquidity_covenant = Covenant::new(
    CovenantType::Custom {
        metric: "liquidity_ratio".to_string(),
        test: ThresholdTest::Minimum(1.1),
    },
    Tenor::quarterly(),
);

engine.add_spec(CovenantSpec::with_metric(
    liquidity_covenant,
    MetricId::custom("liquidity_ratio"),
));
```

### 6. Covenant Windows

```rust
use finstack_valuations::covenants::CovenantWindow;

// Define window-specific covenants
let window = CovenantWindow {
    start: Date::from_ymd(2025, 1, 1),
    end: Date::from_ymd(2025, 6, 30),
    covenants: vec![
        CovenantSpec::with_metric(
            Covenant::new(
                CovenantType::MaxDebtToEBITDA { threshold: 3.5 },
                Tenor::quarterly(),
            ),
            MetricId::custom("debt_to_ebitda"),
        ),
    ],
    is_grace_period: false,
};

engine.add_window(window);

// During window period, only window covenants are tested
// Outside window, base specs apply
```

### 7. Forward Projection (Deterministic)

```rust
use finstack_valuations::covenants::{
    CovenantForecastConfig, forecast_covenant_generic, ModelTimeSeries,
};
use finstack_core::dates::PeriodId;

// Implement adapter for your time-series model
struct MyModelAdapter<'a> {
    model: &'a MyModel,
}

impl ModelTimeSeries for MyModelAdapter<'_> {
    fn get_scalar(&self, node_id: &str, period: &PeriodId) -> Option<f64> {
        self.model.get_value(node_id, period)
    }

    fn period_end_date(&self, period: &PeriodId) -> Date {
        self.model.period_end(period)
    }
}

// Forecast covenant
let spec = CovenantSpec::with_metric(
    Covenant::new(
        CovenantType::MaxDebtToEBITDA { threshold: 5.0 },
        Tenor::quarterly(),
    ),
    MetricId::custom("debt_to_ebitda"),
);

let periods = vec![
    PeriodId::quarter(2025, 1),
    PeriodId::quarter(2025, 2),
    PeriodId::quarter(2025, 3),
    PeriodId::quarter(2025, 4),
];

let adapter = MyModelAdapter { model: &my_model };
let config = CovenantForecastConfig::default(); // deterministic

let forecast = forecast_covenant_generic(&spec, &adapter, &periods, config)?;

println!("Covenant: {}", forecast.covenant_id);
println!("First breach: {:?}", forecast.first_breach_date);
println!("Min headroom: {:.2}% on {}",
    forecast.min_headroom_value * 100.0,
    forecast.min_headroom_date,
);

for i in 0..forecast.test_dates.len() {
    println!("{}: value={:.2}, threshold={:.2}, headroom={:.1}%",
        forecast.test_dates[i],
        forecast.projected_values[i],
        forecast.thresholds[i],
        forecast.headroom[i] * 100.0,
    );
}
```

### 8. Forward Projection (Stochastic MC)

```rust
use finstack_valuations::covenants::{CovenantForecastConfig, McConfig};

let config = CovenantForecastConfig {
    stochastic: true,
    num_paths: 10_000,
    volatility: Some(0.20), // 20% volatility
    random_seed: Some(42),
    mc: Some(McConfig {
        antithetic: true, // variance reduction
    }),
};

let forecast = forecast_covenant_generic(&spec, &adapter, &periods, config)?;

// Breach probabilities now reflect stochastic variation
for i in 0..forecast.test_dates.len() {
    println!("{}: breach probability = {:.1}%",
        forecast.test_dates[i],
        forecast.breach_probability[i] * 100.0,
    );
}

// Find warning periods (headroom < 10%)
let warning_indices = forecast.warning_indices(0.10);
for idx in warning_indices {
    println!("WARNING: {} - headroom only {:.1}%",
        forecast.test_dates[idx],
        forecast.headroom[idx] * 100.0,
    );
}

// Human-readable summary
println!("{}", forecast.explain());
```

### 9. Threshold Schedules

```rust
use finstack_valuations::covenants::{ThresholdSchedule, threshold_for_date};

// Define step-down schedule
let schedule = ThresholdSchedule(vec![
    (Date::from_ymd(2025, 1, 1), 6.0),
    (Date::from_ymd(2025, 7, 1), 5.5),
    (Date::from_ymd(2026, 1, 1), 5.0),
]);

// Lookup threshold for test date
let test_date = Date::from_ymd(2025, 9, 15);
let threshold = threshold_for_date(&schedule, test_date); // Some(5.5)
```

---

## Extending the Module

### Adding a New Covenant Type

**1. Extend the `CovenantType` enum** (`engine.rs`):

```rust
#[derive(Clone, Debug)]
pub enum CovenantType {
    // ... existing variants ...

    /// New covenant: Tangible net worth must exceed threshold
    MinTangibleNetWorth { threshold: f64 },
}
```

**2. Add description logic** (in `Covenant::description()` and `CovenantEngine::get_covenant_description()`):

```rust
CovenantType::MinTangibleNetWorth { threshold } => {
    format!("Tangible Net Worth ≥ ${:.0}", threshold)
}
```

**3. Add evaluation logic** (in `CovenantEngine::evaluate_spec()`):

```rust
CovenantType::MinTangibleNetWorth { threshold } => {
    let tnw = self.get_metric_value(context, &MetricId::custom("tangible_net_worth"))?;
    (tnw, *threshold)
}
```

```rust
// Test direction
CovenantType::MinTangibleNetWorth { .. } => metric_value >= threshold,
```

**4. Add forward projection support** (in `forward.rs`):

```rust
// comparator_for:
CovenantType::MinTangibleNetWorth { .. } => Comparator::GreaterOrEqual,

// base_threshold_from_spec:
CovenantType::MinTangibleNetWorth { threshold } => Some(*threshold),

// metric_value_for_spec:
CovenantType::MinTangibleNetWorth { .. } => model.get_scalar("tangible_net_worth", period),
```

**5. Test**:

```rust
#[test]
fn evaluate_tangible_net_worth_covenant() {
    let covenant = Covenant::new(
        CovenantType::MinTangibleNetWorth { threshold: 50_000_000.0 },
        Tenor::quarterly(),
    );

    let mut engine = CovenantEngine::new();
    engine.add_spec(CovenantSpec::with_metric(
        covenant,
        MetricId::custom("tangible_net_worth"),
    ));

    let mut ctx = metric_context(&instrument, test_date);
    ctx.computed.insert(MetricId::custom("tangible_net_worth"), 55_000_000.0);

    let reports = engine.evaluate(&mut ctx, test_date).unwrap();
    let tnw_report = reports.get("Tangible Net Worth ≥ $50000000").unwrap();
    assert!(tnw_report.passed);
}
```

---

### Adding a New Consequence Type

**1. Extend `CovenantConsequence` enum** (`engine.rs`):

```rust
#[derive(Clone, Debug)]
pub enum CovenantConsequence {
    // ... existing variants ...

    /// Require quarterly appraisals
    RequireAppraisals { frequency: Tenor },
}
```

**2. Extend `InstrumentMutator` trait** (if mutation needed):

```rust
pub trait InstrumentMutator {
    // ... existing methods ...

    fn set_appraisal_requirement(&mut self, frequency: Tenor) -> Result<()>;
}
```

**3. Add application logic** (in `CovenantEngine::apply_single_consequence()`):

```rust
CovenantConsequence::RequireAppraisals { frequency } => {
    instrument.set_appraisal_requirement(*frequency)?;
    Ok(ConsequenceApplication {
        consequence_type: "Require Appraisals".to_string(),
        applied_date: as_of,
        details: format!("Appraisals required {:?}", frequency),
    })
}
```

**4. Implement trait method** (in your instrument):

```rust
impl InstrumentMutator for MyInstrument {
    // ... existing methods ...

    fn set_appraisal_requirement(&mut self, frequency: Frequency) -> Result<()> {
        self.appraisal_frequency = Some(frequency);
        Ok(())
    }
}
```

---

### Adding Custom Metrics

Register calculators dynamically without modifying core types:

```rust
engine.register_metric("ebitda_margin", |ctx| {
    let ebitda = ctx.computed.get(&MetricId::custom("ebitda"))
        .ok_or(InputError::NotFound { id: "ebitda".to_string() })?;
    let revenue = ctx.computed.get(&MetricId::custom("revenue"))
        .ok_or(InputError::NotFound { id: "revenue".to_string() })?;
    Ok(ebitda / revenue)
});

let margin_covenant = Covenant::new(
    CovenantType::Custom {
        metric: "ebitda_margin".to_string(),
        test: ThresholdTest::Minimum(0.15), // 15%
    },
    Tenor::quarterly(),
);
```

---

### Integrating with Time-Series Models

Implement the `ModelTimeSeries` trait for your model:

```rust
use finstack_core::dates::{Date, PeriodId};
use finstack_valuations::covenants::ModelTimeSeries;

struct MyFinancialModel {
    // ... your model state ...
}

impl ModelTimeSeries for MyFinancialModel {
    fn get_scalar(&self, node_id: &str, period: &PeriodId) -> Option<f64> {
        // Lookup node value for period
        self.nodes.get(node_id)?.get_value(period)
    }

    fn period_end_date(&self, period: &PeriodId) -> Date {
        // Map period to end date
        self.calendar.period_end(period)
    }
}
```

Then use with `forecast_covenant_generic` as shown in examples.

---

### Adding Threshold Schedules to Covenants

Currently, `CovenantType` has static thresholds. To support time-varying thresholds:

**Option 1: Use Custom evaluator**

```rust
let schedule = Arc::new(ThresholdSchedule(vec![
    (Date::from_ymd(2025, 1, 1), 6.0),
    (Date::from_ymd(2026, 1, 1), 5.0),
]));

let spec = CovenantSpec::with_evaluator(leverage_covenant, move |ctx| {
    let as_of = ctx.as_of;
    let threshold = threshold_for_date(&schedule, as_of)
        .ok_or(InputError::NotFound { id: "threshold".to_string() })?;
    let leverage = ctx.computed.get(&MetricId::custom("total_leverage"))
        .ok_or(InputError::NotFound { id: "leverage".to_string() })?;
    Ok(*leverage <= threshold)
});
```

**Option 2: Extend `CovenantType`** (future enhancement):

```rust
pub enum CovenantType {
    // ... existing ...
    MaxTotalLeverageSchedule { schedule: ThresholdSchedule },
}
```

---

## Testing

All covenant types, evaluators, consequences, and forward projections should be covered by unit tests. Follow the patterns in `engine.rs` and `forward.rs` test modules:

```rust
#[test]
fn my_new_covenant_test() {
    let mut engine = CovenantEngine::new();

    // Setup covenant
    let covenant = Covenant::new(/* ... */);
    engine.add_spec(CovenantSpec::with_metric(covenant, metric_id));

    // Setup context
    let mut ctx = metric_context(&instrument, test_date);
    ctx.computed.insert(metric_id, value);

    // Evaluate
    let reports = engine.evaluate(&mut ctx, test_date).unwrap();

    // Assert
    assert!(reports.get("description").unwrap().passed);
}
```

For forward projection tests:

- Use `MockTs` adapter (see `forward.rs` tests)
- Test deterministic headroom calculation
- Test MC breach probabilities (with `#[cfg(feature = "mc")]`)

---

## Integration Points

### With Metrics System

- `MetricContext` provides computed metric values
- `MetricId` identifies metrics (standard or custom)
- Custom metric calculators registered on engine

### With Instruments

- Implements `InstrumentMutator` trait for consequence application
- Instrument attributes can be queried in custom evaluators

### With Time-Series Models

- Implement `ModelTimeSeries` trait adapter
- No direct dependency on statements crate (generic design)
- Statements bridge lives in meta crate

### With Market Data

- Access via `MetricContext::curves` in custom evaluators
- Market scenarios can be reflected in projected metrics

---

## Design Patterns

### Trait-Based Mutation

`InstrumentMutator` decouples covenant logic from instrument implementations. Instruments opt-in to support consequences.

### Arc-Wrapped Closures

Custom evaluators and metric calculators use `Arc<dyn Fn + Send + Sync>` for thread-safe sharing without Clone bounds on closures.

### Adapter Pattern

`ModelTimeSeries` trait enables covenant forecasting without tight coupling to statement/model implementations.

### Builder Pattern

`Covenant::new().with_cure_period().with_consequence()` provides fluent configuration.

---

## Performance Considerations

- **Linear scan for threshold lookup**: Assumes small schedules; deterministic ordering
- **MC simulation**: Scales with `num_paths`; use antithetic variates for variance reduction
- **Metric caching**: `MetricContext::computed` avoids redundant calculations
- **Window evaluation**: Short-circuits to window covenants when applicable

---

## Future Enhancements

- [ ] Covenant groups with AND/OR logic
- [ ] Multi-level consequence escalation (warning → default → acceleration)
- [ ] Covenant waivers and amendments tracking
- [ ] Polars DataFrame exports for forecasts (in meta crate)
- [ ] Covenant package templates (e.g., "Leveraged Loan Standard")
- [ ] Integration with reporting/disclosure systems

---

## References

- **Financial covenants**: Common credit agreement structures (bank loans, bonds, structured finance)
- **Cure periods**: Standard commercial loan documentation
- **MC simulation**: Lognormal shocks for financial metric volatility
- **Headroom analytics**: Credit risk early warning systems

For further details, see individual module documentation and test suites.
