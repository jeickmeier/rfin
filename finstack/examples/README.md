# Finstack Examples

This directory contains comprehensive examples demonstrating the capabilities of the Finstack financial computation library. Examples are organized by crate/functionality for easy navigation.

## Running Examples

From the project root, run examples using:

```bash
# Run a specific example
cargo run --example <example_name> --features <required_features>

# Run with all features enabled
cargo run --example <example_name> --features all
```

## Directory Structure

### Core Examples (`examples/core/`)

**Core functionality and foundational concepts**

- **[market_context_v2_demo.rs](./core/market_context_v2_demo.rs)**
  - MarketContextV2 benefits and enum-based storage
  - Features: `core`

- **[rate_conversions.rs](./core/rate_conversions.rs)**
  - Interest rate compounding convention conversions (simple, periodic, continuous)
  - ISDA-standard transformations for market data and derivatives pricing
  - Features: `core`

### Statements Examples (`examples/statements/`)

**Financial statement modeling and DSL engine**

- **[statements_phase1_example.rs](./statements/statements_phase1_example.rs)**
  - Phase 1: Type-state builder pattern, models with periods and nodes
  - Features: `statements`

- **[statements_phase2_example.rs](./statements/statements_phase2_example.rs)**
  - Phase 2: DSL engine, formula parsing, AST inspection, time-series functions
  - Features: `statements`

- **[statements_phase3_example.rs](./statements/statements_phase3_example.rs)**
  - Phase 3: Model evaluation, DAG construction, precedence resolution
  - Features: `statements`

- **[statements_phase5_example.rs](./statements/statements_phase5_example.rs)**
  - Phase 5: Dynamic registry, built-in financial metrics
  - Features: `statements`

- **[statements_phase6_example.rs](./statements/statements_phase6_example.rs)**
  - Phase 6: Advanced features and integrations
  - Features: `statements`

- **[statements_phase7_example.rs](./statements/statements_phase7_example.rs)**
  - Phase 7: Polars integration and DataFrame exports
  - Features: `statements`

- **[capital_structure_dsl_example.rs](./statements/capital_structure_dsl_example.rs)**
  - Capital structure DSL with `cs.*` namespace references
  - Features: `statements`

- **[lbo_model_complete.rs](./statements/lbo_model_complete.rs)**
  - Complete LBO model with capital structure integration
  - Features: `statements`

### Valuations Examples (`examples/valuations/`)

**Financial instrument pricing and risk analytics**

- **[bond_custom_cashflows_example.rs](./valuations/bond_custom_cashflows_example.rs)**
  - Custom cashflow schedules for bonds (step-up, PIK toggle, amortizing)
  - Features: `valuations`

- **[enhanced_builders_example.rs](./valuations/enhanced_builders_example.rs)**
  - Enhanced builder patterns for financial instruments
  - Features: `valuations`

- **[calibration_report_builder_example.rs](./valuations/calibration_report_builder_example.rs)**
  - Calibration report generation and one-liner API
  - Features: `valuations`

- **[multi_curve_framework_example.rs](./valuations/multi_curve_framework_example.rs)**
  - Post-2008 multi-curve framework with basis swaps
  - Features: `valuations`

- **[tranche_valuation_example.rs](./valuations/tranche_valuation_example.rs)**
  - Structured credit tranche valuation
  - Features: `valuations`

### Scenarios Examples (`examples/scenarios/`)

**Scenario analysis and stress testing**

- **[scenarios_lite_example.rs](./scenarios/scenarios_lite_example.rs)**
  - Basic scenario analysis with market data modifications
  - Features: `scenarios`

### Portfolio Examples (`examples/portfolio/`)

**Portfolio management and aggregation**

- **[portfolio_example.rs](./portfolio/portfolio_example.rs)**
  - Multi-asset portfolio with entity-based and standalone positions
  - Cross-currency FX conversion and scenario application
  - Features: `portfolio`

## Quick Start Guide

### 1. Basic Core Example

```bash
cargo run --example market_context_v2_demo --features core
```

### 2. Financial Statements Modeling

```bash
cargo run --example statements_phase1_example --features statements
```

### 3. Bond Pricing

```bash
cargo run --example bond_custom_cashflows_example --features valuations
```

### 4. Portfolio Analysis

```bash
cargo run --example portfolio_example --features portfolio
```

### 5. Scenario Analysis

```bash
cargo run --example scenarios_lite_example --features scenarios
```

## Feature Requirements

Most examples require specific feature flags to run. The main feature groups are:

- `core` - Core functionality (always required)
- `statements` - Financial statement modeling
- `valuations` - Instrument pricing and risk
- `scenarios` - Scenario analysis
- `portfolio` - Portfolio management
- `all` - All features enabled

## Example Categories

### Beginner Examples

Start with these for basic understanding:

- `market_context_v2_demo`
- `statements_phase1_example`
- `bond_custom_cashflows_example`

### Intermediate Examples

For more complex use cases:

- `statements_phase3_example`
- `enhanced_builders_example`
- `scenarios_lite_example`

### Advanced Examples

For comprehensive, production-like scenarios:

- `lbo_model_complete`
- `portfolio_example`
- `scenarios_lite_example`

## Contributing

When adding new examples:

1. Place the file in the appropriate subdirectory (`core/`, `statements/`, `valuations/`, `scenarios/`, `portfolio/`)
2. Add the example configuration to `finstack/Cargo.toml` with proper feature requirements
3. Update this README with a brief description
4. Include comprehensive inline documentation in the example
5. Test the example with `cargo run --example <name> --features <required>`

## Notes

- All examples are designed to be self-contained and educational
- Examples use realistic financial data and scenarios
- Error handling is demonstrated throughout
- Performance considerations are noted where relevant
- Examples follow the project's coding standards and best practices
