# Testing Strategy

**Last updated:** 2025-09-30

---

## Overview

Comprehensive testing strategy for the `finstack-statements` crate covering unit tests, integration tests, property-based tests, and golden tests.

**Coverage Goals:**
- 90%+ code coverage
- 100% public API coverage
- All examples are tested
- Property-based tests for critical invariants

---

## 1. Test Pyramid

```
           ┌────────────┐
           │  Golden    │  ~10 tests
           │  Tests     │  (End-to-end, serialization stability)
           └────────────┘
         ┌──────────────────┐
         │   Integration    │  ~30 tests
         │   Tests          │  (Multi-component, realistic scenarios)
         └──────────────────┘
      ┌───────────────────────────┐
      │   Unit Tests              │  ~150 tests
      │   (Per-component)         │  (Focused, fast, isolated)
      └───────────────────────────┘
   ┌────────────────────────────────────┐
   │   Property-Based Tests             │  ~20 tests
   │   (Invariants, fuzz testing)       │  (Randomized inputs, QuickCheck-style)
   └────────────────────────────────────┘
```

---

## 2. Unit Tests

### 2.1 Builder Tests (`tests/builder_tests.rs`)

**Coverage:**
- Type-state enforcement (compile-time checks)
- Value node storage
- Calculated node creation
- Mixed node configuration
- Period parsing and validation
- Error handling

**Examples:**

```rust
#[test]
fn value_node_stores_correctly() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None).unwrap()
        .value("revenue", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(100.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::Scalar(110.0)),
        ])
        .build().unwrap();
    
    assert_eq!(model.nodes.len(), 1);
    assert_eq!(model.nodes["revenue"].spec.node_type, NodeType::Value);
}

#[test]
fn builder_validates_period_order() {
    let result = ModelBuilder::new("test")
        .periods("2025Q4..Q1", None); // Invalid: Q4 > Q1
    
    assert!(result.is_err());
}
```

---

### 2.2 DSL Tests (`tests/dsl_tests.rs`)

**Coverage:**
- Parser correctness (arithmetic, functions, references)
- Compiler output (AST → core Expr)
- Operator precedence
- Error messages

**Examples:**

```rust
#[test]
fn parser_handles_arithmetic() {
    let formula = "revenue - cogs";
    let ast = parse_formula(formula).unwrap();
    
    match ast {
        StmtExpr::BinOp { op: BinOp::Sub, .. } => {},
        _ => panic!("Expected subtraction"),
    }
}

#[test]
fn parser_handles_nested_functions() {
    let formula = "rolling_mean(lag(revenue, 1), 4)";
    let ast = parse_formula(formula).unwrap();
    
    // Verify nested structure
    match ast {
        StmtExpr::Call { func, args } => {
            assert_eq!(func, "rolling_mean");
            assert_eq!(args.len(), 2);
        },
        _ => panic!("Expected function call"),
    }
}

#[test]
fn parser_errors_on_unknown_function() {
    let formula = "unknown_func(revenue)";
    let result = parse_formula(formula);
    
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("unknown_func"));
}
```

---

### 2.3 Evaluator Tests (`tests/evaluator_tests.rs`)

**Coverage:**
- Precedence resolution (Value > Forecast > Formula)
- DAG construction and topological sort
- Circular dependency detection
- Per-period evaluation
- Where clause masking

**Examples:**

```rust
#[test]
fn precedence_value_over_forecast() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None).unwrap()
        .value("revenue", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(100.0)),
        ])
        .forecast("revenue", ForecastSpec {
            method: ForecastMethod::GrowthPct,
            params: indexmap! { "rate".into() => json!(0.1) },
        })
        .build().unwrap();
    
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();
    
    // Q1 should use explicit value (100), not forecast
    assert_eq!(
        results.nodes["revenue"][&PeriodId::quarter(2025, 1)],
        100.0
    );
    
    // Q2 should use forecast: 100 * 1.1 = 110
    assert_eq!(
        results.nodes["revenue"][&PeriodId::quarter(2025, 2)],
        110.0
    );
}

#[test]
fn detects_circular_dependency() {
    let result = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None).unwrap()
        .compute("a", "b + 1").unwrap()
        .compute("b", "c + 1").unwrap()
        .compute("c", "a + 1").unwrap()
        .build();
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, Error::CircularDependency { .. }));
}
```

---

### 2.4 Forecast Tests (`tests/forecast_tests.rs`)

**Coverage:**
- Forward fill correctness
- Growth percentage calculations
- Statistical distributions (determinism, parameters)
- Overrides

**Examples:**

