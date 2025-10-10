<!-- 9ad33b96-df82-4f53-8fe2-45128addb8c4 a8135e4f-6bfc-428e-9c3d-185cbd2d61e2 -->
# Python Bindings for Statements Crate (100% Parity)

## Overview

Add comprehensive Python bindings for the `finstack-statements` crate, mirroring all functionality from the Rust API including types, builders, evaluators, extensions, and registry systems.

## Module Structure

The bindings will be organized under `finstack.statements` with the following structure:

```
finstack-py/src/statements/
├── mod.rs                    # Module registration & exports
├── types/
│   ├── mod.rs               # Node types & specs
│   ├── node.rs              # NodeSpec, NodeType
│   ├── forecast.rs          # ForecastSpec, ForecastMethod, SeasonalMode  
│   ├── value.rs             # AmountOrScalar
│   └── model.rs             # FinancialModelSpec, CapitalStructureSpec
├── builder/
│   └── mod.rs               # ModelBuilder (type-state pattern)
├── evaluator/
│   ├── mod.rs               # Evaluator, Results, ResultsMeta
│   └── export.rs            # Polars export (optional feature)
├── extensions/
│   ├── mod.rs               # Extension protocol & registry
│   └── builtins.rs          # CorkscrewExtension, CreditScorecardExtension
└── registry/
    ├── mod.rs               # Registry class
    └── schema.rs            # MetricDefinition, MetricRegistry, UnitType
```

## Implementation Details

### 1. Core Types (`finstack-py/src/statements/types/`)

**File: `types/node.rs`**

- `PyNodeSpec` wrapping `NodeSpec`:
  - Constructor: `new(node_id, node_type)`
  - Builder methods: `with_name`, `with_values`, `with_formula`, `with_forecast`, `with_tags`
  - Properties: `node_id`, `name`, `node_type`, `values`, `forecast`, `formula_text`, `where_text`, `tags`, `meta`
  - JSON serialization: `to_json()`, `from_json()`
- `PyNodeType` enum:
  - Variants: `Value`, `Calculated`, `Mixed`
  - Class attributes for ergonomic access

**File: `types/forecast.rs`**

- `PyForecastSpec` wrapping `ForecastSpec`:
  - Constructor: `new(method, params)`
  - Class methods: `forward_fill()`, `growth(rate)`, `normal(mean, std, seed)`
  - JSON serialization
- `PyForecastMethod` enum:
  - Variants: `ForwardFill`, `GrowthPct`, `CurvePct`, `Override`, `Normal`, `LogNormal`, `TimeSeries`, `Seasonal`
- `PySeasonalMode` enum:
  - Variants: `Additive`, `Multiplicative`

**File: `types/value.rs`**

- `PyAmountOrScalar` wrapping `AmountOrScalar`:
  - Class methods: `scalar(value)`, `amount(value, currency)`
  - Property: `is_scalar()`, `value()`, `currency()`
  - Arithmetic operators where applicable

**File: `types/model.rs`**

- `PyFinancialModelSpec` wrapping `FinancialModelSpec`:
  - Constructor: `new(id, periods)`
  - Methods: `add_node`, `get_node`, `has_node`, `get_node_mut`
  - Properties: `id`, `periods`, `nodes`, `capital_structure`, `meta`, `schema_version`
  - JSON serialization
- `PyCapitalStructureSpec` and `PyDebtInstrumentSpec` for capital structure integration

### 2. Builder API (`finstack-py/src/statements/builder/`)

**File: `builder/mod.rs`**

- `PyModelBuilder` wrapping `ModelBuilder<Ready>`:
  - Hide Rust type-state pattern (Python uses single class)
  - Constructor: `new(id)` → starts in NeedPeriods state internally
  - Methods:
    - `periods(range, actuals_until=None)` → transitions to Ready
    - `periods_explicit(periods)` → alternative period definition
    - `value(node_id, values)` → add value node (accepts list of tuples or dict)
    - `compute(node_id, formula)` → add calculated node
    - `forecast(node_id, forecast_spec)` → add forecast to node
    - `mixed(node_id, values=None, forecast=None, formula=None)` → add mixed node
    - `with_where(node_id, where_clause)` → add conditional evaluation
    - `with_meta(key, value)` → add model metadata
    - `capital_structure(spec)` → add capital structure
    - `build()` → returns `PyFinancialModelSpec`
  - All builder methods return `self` for chaining (Pythonic)
  - Error handling with PyErr conversion

