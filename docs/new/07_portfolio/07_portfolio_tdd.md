# `/Portfolio` Crate — Technical Design

**Status:** Draft (implementation-ready)
**Last updated:** 2025-01-25
**MSRV:** 1.75 (aligned with core)
**License:** Apache-2.0 (project standard)

## 1) Purpose & Scope

`/portfolio` is the portfolio management and aggregation engine for finstack. It provides:

* **Portfolio hierarchy**: Book/folder trees with positions and entities
* **Entity management**: Companies/funds with statements and capital structures
* **Position tracking**: Holdings in instruments with quantities, dates, and tags
* **Currency-safe aggregation**: Multi-currency rollups with explicit FX conversion
* **Period alignment**: Statement aggregation across entities with different frequencies
* **Risk aggregation**: Portfolio-level risk measures across positions
* **Scenario application**: Portfolio-specific scenario paths and overrides
* **Tag-based grouping**: Flexible aggregation by arbitrary dimensions

**Depends on:**
* `core` for: types, FX, periods, validation, expression engine, Polars
* `statements` for: financial models and metrics
* `valuations` for: instrument pricing and risk measures
* `scenarios` for: scenario DSL and execution (optional)
* `analysis` for: portfolio analytics (optional)

**Out of scope:**
* Market data connectivity (inputs provided externally)
* Trade execution or order management
* Regulatory reporting (can be built on top)
* Accounting ledger or GL system
* Real-time position updates (batch-oriented)

---

## 2) Architecture & Dependencies

### 2.0 Overall Conformance & Boundaries

- Reuse-first orchestration: delegate to `finstack-core`, `finstack-statements`, `finstack-valuations`, and `finstack-scenarios` before adding new logic in this crate.
- No new numeric kernels here. Cashflow generation, discounting, interpolation, XIRR/TWR/MWR, and greeks live in `valuations`.
- Statement evaluation uses `statements::Evaluator` (vectorized via Polars re-exports from `core`).
- Scenario application uses `scenarios` (parsing, composition, execution phases, and cache invalidation order).
- Currency policy and FX come from `core::money` and `valuations::MarketData.fx`; aggregation is currency-preserving first, explicit FX collapse to `base_ccy` per policy. The portfolio layer MUST surface FX policy choices and stamp them in results.
- Period alignment follows the overall resampling rules (Overall §17.2). Node-level overrides may be provided via metadata; defaults come from family rules (flow=sum, stock=last, ratios=avg).

```
finstack/
├─ core/               # Foundation layer
├─ statements/         # Financial statements
├─ valuations/         # Pricing and risk
├─ scenarios/          # Scenario engine
├─ portfolio/          # THIS CRATE
│  ├─ src/
│  │  ├─ lib.rs
│  │  ├─ types.rs            # Core portfolio types (EntityId, PositionId, BookId)
│  │  ├─ entity.rs           # Entity and EntityRefs
│  │  ├─ position.rs         # Position and PositionUnit
│  │  ├─ book.rs             # BookNode hierarchy
│  │  ├─ portfolio.rs        # Portfolio struct and builder
│  │  ├─ aggregation/        # Aggregation engines
│  │  │  ├─ mod.rs
│  │  │  ├─ valuation.rs     # Position/book valuation rollups
│  │  │  ├─ statements.rs    # Entity statement aggregation
│  │  │  ├─ risk.rs          # Risk measure aggregation
│  │  │  └─ 
│  │  ├─ alignment/          # Period alignment logic
│  │  │  ├─ mod.rs
│  │  │  ├─ resampling.rs    # Flow/stock/ratio resampling
│  │  │  └─ aliasing.rs      # Node alias mapping
│  │  ├─ runner.rs           # PortfolioRunner with caching
│  │  ├─ results.rs          # Result types
│  │  ├─ scenario_paths.rs   # Portfolio-specific scenario paths
│  │  └─ error.rs
│  ├─ benches/
│  └─ tests/
├─ analysis/           # Optional analytics
└─ data/              # Optional I/O
```

**Cargo.toml:**

```toml
[package]
name = "finstack-portfolio"
version = "0.2.0"
edition = "2021"
license = "Apache-2.0"

[dependencies]
finstack-core = { path = "../core" }
finstack-statements = { path = "../statements" }
finstack-valuations = { path = "../valuations" }
finstack-scenarios = { path = "../scenarios", optional = true }
finstack-analysis = { path = "../analysis", optional = true }

thiserror = "1"
serde = { version = "1", features = ["derive"] }
indexmap = { version = "2", features = ["serde"] }
hashbrown = "0.14"
smallvec = "1"
rayon = { version = "1", optional = true }
tracing = "0.1"

# Use Polars via core re-exports
# No direct Polars dependency

[features]
default = ["scenarios"]
scenarios = ["dep:finstack-scenarios"]
analysis = ["dep:finstack-analysis"]
parallel = ["rayon", "finstack-core/rayon"]

[dev-dependencies]
proptest = "1"
criterion = "0.5"
approx = "0.6"
```

---

## 3) Core Types & Concepts

### 3.1 Identity Types

```rust
use finstack_core::types::Id;

// Type-safe IDs with zero-cost abstraction
pub type EntityId = Id<Entity>;
pub type PositionId = Id<Position>;
pub type BookId = Id<Book>;

// Or if using strings for simplicity:
pub type EntityId = String;
pub type PositionId = String;
pub type BookId = String;
```

### 3.2 Entity & References