```rust
#[test]
fn normal_distribution_deterministic_with_seed() {
    let forecast = NormalForecast {
        mean: 100_000.0,
        std_dev: 15_000.0,
        seed: Some(42),
    };
    
    // Sample twice with same seed
    let sample1 = forecast.sample();
    let sample2 = forecast.sample();
    
    // Should be deterministic
    assert_eq!(sample1, sample2);
}

#[test]
fn growth_forecast_compounds_correctly() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q4", None).unwrap()
        .value("revenue", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(100.0)),
        ])
        .forecast("revenue", ForecastSpec {
            method: ForecastMethod::GrowthPct,
            params: indexmap! { "rate".into() => json!(0.05) },
        })
        .build().unwrap();
    
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();
    
    assert!((results.nodes["revenue"][&PeriodId::quarter(2025, 2)] - 105.0).abs() < 0.01);
    assert!((results.nodes["revenue"][&PeriodId::quarter(2025, 3)] - 110.25).abs() < 0.01);
    assert!((results.nodes["revenue"][&PeriodId::quarter(2025, 4)] - 115.76).abs() < 0.01);
}
```

---

### 2.5 Registry Tests (`tests/registry_tests.rs`)

**Coverage:**
- JSON loading
- Namespace management
- Collision detection
- Formula compilation from JSON

**Examples:**

```rust
#[test]
fn load_metrics_from_json() {
    let mut registry = Registry::new();
    
    let json = r#"{
        "namespace": "test",
        "schema_version": 1,
        "metrics": [
            {
                "id": "gross_margin",
                "name": "Gross Margin",
                "formula": "gross_profit / revenue",
                "description": "Margin percentage",
                "category": "margins",
                "unit_type": "percentage"
            }
        ]
    }"#;
    
    let metric_registry: MetricRegistry = serde_json::from_str(json).unwrap();
    registry.load_registry(metric_registry).unwrap();
    
    assert!(registry.get("test.gross_margin").is_some());
}

#[test]
fn namespace_prevents_collisions() {
    let mut registry = Registry::new();
    
    // Load "fin" namespace
    registry.load_builtins().unwrap();
    
    // Try to add conflicting metric
    let conflicting = MetricRegistry {
        namespace: "fin".into(),
        metrics: vec![
            MetricDefinition {
                id: "gross_profit".into(),  // Already exists
                ...
            }
        ],
    };
    
    let result = registry.load_registry(conflicting);
    assert!(result.is_err());
}
```

---

### 2.6 Capital Structure Tests (`tests/capital_structure_tests.rs`)

**Coverage:**
- Instrument construction
- Cashflow aggregation
- Interest expense calculation
- Principal schedule tracking

**Examples:**

```rust
#[test]
fn bond_generates_cashflows() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..2025Q4", None).unwrap()
        .add_bond(
            "BOND-001",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            Date::from_calendar_date(2025, Month::January, 15).unwrap(),
            Date::from_calendar_date(2030, Month::January, 15).unwrap(),
            "USD-OIS",
        ).unwrap()
        .build().unwrap();
    
    assert!(model.capital_structure.is_some());
    
    let cs = model.capital_structure.as_ref().unwrap();
    let flows = cs.aggregate_cashflows(&model.periods).unwrap();
    
    assert!(flows.contains_key("BOND-001"));
}
```

---

## 3. Integration Tests

### 3.1 Complete P&L Model (`tests/integration_tests.rs`)

```rust
#[test]
fn complete_pl_model_evaluates() {
    let model = ModelBuilder::new("Acme Corp")
        .periods("2024Q1..2024Q4", Some("2024Q1..Q2")).unwrap()
        
        // Revenue with forecast
        .value("revenue", &[...])
        .forecast("revenue", ...)
        
        // COGS and expenses
        .compute("cogs", "revenue * 0.6").unwrap()
        .value("operating_expenses", &[...])
        
        // Load standard metrics
        .with_builtin_metrics().unwrap()
        
        .build().unwrap();
    
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();
    
    // Verify all nodes computed
    assert!(results.nodes.contains_key("revenue"));
    assert!(results.nodes.contains_key("cogs"));
    assert!(results.nodes.contains_key("gross_profit"));
    
    // Verify forecast periods
    let q3_revenue = results.nodes["revenue"][&PeriodId::quarter(2024, 3)];
    assert!(q3_revenue > 0.0);
}
```

---

## 4. Property-Based Tests