### 3. Evaluator (`finstack-py/src/statements/evaluator/`)

**File: `evaluator/mod.rs`**

- `PyEvaluator` wrapping `Evaluator`:
  - Constructor: `new()`
  - Methods:
    - `evaluate(model)` → returns `PyResults`
    - `evaluate_with_market_context(model, market_ctx, as_of)` → with pricing
  - Internal caching (compiled expressions, forecasts)
- `PyResults` wrapping `Results`:
  - Properties: `nodes`, `meta`
  - Methods:
    - `get(node_id, period_id)` → optional value
    - `get_node(node_id)` → dict of period → value
    - `get_or(node_id, period_id, default)`
    - `all_periods(node_id)` → iterator
    - JSON serialization
- `PyResultsMeta` wrapping `ResultsMeta`:
  - Properties: `eval_time_ms`, `num_nodes`, `num_periods`

**File: `evaluator/export.rs`** (conditional on `polars_export` feature)

- Polars DataFrame export methods for `PyResults`:
  - `to_polars_long()` → long format (node_id, period_id, value)
  - `to_polars_long_filtered(node_filter)` → filtered long format
  - `to_polars_wide()` → wide format (period_id, node1, node2, ...)

### 4. Extensions (`finstack-py/src/statements/extensions/`)

**File: `extensions/mod.rs`**

- Python protocol for `Extension` trait (using ABC):
  - Methods: `metadata()`, `execute(context)`, `is_enabled()`, `config_schema()`, `validate_config()`
- `PyExtensionMetadata` wrapping `ExtensionMetadata`
- `PyExtensionContext` wrapping `ExtensionContext`
- `PyExtensionResult` wrapping `ExtensionResult`
- `PyExtensionRegistry` wrapping `ExtensionRegistry`:
  - Methods: `register(extension)`, `execute_all(model, results)`, `list_extensions()`

**File: `extensions/builtins.rs`**

- `PyCorkscrewExtension` wrapping `CorkscrewExtension`
- `PyCreditScorecardExtension` wrapping `CreditScorecardExtension`

### 5. Registry (`finstack-py/src/statements/registry/`)

**File: `registry/mod.rs`**

- `PyRegistry` wrapping `Registry`:
  - Constructor: `new()`
  - Methods:
    - `load_builtins()` → load fin.* metrics
    - `load_from_file(path)` → load JSON registry
    - `load_from_json(json_str)` → load from JSON string
    - `add_metric(definition)` → add single metric
    - `get(metric_id)` → get metric definition
    - `list_metrics(namespace=None)` → list available metrics
    - `has_metric(metric_id)` → check existence

**File: `registry/schema.rs`**

- `PyMetricDefinition` wrapping `MetricDefinition`:
  - Properties: `id`, `name`, `formula`, `description`, `category`, `unit_type`, `requires`, `tags`, `meta`
- `PyMetricRegistry` wrapping `MetricRegistry`:
  - Properties: `namespace`, `schema_version`, `metrics`, `meta`
- `PyUnitType` enum:
  - Variants: `Percentage`, `Currency`, `Ratio`, `Count`, `TimePeriod`

### 6. Module Registration (`finstack-py/src/lib.rs` & `finstack-py/src/statements/mod.rs`)

**Update `finstack-py/src/lib.rs`:**

```rust
mod statements;

// In finstack() function:
statements::register(py, &m)?;
```

**Create `finstack-py/src/statements/mod.rs`:**