```rust
use std::sync::Arc;
use finstack_statements::FinancialModel;
use finstack_valuations::CapitalStructure;
use finstack_core::types::TagSet;

/// References to entity's financial data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityRefs {
    /// Financial statements model (optional)
    pub model: Option<Arc<FinancialModel>>,
    
    /// Capital structure with instruments (optional)
    pub capital: Option<Arc<CapitalStructure>>,
    
    /// Entity-level tags for grouping
    pub tags: TagSet,
    
    /// Additional metadata
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub meta: IndexMap<String, serde_json::Value>,
}

/// Capital structure container
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapitalStructure {
    pub entity_id: EntityId,
    pub instruments: IndexMap<String, Arc<dyn Instrument>>,
    pub issue_dates: IndexMap<String, time::Date>,
    pub maturity_dates: IndexMap<String, Option<time::Date>>,
    pub tags: IndexMap<String, TagSet>,
}
```

### 3.3 Position

```rust
use finstack_core::money::{Currency, Decimal};

/// Unit of position measurement
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionUnit {
    /// Number of units/shares
    Units,
    
    /// Notional amount (optionally in specific currency)
    Notional(Option<Currency>),
    
    /// Face value of debt instruments
    FaceValue,
    
    /// Percentage of ownership
    Percentage,
}

/// A position in an instrument
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub position_id: PositionId,
    pub entity_id: EntityId,
    pub instrument_id: String,
    
    /// Signed quantity (positive=long, negative=short)
    pub quantity: Decimal,
    pub unit: PositionUnit,
    
    /// Position lifecycle
    pub open_date: time::Date,
    pub close_date: Option<time::Date>,
    
    /// Cost basis
    pub cost_basis: Option<Amount>,
    
    /// Position-level tags
    pub tags: TagSet,
    
    /// Additional metadata
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub meta: IndexMap<String, serde_json::Value>,
}

impl Position {
    /// Check if position is active on a given date
    pub fn is_active(&self, as_of: time::Date) -> bool {
        as_of >= self.open_date && 
        self.close_date.map_or(true, |close| as_of <= close)
    }
    
    /// Get effective quantity on a date (0 if not active)
    pub fn effective_quantity(&self, as_of: time::Date) -> Decimal {
        if self.is_active(as_of) {
            self.quantity
        } else {
            Decimal::ZERO
        }
    }
}
```

#### 3.3.1 Units for Equity; Notional for FX Spot

For simple equities and FX spot positions:

- Equity positions use `PositionUnit::Units` where `quantity` is the number of shares.
- FX spot positions use `PositionUnit::Notional(Some(base_ccy))` where `quantity` is the amount of base currency.

```rust
// Equity: 100 shares of AAPL (priced via valuations::MarketData.prices)
let eq_pos = Position {
    position_id: "EQ_AAPL_100".into(),
    entity_id: "HoldCo".into(),
    instrument_id: "EQ_AAPL".into(), // valuations::Equity { id:"EQ_AAPL", ticker:"AAPL", ... }
    quantity: dec!(100),
    unit: PositionUnit::Units,
    open_date: date!(2024-01-01),
    close_date: None,
    cost_basis: None,
    tags: TagSet::default(),
    meta: IndexMap::new(),
};

// FX Spot: long 1mm USD vs EUR (priced via FxMatrix)
let fx_pos = Position {
    position_id: "FX_USD_EUR_1MM".into(),
    entity_id: "HoldCo".into(),
    instrument_id: "FX_USD_EUR".into(), // valuations::FxSpot { id:"FX_USD_EUR", base: USD, quote: EUR }
    quantity: dec!(1_000_000),
    unit: PositionUnit::Notional(Some(Currency::USD)),
    open_date: date!(2024-01-01),
    close_date: None,
    cost_basis: None,
    tags: TagSet::default(),
    meta: IndexMap::new(),
};
```

### 3.4 Book Hierarchy

```rust
/// Hierarchical book/folder structure
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BookNode {
    /// Internal node (folder)
    Book {
        book_id: BookId,
        name: Option<String>,
        children: Vec<BookNode>,
        tags: TagSet,
        #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
        meta: IndexMap<String, serde_json::Value>,
    },
    
    /// Leaf node (position reference)
    Position {
        position_id: PositionId,
    },
}

impl BookNode {
    /// Recursively collect all position IDs
    pub fn collect_positions(&self) -> Vec<PositionId> {
        match self {
            BookNode::Book { children, .. } => {
                children.iter()
                    .flat_map(|child| child.collect_positions())
                    .collect()
            }
            BookNode::Position { position_id } => vec![position_id.clone()],
        }
    }
    
    /// Find a book by ID
    pub fn find_book(&self, target: &BookId) -> Option<&BookNode> {
        match self {
            BookNode::Book { book_id, children, .. } => {
                if book_id == target {
                    Some(self)
                } else {
                    children.iter()
                        .find_map(|child| child.find_book(target))
                }
            }
            BookNode::Position { .. } => None,
        }
    }
}
```

### 3.5 Node Aliasing

```rust
/// Maps canonical node IDs to entity-specific node IDs
/// Enables cross-entity statement aggregation
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NodeAliasMap {
    /// canonical_node_id -> { entity_id -> entity_node_id }
    pub canonical_to_entity: IndexMap<String, IndexMap<EntityId, String>>,
}

impl NodeAliasMap {
    /// Add an alias mapping
    pub fn add_alias(
        &mut self,
        canonical: impl Into<String>,
        entity_id: impl Into<EntityId>,
        entity_node: impl Into<String>,
    ) {
        self.canonical_to_entity
            .entry(canonical.into())
            .or_default()
            .insert(entity_id.into(), entity_node.into());
    }
    
    /// Get entity-specific node ID
    pub fn get_entity_node(
        &self,
        canonical: &str,
        entity_id: &EntityId,
    ) -> Option<&str> {
        self.canonical_to_entity
            .get(canonical)?
            .get(entity_id)
            .map(String::as_str)
    }
}
```

