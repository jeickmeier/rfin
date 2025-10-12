//! Tests for statements WASM bindings.

use finstack_wasm::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_node_type_enum() {
    let value = NodeType::VALUE();
    let calculated = NodeType::CALCULATED();
    let mixed = NodeType::MIXED();

    // Test that we can create instances
    assert_eq!(value.to_string_js(), "Value");
    assert_eq!(calculated.to_string_js(), "Calculated");
    assert_eq!(mixed.to_string_js(), "Mixed");
}

#[wasm_bindgen_test]
fn test_forecast_method_enum() {
    let forward_fill = ForecastMethod::FORWARD_FILL();
    let growth = ForecastMethod::GROWTH_PCT();
    let curve = ForecastMethod::CURVE_PCT();
    let override_method = ForecastMethod::OVERRIDE();
    let normal = ForecastMethod::NORMAL();
    let log_normal = ForecastMethod::LOG_NORMAL();
    let time_series = ForecastMethod::TIME_SERIES();
    let seasonal = ForecastMethod::SEASONAL();

    // Test all 8 methods exist
    assert_eq!(forward_fill.to_string_js(), "ForwardFill");
    assert_eq!(growth.to_string_js(), "GrowthPct");
    assert_eq!(curve.to_string_js(), "CurvePct");
    assert_eq!(override_method.to_string_js(), "Override");
    assert_eq!(normal.to_string_js(), "Normal");
    assert_eq!(log_normal.to_string_js(), "LogNormal");
    assert_eq!(time_series.to_string_js(), "TimeSeries");
    assert_eq!(seasonal.to_string_js(), "Seasonal");
}

#[wasm_bindgen_test]
fn test_seasonal_mode_enum() {
    let additive = SeasonalMode::ADDITIVE();
    let multiplicative = SeasonalMode::MULTIPLICATIVE();

    assert_eq!(additive.to_string_js(), "Additive");
    assert_eq!(multiplicative.to_string_js(), "Multiplicative");
}

#[wasm_bindgen_test]
fn test_amount_or_scalar_creation() {
    // Test scalar creation
    let scalar = AmountOrScalar::scalar(100.0);
    assert!(scalar.is_scalar());
    assert!(!scalar.is_amount());
    assert_eq!(scalar.get_value(), 100.0);
    assert_eq!(scalar.to_string_js(), "100");

    // Test amount creation
    let currency = Currency::new("USD").unwrap();
    let amount = AmountOrScalar::amount(250.0, &currency);
    assert!(!amount.is_scalar());
    assert!(amount.is_amount());
    assert_eq!(amount.get_value(), 250.0);
    assert_eq!(amount.to_string_js(), "250 USD");
}

#[wasm_bindgen_test]
fn test_forecast_spec_constructors() {
    // Test forward fill
    let ff = ForecastSpec::forward_fill();
    assert_eq!(ff.method().to_string_js(), "ForwardFill");

    // Test growth
    let growth = ForecastSpec::growth(0.05);
    assert_eq!(growth.method().to_string_js(), "GrowthPct");

    // Test curve
    let curve = ForecastSpec::curve(vec![0.02, 0.03, 0.04]);
    assert_eq!(curve.method().to_string_js(), "CurvePct");

    // Test normal
    let normal = ForecastSpec::normal(100.0, 10.0, 12345);
    assert_eq!(normal.method().to_string_js(), "Normal");

    // Test lognormal
    let lognormal = ForecastSpec::lognormal(0.15, 0.05, 54321);
    assert_eq!(lognormal.method().to_string_js(), "LogNormal");
}

#[wasm_bindgen_test]
fn test_model_builder_basic() {
    let mut builder = ModelBuilder::new("Test Model".to_string());

    // Set periods
    builder = builder
        .periods("2025Q1..Q4", None)
        .expect("Failed to set periods");

    // Add a value node
    let values = js_sys::Object::new();
    js_sys::Reflect::set(
        &values,
        &wasm_bindgen::JsValue::from_str("2025Q1"),
        &wasm_bindgen::JsValue::from_f64(1000000.0),
    )
    .unwrap();

    builder = builder
        .value("revenue".to_string(), values.into())
        .expect("Failed to add value");

    // Add a computed node
    builder = builder
        .compute("cogs".to_string(), "revenue * 0.6".to_string())
        .expect("Failed to add compute");

    // Build the model
    let model = builder.build().expect("Failed to build model");

    assert_eq!(model.id(), "Test Model");
    assert_eq!(model.period_count(), 4);
    assert_eq!(model.node_count(), 2);
}

