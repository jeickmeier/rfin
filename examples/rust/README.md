# Rust Examples

This directory contains working examples for various Finstack crates.

## Running Examples

```bash
# From the project root
cargo run --example <example_name>
```

## Available Examples

### Statements Crate

**[statements_phase1_example.rs](./statements_phase1_example.rs)**
- Demonstrates Phase 1 features of the statements crate
- Type-state builder pattern for compile-time safety
- Creating models with periods and nodes
- Currency-aware and unitless values
- Model structure inspection
- Complete income statement example

```bash
cargo run --example statements_phase1_example
```

**[statements_phase2_example.rs](./statements_phase2_example.rs)**
- Demonstrates Phase 2 DSL engine features
- Formula parsing (arithmetic, comparison, logical operators)
- AST (Abstract Syntax Tree) inspection
- Compilation to core Expr
- Time-series functions (lag, lead, diff, pct_change)
- Rolling window functions (rolling_mean, rolling_sum, rolling_std)
- Statistical functions (std, var, median)
- Conditional expressions (if-then-else)
- Complex nested expressions
- Error handling

```bash
cargo run --example statements_phase2_example
```

**[statements_phase3_example.rs](./statements_phase3_example.rs)**
- Demonstrates Phase 3 evaluator features
- Model evaluation with value and calculated nodes
- Arithmetic and comparison operations
- DAG construction and topological sorting
- Precedence resolution (Value > Forecast > Formula)
- Circular dependency detection
- Multi-period sequential evaluation
- Complete P&L statement with derived metrics
- Results metadata and performance tracking

```bash
cargo run --example statements_phase3_example
```

**[statements_phase4_example.rs](./statements_phase4_example.rs)**
- Demonstrates Phase 4 forecasting features
- Forward Fill: Carry last value forward
- Growth Percentage: Compound growth rates
- Curve Percentage: Period-specific growth rates
- Override: Explicit period values with forward fill
- Normal Distribution: Statistical forecast with deterministic seeding
- LogNormal Distribution: Always-positive statistical forecast
- Forecast integration with formula dependencies
- Complete P&L with mixed forecast methods
- Negative growth scenarios

```bash
cargo run --example statements_phase4_example
```

**[statements_phase5_example.rs](./statements_phase5_example.rs)**
- Demonstrates Phase 5 dynamic registry features
- Loading built-in financial metrics (22 standard metrics)
- Exploring metrics by namespace (fin.*)
- Creating custom metrics from JSON
- Namespace management and collision prevention
- Registry integration with ModelBuilder
- Selective vs. bulk metric loading
- Evaluating models with registry metrics
- Complete P&L with margin and profitability metrics

```bash
cargo run --example statements_phase5_example
```

**[capital_structure_dsl_example.rs](./capital_structure_dsl_example.rs)**
- Demonstrates cs.* DSL namespace for capital structure references
- Parsing cs.interest_expense.total, cs.principal_payment.BOND-001, etc.
- Compiling cs.* references into internal representation
- Context setup with mock capital structure cashflows
- Formula evaluation with cs.* data

```bash
cargo run --example capital_structure_dsl_example
```

**[lbo_model_complete.rs](./lbo_model_complete.rs)**
- Complete end-to-end LBO (Leveraged Buyout) model example
- Operating performance metrics with forecasts
- Multiple debt instruments (senior and subordinated notes)
- Formulas with cs.* references for interest, principal, and balances
- Credit metrics (leverage, interest coverage, DSCR)
- Automatic capital structure cashflow computation
- Evaluation with market context
- Comprehensive P&L and margin analysis

```bash
cargo run --example lbo_model_complete
```

### Valuations Crate

**[bond_custom_cashflows_example.rs](./bond_custom_cashflows_example.rs)**
- Custom cashflow generation for bonds
- Detailed bond analytics

**[enhanced_builders_example.rs](./enhanced_builders_example.rs)**
- Advanced builder patterns for financial instruments

**[calibration_report_builder_example.rs](./calibration_report_builder_example.rs)**
- Calibration report generation

**[multi_curve_framework_example.rs](./multi_curve_framework_example.rs)**
- Multi-curve interest rate framework

**[multi_curve_calibration_example.rs](./multi_curve_calibration_example.rs)**
- Calibrating multiple yield curves

**[repo_example.rs](./repo_example.rs)**
- Repurchase agreement (repo) modeling

### Core Crate

**[serde_date_types_example.rs](./serde_date_types_example.rs)**
- Date serialization and deserialization

**[validation_example.rs](./validation_example.rs)**
- Input validation patterns

**[validation_framework_example.rs](./validation_framework_example.rs)**
- Comprehensive validation framework

**[market_context_v2_demo.rs](./market_context_v2_demo.rs)**
- Market context management

## Contributing

When adding new examples:
1. Place the file in `examples/rust/`
2. Add the example configuration to the appropriate crate's `Cargo.toml`:
   ```toml
   [[example]]
   name = "example_name"
   path = "../../examples/rust/example_name.rs"
   ```
3. Update this README with a brief description
4. Include comprehensive inline documentation in the example