### 3.6 Portfolio

```rust
/// Complete portfolio definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Portfolio {
    pub id: String,
    pub name: Option<String>,
    
    /// Base currency for aggregation
    pub base_ccy: Currency,
    
    /// Valuation date
    pub as_of: time::Date,
    
    /// Entity definitions
    pub entities: IndexMap<EntityId, EntityRefs>,
    
    /// Position inventory
    pub positions: IndexMap<PositionId, Position>,
    
    /// Book hierarchy (single root)
    pub book_tree: BookNode,
    
    /// Node alias mappings for statements
    pub node_aliases: NodeAliasMap,
    
    /// Portfolio-wide evaluation periods
    pub periods: Vec<Period>,
    
    /// FX policy configuration (explicit and visible)
    #[serde(default)]
    pub fx_policy: FxPolicyConfig,
    
    /// Portfolio-level tags
    pub tags: TagSet,
    
    /// Additional metadata
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub meta: IndexMap<String, serde_json::Value>,
}

/// FX policy configuration surfaced at the portfolio layer
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FxPolicyConfig {
    /// Strategy used when collapsing instrument cashflows/values during valuation aggregation
    /// Default: CashflowDate
    pub valuations: finstack_core::money::FxConversionPolicy,
    /// Strategy used when statements convert `Amount` to a model currency
    /// Default: PeriodEnd
    pub statements: finstack_core::money::FxConversionPolicy,
}

impl Default for FxPolicyConfig {
    fn default() -> Self {
        Self {
            valuations: finstack_core::money::FxConversionPolicy::CashflowDate,
            statements: finstack_core::money::FxConversionPolicy::PeriodEnd,
        }
    }
}

impl Portfolio {
    /// Validate internal consistency
    pub fn validate(&self) -> ValidationResult<()> {
        let mut warnings = Vec::new();
        
        // Check all positions reference valid entities
        for position in self.positions.values() {
            if !self.entities.contains_key(&position.entity_id) {
                warnings.push(ValidationWarning {
                    path: format!("positions.{}", position.position_id),
                    message: format!("References unknown entity: {}", position.entity_id),
                    severity: Severity::Error,
                });
            }
        }
        
        // Check book tree references valid positions
        for pos_id in self.book_tree.collect_positions() {
            if !self.positions.contains_key(&pos_id) {
                warnings.push(ValidationWarning {
                    path: format!("book_tree"),
                    message: format!("References unknown position: {}", pos_id),
                    severity: Severity::Error,
                });
            }
        }
        
        // Check periods are in order
        if !self.periods.windows(2).all(|w| w[0].end <= w[1].start) {
            warnings.push(ValidationWarning {
                path: "periods".to_string(),
                message: "Periods are not in chronological order".to_string(),
                severity: Severity::Warning,
            });
        }
        
        ValidationResult {
            value: (),
            warnings,
            passed: warnings.iter().all(|w| w.severity != Severity::Error),
        }
    }
}
```

---

## 4) Aggregation Engines

### 4.1 Valuation Aggregation

```rust
use finstack_valuations::{MarketData, ValuationResult, Priceable};
use finstack_core::money::{Currency, FxProvider};
use finstack_core::prelude::Decimal;

/// Portfolio valuation results (conformant with Overall §7.3)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioValuation {
    /// Position values by currency (before FX), currency-preserving
    pub position_values_ccy: IndexMap<PositionId, IndexMap<Currency, Decimal>>,
    /// Position values collapsed to base currency (explicit FX)
    pub position_values_base: IndexMap<PositionId, Decimal>,
    /// Book-level totals in base currency
    pub book_totals_base: IndexMap<BookId, Decimal>,
    /// Portfolio total in base currency
    pub portfolio_total_base: Decimal,
}

// Note: additional rollups (e.g., entity totals) are derivable conveniences

/// Valuation aggregation engine
pub struct ValuationAggregator<'a> {
    portfolio: &'a Portfolio,
    market: &'a MarketData,
    fx: &'a dyn FxProvider,
}

impl<'a> ValuationAggregator<'a> {
    pub fn aggregate(&self) -> Result<PortfolioValuation, PortfolioError> {
        let mut position_values_ccy: IndexMap<PositionId, IndexMap<Currency, Decimal>> = IndexMap::new();
        let mut position_values_base: IndexMap<PositionId, Decimal> = IndexMap::new();
        
        // Phase 1: Value all positions (parallelizable)
        let position_results = self.value_positions()?;
        
        // Phase 2: Convert to base currency
        for (pos_id, valuation) in position_results {
            let base_value = self.convert_to_base(&valuation)?;
            // Currency-preserving map from the primary valuation amount
            let mut ccy_map = IndexMap::new();
            ccy_map.insert(valuation.value.ccy, valuation.value.value);
            position_values_ccy.insert(pos_id.clone(), ccy_map);
            position_values_base.insert(pos_id, base_value);
        }
        
        // Phase 3: Aggregate by book hierarchy
        let book_totals = self.aggregate_books(&position_values_base)?;
        
        // Phase 4: Portfolio total
        let portfolio_total = position_values_base.values().copied().sum();
        
        Ok(PortfolioValuation {
            position_values_ccy,
            position_values_base,
            book_totals_base: book_totals,
            portfolio_total_base: portfolio_total,
        })
    }
    
    fn value_positions(&self) -> Result<IndexMap<PositionId, ValuationResult>, PortfolioError> {
        // Implementation: iterate positions, get instruments, call price()
        todo!()
    }
    
    fn convert_to_base(&self, val: &ValuationResult) -> Result<Decimal, PortfolioError> {
        // Explicit FX collapse using MarketData.fx per policy (Overall §17.1)
        // Implemented by converting the primary amount to base_ccy at the chosen policy date
        todo!()
    }
    
    fn aggregate_books(
        &self,
        position_values: &IndexMap<PositionId, Decimal>
    ) -> Result<IndexMap<BookId, Decimal>, PortfolioError> {
        // Recursive aggregation following book tree, deterministic order
        todo!()
    }
}
// DataFrame outputs for valuation aggregation (required)
impl PortfolioValuation {
    /// (position_id, value_base)
    pub fn to_polars_positions_base(&self) -> polars::prelude::DataFrame { /* impl */ }

    /// (position_id, currency, value)
    pub fn to_polars_positions_ccy(&self) -> polars::prelude::DataFrame { /* impl */ }

    /// (book_id, total_base)
    pub fn to_polars_books_base(&self) -> polars::prelude::DataFrame { /* impl */ }
}
```