```rust
pub(crate) mod types;
pub(crate) mod builder;
pub(crate) mod evaluator;
pub(crate) mod extensions;
pub(crate) mod registry;

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "statements")?;
    module.setattr("__doc__", "Financial statement modeling engine bindings.");
    
    // Register all submodules
    types::register(py, &module)?;
    builder::register(py, &module)?;
    evaluator::register(py, &module)?;
    extensions::register(py, &module)?;
    registry::register(py, &module)?;
    
    parent.add_submodule(&module)?;
    parent.setattr("statements", &module)?;
    Ok(())
}
```

### 7. Cargo.toml Updates

**Update `finstack-py/Cargo.toml` dependencies:**

```toml
[dependencies]
finstack-statements = { path = "../finstack/statements", features = ["polars_export"] }
```

### 8. Type Stubs & Documentation

- Generate `.pyi` stubs using `pyo3-stubgen`: `uv run pyo3-stubgen finstack`
- Place stubs in `finstack-py/finstack/statements/`
- Add comprehensive docstrings to all classes and methods matching Rust documentation
- Include Python examples in docstrings
- Add `finstack-py/examples/scripts/statements_example.py` demonstrating full API

### 9. Tests

- Add `finstack-py/tests/test_statements.py`:
  - Type creation and serialization
  - Builder pattern with chaining
  - Model evaluation (basic and with market context)
  - Forecast methods (all variants)
  - Extensions (corkscrew, scorecard)
  - Registry operations
  - Edge cases and error handling
- Verify parity with Rust examples (`examples/rust/statements_phase*.rs`)

### 10. Python-Specific Ergonomics

- Accept both lists and dicts for period values in builder methods
- Support context managers where appropriate
- Use `@property` for read-only attributes
- Implement `__repr__` and `__str__` for all types
- Support iteration protocols (`__iter__`, `__len__`) where applicable
- Type hints in stubs for IDE support
- Optional parameters with sensible defaults

## Testing Strategy

1. **Unit tests**: Test each binding independently
2. **Integration tests**: Full model build → evaluate → export workflows
3. **Parity tests**: Compare Python outputs with Rust examples
4. **Serialization tests**: JSON roundtrip for all types
5. **Error handling tests**: Proper Python exception conversion

## Documentation

1. Module-level docstrings explaining purpose and usage
2. Class docstrings with examples
3. Method docstrings with parameter types and return values
4. Python example scripts mirroring Rust examples
5. Add statements section to `finstack-py/README.md`

## Success Criteria

✅ All Rust types exposed to Python

✅ Builder API fully functional with method chaining

✅ Evaluator works with and without market context

✅ Extensions system accessible from Python

✅ Registry system for dynamic metrics

✅ Polars export when feature enabled

✅ Type stubs generated and validated

✅ Comprehensive test coverage

✅ Documentation complete

✅ Example scripts working

✅ Zero regression in existing bindings

### To-dos

- [ ] Create module structure: finstack-py/src/statements/ with submodules for types, builder, evaluator, extensions, registry
- [ ] Implement core type bindings: PyNodeSpec, PyNodeType, PyForecastSpec, PyForecastMethod, PyAmountOrScalar, PyFinancialModelSpec
- [ ] Implement PyModelBuilder with all builder methods (value, compute, forecast, mixed, etc.) supporting method chaining
- [ ] Implement PyEvaluator, PyResults, PyResultsMeta with evaluation methods and optional Polars export
- [ ] Implement extension system: PyExtensionRegistry, PyCorkscrewExtension, PyCreditScorecardExtension
- [ ] Implement PyRegistry, PyMetricDefinition, PyMetricRegistry for dynamic metric management
- [ ] Update finstack-py/src/lib.rs and Cargo.toml to register statements module and add dependency
- [ ] Generate and validate .pyi type stubs using pyo3-stubgen for IDE support
- [ ] Write comprehensive test suite in test_statements.py covering all functionality with parity checks
- [ ] Create Python example scripts mirroring Rust examples (statements_phase*.py)
- [ ] Update documentation: docstrings, README.md, and ensure all public APIs are documented