#[wasm_bindgen_test]
fn test_evaluator_basic() {
    // Build a simple model
    let mut builder = ModelBuilder::new("Simple P&L".to_string());
    builder = builder
        .periods("2025Q1..Q2", None)
        .expect("Failed to set periods");

    // Add revenue
    let values = js_sys::Object::new();
    js_sys::Reflect::set(
        &values,
        &wasm_bindgen::JsValue::from_str("2025Q1"),
        &wasm_bindgen::JsValue::from_f64(1000000.0),
    )
    .unwrap();
    js_sys::Reflect::set(
        &values,
        &wasm_bindgen::JsValue::from_str("2025Q2"),
        &wasm_bindgen::JsValue::from_f64(1100000.0),
    )
    .unwrap();

    builder = builder
        .value("revenue".to_string(), values.into())
        .expect("Failed to add revenue");

    // Add computed nodes
    builder = builder
        .compute("cogs".to_string(), "revenue * 0.6".to_string())
        .expect("Failed to add cogs");
    builder = builder
        .compute("gross_profit".to_string(), "revenue - cogs".to_string())
        .expect("Failed to add gross_profit");

    let model = builder.build().expect("Failed to build model");

    // Evaluate
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).expect("Failed to evaluate");

    // Check results
    let revenue_q1 = results
        .get("revenue", "2025Q1")
        .expect("Failed to get revenue Q1");
    assert_eq!(revenue_q1, Some(1000000.0));

    let gross_profit_q1 = results
        .get("gross_profit", "2025Q1")
        .expect("Failed to get gross_profit Q1");
    assert_eq!(gross_profit_q1, Some(400000.0)); // 1M - 600k

    // Check metadata
    let meta = results.meta();
    assert_eq!(meta.num_nodes(), 3);
    assert_eq!(meta.num_periods(), 2);
}

#[wasm_bindgen_test]
fn test_registry_builtins() {
    let mut registry = Registry::new();
    registry.load_builtins().expect("Failed to load builtins");

    // Check that we have metrics
    assert!(registry.metric_count() > 0);

    // Check specific metric
    assert!(registry.has_metric("fin.gross_margin"));

    let metric = registry
        .get("fin.gross_margin")
        .expect("Failed to get metric");
    let id = metric.id();
    assert!(id == "fin.gross_margin" || id == "gross_margin");
    let name = metric.name();
    assert!(name == "Gross Margin" || name == "Gross Margin %");

    // List metrics in fin namespace
    let fin_metrics = registry.list_metrics(Some("fin".to_string()));
    assert!(!fin_metrics.is_empty());
}

#[wasm_bindgen_test]
fn test_extension_creation() {
    // Test extension creation
    let _corkscrew = CorkscrewExtension::new();
    let _scorecard = CreditScorecardExtension::new();

    // Test registry creation
    let _registry = ExtensionRegistry::new();
}

#[wasm_bindgen_test]
fn test_extension_status() {
    let success = ExtensionStatus::SUCCESS();
    let failed = ExtensionStatus::FAILED();
    let not_impl = ExtensionStatus::NOT_IMPLEMENTED();
    let skipped = ExtensionStatus::SKIPPED();

    assert_eq!(success.to_string_js(), "Success");
    assert_eq!(failed.to_string_js(), "Failed");
    assert_eq!(not_impl.to_string_js(), "NotImplemented");
    assert_eq!(skipped.to_string_js(), "Skipped");
}

#[wasm_bindgen_test]
fn test_extension_result() {
    let success = ExtensionResult::success("All checks passed".to_string());
    assert_eq!(success.message(), "All checks passed");

    let failure = ExtensionResult::failure("Validation failed".to_string());
    assert_eq!(failure.message(), "Validation failed");

    let skipped = ExtensionResult::skipped("Not applicable".to_string());
    assert_eq!(skipped.message(), "Not applicable");
}