### 4.2 Statement Aggregation

```rust
use finstack_statements::{Results as StatementResults};

/// Period alignment policy
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResampleMethod {
    /// Sum values (for flow items like revenue)
    Sum,
    
    /// Take last value (for stock items like assets)
    Last,
    
    /// Time-weighted average (for ratios)
    Average,
    
    /// First value in period
    First,
    
    /// Maximum value in period
    Max,
    
    /// Minimum value in period
    Min,
}

/// Statement aggregation configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatementAggConfig {
    /// Default resampling by node type
    pub default_resample: IndexMap<String, ResampleMethod>,
    
    /// Node-specific overrides
    pub node_overrides: IndexMap<String, ResampleMethod>,
    
    /// Whether to use node aliases
    pub use_aliases: bool,
}

impl Default for StatementAggConfig {
    fn default() -> Self {
        let mut defaults = IndexMap::new();
        // Defaults follow Overall §17.2 (flows=sum, stocks=last, ratios=avg)
        defaults.insert("revenue".to_string(), ResampleMethod::Sum);
        defaults.insert("expense".to_string(), ResampleMethod::Sum);
        defaults.insert("income".to_string(), ResampleMethod::Sum);
        
        // Stock items (B/S)
        defaults.insert("asset".to_string(), ResampleMethod::Last);
        defaults.insert("liability".to_string(), ResampleMethod::Last);
        defaults.insert("equity".to_string(), ResampleMethod::Last);
        
        // Ratios
        defaults.insert("ratio".to_string(), ResampleMethod::Average);
        defaults.insert("margin".to_string(), ResampleMethod::Average);
        
        Self {
            default_resample: defaults,
            node_overrides: IndexMap::new(),
            use_aliases: true,
        }
    }
}

/// Statement aggregation engine
pub struct StatementAggregator<'a> {
    portfolio: &'a Portfolio,
    config: StatementAggConfig,
}

impl<'a> StatementAggregator<'a> {
    pub fn aggregate(
        &self,
        entity_results: &IndexMap<EntityId, StatementResults>,
    ) -> Result<PortfolioStatements, PortfolioError> {
        let mut aggregated = IndexMap::new();
        
        // For each canonical node
        for (canonical_node, entity_map) in &self.portfolio.node_aliases.canonical_to_entity {
            let mut period_values = IndexMap::new();
            
            // For each portfolio period
            for period in &self.portfolio.periods {
                let mut period_sum = Decimal::ZERO;
                
                // Aggregate across entities
                for (entity_id, entity_node) in entity_map {
                    if let Some(entity_result) = entity_results.get(entity_id) {
                        let value = self.resample_value(
                            entity_result,
                            entity_node,
                            period,
                            self.get_resample_method(canonical_node),
                        )?;
                        period_sum += value;
                    }
                }
                
                period_values.insert(period.id.clone(), period_sum);
            }
            
            aggregated.insert(canonical_node.clone(), period_values);
        }
        
        Ok(PortfolioStatements {
            per_node_per_period_base: aggregated,
        })
    }
    
    fn get_resample_method(&self, node: &str) -> ResampleMethod {
        self.config.node_overrides.get(node)
            .or_else(|| {
                // Infer from node name patterns
                if node.contains("revenue") || node.contains("expense") {
                    Some(&ResampleMethod::Sum)
                } else if node.contains("asset") || node.contains("liability") {
                    Some(&ResampleMethod::Last)
                } else if node.contains("margin") || node.contains("ratio") {
                    Some(&ResampleMethod::Average)
                } else {
                    None
                }
            })
            .copied()
            .unwrap_or(ResampleMethod::Last)
    }
    
    fn resample_value(
        &self,
        results: &StatementResults,
        node: &str,
        target_period: &Period,
        method: ResampleMethod,
    ) -> Result<Decimal, PortfolioError> {
        // Get source periods that overlap with target
        let source_values = self.get_overlapping_values(results, node, target_period)?;
        
        match method {
            ResampleMethod::Sum => Ok(source_values.iter().sum()),
            ResampleMethod::Last => Ok(source_values.last().copied().unwrap_or_default()),
            ResampleMethod::Average => {
                if source_values.is_empty() {
                    Ok(Decimal::ZERO)
                } else {
                    Ok(source_values.iter().sum::<Decimal>() / Decimal::from(source_values.len()))
                }
            }
            ResampleMethod::First => Ok(source_values.first().copied().unwrap_or_default()),
            ResampleMethod::Max => Ok(source_values.iter().max().copied().unwrap_or_default()),
            ResampleMethod::Min => Ok(source_values.iter().min().copied().unwrap_or_default()),
        }
    }
}
// DataFrame outputs for statements (required)
impl PortfolioStatements {
    /// Long format: (node, period, value)
    pub fn to_polars_long(&self) -> polars::prelude::DataFrame { /* impl */ }

    /// Wide format: periods as rows, nodes as columns
    pub fn to_polars_wide(&self) -> polars::prelude::DataFrame { /* impl */ }
}
```

