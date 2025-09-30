# Statements Crate — Detailed Implementation Plan

**Status:** Draft (implementation-ready)
**Last updated:** 2025-09-30
**MSRV:** 1.75 (target)
**License:** Apache-2.0 (project standard)

---

## Executive Summary

This document provides a comprehensive, phase-by-phase implementation plan for the `finstack-statements` crate. The statements engine will enable users to build financial statement models as directed graphs of metrics evaluated over discrete periods (monthly, quarterly, annually). 

**Key Design Principles:**
- **Leverage core/valuations**: Reuse Period system, Expression engine, Money types, and Polars re-exports
- **Extensibility-first**: Plugin architecture for custom analyses (corkscrew schedules, credit scorecards, etc.)
- **Dynamic metrics**: JSON-based metric registry (no recompilation needed)
- **Rich DSL**: Statistical operators, time-series functions, and capital structure integration
- **Determinism**: Stable ordering, currency-safe arithmetic, serial ≡ parallel

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Crate Structure](#2-crate-structure)
3. [Core Type Definitions](#3-core-type-definitions)
4. [Statements DSL Design](#4-statements-dsl-design)
5. [Dynamic Metric Registry](#5-dynamic-metric-registry)
6. [Capital Structure Integration](#6-capital-structure-integration)
7. [Extension Plugin Architecture](#7-extension-plugin-architecture)
8. [Implementation Phases](#8-implementation-phases)
9. [Testing Strategy](#9-testing-strategy)
10. [Examples](#10-examples)

---

## 1. Architecture Overview

### 1.1 Dependency Graph

```
┌─────────────────────────────────────────────┐
│         finstack-statements                 │
│                                             │
│  ┌─────────────────────────────────────┐   │
│  │  Builder (Type-State Pattern)       │   │
│  │  - ModelBuilder<NeedPeriods>        │   │
│  │  - ModelBuilder<Ready>              │   │
│  └──────────────┬──────────────────────┘   │
│                 │                           │
│  ┌──────────────▼──────────────────────┐   │
│  │  Core Types (Wire + Runtime)        │   │
│  │  - NodeSpec, FinancialModelSpec     │   │
│  │  - AmountOrScalar                   │   │
│  └──────────────┬──────────────────────┘   │
│                 │                           │
│  ┌──────────────▼──────────────────────┐   │
│  │  Evaluator                          │   │
│  │  - DAG construction                 │   │
│  │  - Precedence resolution            │   │
│  │  - Per-period evaluation            │   │
│  └──────────────┬──────────────────────┘   │
│                 │                           │
│  ┌──────────────▼──────────────────────┐   │
│  │  DSL Engine                         │   │
│  │  - Parser (formula_text → AST)      │   │
│  │  - Time-series operators            │   │
│  │  - Statistical functions            │   │
│  └──────────────┬──────────────────────┘   │
│                 │                           │
│  ┌──────────────▼──────────────────────┐   │
│  │  Forecast Methods                   │   │
│  │  - ForwardFill, GrowthPct           │   │
│  │  - Statistical (Normal, etc.)       │   │
│  └──────────────┬──────────────────────┘   │
│                 │                           │
│  ┌──────────────▼──────────────────────┐   │
│  │  Dynamic Registry (JSON)            │   │
│  │  - Load metrics from JSON           │   │
│  │  - Namespace management (fin.*)     │   │
│  └──────────────┬──────────────────────┘   │
│                 │                           │
│  ┌──────────────▼──────────────────────┐   │
│  │  Extension Plugins                  │   │
│  │  - Extension trait                  │   │
│  │  - Plugin registry                  │   │
│  └─────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
         ▲                      ▲
         │                      │
    ┌────┴─────┐         ┌─────┴──────────┐
    │  core/   │         │  valuations/   │
    │  - Period│         │  - Instruments │
    │  - Expr  │         │  - Cashflow    │
    │  - Money │         │  - Aggregation │
    └──────────┘         └────────────────┘
```

### 1.2 Integration Points

**From `finstack-core`:**
- ✅ `Period`, `PeriodPlan`, `PeriodId` - period system and parsing
- ✅ `Money`, `Currency` - currency-safe amounts
- ✅ `Date`, `DayCount`, `BusinessDayConvention` - date utilities
- ✅ `Expr`, `ExprNode`, `Function`, `CompiledExpr` - expression AST
- ✅ `ExpressionContext` - evaluation context trait
- ✅ `ResultsMeta`, `FinstackConfig` - metadata stamping
- ✅ Polars `DataFrame`/`Series` re-exports - vectorization

**From `finstack-valuations`:**
- ✅ Instrument types (`Bond`, `InterestRateSwap`, etc.) - capital structure
- ✅ `aggregate_by_period` - cashflow aggregation
- ✅ `CashflowBuilder` - debt schedule generation
- ✅ Metric calculation - interest expense, principal payments

**Extension Points (Future):**
- ⚠️ Corkscrew schedules (roll-forward analysis)
- ⚠️ Credit scorecards (rating-based stress tests)
- ⚠️ Real estate (property cash flows, waterfalls)

---

## 2. Crate Structure

```
finstack/statements/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs                    # Public API surface
│   ├── error.rs                  # Typed error hierarchy
│   │
│   ├── types/
│   │   ├── mod.rs
│   │   ├── node.rs               # NodeSpec, Node, NodeType
│   │   ├── value.rs              # AmountOrScalar, ValueType
│   │   ├── forecast.rs           # ForecastSpec, ForecastMethod
│   │   ├── model.rs              # FinancialModelSpec, FinancialModel
│   │   └── capital_structure.rs  # CapitalStructureSpec (debt instruments)
│   │
│   ├── dsl/
│   │   ├── mod.rs
│   │   ├── parser.rs             # Formula text → AST
│   │   ├── ast.rs                # Statements DSL AST (extends core Expr)
│   │   ├── operators.rs          # Lag, Lead, RollingMean, etc.
│   │   ├── functions.rs          # Built-in functions (sum, mean, etc.)
│   │   └── compiler.rs           # AST → CompiledExpr
│   │
│   ├── builder/
│   │   ├── mod.rs
│   │   ├── model_builder.rs      # Type-state builder pattern
│   │   ├── node_builder.rs       # Helper builders for nodes
│   │   └── capital_builder.rs    # Capital structure builder helpers
│   │
│   ├── evaluator/
│   │   ├── mod.rs
│   │   ├── evaluator.rs          # Main evaluation orchestrator
│   │   ├── context.rs            # StatementContext (ExpressionContext impl)
│   │   ├── precedence.rs         # Value > Forecast > Formula
│   │   ├── dag.rs                # Dependency graph construction
│   │   └── capital_integration.rs # Integrate valuations cashflow aggregation
│   │
│   ├── forecast/
│   │   ├── mod.rs
│   │   ├── deterministic.rs      # ForwardFill, GrowthPct
│   │   ├── statistical.rs        # Normal, LogNormal distributions
│   │   ├── time_series.rs        # Curve growth rates, indexed growth
│   │   └── override.rs           # Explicit override method
│   │
│   ├── registry/
│   │   ├── mod.rs
│   │   ├── dynamic.rs            # JSON-based metric loader
│   │   ├── builtins.rs           # Hardcoded fin.* metrics (fallback)
│   │   ├── schema.rs             # JSON schema for metrics
│   │   └── validation.rs         # Registry validation
│   │
│   ├── extensions/
│   │   ├── mod.rs
│   │   ├── plugin.rs             # Extension trait & registry
│   │   ├── corkscrew.rs          # Placeholder for corkscrew analysis
│   │   └── scorecards.rs         # Placeholder for credit scorecards
│   │
│   ├── results/
│   │   ├── mod.rs
│   │   ├── results.rs            # Results struct
│   │   ├── export.rs             # DataFrame exports (long/wide)
│   │   └── metadata.rs           # Result metadata tracking
│   │
│   └── validation/
│       ├── mod.rs
│       └── checks.rs             # Model validation rules
│
├── tests/
│   ├── builder_tests.rs
│   ├── evaluator_tests.rs
│   ├── dsl_tests.rs
│   ├── forecast_tests.rs
│   ├── registry_tests.rs
│   ├── capital_structure_tests.rs
│   ├── integration_tests.rs
│   └── golden/
│       ├── basic_model.json
│       ├── capital_structure_model.json
│       └── statistical_forecast_model.json
│
└── data/
    └── metrics/
        ├── fin_basic.json        # Basic financial metrics
        ├── fin_margins.json      # Margin calculations
        ├── fin_returns.json      # Return metrics (ROE, ROA, etc.)
        └── fin_leverage.json     # Leverage ratios
```

---

## 3. Core Type Definitions

### 3.1 Wire Types (Serde-stable, public API)

```rust
// types/node.rs
use finstack_core::prelude::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Wire-format node specification with stable serde names.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeSpec {
    /// Unique identifier for this node
    pub node_id: String,
    
    /// Optional display name
    pub name: Option<String>,
    
    /// Node computation type
    pub node_type: NodeType,
    
    /// Sparse map of explicit values by period
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<IndexMap<PeriodId, AmountOrScalar>>,
    
    /// Forecast specifications (applied to non-actual periods)
    #[serde(default)]
    pub forecasts: Vec<ForecastSpec>,
    
    /// Formula text (statements DSL)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formula_text: Option<String>,
    
    /// Where clause (boolean mask over periods)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub where_text: Option<String>,
    
    /// Optional tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    
    /// Arbitrary metadata
    #[serde(default)]
    pub meta: IndexMap<String, serde_json::Value>,
}

/// Node computation type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    /// Only explicit values (no formula/forecast)
    Value,
    
    /// Only formula (computed from other nodes)
    Calculated,
    
    /// Value OR Forecast OR Formula (precedence: Value > Forecast > Formula)
    Mixed,
}

// types/value.rs

/// Value that can be either a currency amount or unitless scalar.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AmountOrScalar {
    /// Currency-aware monetary amount
    Amount(Money),
    
    /// Unitless scalar (for ratios, percentages, counts, etc.)
    Scalar(f64),
}

impl AmountOrScalar {
    /// Extract as f64 (Amount → amount, Scalar → value)
    pub fn as_f64(&self) -> f64 {
        match self {
            AmountOrScalar::Amount(m) => m.amount(),
            AmountOrScalar::Scalar(s) => *s,
        }
    }
    
    /// Check if this is a currency amount
    pub fn is_amount(&self) -> bool {
        matches!(self, AmountOrScalar::Amount(_))
    }
    
    /// Get currency if this is an Amount
    pub fn currency(&self) -> Option<Currency> {
        match self {
            AmountOrScalar::Amount(m) => Some(m.currency()),
            AmountOrScalar::Scalar(_) => None,
        }
    }
}

// types/forecast.rs

/// Forecast specification.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForecastSpec {
    /// Forecast method to apply
    pub method: ForecastMethod,
    
    /// Method-specific parameters (validated per method)
    #[serde(default)]
    pub params: IndexMap<String, serde_json::Value>,
}

/// Forecast methods available to users.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForecastMethod {
    /// Carry last known value into forecast periods
    ForwardFill,
    
    /// Apply constant growth rate: v[t] = v[t-1] * (1 + g)
    GrowthPct,
    
    /// Sample from normal distribution
    Normal,
    
    /// Sample from log-normal distribution
    LogNormal,
    
    /// Explicit period overrides (sparse map)
    Override,
    
    /// Reference an external time series node
    TimeSeries,
    
    /// Apply seasonal pattern (multiplicative or additive)
    Seasonal,
}

// types/model.rs

/// Wire-format financial model specification.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinancialModelSpec {
    /// Unique model identifier
    pub id: String,
    
    /// Period definitions (from core)
    pub periods: Vec<Period>,
    
    /// Node specifications (deterministic order via IndexMap)
    pub nodes: IndexMap<String, NodeSpec>,
    
    /// Optional capital structure specification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capital_structure: Option<CapitalStructureSpec>,
    
    /// Arbitrary metadata
    #[serde(default)]
    pub meta: IndexMap<String, serde_json::Value>,
    
    /// Schema version for forward compatibility
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
}

fn default_schema_version() -> u32 { 1 }

// types/capital_structure.rs

/// Capital structure specification for company debt/equity modeling.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapitalStructureSpec {
    /// Debt instruments (bonds, loans, etc.)
    #[serde(default)]
    pub debt: Vec<DebtInstrumentSpec>,
    
    /// Equity instruments
    #[serde(default)]
    pub equity: Vec<EquityInstrumentSpec>,
    
    /// FX provider configuration for multi-currency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fx_config: Option<FxConfigSpec>,
}

/// Debt instrument specification (delegates to valuations crate).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "instrument_type", rename_all = "snake_case")]
pub enum DebtInstrumentSpec {
    /// Fixed-rate bond
    Bond {
        id: String,
        notional: Money,
        coupon_rate: f64,
        issue_date: Date,
        maturity_date: Date,
        discount_curve_id: String,
        #[serde(default)]
        frequency: Option<Frequency>,
        #[serde(default)]
        day_count: Option<DayCount>,
    },
    
    /// Interest rate swap
    Swap {
        id: String,
        notional: Money,
        fixed_rate: f64,
        start_date: Date,
        maturity_date: Date,
        discount_curve_id: String,
        forward_curve_id: String,
        #[serde(default)]
        frequency: Option<Frequency>,
    },
    
    /// Generic instrument (JSON-based spec)
    Generic {
        id: String,
        spec: serde_json::Value,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EquityInstrumentSpec {
    pub id: String,
    pub shares: f64,
    pub ticker: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FxConfigSpec {
    pub pivot_currency: Currency,
    pub enable_triangulation: bool,
}
```

### 3.2 Runtime Types

```rust
// evaluator/context.rs

/// Runtime node (compiled from NodeSpec).
pub struct Node {
    /// Wire specification
    pub spec: NodeSpec,
    
    /// Compiled formula (if present)
    pub formula: Option<CompiledExpr>,
    
    /// Compiled where clause (if present)
    pub where_clause: Option<CompiledExpr>,
    
    /// Dependency set (node IDs referenced in formula)
    pub dependencies: Vec<String>,
}

/// Runtime financial model (compiled from FinancialModelSpec).
pub struct FinancialModel {
    /// Model identifier
    pub id: String,
    
    /// Periods from core
    pub periods: Vec<Period>,
    
    /// Compiled nodes (deterministic order)
    pub nodes: IndexMap<String, Node>,
    
    /// Dynamic metric registry
    pub registry: Registry,
    
    /// Capital structure integration
    pub capital_structure: Option<CapitalStructure>,
    
    /// Extension plugins
    pub extensions: Vec<Box<dyn Extension>>,
    
    /// Arbitrary metadata
    pub meta: IndexMap<String, serde_json::Value>,
}

/// Capital structure runtime representation.
pub struct CapitalStructure {
    /// Debt instruments (from valuations crate)
    pub debt: Vec<Box<dyn Instrument>>,
    
    /// Equity instruments
    pub equity: Vec<EquityInstrument>,
    
    /// FX matrix for multi-currency
    pub fx_matrix: Option<Arc<FxMatrix>>,
}

/// Statement evaluation context implementing ExpressionContext.
pub struct StatementContext<'m> {
    model: &'m FinancialModel,
    period_index: usize,
    period: &'m Period,
    
    /// Column name → column index mapping
    column_mapping: HashMap<String, usize>,
    
    /// Column data (one per node, aligned with column_mapping)
    column_data: Vec<Vec<f64>>,
}

impl<'m> ExpressionContext for StatementContext<'m> {
    fn resolve_index(&self, name: &str) -> Option<usize> {
        self.column_mapping.get(name).copied()
    }
}

impl<'m> StatementContext<'m> {
    /// Create context for evaluating nodes in a specific period.
    pub fn new(
        model: &'m FinancialModel,
        period_index: usize,
        prior_results: &IndexMap<String, IndexMap<PeriodId, f64>>,
    ) -> Self {
        let period = &model.periods[period_index];
        
        // Build column mapping: node_id → column index
        let mut column_mapping = HashMap::new();
        let mut column_data = Vec::new();
        
        for (idx, node_id) in model.nodes.keys().enumerate() {
            column_mapping.insert(node_id.clone(), idx);
            
            // Get value for this node in this period (if available)
            let value = prior_results
                .get(node_id)
                .and_then(|period_map| period_map.get(&period.id))
                .copied()
                .unwrap_or(0.0);
            
            // Create column with single value (per-period evaluation)
            column_data.push(vec![value]);
        }
        
        Self {
            model,
            period_index,
            period,
            column_mapping,
            column_data,
        }
    }
    
    /// Get column data as slice references for expression evaluation.
    pub fn columns(&self) -> Vec<&[f64]> {
        self.column_data.iter().map(|v| v.as_slice()).collect()
    }
}
```

---

## 4. Statements DSL Design

### 4.1 DSL Features

The statements DSL extends core's expression engine with domain-specific operators:

**Time-Series Operators:**
- `lag(node, n)` - Previous n periods
- `lead(node, n)` - Future n periods
- `diff(node, n)` - First difference
- `pct_change(node, n)` - Percentage change
- `rolling_mean(node, window)` - Rolling average
- `rolling_sum(node, window)` - Rolling sum
- `rolling_std(node, window)` - Rolling standard deviation
- `cumsum(node)` - Cumulative sum
- `cumprod(node)` - Cumulative product

**Statistical Functions:**
- `mean(node)` - Period average
- `median(node)` - Period median
- `std(node)` - Standard deviation
- `var(node)` - Variance
- `quantile(node, q)` - Quantile calculation

**Aggregation Functions:**
- `sum(node1, node2, ...)` - Sum across nodes
- `min(node1, node2, ...)` - Minimum
- `max(node1, node2, ...)` - Maximum

**Conditional Operators:**
- `if(condition, true_value, false_value)` - Ternary conditional
- `coalesce(node, default)` - Null coalescing

**Financial Functions:**
- `annualize(node, periods_per_year)` - Annualize a value
- `ttm(node, periods)` - Trailing twelve months

### 4.2 Parser Design

```rust
// dsl/parser.rs

use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{char, multispace0},
    combinator::{map, opt},
    multi::separated_list0,
    sequence::{delimited, tuple},
};

/// Parse statements DSL formula text into AST.
pub fn parse_formula(input: &str) -> crate::Result<StmtExpr> {
    match formula_parser(input) {
        Ok(("", expr)) => Ok(expr),
        Ok((remaining, _)) => Err(crate::Error::FormulaParse(
            format!("Unexpected input remaining: {}", remaining)
        )),
        Err(e) => Err(crate::Error::FormulaParse(
            format!("Parse error: {}", e)
        )),
    }
}

/// Statements DSL expression (extends core Expr).
#[derive(Clone, Debug)]
pub enum StmtExpr {
    /// Reference to a node
    Node(String),
    
    /// Literal value
    Literal(f64),
    
    /// Binary operation
    BinOp {
        op: BinOp,
        left: Box<StmtExpr>,
        right: Box<StmtExpr>,
    },
    
    /// Function call
    Call {
        func: String,
        args: Vec<StmtExpr>,
    },
    
    /// Conditional expression
    If {
        condition: Box<StmtExpr>,
        true_expr: Box<StmtExpr>,
        false_expr: Box<StmtExpr>,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
}

// Parser combinators
fn formula_parser(input: &str) -> IResult<&str, StmtExpr> {
    expression(input)
}

fn expression(input: &str) -> IResult<&str, StmtExpr> {
    logical_or(input)
}

fn logical_or(input: &str) -> IResult<&str, StmtExpr> {
    let (input, left) = logical_and(input)?;
    let (input, pairs) = nom::multi::many0(
        tuple((ws(tag("or")), logical_and))
    )(input)?;
    
    Ok((input, build_left_assoc(left, pairs, BinOp::Or)))
}

fn logical_and(input: &str) -> IResult<&str, StmtExpr> {
    let (input, left) = comparison(input)?;
    let (input, pairs) = nom::multi::many0(
        tuple((ws(tag("and")), comparison))
    )(input)?;
    
    Ok((input, build_left_assoc(left, pairs, BinOp::And)))
}

fn comparison(input: &str) -> IResult<&str, StmtExpr> {
    let (input, left) = additive(input)?;
    let (input, op_right) = opt(tuple((
        ws(comparison_op),
        additive
    )))(input)?;
    
    Ok((input, match op_right {
        Some((op, right)) => StmtExpr::BinOp {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
        None => left,
    }))
}

fn comparison_op(input: &str) -> IResult<&str, BinOp> {
    alt((
        map(tag("=="), |_| BinOp::Eq),
        map(tag("!="), |_| BinOp::Ne),
        map(tag("<="), |_| BinOp::Le),
        map(tag(">="), |_| BinOp::Ge),
        map(tag("<"), |_| BinOp::Lt),
        map(tag(">"), |_| BinOp::Gt),
    ))(input)
}

fn additive(input: &str) -> IResult<&str, StmtExpr> {
    let (input, left) = multiplicative(input)?;
    let (input, pairs) = nom::multi::many0(
        tuple((ws(alt((char('+'), char('-')))), multiplicative))
    )(input)?;
    
    Ok((input, build_left_assoc_char(left, pairs)))
}

fn multiplicative(input: &str) -> IResult<&str, StmtExpr> {
    let (input, left) = primary(input)?;
    let (input, pairs) = nom::multi::many0(
        tuple((ws(alt((char('*'), char('/'), char('%')))), primary))
    )(input)?;
    
    Ok((input, build_left_assoc_char(left, pairs)))
}

fn primary(input: &str) -> IResult<&str, StmtExpr> {
    alt((
        function_call,
        literal,
        node_reference,
        delimited(ws(char('(')), expression, ws(char(')'))),
    ))(input)
}

fn function_call(input: &str) -> IResult<&str, StmtExpr> {
    let (input, func_name) = identifier(input)?;
    let (input, _) = ws(char('('))(input)?;
    let (input, args) = separated_list0(ws(char(',')), expression)(input)?;
    let (input, _) = ws(char(')'))(input)?;
    
    Ok((input, StmtExpr::Call {
        func: func_name.to_string(),
        args,
    }))
}

fn literal(input: &str) -> IResult<&str, StmtExpr> {
    use nom::character::complete::digit1;
    use nom::combinator::recognize;
    use nom::sequence::tuple;
    
    let (input, num_str) = ws(recognize(tuple((
        opt(char('-')),
        digit1,
        opt(tuple((char('.'), digit1))),
    ))))(input)?;
    
    let value = num_str.parse::<f64>()
        .map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;
    
    Ok((input, StmtExpr::Literal(value)))
}

fn node_reference(input: &str) -> IResult<&str, StmtExpr> {
    map(identifier, |name| StmtExpr::Node(name.to_string()))(input)
}

fn identifier(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '.')(input)
}

fn ws<'a, F, O>(parser: F) -> impl FnMut(&'a str) -> IResult<&'a str, O>
where
    F: nom::Parser<&'a str, O, nom::error::Error<&'a str>>,
{
    delimited(multispace0, parser, multispace0)
}

fn build_left_assoc(
    left: StmtExpr,
    pairs: Vec<(&str, StmtExpr)>,
    op: BinOp,
) -> StmtExpr {
    pairs.into_iter().fold(left, |acc, (_, right)| {
        StmtExpr::BinOp {
            op,
            left: Box::new(acc),
            right: Box::new(right),
        }
    })
}

fn build_left_assoc_char(
    left: StmtExpr,
    pairs: Vec<(char, StmtExpr)>,
) -> StmtExpr {
    pairs.into_iter().fold(left, |acc, (ch, right)| {
        let op = match ch {
            '+' => BinOp::Add,
            '-' => BinOp::Sub,
            '*' => BinOp::Mul,
            '/' => BinOp::Div,
            '%' => BinOp::Mod,
            _ => unreachable!(),
        };
        StmtExpr::BinOp {
            op,
            left: Box::new(acc),
            right: Box::new(right),
        }
    })
}
```

### 4.3 DSL Compiler (AST → Core Expr)

```rust
// dsl/compiler.rs

/// Compile statements DSL AST into core expression AST.
pub fn compile(stmt_expr: &StmtExpr) -> crate::Result<Expr> {
    match stmt_expr {
        StmtExpr::Node(name) => {
            // Node references become Column references in core
            Ok(Expr::column(name))
        },
        
        StmtExpr::Literal(value) => {
            Ok(Expr::literal(*value))
        },
        
        StmtExpr::BinOp { op, left, right } => {
            compile_binop(*op, left, right)
        },
        
        StmtExpr::Call { func, args } => {
            compile_function_call(func, args)
        },
        
        StmtExpr::If { condition, true_expr, false_expr } => {
            compile_conditional(condition, true_expr, false_expr)
        },
    }
}

fn compile_binop(
    op: BinOp,
    left: &StmtExpr,
    right: &StmtExpr,
) -> crate::Result<Expr> {
    let left_expr = compile(left)?;
    let right_expr = compile(right)?;
    
    match op {
        BinOp::Add => Ok(build_binary_expr(left_expr, right_expr, "add")),
        BinOp::Sub => Ok(build_binary_expr(left_expr, right_expr, "sub")),
        BinOp::Mul => Ok(build_binary_expr(left_expr, right_expr, "mul")),
        BinOp::Div => Ok(build_binary_expr(left_expr, right_expr, "div")),
        _ => Err(crate::Error::FormulaParse(
            format!("Unsupported binary operator: {:?}", op)
        )),
    }
}

fn compile_function_call(
    func: &str,
    args: &[StmtExpr],
) -> crate::Result<Expr> {
    // Map statement functions to core functions
    let core_func = match func {
        "lag" => Function::Lag,
        "lead" => Function::Lead,
        "diff" => Function::Diff,
        "pct_change" => Function::PctChange,
        "rolling_mean" => Function::RollingMean,
        "rolling_sum" => Function::RollingSum,
        "cumsum" => Function::CumSum,
        "cumprod" => Function::CumProd,
        "std" => Function::Std,
        "var" => Function::Var,
        "median" => Function::Median,
        
        // Custom statements functions
        "sum" => return compile_sum_function(args),
        "mean" => return compile_mean_function(args),
        "annualize" => return compile_annualize_function(args),
        "ttm" => return compile_ttm_function(args),
        "coalesce" => return compile_coalesce_function(args),
        
        _ => return Err(crate::Error::FormulaParse(
            format!("Unknown function: {}", func)
        )),
    };
    
    // Compile arguments
    let compiled_args: Result<Vec<Expr>, _> = args.iter()
        .map(|arg| compile(arg))
        .collect();
    
    Ok(Expr::call(core_func, compiled_args?))
}

fn compile_sum_function(args: &[StmtExpr]) -> crate::Result<Expr> {
    // sum(a, b, c) → a + b + c
    if args.is_empty() {
        return Ok(Expr::literal(0.0));
    }
    
    let mut result = compile(&args[0])?;
    for arg in &args[1..] {
        let arg_expr = compile(arg)?;
        result = build_binary_expr(result, arg_expr, "add");
    }
    Ok(result)
}

fn compile_mean_function(args: &[StmtExpr]) -> crate::Result<Expr> {
    // mean(a, b, c) → (a + b + c) / 3
    if args.is_empty() {
        return Ok(Expr::literal(0.0));
    }
    
    let sum_expr = compile_sum_function(args)?;
    let count = Expr::literal(args.len() as f64);
    Ok(build_binary_expr(sum_expr, count, "div"))
}

fn compile_annualize_function(args: &[StmtExpr]) -> crate::Result<Expr> {
    // annualize(value, periods_per_year) → value * periods_per_year
    if args.len() != 2 {
        return Err(crate::Error::FormulaParse(
            "annualize requires 2 arguments: value, periods_per_year".into()
        ));
    }
    
    let value = compile(&args[0])?;
    let periods = compile(&args[1])?;
    Ok(build_binary_expr(value, periods, "mul"))
}

fn compile_ttm_function(args: &[StmtExpr]) -> crate::Result<Expr> {
    // ttm(revenue) → sum of last 4 quarters (if quarterly)
    // This is equivalent to: rolling_sum(revenue, 4) evaluated at current period
    if args.len() != 1 {
        return Err(crate::Error::FormulaParse(
            "ttm requires 1 argument: node".into()
        ));
    }
    
    let node = compile(&args[0])?;
    let window = Expr::literal(4.0); // Assume quarterly
    Ok(Expr::call(Function::RollingSum, vec![node, window]))
}

fn compile_coalesce_function(args: &[StmtExpr]) -> crate::Result<Expr> {
    // coalesce(node, default) → if node is null/NaN, use default
    // Implementation: use conditional or null handling
    if args.len() != 2 {
        return Err(crate::Error::FormulaParse(
            "coalesce requires 2 arguments: value, default".into()
        ));
    }
    
    let value = compile(&args[0])?;
    let default = compile(&args[1])?;
    
    // For now, NaN handling is implicit in core's expression engine
    // This would need special handling in the evaluator
    Ok(value) // Simplified for MVP
}

fn build_binary_expr(left: Expr, right: Expr, op: &str) -> Expr {
    // Map string operation to actual implementation
    // This is a simplified version; actual implementation would use
    // proper Expr construction
    match op {
        "add" => {
            // In reality, we'd build a proper binary expression
            // For now, this is a placeholder
            Expr::call(Function::CumSum, vec![left, right])
        },
        _ => Expr::literal(0.0), // Placeholder
    }
}

// Note: Actual implementation would properly construct Expr nodes
// The above is simplified for illustration
```

### 4.4 Extended Operators

```rust
// dsl/operators.rs

/// Register statements-specific operators with the expression engine.
pub fn register_stmt_operators() -> HashMap<String, OperatorDef> {
    let mut ops = HashMap::new();
    
    // Time-series operators
    ops.insert("lag".into(), OperatorDef {
        core_function: Function::Lag,
        min_args: 1,
        max_args: 2,
        description: "Get value from n periods ago".into(),
    });
    
    ops.insert("rolling_mean".into(), OperatorDef {
        core_function: Function::RollingMean,
        min_args: 2,
        max_args: 2,
        description: "Calculate rolling average over window".into(),
    });
    
    // ... register all operators
    
    ops
}

pub struct OperatorDef {
    pub core_function: Function,
    pub min_args: usize,
    pub max_args: usize,
    pub description: String,
}
```

---

## 5. Dynamic Metric Registry

### 5.1 JSON Schema for Metrics

```json
// data/metrics/fin_basic.json
{
  "namespace": "fin",
  "schema_version": 1,
  "metrics": [
    {
      "id": "gross_profit",
      "name": "Gross Profit",
      "formula": "revenue - cogs",
      "description": "Revenue minus cost of goods sold",
      "category": "profitability",
      "unit_type": "currency"
    },
    {
      "id": "gross_margin",
      "name": "Gross Margin %",
      "formula": "gross_profit / revenue",
      "description": "Gross profit as percentage of revenue",
      "category": "margins",
      "unit_type": "percentage"
    },
    {
      "id": "operating_income",
      "name": "Operating Income (EBIT)",
      "formula": "gross_profit - operating_expenses",
      "description": "Earnings before interest and taxes",
      "category": "profitability",
      "unit_type": "currency"
    },
    {
      "id": "ebitda",
      "name": "EBITDA",
      "formula": "operating_income + depreciation + amortization",
      "description": "Earnings before interest, taxes, depreciation, and amortization",
      "category": "profitability",
      "unit_type": "currency"
    },
    {
      "id": "net_income",
      "name": "Net Income",
      "formula": "operating_income - interest_expense - tax_expense",
      "description": "Bottom-line profit after all expenses",
      "category": "profitability",
      "unit_type": "currency"
    },
    {
      "id": "roe",
      "name": "Return on Equity",
      "formula": "annualize(net_income, 4) / total_equity",
      "description": "Annualized net income as % of equity",
      "category": "returns",
      "unit_type": "percentage",
      "requires": ["net_income", "total_equity"]
    },
    {
      "id": "debt_to_ebitda",
      "name": "Total Debt / EBITDA",
      "formula": "total_debt / ttm(ebitda)",
      "description": "Leverage ratio using trailing twelve months EBITDA",
      "category": "leverage",
      "unit_type": "ratio",
      "requires": ["total_debt", "ebitda"]
    }
  ]
}
```

### 5.2 Dynamic Registry Implementation

```rust
// registry/dynamic.rs

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricDefinition {
    pub id: String,
    pub name: String,
    pub formula: String,
    pub description: String,
    pub category: String,
    pub unit_type: UnitType,
    #[serde(default)]
    pub requires: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitType {
    Currency,
    Percentage,
    Ratio,
    Count,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricRegistry {
    pub namespace: String,
    pub schema_version: u32,
    pub metrics: Vec<MetricDefinition>,
}

/// Dynamic registry loader from JSON.
pub struct Registry {
    /// Loaded metrics by fully-qualified ID (e.g., "fin.gross_profit")
    metrics: IndexMap<String, CompiledMetric>,
    
    /// Namespace -> registry mapping
    namespaces: HashMap<String, MetricRegistry>,
}

pub struct CompiledMetric {
    pub definition: MetricDefinition,
    pub compiled_expr: CompiledExpr,
}

impl Registry {
    /// Create empty registry.
    pub fn new() -> Self {
        Self {
            metrics: IndexMap::new(),
            namespaces: HashMap::new(),
        }
    }
    
    /// Load metrics from JSON file.
    pub fn load_from_json(
        &mut self,
        json_path: &std::path::Path,
    ) -> crate::Result<()> {
        let json_str = std::fs::read_to_string(json_path)
            .map_err(|e| crate::Error::Build(format!("Failed to read {}: {}", json_path.display(), e)))?;
        
        let registry: MetricRegistry = serde_json::from_str(&json_str)
            .map_err(|e| crate::Error::Build(format!("Failed to parse JSON: {}", e)))?;
        
        self.load_registry(registry)
    }
    
    /// Load metrics from registry object.
    pub fn load_registry(
        &mut self,
        registry: MetricRegistry,
    ) -> crate::Result<()> {
        let namespace = registry.namespace.clone();
        
        for metric_def in registry.metrics {
            // Compile formula
            let stmt_ast = crate::dsl::parse_formula(&metric_def.formula)?;
            let core_expr = crate::dsl::compile(&stmt_ast)?;
            let compiled_expr = CompiledExpr::new(core_expr);
            
            // Store with fully-qualified ID
            let fq_id = format!("{}.{}", namespace, metric_def.id);
            
            self.metrics.insert(fq_id.clone(), CompiledMetric {
                definition: metric_def.clone(),
                compiled_expr,
            });
        }
        
        self.namespaces.insert(namespace, registry);
        Ok(())
    }
    
    /// Load built-in metrics from embedded JSON.
    pub fn load_builtins(&mut self) -> crate::Result<()> {
        // Embed JSON files at compile time
        const FIN_BASIC: &str = include_str!("../../data/metrics/fin_basic.json");
        const FIN_MARGINS: &str = include_str!("../../data/metrics/fin_margins.json");
        const FIN_RETURNS: &str = include_str!("../../data/metrics/fin_returns.json");
        const FIN_LEVERAGE: &str = include_str!("../../data/metrics/fin_leverage.json");
        
        for json_str in [FIN_BASIC, FIN_MARGINS, FIN_RETURNS, FIN_LEVERAGE] {
            let registry: MetricRegistry = serde_json::from_str(json_str)
                .map_err(|e| crate::Error::Build(format!("Failed to parse builtin metrics: {}", e)))?;
            self.load_registry(registry)?;
        }
        
        Ok(())
    }
    
    /// Get a metric by fully-qualified ID.
    pub fn get(&self, fq_id: &str) -> Option<&CompiledMetric> {
        self.metrics.get(fq_id)
    }
    
    /// Get all metrics in a namespace.
    pub fn namespace(&self, ns: &str) -> impl Iterator<Item = (&String, &CompiledMetric)> {
        self.metrics.iter()
            .filter(move |(fq_id, _)| fq_id.starts_with(&format!("{}.", ns)))
    }
    
    /// List all available namespaces.
    pub fn namespaces(&self) -> Vec<&str> {
        self.namespaces.keys().map(|s| s.as_str()).collect()
    }
}

// registry/builtins.rs

/// Fallback hardcoded metrics (if JSON loading fails).
pub fn fallback_metrics() -> IndexMap<String, String> {
    indexmap::indexmap! {
        "fin.gross_profit".into() => "revenue - cogs".into(),
        "fin.gross_margin".into() => "gross_profit / revenue".into(),
        "fin.operating_income".into() => "gross_profit - operating_expenses".into(),
        "fin.net_income".into() => "operating_income - interest_expense - tax_expense".into(),
    }
}
```

---

## 6. Capital Structure Integration

### 6.1 Capital Structure Specification

```rust
// types/capital_structure.rs

/// Capital structure runtime representation.
pub struct CapitalStructure {
    /// Debt instruments (from valuations)
    pub instruments: Vec<CapitalInstrument>,
    
    /// Market context for valuation
    pub market_context: Arc<MarketContext>,
    
    /// Aggregation configuration
    pub aggregation_config: AggregationConfig,
}

/// Wrapper for instruments in capital structure.
pub enum CapitalInstrument {
    Bond(Box<finstack_valuations::instruments::Bond>),
    Swap(Box<finstack_valuations::instruments::InterestRateSwap>),
    Loan(Box<finstack_valuations::instruments::Bond>), // Use Bond for simple loans
    // Add more as needed
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AggregationConfig {
    /// How to aggregate multi-currency cashflows
    pub fx_policy: FxConversionPolicy,
    
    /// Target currency for aggregation (if specified)
    pub model_currency: Option<Currency>,
}

impl CapitalStructure {
    /// Build from specification.
    pub fn from_spec(
        spec: CapitalStructureSpec,
        market_context: Arc<MarketContext>,
    ) -> crate::Result<Self> {
        let mut instruments = Vec::new();
        
        for debt_spec in spec.debt {
            let instrument = match debt_spec {
                DebtInstrumentSpec::Bond {
                    id, notional, coupon_rate, issue_date, maturity_date,
                    discount_curve_id, frequency, day_count,
                } => {
                    use finstack_valuations::instruments::Bond;
                    
                    let bond = Bond::fixed_semiannual(
                        id,
                        notional,
                        coupon_rate,
                        issue_date,
                        maturity_date,
                        discount_curve_id,
                    );
                    
                    CapitalInstrument::Bond(Box::new(bond))
                },
                
                DebtInstrumentSpec::Swap { .. } => {
                    // Similar construction for swaps
                    todo!("Implement swap construction")
                },
                
                DebtInstrumentSpec::Generic { .. } => {
                    todo!("Implement generic instrument loading")
                },
            };
            
            instruments.push(instrument);
        }
        
        Ok(Self {
            instruments,
            market_context,
            aggregation_config: AggregationConfig {
                fx_policy: FxConversionPolicy::PeriodEnd,
                model_currency: None,
            },
        })
    }
    
    /// Generate aggregate cashflows for all instruments.
    pub fn aggregate_cashflows(
        &self,
        periods: &[Period],
    ) -> crate::Result<IndexMap<String, AggregatedFlows>> {
        let mut results = IndexMap::new();
        
        for instrument in &self.instruments {
            let (id, flows) = match instrument {
                CapitalInstrument::Bond(bond) => {
                    // Generate cashflows using valuations builder
                    let schedule = bond.generate_cashflow_schedule()?;
                    (bond.id().clone(), schedule.flows)
                },
                
                CapitalInstrument::Swap(swap) => {
                    let schedule = swap.generate_cashflow_schedule()?;
                    (swap.id().clone(), schedule.flows)
                },
                
                CapitalInstrument::Loan(loan) => {
                    let schedule = loan.generate_cashflow_schedule()?;
                    (loan.id().clone(), schedule.flows)
                },
            };
            
            // Convert to (Date, Money) tuples
            let dated_flows: Vec<(Date, Money)> = flows.iter()
                .map(|cf| (cf.date, cf.amount))
                .collect();
            
            // Aggregate by period using valuations helper
            let aggregated = finstack_valuations::cashflow::aggregate_by_period(
                &dated_flows,
                periods,
            );
            
            results.insert(id, AggregatedFlows {
                by_period: aggregated,
                total_flows: dated_flows.len(),
            });
        }
        
        Ok(results)
    }
    
    /// Calculate interest expense for a period.
    pub fn interest_expense(
        &self,
        period: &Period,
    ) -> crate::Result<Money> {
        let aggregated = self.aggregate_cashflows(&[period.clone()])?;
        
        // Sum interest payments (CFKind::Fixed, CFKind::FloatReset)
        let mut total = Money::new(0.0, Currency::USD); // TODO: Handle multi-currency
        
        for (_, flows) in aggregated {
            if let Some(period_flows) = flows.by_period.get(&period.id) {
                for (ccy, amount) in period_flows {
                    // TODO: Convert to model currency if needed
                    let money = Money::new(*amount, *ccy);
                    total = (total + money)?;
                }
            }
        }
        
        Ok(total)
    }
    
    /// Calculate principal payments for a period.
    pub fn principal_payments(
        &self,
        period: &Period,
    ) -> crate::Result<Money> {
        // Similar to interest_expense but filter for CFKind::Notional, CFKind::Amortization
        todo!("Implement principal payments aggregation")
    }
}

pub struct AggregatedFlows {
    pub by_period: IndexMap<PeriodId, IndexMap<Currency, f64>>,
    pub total_flows: usize,
}
```

### 6.2 Capital Structure Nodes

```rust
// evaluator/capital_integration.rs

impl Evaluator {
    /// Evaluate capital structure nodes (special handling).
    fn evaluate_capital_node(
        &self,
        node_id: &str,
        capital_structure: &CapitalStructure,
        period: &Period,
    ) -> crate::Result<f64> {
        // Check if this is a capital-structure-derived node
        match node_id {
            id if id.starts_with("cs.interest_expense") => {
                let expense = capital_structure.interest_expense(period)?;
                Ok(expense.amount())
            },
            
            id if id.starts_with("cs.principal_payment") => {
                let payment = capital_structure.principal_payments(period)?;
                Ok(payment.amount())
            },
            
            id if id.starts_with("cs.debt_balance") => {
                // Calculate outstanding debt at period end
                let balance = capital_structure.debt_balance_at_period_end(period)?;
                Ok(balance.amount())
            },
            
            _ => Err(crate::Error::NodeNotFound(node_id.to_string())),
        }
    }
}
```

---

## 7. Extension Plugin Architecture

### 7.1 Extension Trait

```rust
// extensions/plugin.rs

/// Extension trait for pluggable analysis modules.
pub trait Extension: Send + Sync {
    /// Extension identifier (e.g., "corkscrew", "credit_scorecard")
    fn id(&self) -> &str;
    
    /// Extension name (human-readable)
    fn name(&self) -> &str;
    
    /// Validate this extension can run on the given model.
    fn validate(&self, model: &FinancialModel) -> crate::Result<()>;
    
    /// Execute extension analysis.
    fn execute(
        &self,
        model: &FinancialModel,
        results: &mut Results,
    ) -> crate::Result<ExtensionResult>;
}

/// Result from extension execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtensionResult {
    pub extension_id: String,
    pub data: IndexMap<String, serde_json::Value>,
    pub metadata: IndexMap<String, String>,
}

/// Extension registry for managing plugins.
pub struct ExtensionRegistry {
    extensions: HashMap<String, Box<dyn Extension>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            extensions: HashMap::new(),
        }
    }
    
    /// Register an extension.
    pub fn register(&mut self, extension: Box<dyn Extension>) {
        self.extensions.insert(extension.id().to_string(), extension);
    }
    
    /// Get extension by ID.
    pub fn get(&self, id: &str) -> Option<&dyn Extension> {
        self.extensions.get(id).map(|b| b.as_ref())
    }
    
    /// Execute all registered extensions.
    pub fn execute_all(
        &self,
        model: &FinancialModel,
        results: &mut Results,
    ) -> crate::Result<Vec<ExtensionResult>> {
        let mut extension_results = Vec::new();
        
        for (_, extension) in &self.extensions {
            extension.validate(model)?;
            let result = extension.execute(model, results)?;
            extension_results.push(result);
        }
        
        Ok(extension_results)
    }
}
```

### 7.2 Example Extension: Corkscrew Placeholder

```rust
// extensions/corkscrew.rs

/// Corkscrew schedule analysis extension (roll-forward validation).
/// 
/// Future implementation will:
/// - Validate begin/end identities: end[t] = begin[t] + Σ flows[t]
/// - Enforce cross-period continuity: begin[t] = end[t-1]
/// - Generate waterfall reconciliation reports
pub struct CorkscrewExtension;

impl Extension for CorkscrewExtension {
    fn id(&self) -> &str {
        "corkscrew"
    }
    
    fn name(&self) -> &str {
        "Roll-Forward Schedule Validation"
    }
    
    fn validate(&self, model: &FinancialModel) -> crate::Result<()> {
        // Check that required nodes exist
        // Future: Validate CorkscrewSpec structure
        Ok(())
    }
    
    fn execute(
        &self,
        model: &FinancialModel,
        results: &mut Results,
    ) -> crate::Result<ExtensionResult> {
        // Placeholder for future implementation
        Ok(ExtensionResult {
            extension_id: self.id().to_string(),
            data: indexmap::indexmap! {
                "status".into() => serde_json::json!("not_implemented"),
            },
            metadata: indexmap::indexmap! {
                "note".into() => "Corkscrew validation will be implemented in Phase 7".into(),
            },
        })
    }
}
```

### 7.3 Example Extension: Credit Scorecard Placeholder

```rust
// extensions/scorecards.rs

/// Credit scorecard extension for rating-based stress testing.
///
/// Future implementation will:
/// - Calculate credit metrics (leverage, coverage ratios)
/// - Map to credit ratings using configurable scorecards
/// - Run stress scenarios based on rating transitions
pub struct CreditScorecardExtension {
    scorecard_config: ScorecardConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScorecardConfig {
    /// Metric → weight mapping
    pub weights: IndexMap<String, f64>,
    
    /// Rating thresholds
    pub rating_grid: Vec<RatingThreshold>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RatingThreshold {
    pub rating: String,
    pub min_score: f64,
    pub max_score: Option<f64>,
}

impl Extension for CreditScorecardExtension {
    fn id(&self) -> &str {
        "credit_scorecard"
    }
    
    fn name(&self) -> &str {
        "Credit Rating Scorecard Analysis"
    }
    
    fn validate(&self, model: &FinancialModel) -> crate::Result<()> {
        // Validate required metrics exist
        for metric_id in self.scorecard_config.weights.keys() {
            if !model.nodes.contains_key(metric_id) && !model.registry.metrics.contains_key(metric_id) {
                return Err(crate::Error::Build(
                    format!("Scorecard requires node: {}", metric_id)
                ));
            }
        }
        Ok(())
    }
    
    fn execute(
        &self,
        model: &FinancialModel,
        results: &mut Results,
    ) -> crate::Result<ExtensionResult> {
        // Placeholder for future implementation
        Ok(ExtensionResult {
            extension_id: self.id().to_string(),
            data: indexmap::indexmap! {
                "status".into() => serde_json::json!("not_implemented"),
            },
            metadata: indexmap::indexmap! {
                "note".into() => "Credit scorecard will be implemented in Phase 8".into(),
            },
        })
    }
}
```

---

## 8. Implementation Phases

### Phase 1: Foundation (Week 1-2)

**PR #1.1 - Crate Bootstrap**
- Create crate structure and `Cargo.toml`
- Add basic error types
- Wire types: `NodeSpec`, `NodeType`, `AmountOrScalar`
- Type-state builder skeleton
- **Acceptance:** `cargo check` passes, CI green

**PR #1.2 - Period Integration**
- Implement `ModelBuilder::periods()` using core's `build_periods`
- Add `FinancialModelSpec` wire type
- Basic builder tests
- **Acceptance:** Can create model with periods, serialize/deserialize

**PR #1.3 - Value Nodes**
- Implement `ModelBuilder::value()` for explicit values
- Add value precedence resolution
- Unit tests for value storage and retrieval
- **Acceptance:** Can set and evaluate value nodes

---

### Phase 2: DSL Engine (Week 2-3)

**PR #2.1 - DSL Parser**
- Implement `StmtExpr` AST
- Parser for basic arithmetic: `+`, `-`, `*`, `/`
- Node references and literals
- Unit tests for parser
- **Acceptance:** Can parse `"revenue - cogs"`

**PR #2.2 - DSL Compiler**
- Implement `compile()` to convert `StmtExpr` → core `Expr`
- Handle basic binary operations
- Unit tests for compilation
- **Acceptance:** Compiled expressions evaluate correctly

**PR #2.3 - Time-Series Operators**
- Add `lag`, `lead`, `diff`, `pct_change` to parser
- Map to core's `Function` enum
- Integration tests
- **Acceptance:** Can evaluate `"lag(revenue, 1)"` and `"pct_change(revenue, 1)"`

**PR #2.4 - Rolling Window Functions**
- Add `rolling_mean`, `rolling_sum`, `rolling_std`
- Map to core's rolling functions
- Unit tests for window semantics
- **Acceptance:** Can calculate `"rolling_mean(revenue, 4)"`

**PR #2.5 - Statistical Functions**
- Add `mean`, `median`, `std`, `var`
- Map to core's statistical functions
- Unit tests
- **Acceptance:** Can calculate `"std(revenue)"` across periods

**PR #2.6 - Custom Functions**
- Implement `sum()`, `mean()`, `annualize()`, `ttm()`
- Function argument validation
- Integration tests
- **Acceptance:** Can use `"ttm(revenue)"` for trailing twelve months

---

### Phase 3: Evaluator (Week 3-4)

**PR #3.1 - Evaluation Context**
- Implement `StatementContext`
- Implement `ExpressionContext` trait
- Column mapping for node references
- **Acceptance:** Context resolves node references correctly

**PR #3.2 - Basic Evaluator**
- Implement `Evaluator::evaluate()`
- Per-period evaluation loop
- Formula evaluation via core's `CompiledExpr`
- **Acceptance:** Can evaluate simple calculated nodes

**PR #3.3 - DAG Construction**
- Build dependency graph from node formulas
- Topological sort for evaluation order
- Circular dependency detection
- **Acceptance:** Detects cycles, evaluates in correct order

**PR #3.4 - Precedence Resolution**
- Implement Value > Forecast > Formula precedence
- Per-period precedence logic
- Unit tests for each precedence level
- **Acceptance:** Precedence rules enforced correctly

**PR #3.5 - Where Clause Masking**
- Implement where clause evaluation
- Boolean mask application
- Tests for conditional inclusion
- **Acceptance:** Where clause filters periods correctly

---

### Phase 4: Forecasting (Week 4-5)

**PR #4.1 - Forward Fill**
- Implement `ForwardFill` method
- Carry last actual value into forecast periods
- Unit tests
- **Acceptance:** Forward fill extends values correctly

**PR #4.2 - Growth Percentage**
- Implement `GrowthPct` method
- Apply compound growth: `v[t] = v[t-1] * (1 + g)`
- Unit tests with various growth rates
- **Acceptance:** Growth calculations match expected values

**PR #4.3 - Statistical Forecasting (Normal Distribution)**
- Implement `Normal` forecast method
- Use `finstack-core::math::random::SimpleRng` for sampling
- Parameters: `mean`, `std_dev`, `seed`
- Deterministic with seed
- **Example:**
```rust
ForecastSpec {
    method: ForecastMethod::Normal,
    params: indexmap! {
        "mean".into() => json!(100_000.0),
        "std_dev".into() => json!(15_000.0),
        "seed".into() => json!(42),
    },
}
```
- **Acceptance:** Samples from normal distribution deterministically

**PR #4.4 - Log-Normal Forecasting**
- Implement `LogNormal` method
- Use for positive-only values (revenue, prices)
- Unit tests
- **Acceptance:** Log-normal samples are positive

**PR #4.5 - Override Method**
- Implement `Override` with sparse period map
- Allow explicit overrides per period
- Unit tests
- **Acceptance:** Overrides work correctly

---

### Phase 5: Dynamic Registry (Week 5-6)

**PR #5.1 - JSON Schema**
- Define `MetricDefinition` and `MetricRegistry` types
- Create JSON schema documentation
- Validation helpers
- **Acceptance:** JSON schema is well-documented

**PR #5.2 - Registry Loader**
- Implement `Registry::load_from_json()`
- Compile formulas from JSON
- Error handling for invalid formulas
- **Acceptance:** Can load metrics from JSON file

**PR #5.3 - Built-in Metrics JSON**
- Create `fin_basic.json`, `fin_margins.json`, etc.
- Embed in crate using `include_str!`
- Load on `Registry::load_builtins()`
- **Acceptance:** Built-in metrics load correctly

**PR #5.4 - Registry Integration**
- Add registry to `FinancialModel`
- ModelBuilder method: `.with_metrics(path)` and `.with_builtin_metrics()`
- Integration tests
- **Acceptance:** Can add metrics from registry to model

**PR #5.5 - Namespace Management**
- Implement namespace scoping (`fin.*`, custom namespaces)
- Collision detection
- List available metrics per namespace
- **Acceptance:** Namespaces prevent collisions

---

### Phase 6: Capital Structure Integration (Week 6-7)

**PR #6.1 - Instrument Construction**
- Implement `DebtInstrumentSpec` types
- Build instruments from specs using valuations
- Unit tests for each instrument type
- **Acceptance:** Can construct Bond, Swap from spec

**PR #6.2 - Cashflow Aggregation**
- Use `finstack_valuations::cashflow::aggregate_by_period`
- Map cashflow kinds to statement categories
- Integration tests
- **Acceptance:** Cashflows aggregate correctly by period

**PR #6.3 - Interest Expense Calculation**
- Calculate interest expense per period
- Handle fixed and floating coupons
- Unit tests
- **Acceptance:** Interest expense matches instrument schedules

**PR #6.4 - Principal Schedule**
- Calculate principal payments (amortization)
- Track outstanding balance
- Unit tests
- **Acceptance:** Principal schedules match instrument specs

**PR #6.5 - Capital Structure Builder API**
- Implement `ModelBuilder::add_debt()`, `ModelBuilder::add_bond()`
- Fluent API for capital structure
- Integration tests
- **Example:**
```rust
ModelBuilder::new("Acme Corp")
    .periods("2025Q1..2026Q4", Some("2025Q1..Q2"))?
    .add_bond(
        "BOND-001",
        Money::new(10_000_000.0, Currency::USD),
        0.05, // 5% coupon
        Date::from_calendar_date(2025, Month::January, 15).unwrap(),
        Date::from_calendar_date(2030, Month::January, 15).unwrap(),
        "USD-OIS",
    )?
    .compute("interest_expense", "cs.interest_expense.BOND-001")?
    .build()?
```
- **Acceptance:** Can build capital structure fluently

---

### Phase 7: Results & Export (Week 7)

**PR #7.1 - Results Structure**
- Implement `Results` type
- Period-by-node value storage
- Metadata tracking
- **Acceptance:** Results store all evaluated values

**PR #7.2 - Long-Format Export**
- Implement `Results::to_polars_long()`
- Schema: `(node_id, period_id, value)`
- Unit tests
- **Acceptance:** Long format matches expected schema

**PR #7.3 - Wide-Format Export**
- Implement `Results::to_polars_wide()`
- Schema: periods as rows, nodes as columns
- Unit tests
- **Acceptance:** Wide format matches expected schema

**PR #7.4 - Metadata Stamping**
- Include `ResultsMeta` from core
- Track FX policies, rounding context
- Serialize to JSON
- **Acceptance:** Metadata is complete and serializable

---

### Phase 8: Extensions (Week 8+)

**PR #8.1 - Extension Plugin System**
- Finalize `Extension` trait
- Implement `ExtensionRegistry`
- Registration and execution
- **Acceptance:** Can register and execute extensions

**PR #8.2 - Corkscrew Extension (Placeholder)**
- Create skeleton `CorkscrewExtension`
- Validate it loads correctly
- Documentation for future implementation
- **Acceptance:** Extension compiles, no-ops gracefully

**PR #8.3 - Credit Scorecard Extension (Placeholder)**
- Create skeleton `CreditScorecardExtension`
- Define `ScorecardConfig` schema
- Documentation
- **Acceptance:** Extension compiles, validates config

---

## 9. Testing Strategy

### 9.1 Unit Tests

```rust
// tests/builder_tests.rs

#[test]
fn builder_type_state_enforces_periods() {
    let builder = ModelBuilder::new("test");
    // Compiler error: cannot call .value() without .periods() first
    // This demonstrates type-state safety
}

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

// tests/dsl_tests.rs

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
fn parser_handles_function_calls() {
    let formula = "lag(revenue, 1)";
    let ast = parse_formula(formula).unwrap();
    
    match ast {
        StmtExpr::Call { func, args } => {
            assert_eq!(func, "lag");
            assert_eq!(args.len(), 2);
        },
        _ => panic!("Expected function call"),
    }
}

#[test]
fn dsl_compiles_to_core_expr() {
    let formula = "revenue * 1.05";
    let ast = parse_formula(formula).unwrap();
    let expr = compile(&ast).unwrap();
    
    // Verify it compiles to a valid core Expr
    let compiled = CompiledExpr::new(expr);
    // Would need context to actually evaluate
}

// tests/evaluator_tests.rs

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
fn precedence_forecast_over_formula() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None).unwrap()
        .value("base", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(100.0)),
        ])
        .forecast("derived", ForecastSpec {
            method: ForecastMethod::ForwardFill,
            params: indexmap! {},
        })
        .compute("derived", "base * 2").unwrap()
        .build().unwrap();
    
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();
    
    // derived should use forecast, not formula
    // Since there's no value to forward-fill in Q1, it falls back to formula
    // But in Q2, if Q1 had a value, forecast would win
}

// tests/forecast_tests.rs

#[test]
fn normal_distribution_forecast_deterministic() {
    use crate::forecast::statistical::NormalForecast;
    
    let forecast = NormalForecast {
        mean: 100_000.0,
        std_dev: 15_000.0,
        seed: Some(42),
    };
    
    // Sample twice with same seed
    let sample1 = forecast.sample();
    let sample2 = forecast.sample();
    
    // Should be deterministic with seed
    assert_eq!(sample1, sample2);
}

#[test]
fn normal_distribution_respects_parameters() {
    let forecast = NormalForecast {
        mean: 100_000.0,
        std_dev: 15_000.0,
        seed: Some(42),
    };
    
    // Generate many samples
    let samples: Vec<f64> = (0..1000)
        .map(|_| forecast.sample())
        .collect();
    
    let sample_mean = mean(&samples);
    let sample_std = variance(&samples).sqrt();
    
    // Should be approximately correct (within 5% for large sample)
    assert!((sample_mean - 100_000.0).abs() < 5_000.0);
    assert!((sample_std - 15_000.0).abs() < 2_000.0);
}

// tests/capital_structure_tests.rs

#[test]
fn bond_interest_expense_calculated() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 15).unwrap();
    
    let model = ModelBuilder::new("test")
        .periods("2025Q1..2025Q4", None).unwrap()
        .add_bond(
            "BOND-001",
            Money::new(1_000_000.0, Currency::USD),
            0.05, // 5% semiannual
            issue,
            maturity,
            "USD-OIS",
        ).unwrap()
        .compute("interest_expense", "cs.interest_expense.BOND-001").unwrap()
        .build().unwrap();
    
    // Would need market context for actual evaluation
    // This test validates construction only
    assert!(model.capital_structure.is_some());
}

// tests/registry_tests.rs

#[test]
fn load_metrics_from_json() {
    let mut registry = Registry::new();
    
    let json = r#"{
        "namespace": "test",
        "schema_version": 1,
        "metrics": [
            {
                "id": "test_metric",
                "name": "Test Metric",
                "formula": "a + b",
                "description": "Sum of a and b",
                "category": "test",
                "unit_type": "scalar",
                "requires": ["a", "b"]
            }
        ]
    }"#;
    
    let metric_registry: MetricRegistry = serde_json::from_str(json).unwrap();
    registry.load_registry(metric_registry).unwrap();
    
    assert!(registry.get("test.test_metric").is_some());
}

#[test]
fn builtin_metrics_load() {
    let mut registry = Registry::new();
    registry.load_builtins().unwrap();
    
    // Check fin.* namespace is populated
    assert!(registry.get("fin.gross_profit").is_some());
    assert!(registry.get("fin.gross_margin").is_some());
}
```

### 9.2 Integration Tests

```rust
// tests/integration_tests.rs

#[test]
fn complete_pl_model() {
    let model = ModelBuilder::new("Acme Corp")
        .periods("2024Q1..2024Q4", Some("2024Q1..Q2")).unwrap()
        
        // Revenue
        .value("revenue", &[
            (PeriodId::quarter(2024, 1), AmountOrScalar::Scalar(10_000_000.0)),
            (PeriodId::quarter(2024, 2), AmountOrScalar::Scalar(11_000_000.0)),
        ])
        .forecast("revenue", ForecastSpec {
            method: ForecastMethod::GrowthPct,
            params: indexmap! { "rate".into() => json!(0.05) },
        })
        
        // COGS
        .compute("cogs", "revenue * 0.6").unwrap()
        
        // Operating expenses
        .value("operating_expenses", &[
            (PeriodId::quarter(2024, 1), AmountOrScalar::Scalar(2_000_000.0)),
            (PeriodId::quarter(2024, 2), AmountOrScalar::Scalar(2_100_000.0)),
        ])
        .forecast("operating_expenses", ForecastSpec {
            method: ForecastMethod::ForwardFill,
            params: indexmap! {},
        })
        
        // Load standard metrics
        .with_builtin_metrics().unwrap()
        
        .build().unwrap();
    
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();
    
    // Verify results
    assert_eq!(results.nodes.len(), 6); // revenue, cogs, opex, + 3 metrics
    
    // Check Q3 forecast: 11M * 1.05 = 11.55M
    assert!((results.nodes["revenue"][&PeriodId::quarter(2024, 3)] - 11_550_000.0).abs() < 1.0);
}

#[test]
fn capital_structure_model() {
    // Create market context
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (5.0, 0.9)])
        .build().unwrap();
    
    let market_ctx = MarketContext::new()
        .insert_discount(disc_curve);
    
    // Build model with debt
    let model = ModelBuilder::new("test")
        .periods("2025Q1..2025Q4", None).unwrap()
        .add_bond(
            "BOND-001",
            Money::new(10_000_000.0, Currency::USD),
            0.05,
            Date::from_calendar_date(2025, Month::January, 15).unwrap(),
            Date::from_calendar_date(2030, Month::January, 15).unwrap(),
            "USD-OIS",
        ).unwrap()
        .compute("interest_expense", "cs.interest_expense.total").unwrap()
        .build().unwrap();
    
    let mut evaluator = Evaluator::with_market_context(Arc::new(market_ctx));
    let results = evaluator.evaluate(&model, false).unwrap();
    
    // Verify interest expense is calculated
    assert!(results.nodes.contains_key("interest_expense"));
}
```

### 9.3 Golden Tests

```json
// tests/golden/basic_model.json
{
  "id": "basic_pl_model",
  "schema_version": 1,
  "periods": [
    {
      "id": "2025Q1",
      "start": "2025-01-01",
      "end": "2025-04-01",
      "is_actual": true
    },
    {
      "id": "2025Q2",
      "start": "2025-04-01",
      "end": "2025-07-01",
      "is_actual": false
    }
  ],
  "nodes": {
    "revenue": {
      "node_id": "revenue",
      "node_type": "mixed",
      "values": {
        "2025Q1": {
          "scalar": 1000000.0
        }
      },
      "forecasts": [
        {
          "method": "growth_pct",
          "params": {
            "rate": 0.05
          }
        }
      ]
    },
    "cogs": {
      "node_id": "cogs",
      "node_type": "calculated",
      "formula_text": "revenue * 0.6"
    },
    "gross_profit": {
      "node_id": "gross_profit",
      "node_type": "calculated",
      "formula_text": "revenue - cogs"
    }
  }
}
```

---

## 10. Examples

### 10.1 Basic P&L Model

```rust
use finstack_statements::prelude::*;
use finstack_core::prelude::*;

fn main() -> Result<()> {
    // Build simple P&L
    let model = ModelBuilder::new("Acme Corp")
        .periods("2024Q1..2024Q4", Some("2024Q1..Q2"))?
        
        // Actuals (Q1-Q2)
        .value("revenue", &[
            (PeriodId::quarter(2024, 1), AmountOrScalar::Scalar(10_000_000.0)),
            (PeriodId::quarter(2024, 2), AmountOrScalar::Scalar(11_000_000.0)),
        ])
        
        // Forecast (Q3-Q4): 5% growth
        .forecast("revenue", ForecastSpec {
            method: ForecastMethod::GrowthPct,
            params: indexmap! {
                "rate".into() => json!(0.05),
            },
        })
        
        // Calculated nodes
        .compute("cogs", "revenue * 0.6")?
        .compute("gross_profit", "revenue - cogs")?
        .compute("gross_margin", "gross_profit / revenue")?
        
        // Load standard metrics
        .with_builtin_metrics()?
        
        .build()?;
    
    // Evaluate
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;
    
    // Export to DataFrame
    let df_wide = results.to_polars_wide()?;
    println!("{}", df_wide);
    
    Ok(())
}
```

### 10.2 Statistical Forecasting

```rust
use finstack_statements::prelude::*;

fn main() -> Result<()> {
    let model = ModelBuilder::new("Monte Carlo Test")
        .periods("2025Q1..2025Q4", Some("2025Q1"))?
        
        // Q1 actual
        .value("revenue", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(100_000.0)),
        ])
        
        // Q2-Q4: Sample from normal distribution
        .forecast("revenue", ForecastSpec {
            method: ForecastMethod::Normal,
            params: indexmap! {
                "mean".into() => json!(110_000.0),
                "std_dev".into() => json!(15_000.0),
                "seed".into() => json!(42), // Deterministic
            },
        })
        
        .build()?;
    
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;
    
    println!("Revenue forecast (Normal distribution):");
    for period in ["2025Q1", "2025Q2", "2025Q3", "2025Q4"] {
        let pid = PeriodId::from_str(period).unwrap();
        let value = results.nodes["revenue"][&pid];
        println!("  {}: ${:,.2}", period, value);
    }
    
    Ok(())
}
```

### 10.3 Capital Structure Integration

```rust
use finstack_statements::prelude::*;
use finstack_core::prelude::*;
use finstack_valuations::instruments::Bond;

fn main() -> Result<()> {
    // Create market context
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (5.0, 0.9)])
        .build()?;
    
    let market_ctx = MarketContext::new()
        .insert_discount(disc_curve);
    
    // Build model with debt
    let model = ModelBuilder::new("Acme Corp")
        .periods("2025Q1..2026Q4", Some("2025Q1..Q2"))?
        
        // Operating metrics
        .value("ebitda", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(5_000_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::Scalar(5_500_000.0)),
        ])
        .forecast("ebitda", ForecastSpec {
            method: ForecastMethod::GrowthPct,
            params: indexmap! { "rate".into() => json!(0.03) },
        })
        
        // Add debt
        .add_bond(
            "SR-NOTES-2028",
            Money::new(50_000_000.0, Currency::USD),
            0.06, // 6% coupon
            Date::from_calendar_date(2025, Month::January, 15).unwrap(),
            Date::from_calendar_date(2028, Month::January, 15).unwrap(),
            "USD-OIS",
        )?
        
        // Capital structure-derived nodes
        .compute("interest_expense", "cs.interest_expense.SR-NOTES-2028")?
        .compute("debt_balance", "cs.debt_balance.SR-NOTES-2028")?
        
        // Credit metrics
        .compute("debt_to_ebitda", "debt_balance / ttm(ebitda)")?
        
        .build()?;
    
    // Evaluate with market context
    let mut evaluator = Evaluator::with_market_context(Arc::new(market_ctx));
    let results = evaluator.evaluate(&model, false)?;
    
    // Export
    let df = results.to_polars_long()?;
    println!("{}", df.filter(
        col("node_id").eq(lit("debt_to_ebitda"))
    )?);
    
    Ok(())
}
```

### 10.4 Time-Series Analysis

```rust
use finstack_statements::prelude::*;

fn main() -> Result<()> {
    let model = ModelBuilder::new("Time Series Analysis")
        .periods("2024Q1..2025Q4", Some("2024Q1..2024Q4"))?
        
        // Historical revenue (actuals)
        .value("revenue", &[
            (PeriodId::quarter(2024, 1), AmountOrScalar::Scalar(95_000.0)),
            (PeriodId::quarter(2024, 2), AmountOrScalar::Scalar(98_000.0)),
            (PeriodId::quarter(2024, 3), AmountOrScalar::Scalar(102_000.0)),
            (PeriodId::quarter(2024, 4), AmountOrScalar::Scalar(105_000.0)),
        ])
        
        // Forecast using rolling average
        .forecast("revenue", ForecastSpec {
            method: ForecastMethod::TimeSeries,
            params: indexmap! {
                "method".into() => json!("rolling_mean"),
                "window".into() => json!(3),
                "growth_rate".into() => json!(0.02), // Add 2% growth to average
            },
        })
        
        // Analysis metrics
        .compute("revenue_qoq_growth", "pct_change(revenue, 1)")?
        .compute("revenue_yoy_growth", "pct_change(revenue, 4)")?
        .compute("revenue_ma_4q", "rolling_mean(revenue, 4)")?
        .compute("revenue_volatility", "rolling_std(revenue, 4)")?
        
        .build()?;
    
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;
    
    // Export time series analysis
    let df = results.to_polars_wide()?;
    println!("{}", df);
    
    Ok(())
}
```

---

## 11. Cargo.toml Configuration

```toml
[package]
name = "finstack-statements"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
license = "Apache-2.0"
description = "Deterministic financial statements engine for FinStack"
repository = "https://github.com/yourusername/finstack"
keywords = ["finance", "statements", "modeling", "forecasting"]
categories = ["finance", "mathematics"]

[dependencies]
finstack-core = { path = "../core", features = ["serde"] }
finstack-valuations = { path = "../valuations", optional = true }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Deterministic collections
indexmap = { version = "2", features = ["serde"] }
hashbrown = "0.14"

#I'll read the current state of the file and continue from where it left off:

[1 tool called]

I'll continue the document from where it left off:

```markdown:/Users/joneickmeier/projects/rfin/docs/new/statement_plan.md
# Parsing
nom = "7"

# Tracing
tracing = "0.1"

# Error handling
thiserror = "1"

# Optional: Real number generator
# Used for statistical forecasting
rand = { version = "0.8", optional = true }

[dev-dependencies]
time = "0.3"
polars = { version = "0.44", features = ["lazy"] }

[features]
default = ["serde"]
serde = []
capital_structure = ["finstack-valuations"]  # Capital structure needs valuations
deterministic = ["finstack-core/deterministic"]
parallel = ["finstack-core/parallel"]

# Extension features (for future)
extensions_corkscrew = []
extensions_scorecards = []
```

---

## 12. Detailed Implementation Roadmap

### 12.1 Pre-Implementation Checklist

Before starting implementation:

- [ ] Review this plan with team
- [ ] Ensure core expression engine is stable (lag, rolling functions work)
- [ ] Verify valuations cashflow aggregation is available
- [ ] Create `docs/new/04_statements/examples/` directory
- [ ] Set up CI for statements crate

### 12.2 Phase-by-Phase Deliverables

#### Phase 1: Foundation (Weeks 1-2)

**Deliverables:**
- [ ] Crate structure created
- [ ] Wire types implemented and tested
- [ ] Type-state builder compiles
- [ ] Can create empty model with periods
- [ ] Serde round-trip tests pass

**Success Criteria:**
- `cargo test` passes with 0 warnings
- Can serialize/deserialize `FinancialModelSpec`
- Type system prevents invalid builder usage

#### Phase 2: DSL Engine (Weeks 2-3)

**Deliverables:**
- [ ] Parser handles arithmetic expressions
- [ ] Compiler converts StmtExpr → Expr
- [ ] Time-series operators implemented
- [ ] Statistical functions available
- [ ] Custom functions (sum, mean, etc.)

**Success Criteria:**
- Can parse complex formulas: `"ttm(revenue) / lag(total_debt, 1)"`
- Parser error messages are clear and actionable
- 95%+ of Python formula examples translate successfully

#### Phase 3: Evaluator (Weeks 3-4)

**Deliverables:**
- [ ] StatementContext implements ExpressionContext
- [ ] DAG construction from dependencies
- [ ] Topological sort with cycle detection
- [ ] Per-period evaluation loop
- [ ] Precedence resolution (Value > Forecast > Formula)
- [ ] Where clause masking

**Success Criteria:**
- Can evaluate models with 50+ nodes
- Circular dependencies detected and reported
- Where clauses filter correctly
- Evaluation is deterministic (same seed → same result)

#### Phase 4: Forecasting (Weeks 4-5)

**Deliverables:**
- [ ] ForwardFill method
- [ ] GrowthPct method
- [ ] Normal distribution method
- [ ] LogNormal distribution method
- [ ] Override method
- [ ] Forecast parameter validation

**Success Criteria:**
- ForwardFill carries last actual into forecast periods
- GrowthPct applies compound growth correctly
- Statistical methods are deterministic with seed
- Can mix different forecast methods in same model

#### Phase 5: Dynamic Registry (Weeks 5-6)

**Deliverables:**
- [ ] JSON schema for metrics
- [ ] Registry loader from JSON
- [ ] Built-in metrics in `data/metrics/*.json`
- [ ] Namespace scoping (fin.*, custom)
- [ ] Registry validation and error handling

**Success Criteria:**
- Can load metrics from external JSON file
- Built-in metrics load correctly
- Namespace collisions prevented
- Invalid formulas produce clear errors
- Can list available metrics per namespace

#### Phase 6: Capital Structure (Weeks 6-7)

**Deliverables:**
- [ ] DebtInstrumentSpec types
- [ ] Instrument construction from specs
- [ ] Integration with valuations cashflow builder
- [ ] Cashflow aggregation by period
- [ ] Interest expense calculation
- [ ] Principal payment tracking
- [ ] Debt balance calculation

**Success Criteria:**
- Can construct Bond, Swap from spec
- Cashflows aggregate correctly using valuations
- Interest expense matches instrument schedules
- Multi-currency handling works (with FX)
- Can reference capital nodes in formulas: `"cs.interest_expense.BOND-001"`

#### Phase 7: Results & Export (Week 7)

**Deliverables:**
- [ ] Results structure with metadata
- [ ] Long-format DataFrame export
- [ ] Wide-format DataFrame export
- [ ] Metadata stamping (FX policy, rounding, etc.)
- [ ] Export helpers (CSV, JSON)

**Success Criteria:**
- Long format: `(node_id, period_id, value)` schema
- Wide format: periods × nodes table
- Metadata includes all evaluation context
- Exports integrate with Python/Polars workflows

#### Phase 8: Extensions (Week 8+)

**Deliverables:**
- [ ] Extension trait finalized
- [ ] ExtensionRegistry implementation
- [ ] Corkscrew placeholder extension
- [ ] Credit scorecard placeholder extension
- [ ] Extension loading API
- [ ] Documentation for writing extensions

**Success Criteria:**
- Can register custom extensions
- Extensions execute in deterministic order
- Extension results merge into main results
- Documentation includes extension tutorial

---

## 13. Advanced Features & Future Work

### 13.1 Corkscrew Analysis (Phase 9, Future)

**Purpose:** Roll-forward schedule validation and reconciliation.

**Design Sketch:**
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorkscrewSpec {
    /// Node for beginning balance
    pub begin_node: String,
    
    /// Node for ending balance
    pub end_node: String,
    
    /// Flow nodes (with signs)
    pub flows: Vec<FlowLegDef>,
    
    /// Tolerance for floating-point comparison
    #[serde(default)]
    pub tolerance: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlowLegDef {
    pub name: String,
    pub node: String,
    pub sign: i8, // +1 or -1
}

impl CorkscrewExtension {
    fn validate_identity(
        &self,
        spec: &CorkscrewSpec,
        period: &Period,
        period_index: usize,
        results: &Results,
    ) -> crate::Result<()> {
        let begin = if period_index == 0 {
            // First period: use begin_node value
            results.get_value(&spec.begin_node, &period.id)?
        } else {
            // Subsequent periods: begin = prior end
            let prior_period = &results.periods[period_index - 1];
            results.get_value(&spec.end_node, &prior_period.id)?
        };
        
        let flows_sum: f64 = spec.flows.iter()
            .map(|flow| {
                let value = results.get_value(&flow.node, &period.id)?;
                Ok(value * flow.sign as f64)
            })
            .sum::<crate::Result<f64>>()?;
        
        let expected_end = begin + flows_sum;
        let actual_end = results.get_value(&spec.end_node, &period.id)?;
        
        if (expected_end - actual_end).abs() > spec.tolerance {
            return Err(crate::Error::CorkscrewViolation {
                period: period.id,
                begin,
                flows_sum,
                expected_end,
                actual_end,
                difference: expected_end - actual_end,
            });
        }
        
        Ok(())
    }
}
```

**Example Usage:**
```rust
model.add_extension(CorkscrewExtension::new(CorkscrewSpec {
    begin_node: "ppe_begin".into(),
    end_node: "ppe_end".into(),
    flows: vec![
        FlowLegDef { name: "capex".into(), node: "capex".into(), sign: 1 },
        FlowLegDef { name: "depreciation".into(), node: "depreciation".into(), sign: -1 },
        FlowLegDef { name: "disposals".into(), node: "disposals".into(), sign: -1 },
    ],
    tolerance: 1e-6,
}))?;
```

### 13.2 Credit Scorecard Analysis (Phase 10, Future)

**Purpose:** Quantitative credit rating and covenant monitoring.

**Design Sketch:**
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScorecardConfig {
    /// Metrics with weights
    pub factors: Vec<ScorecardFactor>,
    
    /// Rating grid (score → rating)
    pub rating_grid: Vec<RatingBand>,
    
    /// Normalization method
    pub normalization: NormalizationMethod,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScorecardFactor {
    pub metric_id: String,
    pub weight: f64,
    pub direction: Direction, // Higher is Better / Lower is Better
    pub reference_value: Option<f64>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Direction {
    HigherIsBetter,
    LowerIsBetter,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RatingBand {
    pub rating: String,
    pub min_score: f64,
    pub max_score: Option<f64>,
    pub description: String,
}

impl CreditScorecardExtension {
    fn calculate_score(
        &self,
        config: &ScorecardConfig,
        results: &Results,
        period: &Period,
    ) -> crate::Result<f64> {
        let mut total_score = 0.0;
        let mut total_weight = 0.0;
        
        for factor in &config.factors {
            let value = results.get_value(&factor.metric_id, &period.id)?;
            
            // Normalize value
            let normalized = match config.normalization {
                NormalizationMethod::ZScore => {
                    // (value - reference) / std_dev
                    let reference = factor.reference_value.unwrap_or(0.0);
                    value - reference
                },
                NormalizationMethod::MinMax => {
                    // Scale to 0-1
                    value
                },
            };
            
            // Apply direction
            let adjusted = match factor.direction {
                Direction::HigherIsBetter => normalized,
                Direction::LowerIsBetter => -normalized,
            };
            
            total_score += adjusted * factor.weight;
            total_weight += factor.weight;
        }
        
        Ok(total_score / total_weight)
    }
    
    fn map_to_rating(
        &self,
        score: f64,
        rating_grid: &[RatingBand],
    ) -> String {
        for band in rating_grid {
            if score >= band.min_score {
                if let Some(max) = band.max_score {
                    if score < max {
                        return band.rating.clone();
                    }
                } else {
                    return band.rating.clone();
                }
            }
        }
        "NR".to_string() // Not Rated
    }
}
```

**Example Usage:**
```rust
let scorecard = CreditScorecardExtension::new(ScorecardConfig {
    factors: vec![
        ScorecardFactor {
            metric_id: "debt_to_ebitda".into(),
            weight: 0.35,
            direction: Direction::LowerIsBetter,
            reference_value: Some(4.0),
        },
        ScorecardFactor {
            metric_id: "interest_coverage".into(),
            weight: 0.35,
            direction: Direction::HigherIsBetter,
            reference_value: Some(3.0),
        },
        ScorecardFactor {
            metric_id: "fcf_to_debt".into(),
            weight: 0.30,
            direction: Direction::HigherIsBetter,
            reference_value: Some(0.15),
        },
    ],
    rating_grid: vec![
        RatingBand {
            rating: "AAA".into(),
            min_score: 0.8,
            max_score: None,
            description: "Highest quality".into(),
        },
        RatingBand {
            rating: "AA".into(),
            min_score: 0.6,
            max_score: Some(0.8),
            description: "High quality".into(),
        },
        // ... more bands
    ],
    normalization: NormalizationMethod::ZScore,
});

model.add_extension(Box::new(scorecard))?;
```

---

## 14. Performance Considerations

### 14.1 Optimization Strategy

**Evaluation Performance:**
- **DAG caching:** Content-addressed cache for compiled formulas
- **Vectorization:** Use Polars for multi-period operations where possible
- **Lazy evaluation:** Only compute nodes that are required
- **Parallel execution:** Optional parallel period evaluation (deterministic mode)

**Memory Management:**
- **Arc for shared data:** Market context, instruments
- **IndexMap:** Deterministic ordering with reasonable lookup performance
- **Streaming export:** For very large result sets, stream to DataFrame

**Benchmarks to Track:**
- Model compilation time (target: <100ms for 100 nodes)
- Evaluation time (target: <1s for 100 nodes × 24 periods)
- Memory usage (target: <100MB for typical model)
- DataFrame export time (target: <50ms for 2400 rows)

### 14.2 Scalability Targets

| Model Size | Nodes | Periods | Target Eval Time | Target Memory |
|------------|-------|---------|------------------|---------------|
| Small | 50 | 12 | <100ms | <10MB |
| Medium | 200 | 24 | <500ms | <50MB |
| Large | 500 | 48 | <2s | <200MB |
| Enterprise | 1000+ | 60+ | <5s | <500MB |

---

## 15. Documentation Plan

### 15.1 API Documentation

**Required Documentation:**
- [ ] Module-level docs with quick-start example
- [ ] Type documentation for all public types
- [ ] Builder pattern tutorial
- [ ] DSL reference guide
- [ ] Forecast method reference
- [ ] Capital structure integration guide
- [ ] Extension development guide

### 15.2 Examples

**Core Examples:**
1. `examples/rust/basic_pl_statement.rs` - Simple P&L
2. `examples/rust/forecasting_methods.rs` - All forecast types
3. `examples/rust/capital_structure.rs` - Debt tracking
4. `examples/rust/time_series_analysis.rs` - Rolling metrics
5. `examples/rust/custom_metrics.rs` - Dynamic registry
6. `examples/rust/statistical_modeling.rs` - Monte Carlo forecasts

**Python Bindings Examples:**
7. `examples/python/quickstart.py` - 5-minute tutorial
8. `examples/python/load_from_json.py` - JSON model loading
9. `examples/python/custom_metrics.py` - Add custom metrics
10. `examples/python/debt_analysis.py` - Capital structure

### 15.3 Tutorials

**Tutorial Series:**
1. **Getting Started** (10 minutes)
   - Install, create first model, evaluate, export

2. **Building Financial Models** (20 minutes)
   - Value nodes, calculated nodes, forecasts
   - Formula syntax, node references

3. **Advanced Forecasting** (30 minutes)
   - Growth models, statistical sampling
   - Mixing forecast methods

4. **Capital Structure Modeling** (45 minutes)
   - Adding debt instruments
   - Interest expense tracking
   - Leverage calculations

5. **Custom Metrics & Extensions** (30 minutes)
   - JSON metric definitions
   - Loading custom metrics
   - Writing extensions

---

## 16. Migration Path from Python

### 16.1 Python → Rust Mapping

| Python Concept | Rust Equivalent |
|----------------|-----------------|
| `financial_model()` | `ModelBuilder::new()` |
| `.periods("2024Q1..Q4")` | `.periods("2024Q1..Q4", None)?` |
| `.set_value("revenue", {...})` | `.value("revenue", &[(PeriodId, AmountOrScalar)])` |
| `.compute("margin", "a / b")` | `.compute("margin", "a / b")?` |
| `.forecast(ForwardFill)` | `.forecast("node", ForecastSpec { method: ForwardFill, ... })` |
| `.registry.add_metrics([...])` | `.with_builtin_metrics()?` or `.load_metrics(path)?` |
| `.debt.term_loan(...)` | `.add_bond(...)?` |
| `model.run()` | `evaluator.evaluate(&model, false)?` |
| `results.to_frame()` | `results.to_polars_long()?` |

### 16.2 Formula Translation

**Python Formula → Rust DSL:**

```python
# Python
"revenue - cogs"
"gross_profit / revenue"
"lag(revenue, 1)"
"revenue.rolling(4).mean()"
"ebitda * 4"  # Annualize quarterly
```

```rust
// Rust
"revenue - cogs"
"gross_profit / revenue"
"lag(revenue, 1)"
"rolling_mean(revenue, 4)"
"annualize(ebitda, 4)"
```

**Key Differences:**
- Python uses method chaining: `revenue.rolling(4).mean()`
- Rust uses function calls: `rolling_mean(revenue, 4)`
- Python infers annualization: `ebitda * 4`
- Rust is explicit: `annualize(ebitda, 4)`

---

## 17. Risk Mitigation

### 17.1 Technical Risks

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| DSL parser complexity | High | Medium | Start simple, iterate; use nom for robust parsing |
| Expression engine limitations | High | Low | Verify core expr supports all needed functions first |
| Capital structure integration | Medium | Medium | Prototype integration in Phase 0; validate with valuations team |
| Performance regression | Medium | Low | Benchmark early and often; optimize hot paths |
| Registry JSON schema drift | Medium | Medium | Version JSON schema; validate on load |

### 17.2 Dependency Risks

| Dependency | Risk | Mitigation |
|------------|------|------------|
| finstack-core | API changes | Pin version, coordinate releases |
| finstack-valuations | Breaking changes | Use optional feature, isolate integration |
| Polars | Version compatibility | Use core's re-export, test integration |

---

## 18. Testing Checklist

### 18.1 Functional Tests

- [ ] Can create model with periods
- [ ] Value nodes store correctly
- [ ] Calculated nodes evaluate
- [ ] Precedence enforced (Value > Forecast > Formula)
- [ ] Where clauses mask correctly
- [ ] Forecasts extend into future periods
- [ ] Statistical forecasts are deterministic
- [ ] Capital structure instruments construct
- [ ] Interest expense calculates correctly
- [ ] Dynamic registry loads from JSON
- [ ] Extensions execute correctly
- [ ] Results export to DataFrame

### 18.2 Property Tests

- [ ] Determinism: Same seed → same results
- [ ] Idempotence: Evaluate twice → same results
- [ ] Commutativity: Node order doesn't affect results (for independent nodes)
- [ ] Currency safety: Cross-currency ops fail without FX
- [ ] Serialization: `deserialize(serialize(model)) == model`

### 18.3 Performance Tests

- [ ] 100 nodes × 24 periods < 1s
- [ ] 1000 nodes × 60 periods < 10s
- [ ] Memory usage < 500MB for large models
- [ ] DataFrame export < 100ms

### 18.4 Integration Tests

- [ ] Valuations integration: Cashflows aggregate correctly
- [ ] Core integration: Expression engine evaluates correctly
- [ ] Polars integration: DataFrame exports work
- [ ] Multi-currency: FX conversions work

---

## 19. Code Quality Standards

### 19.1 Code Style

- **Formatting:** `cargo fmt` (enforced in CI)
- **Linting:** `cargo clippy -- -D warnings`
- **Documentation:** 100% coverage for public APIs
- **Tests:** 90%+ code coverage
- **Examples:** Every public API has runnable example

### 19.2 Error Handling

- **Principle:** Errors should be actionable
- **Context:** Include node_id, period, formula in errors
- **Types:** Use specific error variants, not strings
- **Recovery:** Provide suggestions in error messages

**Good Error Example:**
```
Error: Formula parse error in node 'gross_margin'
Formula: "gross_profit / revenu"  // Note typo
                          ^^^^^^
Error: Unknown node reference: 'revenu'
Hint: Did you mean 'revenue'?
Available nodes: revenue, cogs, gross_profit
```

### 19.3 API Stability

- **Semantic versioning:** Follow semver strictly
- **Deprecation policy:** Deprecate for 2 minor versions before removal
- **Wire format:** JSON schema versioning, forward compatibility
- **Breaking changes:** Document in CHANGELOG, provide migration guide

---

## 20. Release Criteria

### 20.1 MVP Release (v0.1.0)

**Must Have:**
- [ ] Core wire types stable
- [ ] Builder pattern works
- [ ] Basic DSL (arithmetic, node references)
- [ ] Value/Calculate/Mixed node types
- [ ] Forward fill and growth forecasts
- [ ] Simple evaluator (no capital structure yet)
- [ ] DataFrame export (long/wide)
- [ ] 50+ passing tests
- [ ] Documentation with examples
- [ ] CHANGELOG started

**Nice to Have:**
- [ ] Statistical forecasting
- [ ] Time-series operators (lag, rolling)
- [ ] Built-in metrics registry

### 20.2 Production Release (v1.0.0)

**Must Have:**
- [ ] All MVP features
- [ ] Complete DSL with all operators
- [ ] Statistical forecasting (Normal, LogNormal)
- [ ] Dynamic metric registry (JSON)
- [ ] Capital structure integration
- [ ] Extension plugin system
- [ ] Python bindings
- [ ] WASM bindings
- [ ] 200+ passing tests
- [ ] Complete documentation
- [ ] Performance benchmarks met

**Nice to Have:**
- [ ] Corkscrew extension
- [ ] Credit scorecard extension
- [ ] Additional forecast methods
- [ ] Real-time formula validation

---

## 21. Dependencies & Cargo Features

### 21.1 Feature Flags

```toml
[features]
default = ["serde"]

# Core features
serde = []

# Capital structure integration
capital_structure = ["dep:finstack-valuations"]

# Performance features
deterministic = ["finstack-core/deterministic"]
parallel = ["finstack-core/parallel"]

# Statistical forecasting
stats = ["dep:rand"]

# Extensions (for future)
ext_corkscrew = []
ext_scorecards = []
ext_real_estate = ["finstack-valuations"]

# All features
full = [
    "capital_structure",
    "stats",
    "ext_corkscrew",
    "ext_scorecards",
]
```

### 21.2 Dependency Matrix

| Crate | Version | Features | Required | Purpose |
|-------|---------|----------|----------|---------|
| finstack-core | 0.2+ | serde | Yes | Period system, expr engine, Money |
| finstack-valuations | 0.2+ | - | Optional | Instruments, cashflow aggregation |
| serde | 1 | derive | Yes | Serialization |
| serde_json | 1 | - | Yes | JSON parsing |
| indexmap | 2 | serde | Yes | Deterministic maps |
| hashbrown | 0.14 | - | Yes | Fast hash maps |
| nom | 7 | - | Yes | Parser combinators |
| thiserror | 1 | - | Yes | Error derive |
| tracing | 0.1 | - | Yes | Observability |
| rand | 0.8 | - | Optional | Statistical forecasting |

---

## 22. Observability & Debugging

### 22.1 Tracing Integration

```rust
// evaluator/evaluator.rs

use tracing::{debug, error, info, instrument, span, warn, Level};

impl Evaluator {
    #[instrument(skip(self, model), fields(model_id = %model.id))]
    pub fn evaluate(
        &mut self,
        model: &FinancialModel,
        parallel: bool,
    ) -> crate::Result<Results> {
        let span = span!(Level::INFO, "evaluate", nodes = model.nodes.len(), periods = model.periods.len());
        let _enter = span.enter();
        
        info!("Starting evaluation");
        
        // Build DAG
        let dag_span = span!(Level::DEBUG, "build_dag");
        let _dag_enter = dag_span.enter();
        let dag = self.build_dag(model)?;
        debug!("DAG built with {} nodes", dag.nodes.len());
        drop(_dag_enter);
        
        // Evaluate periods
        let mut results = IndexMap::new();
        for (i, period) in model.periods.iter().enumerate() {
            let period_span = span!(
                Level::DEBUG,
                "evaluate_period",
                period_id = %period.id,
                period_index = i,
            );
            let _period_enter = period_span.enter();
            
            let period_results = self.evaluate_period(model, period, &dag, &results)?;
            debug!("Period {} evaluated: {} nodes", period.id, period_results.len());
            
            // Merge results
            for (node_id, value) in period_results {
                results.entry(node_id)
                    .or_insert_with(IndexMap::new)
                    .insert(period.id, value);
            }
        }
        
        info!("Evaluation complete");
        
        Ok(Results {
            nodes: results,
            periods: model.periods.clone(),
            meta: results_meta(&FinstackConfig::default()),
        })
    }
}
```

### 22.2 Debug Output

```rust
impl FinancialModel {
    /// Print model structure for debugging.
    pub fn debug_structure(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("Model: {}\n", self.id));
        output.push_str(&format!("Periods: {} ({} to {})\n",
            self.periods.len(),
            self.periods.first().map(|p| p.id.to_string()).unwrap_or_default(),
            self.periods.last().map(|p| p.id.to_string()).unwrap_or_default(),
        ));
        output.push_str(&format!("Nodes: {}\n", self.nodes.len()));
        
        for (node_id, node) in &self.nodes {
            output.push_str(&format!("  - {} ({:?})", node_id, node.spec.node_type));
            if node.formula.is_some() {
                output.push_str(" [has formula]");
            }
            if !node.spec.forecasts.is_empty() {
                output.push_str(&format!(" [forecasts: {}]", node.spec.forecasts.len()));
            }
            output.push('\n');
        }
        
        if let Some(ref capital) = self.capital_structure {
            output.push_str(&format!("\nCapital Structure:\n"));
            output.push_str(&format!("  Debt instruments: {}\n", capital.instruments.len()));
        }
        
        if !self.extensions.is_empty() {
            output.push_str(&format!("\nExtensions: {}\n", self.extensions.len()));
            for ext in &self.extensions {
                output.push_str(&format!("  - {} ({})\n", ext.name(), ext.id()));
            }
        }
        
        output
    }
}
```

---

## 23. Python Bindings Plan

### 23.1 PyO3 Integration

```rust
// finstack-py/src/statements/mod.rs

use pyo3::prelude::*;
use finstack_statements as stmt;

#[pyclass]
pub struct PyModelBuilder {
    inner: Option<stmt::ModelBuilder<stmt::Ready>>,
}

#[pymethods]
impl PyModelBuilder {
    #[new]
    pub fn new(id: String) -> Self {
        Self {
            inner: Some(stmt::ModelBuilder::new(id)),
        }
    }
    
    pub fn periods(
        &mut self,
        range: String,
        actuals: Option<String>,
    ) -> PyResult<()> {
        let builder = self.inner.take()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Builder already consumed"))?;
        
        let ready = builder.periods(&range, actuals.as_deref())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        
        self.inner = Some(ready);
        Ok(())
    }
    
    pub fn value(
        &mut self,
        node_id: String,
        values: HashMap<String, f64>,
    ) -> PyResult<()> {
        // Convert Python dict to Rust types
        let values_vec: Vec<(PeriodId, AmountOrScalar)> = values.iter()
            .map(|(pid_str, &val)| {
                let pid = PeriodId::from_str(pid_str).unwrap();
                (pid, AmountOrScalar::Scalar(val))
            })
            .collect();
        
        let builder = self.inner.take().unwrap();
        let updated = builder.value(node_id, &values_vec);
        self.inner = Some(updated);
        
        Ok(())
    }
    
    pub fn compute(
        &mut self,
        node_id: String,
        formula: String,
    ) -> PyResult<()> {
        let builder = self.inner.take().unwrap();
        let updated = builder.compute(node_id, formula, None::<String>)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        self.inner = Some(updated);
        
        Ok(())
    }
    
    pub fn build(&mut self) -> PyResult<PyFinancialModel> {
        let builder = self.inner.take()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Builder already consumed"))?;
        
        let model = builder.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        
        Ok(PyFinancialModel { inner: model })
    }
}

#[pyclass]
pub struct PyFinancialModel {
    inner: stmt::FinancialModel,
}

#[pyclass]
pub struct PyEvaluator {
    inner: stmt::Evaluator,
}

#[pymethods]
impl PyEvaluator {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: stmt::Evaluator::new(),
        }
    }
    
    pub fn evaluate(
        &mut self,
        model: &PyFinancialModel,
        parallel: bool,
    ) -> PyResult<PyResults> {
        let results = self.inner.evaluate(&model.inner, parallel)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        Ok(PyResults { inner: results })
    }
}

#[pyclass]
pub struct PyResults {
    inner: stmt::Results,
}

#[pymethods]
impl PyResults {
    pub fn to_polars(&self, format: String) -> PyResult<PyObject> {
        let df = match format.as_str() {
            "long" => self.inner.to_polars_long(),
            "wide" => self.inner.to_polars_wide(),
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "format must be 'long' or 'wide'"
            )),
        }.map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        // Convert Polars DataFrame to Python
        Python::with_gil(|py| {
            // Use polars Python API
            todo!("Convert Rust DataFrame to Python")
        })
    }
}
```

### 23.2 Python API Example

```python
# Python usage
from finstack.statements import ModelBuilder, Evaluator

# Build model
builder = ModelBuilder("Acme Corp")
builder.periods("2024Q1..2024Q4", actuals="2024Q1..Q2")

builder.value("revenue", {
    "2024Q1": 10_000_000,
    "2024Q2": 11_000_000,
})

builder.forecast("revenue", {
    "method": "growth_pct",
    "params": {"rate": 0.05}
})

builder.compute("cogs", "revenue * 0.6")
builder.compute("gross_profit", "revenue - cogs")

model = builder.build()

# Evaluate
evaluator = Evaluator()
results = evaluator.evaluate(model, parallel=False)

# Export
df = results.to_polars("wide")
print(df)
```

---

## 24. WASM Bindings Plan

### 24.1 WASM API Design

```rust
// finstack-wasm/src/statements.rs

use wasm_bindgen::prelude::*;
use finstack_statements as stmt;

#[wasm_bindgen]
pub struct WasmModelBuilder {
    inner: stmt::ModelBuilder<stmt::Ready>,
}

#[wasm_bindgen]
impl WasmModelBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(id: String) -> Result<WasmModelBuilder, JsValue> {
        let builder = stmt::ModelBuilder::new(id);
        // Builder starts in NeedPeriods state, so we can't return yet
        Err(JsValue::from_str("Must call periods() first"))
    }
    
    #[wasm_bindgen(js_name = createWithPeriods)]
    pub fn create_with_periods(
        id: String,
        range: String,
        actuals: Option<String>,
    ) -> Result<WasmModelBuilder, JsValue> {
        let builder = stmt::ModelBuilder::new(id);
        let ready = builder.periods(&range, actuals.as_deref())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        Ok(WasmModelBuilder { inner: ready })
    }
    
    #[wasm_bindgen(js_name = addValue)]
    pub fn add_value(
        mut self,
        node_id: String,
        values: JsValue,
    ) -> Result<WasmModelBuilder, JsValue> {
        // Deserialize values from JS object
        let values_map: HashMap<String, f64> = serde_wasm_bindgen::from_value(values)?;
        
        let values_vec: Vec<(PeriodId, AmountOrScalar)> = values_map.iter()
            .map(|(pid_str, &val)| {
                let pid = PeriodId::from_str(pid_str).unwrap();
                (pid, AmountOrScalar::Scalar(val))
            })
            .collect();
        
        self.inner = self.inner.value(node_id, &values_vec);
        Ok(self)
    }
    
    #[wasm_bindgen(js_name = addCompute)]
    pub fn add_compute(
        mut self,
        node_id: String,
        formula: String,
    ) -> Result<WasmModelBuilder, JsValue> {
        self.inner = self.inner.compute(node_id, formula, None::<String>)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(self)
    }
    
    #[wasm_bindgen]
    pub fn build(self) -> Result<WasmFinancialModel, JsValue> {
        let model = self.inner.build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        Ok(WasmFinancialModel { inner: model })
    }
}

#[wasm_bindgen]
pub struct WasmFinancialModel {
    inner: stmt::FinancialModel,
}

#[wasm_bindgen]
pub struct WasmEvaluator {
    inner: stmt::Evaluator,
}

#[wasm_bindgen]
impl WasmEvaluator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: stmt::Evaluator::new(),
        }
    }
    
    #[wasm_bindgen]
    pub fn evaluate(
        &mut self,
        model: &WasmFinancialModel,
    ) -> Result<WasmResults, JsValue> {
        let results = self.inner.evaluate(&model.inner, false)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        Ok(WasmResults { inner: results })
    }
}

#[wasm_bindgen]
pub struct WasmResults {
    inner: stmt::Results,
}

#[wasm_bindgen]
impl WasmResults {
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    
    #[wasm_bindgen(js_name = getNode)]
    pub fn get_node(&self, node_id: String) -> Result<JsValue, JsValue> {
        let node_data = self.inner.nodes.get(&node_id)
            .ok_or_else(|| JsValue::from_str("Node not found"))?;
        
        serde_wasm_bindgen::to_value(node_data)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
```

### 24.2 JavaScript/TypeScript Usage

```typescript
// TypeScript example
import {
  WasmModelBuilder,
  WasmEvaluator
} from 'finstack-wasm';

// Build model
const model = WasmModelBuilder
  .createWithPeriods("Acme Corp", "2024Q1..2024Q4", "2024Q1..Q2")
  .addValue("revenue", {
    "2024Q1": 10_000_000,
    "2024Q2": 11_000_000,
  })
  .addCompute("cogs", "revenue * 0.6")
  .addCompute("gross_profit", "revenue - cogs")
  .build();

// Evaluate
const evaluator = new WasmEvaluator();
const results = evaluator.evaluate(model);

// Export
const json = results.toJson();
console.log(JSON.parse(json));

// Get specific node
const revenue = results.getNode("revenue");
console.log(revenue);
```

---

## 25. Validation & Quality Gates

### 25.1 Pre-Merge Checklist

Every PR must pass:

- [ ] `cargo fmt --check` (formatting)
- [ ] `cargo clippy -- -D warnings` (linting)
- [ ] `cargo test --all-features` (tests)
- [ ] `cargo doc --no-deps` (documentation)
- [ ] Code coverage ≥ 90% for new code
- [ ] Integration test added if applicable
- [ ] Example added if new public API
- [ ] CHANGELOG updated

### 25.2 Release Checklist

Before releasing a version:

- [ ] All tests pass on CI
- [ ] Benchmarks within targets
- [ ] Documentation complete
- [ ] Examples run successfully
- [ ] Python bindings tested
- [ ] WASM bindings tested
- [ ] CHANGELOG reviewed
- [ ] Version bumped in `Cargo.toml`
- [ ] Git tag created
- [ ] crates.io publish (if public)

---

## 26. Open Questions & Decisions Needed

### 26.1 Design Decisions

1. **Formula Syntax:**
   - Q: Use `revenue[t-1]` or `lag(revenue, 1)` for time-series?
   - Recommendation: `lag(revenue, 1)` - more explicit, easier to parse

2. **Multi-Currency Handling:**
   - Q: Auto-convert to model currency or require explicit conversion?
   - Recommendation: Require explicit conversion, stamp FX policy in metadata

3. **Extension Loading:**
   - Q: Dynamic loading (.so/.dll) or compile-time only?
   - Recommendation: Compile-time for MVP, dynamic in v2.0

4. **Parallel Evaluation:**
   - Q: Parallel across periods or across nodes?
   - Recommendation: Across periods (easier to ensure determinism)

### 26.2 Integration Questions

1. **Valuations Coordination:**
   - Q: Which instrument types to support initially?
   - Recommendation: Bond, IRS, Loan (common debt instruments)

2. **Registry Schema Evolution:**
   - Q: How to handle schema v1 → v2 migration?
   - Recommendation: Schema version in JSON, migration helpers

3. **Extension API Stability:**
   - Q: Lock extension trait in v1.0 or mark unstable?
   - Recommendation: Mark unstable until v1.2, allow iteration

---

## 27. Success Metrics

### 27.1 Quantitative Metrics

**Performance:**
- Evaluation time: <1s for 100 nodes × 24 periods
- Memory usage: <100MB for typical model
- Parse time: <10ms for 50-line formula
- Export time: <50ms for 2400-row DataFrame

**Quality:**
- Test coverage: ≥90%
- Documentation coverage: 100% public APIs
- Example coverage: Every public type has example
- Error coverage: All error variants tested

**Adoption (Internal):**
- 3+ production models using statements
- Python bindings functional parity
- WASM demo deployed

### 27.2 Qualitative Metrics

**Developer Experience:**
- New team member can build first model in <30 minutes
- Formula errors are clear and actionable
- IDE autocomplete works for all public APIs
- Examples cover 80% of use cases

**User Experience:**
- Analysts can build models without Rust knowledge (Python)
- Financial engineers can extend with custom metrics (JSON)
- Quants can add statistical methods (Extension trait)

---

## 28. Communication Plan

### 28.1 Stakeholder Updates

**Weekly Updates:**
- Progress on current phase
- Blockers and risks
- Upcoming decisions needed

**Phase Completion:**
- Demo of new functionality
- Request for feedback
- Plan for next phase

### 28.2 Documentation Updates

**During Implementation:**
- Update TDD with implementation notes
- Add design decisions to ADR (Architecture Decision Records)
- Keep examples in sync with API changes

**At Release:**
- Publish migration guide (if breaking changes)
- Update main README
- Announce in changelog

---

## 29. Appendix A: Full Type Hierarchy

```
FinancialModel (runtime)
├── id: String
├── periods: Vec<Period>
├── nodes: IndexMap<String, Node>
│   └── Node
│       ├── spec: NodeSpec (wire)
│       ├── formula: Option<CompiledExpr>
│       ├── where_clause: Option<CompiledExpr>
│       └── dependencies: Vec<String>
├── registry: Registry
│   └── metrics: IndexMap<String, CompiledMetric>
│       └── CompiledMetric
│           ├── definition: MetricDefinition (from JSON)
│           └── compiled_expr: CompiledExpr
├── capital_structure: Option<CapitalStructure>
│   └── CapitalStructure
│       ├── instruments: Vec<CapitalInstrument>
│       ├── market_context: Arc<MarketContext>
│       └── aggregation_config: AggregationConfig
└── extensions: Vec<Box<dyn Extension>>
    └── Extension (trait)
        ├── id() -> &str
        ├── validate() -> Result<()>
        └── execute() -> Result<ExtensionResult>

Results
├── nodes: IndexMap<String, IndexMap<PeriodId, f64>>
├── periods: Vec<Period>
└── meta: ResultsMeta
    ├── numeric_mode: NumericMode
    ├── rounding: RoundingContext
    └── fx_policy_applied: Option<String>
```

---

## 30. Appendix B: DSL Function Reference

| Function | Signature | Description | Example |
|----------|-----------|-------------|---------|
| `lag` | `lag(node, n)` | Value from n periods ago | `lag(revenue, 1)` |
| `lead` | `lead(node, n)` | Value from n periods ahead | `lead(revenue, 1)` |
| `diff` | `diff(node, n)` | First difference | `diff(revenue, 1)` |
| `pct_change` | `pct_change(node, n)` | Percentage change | `pct_change(revenue, 1)` |
| `rolling_mean` | `rolling_mean(node, w)` | Rolling average | `rolling_mean(revenue, 4)` |
| `rolling_sum` | `rolling_sum(node, w)` | Rolling sum | `rolling_sum(revenue, 4)` |
| `rolling_std` | `rolling_std(node, w)` | Rolling std dev | `rolling_std(revenue, 4)` |
| `cumsum` | `cumsum(node)` | Cumulative sum | `cumsum(revenue)` |
| `cumprod` | `cumprod(node)` | Cumulative product | `cumprod(growth_factor)` |
| `mean` | `mean(node)` | Period average | `mean(revenue)` |
| `median` | `median(node)` | Period median | `median(revenue)` |
| `std` | `std(node)` | Standard deviation | `std(revenue)` |
| `var` | `var(node)` | Variance | `var(revenue)` |
| `sum` | `sum(n1, n2, ...)` | Sum across nodes | `sum(a, b, c)` |
| `min` | `min(n1, n2, ...)` | Minimum value | `min(a, b)` |
| `max` | `max(n1, n2, ...)` | Maximum value | `max(a, b)` |
| `if` | `if(cond, t, f)` | Conditional | `if(revenue > 0, 1, 0)` |
| `coalesce` | `coalesce(n, def)` | Null coalescing | `coalesce(revenue, 0)` |
| `annualize` | `annualize(n, freq)` | Annualize value | `annualize(ebitda, 4)` |
| `ttm` | `ttm(node)` | Trailing 12 months | `ttm(revenue)` |

---

## 31. Appendix C: JSON Metric Schema

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "FinStack Metric Registry",
  "type": "object",
  "required": ["namespace", "schema_version", "metrics"],
  "properties": {
    "namespace": {
      "type": "string",
      "pattern": "^[a-z_][a-z0-9_]*$",
      "description": "Namespace for these metrics (e.g., 'fin', 'custom')"
    },
    "schema_version": {
      "type": "integer",
      "minimum": 1,
      "description": "Schema version for forward compatibility"
    },
    "metrics": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["id", "name", "formula", "description", "category", "unit_type"],
        "properties": {
          "id": {
            "type": "string",
            "pattern": "^[a-z_][a-z0-9_]*$",
            "description": "Metric identifier within namespace"
          },
          "name": {
            "type": "string",
            "description": "Human-readable name"
          },
          "formula": {
            "type": "string",
            "description": "Formula text in statements DSL"
          },
          "description": {
            "type": "string",
            "description": "Detailed description of what this metric calculates"
          },
          "category": {
            "type": "string",
            "enum": ["profitability", "margins", "returns", "leverage", "efficiency", "custom"],
            "description": "Metric category for grouping"
          },
          "unit_type": {
            "type": "string",
            "enum": ["currency", "percentage", "ratio", "count", "scalar"],
            "description": "Output unit type"
          },
          "requires": {
            "type": "array",
            "items": {
              "type": "string"
            },
            "description": "List of required nodes/metrics for this metric to work"
          },
          "tags": {
            "type": "array",
            "items": {
              "type": "string"
            },
            "description": "Optional tags for filtering and search"
          }
        }
      }
    }
  }
}
```

---

## 32. Appendix D: Error Reference

### Complete Error Catalog

```rust
#[derive(Debug, Error)]
pub enum Error {
    // Build errors
    #[error("Build error: {0}")]
    Build(String),
    
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    
    #[error("Duplicate node ID: {0}")]
    DuplicateNode(String),
    
    #[error("Invalid period specification: {0}")]
    InvalidPeriods(String),
    
    // Formula errors
    #[error("Formula parse error in node '{node_id}': {message}\nFormula: {formula}")]
    FormulaParse {
        node_id: String,
        formula: String,
        message: String,
    },
    
    #[error("Formula compilation error: {0}")]
    FormulaCompile(String),
    
    #[error("Unknown function: {0}")]
    UnknownFunction(String),
    
    // Evaluation errors
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),
    
    #[error("Missing dependency: node '{node_id}' references '{dependency}' which doesn't exist")]
    MissingDependency {
        node_id: String,
        dependency: String,
    },
    
    #[error("Division by zero in node '{node_id}' for period {period}")]
    DivisionByZero {
        node_id: String,
        period: String,
    },
    
    #[error("Currency mismatch in node '{node_id}': expected {expected}, got {actual}")]
    CurrencyMismatch {
        node_id: String,
        expected: Currency,
        actual: Currency,
    },
    
    // Forecast errors
    #[error("Forecast error in node '{node_id}': {message}")]
    Forecast {
        node_id: String,
        message: String,
    },
    
    #[error("Missing forecast parameter '{param}' for method {method:?}")]
    MissingForecastParam {
        method: ForecastMethod,
        param: String,
    },
    
    // Capital structure errors
    #[error("Capital structure error: {0}")]
    CapitalStructure(String),
    
    #[error("Instrument construction failed for '{id}': {message}")]
    InstrumentConstruction {
        id: String,
        message: String,
    },
    
    // Registry errors
    #[error("Registry error: {0}")]
    Registry(String),
    
    #[error("Metric not found: {0}")]
    MetricNotFound(String),
    
    #[error("Invalid metric definition in '{namespace}.{metric_id}': {message}")]
    InvalidMetric {
        namespace: String,
        metric_id: String,
        message: String,
    },
    
    // Extension errors
    #[error("Extension '{extension_id}' validation failed: {message}")]
    ExtensionValidation {
        extension_id: String,
        message: String,
    },
    
    #[error("Extension '{extension_id}' execution failed: {message}")]
    ExtensionExecution {
        extension_id: String,
        message: String,
    },
    
    // Core errors (wrapped)
    #[error("Core error: {0}")]
    Core(#[from] finstack_core::Error),
    
    // Valuations errors (wrapped, optional)
    #[cfg(feature = "capital_structure")]
    #[error("Valuations error: {0}")]
    Valuations(String),
}
```

---

## 33. Appendix E: Complete Forecast Examples

### E.1 Normal Distribution Forecast

```rust
// Implementation in forecast/statistical.rs

use finstack_core::math::random::{RandomNumberGenerator, SimpleRng};

pub struct NormalForecast {
    pub mean: f64,
    pub std_dev: f64,
    pub seed: Option<u64>,
}

impl NormalForecast {
    pub fn apply(
        &self,
        node_id: &str,
        period: &Period,
        period_index: usize,
    ) -> crate::Result<f64> {
        // Create deterministic RNG with seed
        let seed = self.seed.unwrap_or_else(|| {
            // Hash period and node for deterministic unseeded behavior
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            node_id.hash(&mut hasher);
            period.id.hash(&mut hasher);
            hasher.finish()
        });
        
        let mut rng = SimpleRng::new(seed + period_index as u64);
        let sample = rng.normal(self.mean, self.std_dev);
        
        Ok(sample)
    }
}

// Usage in model
let forecast_spec = ForecastSpec {
    method: ForecastMethod::Normal,
    params: indexmap! {
        "mean".into() => json!(100_000.0),
        "std_dev".into() => json!(15_000.0),
        "seed".into() => json!(42),
    },
};
```

### E.2 Log-Normal Distribution Forecast

```rust
pub struct LogNormalForecast {
    pub mean: f64,
    pub std_dev: f64,
    pub seed: Option<u64>,
}

impl LogNormalForecast {
    pub fn apply(
        &self,
        node_id: &str,
        period: &Period,
        period_index: usize,
    ) -> crate::Result<f64> {
        let seed = self.seed.unwrap_or_else(|| hash_for_seed(node_id, &period.id));
        let mut rng = SimpleRng::new(seed + period_index as u64);
        
        // Sample from normal, then exp() to get log-normal
        let normal_sample = rng.normal(self.mean.ln(), self.std_dev);
        let lognormal_sample = normal_sample.exp();
        
        Ok(lognormal_sample)
    }
}
```

### E.3 Mixed Forecast Strategy

```rust
// Combine multiple forecast methods with fallback
let model = ModelBuilder::new("Mixed Forecast")
    .periods("2024Q1..2025Q4", Some("2024Q1..2024Q4"))?
    
    // Primary forecast: Normal distribution
    .forecast("revenue", ForecastSpec {
        method: ForecastMethod::Normal,
        params: indexmap! {
            "mean".into() => json!(110_000.0),
            "std_dev".into() => json!(15_000.0),
            "seed".into() => json!(42),
        },
    })
    
    // Secondary forecast: Forward fill (fallback if normal fails)
    .forecast("revenue", ForecastSpec {
        method: ForecastMethod::ForwardFill,
        params: indexmap! {},
    })
    
    .build()?;

// Evaluator tries forecasts in order until one succeeds
```

---

## 34. Appendix F: Capital Structure Examples

### F.1 Multi-Instrument Model

```rust
use finstack_statements::prelude::*;
use finstack_core::prelude::*;
use time::Month;

fn build_capital_structure_model() -> Result<FinancialModel> {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    ModelBuilder::new("Multi-Debt Company")
        .periods("2025Q1..2027Q4", Some("2025Q1..2025Q2"))?
        
        // Operating metrics
        .value("ebitda", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(10_000_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::Scalar(10_500_000.0)),
        ])
        .forecast("ebitda", ForecastSpec {
            method: ForecastMethod::Normal,
            params: indexmap! {
                "mean".into() => json!(11_000_000.0),
                "std_dev".into() => json!(1_000_000.0),
                "seed".into() => json!(123),
            },
        })
        
        // Add multiple debt instruments
        .add_bond(
            "SR-NOTES-2030",
            Money::new(100_000_000.0, Currency::USD),
            0.05, // 5% coupon
            Date::from_calendar_date(2025, Month::January, 15).unwrap(),
            Date::from_calendar_date(2030, Month::January, 15).unwrap(),
            "USD-OIS",
        )?
        
        .add_bond(
            "SUB-NOTES-2032",
            Money::new(50_000_000.0, Currency::USD),
            0.08, // 8% coupon (subordinated)
            Date::from_calendar_date(2025, Month::March, 1).unwrap(),
            Date::from_calendar_date(2032, Month::March, 1).unwrap(),
            "USD-OIS",
        )?
        
        // Aggregate capital structure metrics
        .compute("total_interest_expense", "sum(cs.interest_expense.SR-NOTES-2030, cs.interest_expense.SUB-NOTES-2032)")?
        .compute("total_debt_balance", "sum(cs.debt_balance.SR-NOTES-2030, cs.debt_balance.SUB-NOTES-2032)")?
        
        // Credit metrics
        .compute("interest_coverage", "ebitda / total_interest_expense")?
        .compute("debt_to_ebitda", "total_debt_balance / ttm(ebitda)")?
        
        // Load standard leverage metrics from registry
        .load_metrics("data/metrics/fin_leverage.json")?
        
        .build()
}
```

### F.2 Amortizing Loan Example

```rust
// Use valuations cashflow builder for amortizing schedule
use finstack_valuations::cashflow::CashflowBuilder;

fn build_amortizing_loan_model() -> Result<FinancialModel> {
    ModelBuilder::new("Term Loan Model")
        .periods("2025Q1..2030Q4", Some("2025Q1"))?
        
        // Add term loan with linear amortization
        .add_custom_debt(
            "TL-A",
            DebtInstrumentSpec::Generic {
                id: "TL-A".into(),
                spec: json!({
                    "type": "amortizing_loan",
                    "notional": 25_000_000.0,
                    "currency": "USD",
                    "issue_date": "2025-01-15",
                    "maturity_date": "2030-01-15",
                    "coupon_rate": 0.06,
                    "frequency": "quarterly",
                    "amortization": {
                        "type": "linear",
                        "final_notional": 0.0
                    },
                    "discount_curve_id": "USD-OIS"
                }),
            },
        )?
        
        // Reference amortization schedule in formulas
        .compute("principal_payment", "cs.principal_payment.TL-A")?
        .compute("interest_expense", "cs.interest_expense.TL-A")?
        .compute("total_debt_service", "principal_payment + interest_expense")?
        
        .build()
}
```

---

## 35. Appendix G: Performance Optimization Guide

### G.1 Evaluation Optimization

**Lazy Evaluation:**
```rust
impl Evaluator {
    /// Evaluate only nodes required for specific outputs.
    pub fn evaluate_selective(
        &mut self,
        model: &FinancialModel,
        required_nodes: &[String],
    ) -> crate::Result<Results> {
        // Build dependency subgraph
        let subgraph = self.build_dependency_subgraph(model, required_nodes)?;
        
        // Evaluate only required nodes
        let mut results = IndexMap::new();
        for period in &model.periods {
            for node_id in &subgraph {
                // Evaluate node for period
                let value = self.evaluate_node(model, node_id, period, &results)?;
                results.entry(node_id.clone())
                    .or_insert_with(IndexMap::new)
                    .insert(period.id, value);
            }
        }
        
        Ok(Results {
            nodes: results,
            periods: model.periods.clone(),
            meta: results_meta(&FinstackConfig::default()),
        })
    }
}
```

**Vectorized Evaluation:**
```rust
impl Evaluator {
    /// Evaluate all periods for a single node at once (vectorized).
    fn evaluate_node_vectorized(
        &self,
        model: &FinancialModel,
        node: &Node,
    ) -> crate::Result<Vec<f64>> {
        // Build column data across all periods
        let mut period_values = Vec::with_capacity(model.periods.len());
        
        for period in &model.periods {
            let value = self.resolve_node_value(model, node, period, &IndexMap::new())?;
            period_values.push(value.unwrap_or(0.0));
        }
        
        // If node has formula, evaluate once across all periods
        if let Some(ref formula) = node.formula {
            // Build multi-period context
            let ctx = self.build_vectorized_context(model)?;
            let cols = self.build_vectorized_columns(model)?;
            
            let result = formula.eval(&ctx, &cols, EvalOpts::default());
            return Ok(result.values);
        }
        
        Ok(period_values)
    }
}
```

### G.2 Memory Optimization

**Streaming Export for Large Models:**
```rust
impl Results {
    /// Stream results to Parquet file (for very large models).
    #[cfg(feature = "io")]
    pub fn stream_to_parquet(
        &self,
        path: impl AsRef<std::path::Path>,
        chunk_size: usize,
    ) -> crate::Result<()> {
        use polars::prelude::*;
        
        let file = std::fs::File::create(path)?;
        let mut writer = ParquetWriter::new(file);
        
        // Stream in chunks
        for chunk in self.nodes.iter().chunks(chunk_size) {
            let df = self.chunk_to_dataframe(chunk)?;
            writer.write_batch(&df)?;
        }
        
        writer.finish()?;
        Ok(())
    }
}
```

---

## 36. Conclusion & Next Steps

### 36.1 Implementation Sequence

**Immediate (Next 2 Weeks):**
1. Set up crate structure and CI
2. Implement wire types and builder
3. Basic DSL parser and compiler
4. Simple evaluator (no forecasting yet)

**Short-term (Weeks 3-6):**
5. Complete DSL with time-series operators
6. All forecast methods (deterministic + statistical)
7. Dynamic metric registry from JSON
8. Results export to DataFrame

**Medium-term (Weeks 7-10):**
9. Capital structure integration
10. Extension plugin system
11. Python bindings
12. WASM bindings

**Long-term (Weeks 11+):**
13. Corkscrew extension
14. Credit scorecard extension
15. Performance optimizations
16. Advanced features

### 36.2 Success Criteria Summary

**Technical Success:**
- ✅ All phases complete with passing tests
- ✅ Performance targets met
- ✅ Documentation complete
- ✅ Examples cover all features

**Product Success:**
- ✅ Replaces Python implementation functionality
- ✅ Analysts can build models without Rust knowledge
- ✅ Engineers can extend via JSON or Rust plugins
- ✅ Integrated with valuations for capital structure

**Team Success:**
- ✅ Code reviews are smooth (small PRs)
- ✅ No breaking changes between phases
- ✅ Knowledge shared across team
- ✅ Foundations for future features

---

## 37. Contact & Ownership

**Primary Owner:** [Your Name]
**Reviewers:** Core team, Valuations team
**Stakeholders:** Financial modeling team, Data science team

**Communication Channels:**
- Weekly updates: Team meeting
- Blockers: Slack #finstack-dev
- Design discussions: GitHub Discussions
- Code reviews: GitHub PRs

---

## 38. References

1. **Core Documentation:**
   - `/docs/new/02_core/02_core_tdd.md` - Core technical design
   - `/finstack/core/src/expr/mod.rs` - Expression engine
   - `/finstack/core/src/dates/periods.rs` - Period system

2. **Valuations Documentation:**
   - `/docs/new/03_valuations/03_valuations_tdd.md` - Valuations technical design
   - `/finstack/valuations/src/cashflow/aggregation.rs` - Cashflow aggregation

3. **Statements Documentation:**
   - `/docs/new/04_statements/04_statements_prd.md` - Product requirements
   - `/docs/new/04_statements/04_statements_tdd.md` - Technical design

4. **Examples:**
   - `/docs/new/examples.txt` - Python examples for reference
   - `/examples/python/` - Python usage patterns

---

## 39. Glossary

| Term | Definition |
|------|------------|
| **Node** | A single metric/line item in a financial model |
| **Period** | A time interval (quarter, month, year) from core's period system |
| **Precedence** | Evaluation priority: Value > Forecast > Formula |
| **Registry** | Collection of reusable metric definitions |
| **Extension** | Plugin that adds analysis capabilities |
| **Capital Structure** | Collection of debt/equity instruments |
| **DSL** | Domain-Specific Language for formulas |
| **Corkscrew** | Roll-forward schedule validation |
| **Scorecard** | Credit rating calculation framework |

---

**Document Version:** 1.0  
**Last Updated:** 2025-09-30  
**Status:** Ready for Implementation

This plan provides a complete, phased approach to implementing a production-quality statements crate that leverages core and valuations, supports dynamic metrics, includes a rich DSL, integrates capital structure, and provides a plugin architecture for future extensions.