### 4.1 Determinism Properties

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn evaluation_is_deterministic(seed in any::<u64>()) {
        let model = /* build model with seed */;
        
        let mut eval1 = Evaluator::new();
        let results1 = eval1.evaluate(&model, false).unwrap();
        
        let mut eval2 = Evaluator::new();
        let results2 = eval2.evaluate(&model, false).unwrap();
        
        // Results should be identical
        assert_eq!(results1.nodes, results2.nodes);
    }
}
```

### 4.2 Currency Safety

```rust
proptest! {
    #[test]
    fn currency_mismatch_errors(
        amount1 in (0.0..1_000_000.0),
        amount2 in (0.0..1_000_000.0),
    ) {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None).unwrap()
            .value("usd_value", &[(PeriodId::quarter(2025, 1), 
                AmountOrScalar::Amount(Money::new(amount1, Currency::USD)))])
            .value("eur_value", &[(PeriodId::quarter(2025, 1),
                AmountOrScalar::Amount(Money::new(amount2, Currency::EUR)))])
            .compute("sum", "usd_value + eur_value").unwrap()
            .build().unwrap();
        
        let mut evaluator = Evaluator::new();
        let result = evaluator.evaluate(&model, false);
        
        // Should error due to currency mismatch
        assert!(result.is_err());
    }
}
```

---

## 5. Golden Tests

### 5.1 Purpose

Golden tests verify:
- Serialization stability (wire format doesn't change)
- End-to-end evaluation correctness
- Regression detection

### 5.2 Test Structure

```rust
#[test]
fn golden_basic_model() {
    // Load golden JSON
    let json = include_str!("../tests/golden/basic_model.json");
    let spec: FinancialModelSpec = serde_json::from_str(json).unwrap();
    
    // Build model
    let model = FinancialModel::from_spec(spec).unwrap();
    
    // Evaluate
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();
    
    // Load expected results
    let expected_json = include_str!("../tests/golden/basic_model_results.json");
    let expected: IndexMap<String, IndexMap<PeriodId, f64>> = 
        serde_json::from_str(expected_json).unwrap();
    
    // Compare
    assert_eq!(results.nodes, expected);
}
```

### 5.3 Golden Files

```
tests/golden/
├── basic_model.json           # Simple P&L
├── basic_model_results.json   # Expected output
├── capital_structure_model.json
├── capital_structure_results.json
└── statistical_forecast_model.json
```

---

## 6. Testing Checklist

### Phase 1: Foundation
- [ ] Builder type-state enforcement
- [ ] Value node storage
- [ ] Period parsing
- [ ] Serialization roundtrip

### Phase 2: DSL
- [ ] Parser handles all operators
- [ ] Compiler generates correct Expr
- [ ] Error messages are clear
- [ ] Operator precedence correct

### Phase 3: Evaluator
- [ ] DAG construction
- [ ] Topological sort
- [ ] Circular dependency detection
- [ ] Precedence resolution
- [ ] Where clause masking

### Phase 4: Forecasting
- [ ] Forward fill
- [ ] Growth percentage
- [ ] Statistical (determinism)
- [ ] Overrides

### Phase 5: Registry
- [ ] JSON loading
- [ ] Namespace management
- [ ] Formula compilation
- [ ] Built-in metrics

### Phase 6: Capital Structure
- [ ] Instrument construction
- [ ] Cashflow aggregation
- [ ] Interest expense calculation
- [ ] Principal schedules

### Phase 7: Results
- [ ] Long format export
- [ ] Wide format export
- [ ] Metadata stamping

### Phase 8: Extensions
- [ ] Extension registration
- [ ] Extension execution
- [ ] Placeholder extensions

---

## 7. Performance Tests

### 7.1 Benchmark Suite

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_evaluation(c: &mut Criterion) {
    let model = /* build 100-node, 24-period model */;
    
    c.bench_function("evaluate_100x24", |b| {
        b.iter(|| {
            let mut evaluator = Evaluator::new();
            black_box(evaluator.evaluate(&model, false).unwrap());
        });
    });
}

criterion_group!(benches, benchmark_evaluation);
criterion_main!(benches);
```

### 7.2 Performance Targets

| Model Size | Target Time | Status |
|------------|-------------|--------|
| 100 nodes × 24 periods | < 10ms | ✅ |
| 1000 nodes × 60 periods | < 100ms | ✅ |
| 10k nodes × 120 periods | < 1s | ⚠️ |

---

## 8. CI/CD Integration

### 8.1 GitHub Actions

```yaml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Run tests
        run: cargo test --all-features
      
      - name: Run clippy
        run: cargo clippy --all-features -- -D warnings
      
      - name: Check code coverage
        run: cargo tarpaulin --out Xml --all-features
      
      - name: Upload coverage
        uses: codecov/codecov-action@v2
```

---

## 9. Test Organization

```
tests/
├── builder_tests.rs           # Builder API
├── dsl_tests.rs               # DSL parser/compiler
├── evaluator_tests.rs         # Evaluation logic
├── forecast_tests.rs          # Forecast methods
├── registry_tests.rs          # Dynamic registry
├── capital_structure_tests.rs # Capital structure integration
├── integration_tests.rs       # End-to-end scenarios
├── property_tests.rs          # Property-based tests
└── golden/
    ├── basic_model.json
    ├── basic_model_results.json
    └── ...
```

---

## References

- [Implementation Plan](./IMPLEMENTATION_PLAN.md) — Test requirements per phase
- [Examples](./examples/) — Tested example code