### 4.3 Risk Aggregation

```rust
use finstack_valuations::{RiskReport};
use finstack_core::prelude::Decimal;

/// Portfolio risk aggregation
pub struct RiskAggregator<'a> {
    portfolio: &'a Portfolio,
    market: &'a MarketData,
}

impl<'a> RiskAggregator<'a> {
    pub fn aggregate(
        &self,
        position_risks: &IndexMap<PositionId, RiskReport>,
    ) -> Result<PortfolioRiskReport, PortfolioError> {
        // Reduce into a flat by-bucket map (e.g., curve_key -> DV01), deterministically
        let mut by_bucket: IndexMap<String, Decimal> = IndexMap::new();
        for report in position_risks.values() {
            for (bucket, value) in report.by_bucket.iter() {
                *by_bucket.entry(bucket.clone()).or_insert(Decimal::ZERO) += *value;
            }
        }
        Ok(PortfolioRiskReport { by_bucket })
    }
}
// DataFrame outputs for risk (required)
impl PortfolioRiskReport {
    /// Flat buckets: (bucket, value)
    pub fn to_polars_buckets(&self) -> polars::prelude::DataFrame { /* impl */ }
}
```

### 4.4

---

## 5) Portfolio Builder

```rust
/// Type-state builder for portfolio construction
pub struct PortfolioBuilder<S> {
    state: S,
    id: String,
    name: Option<String>,
    entities: IndexMap<EntityId, EntityRefs>,
    positions: IndexMap<PositionId, Position>,
    book_tree: Option<BookNode>,
    node_aliases: NodeAliasMap,
    tags: TagSet,
    meta: IndexMap<String, serde_json::Value>,
}

// Builder states
pub struct NeedPlan;
pub struct NeedBooks;
pub struct Ready;

impl PortfolioBuilder<NeedPlan> {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            state: NeedPlan,
            id: id.into(),
            name: None,
            entities: IndexMap::new(),
            positions: IndexMap::new(),
            book_tree: None,
            node_aliases: NodeAliasMap::default(),
            tags: TagSet::default(),
            meta: IndexMap::new(),
        }
    }
    
    pub fn plan(
        self,
        base_ccy: Currency,
        as_of: time::Date,
        periods: Vec<Period>,
    ) -> Result<PortfolioBuilder<NeedBooks>, PortfolioError> {
        // Validate periods are in order
        if !periods.windows(2).all(|w| w[0].end <= w[1].start) {
            return Err(PortfolioError::InvalidPeriods);
        }
        
        Ok(PortfolioBuilder {
            state: NeedBooks,
            // ... move fields ...
        })
    }
}

impl PortfolioBuilder<NeedBooks> {
    pub fn entity(
        mut self,
        id: impl Into<EntityId>,
        refs: EntityRefs,
    ) -> Self {
        self.entities.insert(id.into(), refs);
        self
    }
    
    pub fn position(mut self, position: Position) -> Self {
        self.positions.insert(position.position_id.clone(), position);
        self
    }
    
    pub fn book_tree(self, root: BookNode) -> PortfolioBuilder<Ready> {
        PortfolioBuilder {
            state: Ready,
            book_tree: Some(root),
            // ... move other fields ...
        }
    }
}

impl PortfolioBuilder<Ready> {
    pub fn alias(
        mut self,
        canonical: impl Into<String>,
        entity_id: impl Into<EntityId>,
        entity_node: impl Into<String>,
    ) -> Self {
        self.node_aliases.add_alias(canonical, entity_id, entity_node);
        self
    }
    
    pub fn tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }
    
    pub fn build(self) -> Result<Portfolio, PortfolioError> {
        let portfolio = Portfolio {
            id: self.id,
            name: self.name,
            // ... all fields ...
        };
        
        // Validate consistency
        portfolio.validate()?;
        
        Ok(portfolio)
    }
}
```

---

## 6) Portfolio Runner