#[wasm_bindgen_test]
fn test_unit_type_enum() {
    let currency = UnitType::CURRENCY();
    let percentage = UnitType::PERCENTAGE();
    let ratio = UnitType::RATIO();
    let count = UnitType::COUNT();
    let time_period = UnitType::TIME_PERIOD();

    assert_eq!(currency.to_string_js(), "Currency");
    assert_eq!(percentage.to_string_js(), "Percentage");
    assert_eq!(ratio.to_string_js(), "Ratio");
    assert_eq!(count.to_string_js(), "Count");
    assert_eq!(time_period.to_string_js(), "TimePeriod");
}

#[wasm_bindgen_test]
fn test_model_with_forecast() {
    // Build model with forecast
    let mut builder = ModelBuilder::new("Forecast Test".to_string());
    builder = builder
        .periods("2025Q1..Q4", Some("2025Q1".to_string()))
        .expect("Failed to set periods");

    // Add initial value
    let values = js_sys::Object::new();
    js_sys::Reflect::set(
        &values,
        &wasm_bindgen::JsValue::from_str("2025Q1"),
        &wasm_bindgen::JsValue::from_f64(1000000.0),
    )
    .unwrap();

    builder = builder
        .value("revenue".to_string(), values.into())
        .expect("Failed to add value");

    // Add growth forecast
    let forecast = ForecastSpec::growth(0.05);
    builder = builder
        .forecast("revenue".to_string(), &forecast)
        .expect("Failed to add forecast");

    let model = builder.build().expect("Failed to build model");

    // Evaluate
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).expect("Failed to evaluate");

    // Check Q1 (actual)
    let q1 = results.get("revenue", "2025Q1").expect("Failed to get Q1");
    assert_eq!(q1, Some(1000000.0));

    // Check Q2 (forecasted with growth)
    let q2 = results.get("revenue", "2025Q2").expect("Failed to get Q2");
    assert!(q2.is_some());
    let q2_val = q2.unwrap();
    // Should be greater than Q1
    assert!(q2_val > 1000000.0);
}

#[wasm_bindgen_test]
fn test_results_methods() {
    // Build simple model
    let mut builder = ModelBuilder::new("Results Test".to_string());
    builder = builder
        .periods("2025Q1..Q2", None)
        .expect("Failed to set periods");

    let values = js_sys::Object::new();
    js_sys::Reflect::set(
        &values,
        &wasm_bindgen::JsValue::from_str("2025Q1"),
        &wasm_bindgen::JsValue::from_f64(100.0),
    )
    .unwrap();
    js_sys::Reflect::set(
        &values,
        &wasm_bindgen::JsValue::from_str("2025Q2"),
        &wasm_bindgen::JsValue::from_f64(200.0),
    )
    .unwrap();

    builder = builder
        .value("test_metric".to_string(), values.into())
        .expect("Failed to add value");
    let model = builder.build().expect("Failed to build model");

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).expect("Failed to evaluate");

    // Test get
    assert_eq!(results.get("test_metric", "2025Q1").unwrap(), Some(100.0));

    // Test getOr
    assert_eq!(results.get_or("test_metric", "2025Q1", 0.0).unwrap(), 100.0);
    assert_eq!(
        results.get_or("nonexistent", "2025Q1", 999.0).unwrap(),
        999.0
    );

    // Test getNode
    let node_values = results.get_node("test_metric").expect("Failed to get node");
    assert!(!node_values.is_null());

    // Test allPeriods
    let periods = results.all_periods("test_metric");
    assert!(js_sys::Array::is_array(&periods));
}

#[wasm_bindgen_test]
fn test_json_serialization() {
    // Build a model
    let mut builder = ModelBuilder::new("JSON Test".to_string());
    builder = builder
        .periods("2025Q1..Q2", None)
        .expect("Failed to set periods");

    let values = js_sys::Object::new();
    js_sys::Reflect::set(
        &values,
        &wasm_bindgen::JsValue::from_str("2025Q1"),
        &wasm_bindgen::JsValue::from_f64(1000.0),
    )
    .unwrap();

    builder = builder
        .value("metric".to_string(), values.into())
        .expect("Failed to add value");
    let model = builder.build().expect("Failed to build model");

    // Serialize to JSON
    let json = model.to_json().expect("Failed to serialize");
    assert!(!json.is_null());

    // Test forecast spec JSON
    let forecast = ForecastSpec::growth(0.05);
    let forecast_json = forecast.to_json().expect("Failed to serialize forecast");
    assert!(!forecast_json.is_null());
}