```rust
use finstack_scenarios::Scenario;

/// Portfolio evaluation runner with caching
pub struct PortfolioRunner {
    /// Cache for entity statement results
    statement_cache: RefCell<LruCache<(EntityId, u64), Arc<StatementResults>>>,
    
    /// Cache for instrument valuations
    valuation_cache: RefCell<LruCache<(String, u64), Arc<ValuationResult>>>,
    
    /// Cache for risk reports
    risk_cache: RefCell<LruCache<(String, u64), Arc<RiskReport>>>,
    
    /// Parallel execution flag
    parallel: bool,
}

impl PortfolioRunner {
    pub fn new() -> Self {
        Self {
            statement_cache: RefCell::new(LruCache::new(NonZeroUsize::new(100).unwrap())),
            valuation_cache: RefCell::new(LruCache::new(NonZeroUsize::new(500).unwrap())),
            risk_cache: RefCell::new(LruCache::new(NonZeroUsize::new(500).unwrap())),
            parallel: false,
        }
    }
    
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }
    
    pub fn run(
        &self,
        portfolio: &Portfolio,
        market: &MarketData,
        scenario: Option<&Scenario>,
    ) -> Result<PortfolioResults, PortfolioError> {
        // Phase 1: Apply scenario (if any)
        let (portfolio_prime, market_prime) = if let Some(sc) = scenario {
            self.apply_scenario(portfolio, market, sc)?
        } else {
            (Cow::Borrowed(portfolio), Cow::Borrowed(market))
        };
        
        // Phase 2: Evaluate entity statements (cached)
        let entity_statements = self.evaluate_statements(&portfolio_prime, &market_prime)?;
        
        // Phase 3: Value all positions
        let position_valuations = self.value_positions(&portfolio_prime, &market_prime)?;
        
        // Phase 4: Calculate risk measures
        let position_risks = self.calculate_risks(&portfolio_prime, &market_prime)?;
        
        // Phase 5: Aggregate valuations
        let valuation_agg = ValuationAggregator {
            portfolio: &portfolio_prime,
            market: &market_prime,
            fx: &market_prime.fx,
        };
        let valuation = valuation_agg.aggregate()?;
        
        // Phase 6: Aggregate statements
        let statement_agg = StatementAggregator {
            portfolio: &portfolio_prime,
            config: StatementAggConfig::default(),
        };
        let statements = statement_agg.aggregate(&entity_statements)?;
        
        // Phase 7: Aggregate risk
        let risk_agg = RiskAggregator {
            portfolio: &portfolio_prime,
            market: &market_prime,
        };
        let risk = risk_agg.aggregate(&position_risks)?;
        
        Ok(PortfolioResults {
            valuation,
            risk,
            statements,
            meta: ResultsMeta {
                numeric_mode: NumericMode::Decimal,
                parallel: self.parallel,
                seed: 0,
                model_currency: Some(portfolio_prime.base_ccy),
                // fx_policies MUST be populated by the runner prior to return
                // Example (logical shape): { "valuations": {strategy, target_ccy}, "statements": {strategy, target_ccy}, "portfolio": {strategy, target_ccy} }
            },
        })
    }
    
    fn apply_scenario<'a>(
        &self,
        portfolio: &'a Portfolio,
        market: &'a MarketData,
        scenario: &Scenario,
    ) -> Result<(Cow<'a, Portfolio>, Cow<'a, MarketData>), PortfolioError> {
        // Apply portfolio-specific scenario paths
        // This involves cloning and modifying the portfolio/market
        todo!()
    }
    
    fn evaluate_statements(
        &self,
        portfolio: &Portfolio,
        market: &MarketData,
    ) -> Result<IndexMap<EntityId, StatementResults>, PortfolioError> {
        let mut results = IndexMap::new();
        
        for (entity_id, refs) in &portfolio.entities {
            if let Some(model) = &refs.model {
                // Check cache
                let cache_key = (entity_id.clone(), self.compute_hash(model, market));
                
                if let Some(cached) = self.statement_cache.borrow().get(&cache_key) {
                    results.insert(entity_id.clone(), (**cached).clone());
                    continue;
                }
                
                // Evaluate using statements::Evaluator (reuse-first)
                let evaluator = finstack_statements::Evaluator::new();
                let result = evaluator.evaluate(model, self.parallel)?;
                
                // Cache
                let result_arc = Arc::new(result.clone());
                self.statement_cache.borrow_mut().put(cache_key, result_arc);
                
                results.insert(entity_id.clone(), result);
            }
        }
        
        Ok(results)
    }
    
    fn compute_hash(&self, model: &FinancialModel, market: &MarketData) -> u64 {
        // Compute hash for cache key
        todo!()
    }
}
```

---

## 7) Scenario Integration

```rust
/// Portfolio-specific scenario paths
impl Portfolio {
    pub fn scenario_paths() -> Vec<ScenarioPath> {
        vec![
            // Portfolio-level
            ScenarioPath::new("portfolio.base_ccy", PathType::Currency),
            ScenarioPath::new("portfolio.as_of", PathType::Date),
            
            // Position-level
            ScenarioPath::new("portfolio.positions.<id>.quantity", PathType::Decimal),
            ScenarioPath::new("portfolio.positions.<id>.close_date", PathType::Date),
            // NEW: Attribute selectors
            // Select by instrument attributes (rating, sector, seniority) via selectors (Scenarios §2.6)
            ScenarioPath::new("valuations.instruments?{rating:\"CCC\"}.*", PathType::Delegated),
            ScenarioPath::new("valuations.instruments?{sector:\"Energy\"}.*", PathType::Delegated),
            
            // Entity statements (delegated)
            ScenarioPath::new("entities.<id>.statements.<node>.*", PathType::Delegated),
            
            // Entity instruments (delegated)
            ScenarioPath::new("entities.<id>.instruments.<id>.*", PathType::Delegated),
        ]
    }
}

/// Apply scenario to portfolio
pub fn apply_portfolio_scenario(
    portfolio: &Portfolio,
    scenario: &Scenario,
) -> Result<Portfolio, PortfolioError> {
    let mut result = portfolio.clone();
    
    for operation in scenario.operations() {
        match operation.path.root() {
            "portfolio" => {
                apply_portfolio_operation(&mut result, operation)?;
            }
            // NEW: Attribute-selected valuations paths expand to concrete instruments
            // before application, then routed to valuations adapter.
            "entities" => {
                apply_entity_operation(&mut result, operation)?;
            }
            _ => {
                // Unknown path, skip or error based on strict mode
            }
        }
    }
    
    Ok(result)
}
```

---

## 8) Results Types

```rust
/// Complete portfolio evaluation results
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioResults {
    pub valuation: PortfolioValuation,
    pub risk: PortfolioRiskReport,
    pub statements: PortfolioStatements,
    pub meta: ResultsMeta, // includes rounding context and fx_conversion_policy
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioStatements {
    /// Canonical node -> period -> value in base currency
    pub per_node_per_period_base: IndexMap<String, IndexMap<PeriodId, Decimal>>,
    pub periods: Vec<Period>,
    pub config: StatementAggConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioRiskReport {
    pub dv01_by_curve: IndexMap<CurveId, Amount>,
    pub cs01_by_issuer: IndexMap<IssuerId, Amount>,
    pub fx_delta_by_pair: IndexMap<CurrencyPair, Amount>,
    pub total_var: Option<Amount>,
    pub position_risks: IndexMap<PositionId, RiskReport>,
    pub as_of: time::Date,
}

 
```

---

## 9) Error Handling

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PortfolioError {
    #[error("Invalid periods: {0}")]
    InvalidPeriods(String),
    
    #[error("Unknown entity: {0}")]
    UnknownEntity(EntityId),
    
    #[error("Unknown position: {0}")]
    UnknownPosition(PositionId),
    
    #[error("Unknown instrument: {entity_id}:{instrument_id}")]
    UnknownInstrument {
        entity_id: EntityId,
        instrument_id: String,
    },
    
    #[error("FX conversion failed: {from} to {to}")]
    FxConversionFailed {
        from: Currency,
        to: Currency,
    },
    
    #[error("Period alignment failed: {0}")]
    AlignmentFailed(String),
    
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Scenario application failed: {0}")]
    ScenarioFailed(String),
    
    
    
    #[error(transparent)]
    Core(#[from] finstack_core::CoreError),
    
    #[error(transparent)]
    Statements(#[from] finstack_statements::StatementError),
    
    #[error(transparent)]
    Valuations(#[from] finstack_valuations::ValuationError),
}
```

---

## 10) Testing Strategy

### 10.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_position_lifecycle() {
        let pos = Position {
            position_id: "P1".into(),
            entity_id: "E1".into(),
            instrument_id: "LOAN_A".into(),
            quantity: dec!(1_000_000),
            unit: PositionUnit::FaceValue,
            open_date: date!(2024-01-01),
            close_date: Some(date!(2024-12-31)),
            cost_basis: Some(Amount { value: dec!(980_000), ccy: Currency::USD }),
            tags: TagSet::default(),
            meta: IndexMap::new(),
        };
        
        assert!(pos.is_active(date!(2024-06-01)));
        assert!(!pos.is_active(date!(2025-01-01)));
        assert_eq!(pos.effective_quantity(date!(2024-06-01)), dec!(1_000_000));
        assert_eq!(pos.effective_quantity(date!(2025-01-01)), Decimal::ZERO);
    }
    
    #[test]
    fn test_book_tree_traversal() {
        let tree = BookNode::Book {
            book_id: "ROOT".into(),
            name: Some("Portfolio".into()),
            children: vec![
                BookNode::Book {
                    book_id: "LOANS".into(),
                    name: Some("Loans".into()),
                    children: vec![
                        BookNode::Position { position_id: "P1".into() },
                        BookNode::Position { position_id: "P2".into() },
                    ],
                    tags: TagSet::default(),
                    meta: IndexMap::new(),
                },
                BookNode::Position { position_id: "P3".into() },
            ],
            tags: TagSet::default(),
            meta: IndexMap::new(),
        };
        
        let positions = tree.collect_positions();
        assert_eq!(positions, vec!["P1".into(), "P2".into(), "P3".into()]);
        
        assert!(tree.find_book(&"LOANS".into()).is_some());
        assert!(tree.find_book(&"UNKNOWN".into()).is_none());
    }
    
    #[test]
    fn test_node_aliasing() {
        let mut aliases = NodeAliasMap::default();
        aliases.add_alias("Revenue", "OpCo", "Sales");
        aliases.add_alias("Revenue", "HoldCo", "Income");
        
        assert_eq!(aliases.get_entity_node("Revenue", &"OpCo".into()), Some("Sales"));
        assert_eq!(aliases.get_entity_node("Revenue", &"HoldCo".into()), Some("Income"));
        assert_eq!(aliases.get_entity_node("Revenue", &"Unknown".into()), None);
    }
}
```

### 10.2 Property Tests

```rust
#[cfg(test)]
mod proptests {
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn test_aggregation_preserves_total(
            positions in prop::collection::vec(position_arb(), 1..100)
        ) {
            // Sum of position values should equal portfolio total
            let portfolio = build_test_portfolio(positions);
            let results = run_portfolio(&portfolio)?;
            
            let position_sum = results.valuation.position_values_base
                .values()
                .fold(Amount { value: Decimal::ZERO, ccy: USD }, |acc, m| acc + m);
            
            prop_assert_eq!(position_sum, results.valuation.portfolio_total_base);
        }
        
        #[test]
        fn test_currency_safety(
            value in money_arb(),
            other_ccy in currency_arb()
        ) {
            // Cannot aggregate different currencies without FX
            if value.currency != other_ccy {
                let other = Amount { value: dec!(100), ccy: other_ccy };
                prop_assert!(std::panic::catch_unwind(|| value + other).is_err());
            }
        }
    }
}
```

### 10.3 Integration Tests

```rust
#[test]
fn test_full_portfolio_evaluation() {
    // Create portfolio with multiple entities and positions
    let portfolio = PortfolioBuilder::new("TEST_FUND")
        .plan(Currency::USD, date!(2024-01-01), build_quarters(2024))
        .unwrap()
        .entity("OpCo", EntityRefs {
            model: Some(Arc::new(build_test_model())),
            capital: Some(Arc::new(build_test_capital())),
            tags: TagSet::default(),
            meta: IndexMap::new(),
        })
        .position(Position {
            position_id: "TL_A".into(),
            entity_id: "OpCo".into(),
            instrument_id: "TERM_LOAN_A".into(),
            quantity: dec!(10_000_000),
            unit: PositionUnit::FaceValue,
            open_date: date!(2024-01-01),
            close_date: None,
            cost_basis: Some(Amount { value: dec!(9_800_000), ccy: Currency::USD }),
            tags: tags!["type" => "senior", "rating" => "B+"],
            meta: IndexMap::new(),
        })
        .book_tree(BookNode::Book {
            book_id: "ROOT".into(),
            name: Some("Fund".into()),
            children: vec![
                BookNode::Position { position_id: "TL_A".into() },
            ],
            tags: TagSet::default(),
            meta: IndexMap::new(),
        })
        .alias("EBITDA", "OpCo", "ebitda_node")
        .build()
        .unwrap();
    
    // Run evaluation
    let runner = PortfolioRunner::new();
    let results = runner.run(&portfolio, &build_test_market(), None).unwrap();
    
    // Verify results
    assert_eq!(results.valuation.position_values_base.len(), 1);
    assert!(results.valuation.portfolio_total_base.value > Decimal::ZERO);
    assert!(results.statements.per_node_per_period_base.contains_key("EBITDA"));
}
```

### 10.4 Benchmark Tests

```rust
#[bench]
fn bench_portfolio_aggregation(b: &mut Bencher) {
    let portfolio = build_large_portfolio(); // 1000 positions, 50 entities
    let market = build_test_market();
    let runner = PortfolioRunner::new();
    
    b.iter(|| {
        runner.run(&portfolio, &market, None).unwrap()
    });
}

#[bench]
fn bench_parallel_vs_serial(b: &mut Bencher) {
    let portfolio = build_large_portfolio();
    let market = build_test_market();
    
    let serial_runner = PortfolioRunner::new().with_parallel(false);
    let parallel_runner = PortfolioRunner::new().with_parallel(true);
    
    let serial_time = bench_once(|| serial_runner.run(&portfolio, &market, None));
    let parallel_time = bench_once(|| parallel_runner.run(&portfolio, &market, None));
    
    println!("Serial: {:?}, Parallel: {:?}", serial_time, parallel_time);
    assert!(parallel_time < serial_time * 0.7); // Expect >30% speedup
}
```

---

## 11) Performance Considerations

### 11.1 Caching Strategy

* **Statement results**: LRU cache keyed by `(entity_id, model_hash, market_hash)`
* **Valuations**: LRU cache keyed by `(instrument_id, market_hash)`
* **Risk reports**: LRU cache keyed by `(instrument_id, market_hash, buckets_hash)`
* **FX rates**: Cached in core's FxMatrix with configurable TTL

### 11.2 Parallelization

* **Position valuation**: Parallel across positions (embarrassingly parallel)
* **Entity statements**: Parallel across entities
* **Book aggregation**: Sequential (tree traversal)
* **Determinism**: Stable reduction order for Decimal mode

### 11.3 Memory Optimization

* Use `Arc` for shared data (models, capital structures, market data)
* `SmallVec` for small collections (tags, small position lists)
* Streaming aggregation for large portfolios (don't materialize all intermediates)

---

## 12) Python Bindings

```python
# Python API sketch
from finstack import Portfolio, Position, Book, MarketData

# Builder pattern
portfolio = (Portfolio.builder("FUND_A")
    .plan(base_ccy="USD", as_of="2024-01-01", periods="2024Q1..2024Q4")
    .entity("OpCo", model=opco_model, capital=opco_capital)
    .position(
        Position(
            id="TL_A",
            entity="OpCo",
            instrument="TERM_LOAN_A",
            quantity=10_000_000,
            unit="face_value",
            open_date="2024-01-01",
        )
    )
    .book(
        Book("ROOT", "Fund")
            .add_book(Book("LOANS", "Loan Portfolio")
                .add_position("TL_A"))
    )
    .alias("EBITDA", {"OpCo": "ebitda_node"})
    .build())

# Run evaluation
runner = PortfolioRunner(parallel=True)
results = runner.run(portfolio, market_data)

# Access results
print(f"Portfolio Value: ${results.valuation.total_base:,.2f}")
 

# DataFrame integration
df = results.to_dataframe()  # Returns Polars DataFrame
```

---

## 13) WASM Bindings

```typescript
// TypeScript API sketch
import { Portfolio, Position, Book } from 'finstack-wasm';

// Builder pattern (similar to Python)
const portfolio = Portfolio.builder("FUND_A")
  .plan({ baseCcy: "USD", asOf: "2024-01-01", periods: "2024Q1..2024Q4" })
  .entity("OpCo", { model: opcoModel, capital: opcoCapital })
  .position({
    id: "TL_A",
    entity: "OpCo",
    instrument: "TERM_LOAN_A",
    quantity: 10_000_000,
    unit: "face_value",
    openDate: "2024-01-01",
  })
  .book(
    new Book("ROOT", "Fund")
      .addBook(new Book("LOANS", "Loan Portfolio")
        .addPosition("TL_A"))
  )
  .build();

// Run evaluation
const runner = new PortfolioRunner({ parallel: false }); // No threads in WASM
const results = await runner.run(portfolio, marketData);

// Access results
console.log(`Portfolio Value: $${results.valuation.totalBase.toFixed(2)}`);
```

---

## 14) Summary

The `/portfolio` crate provides:

1. **Type-safe portfolio modeling** with entities, positions, and books
2. **Currency-safe aggregation** requiring explicit FX conversion
3. **Period alignment** for cross-entity statement aggregation
4. 
5. **Flexible aggregation** by books, entities, or tags
6. **Scenario integration** via portfolio-specific paths
7. **Efficient evaluation** with caching and optional parallelization

Key design principles:
* **Composability**: Built on core, statements, and valuations
* **Determinism**: Stable ordering and reduction for reproducible results
* **Safety**: Currency-aware operations, validated references
* **Performance**: Parallel evaluation with caching
* **Extensibility**: Tag-based grouping, custom aggregation methods

Integration points:
* Uses core's FX infrastructure and period system
* Uses statements' financial models and metrics
* Uses valuations' pricing and risk engines
* Provides scenario paths for portfolio modifications
* Exports structured results for analysis

This design ensures the portfolio crate integrates seamlessly with the rest of the finstack architecture while providing robust portfolio management capabilities.
