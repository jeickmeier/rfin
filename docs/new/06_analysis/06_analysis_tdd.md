# Analysis Crate — Technical Design

**Version:** 1.0
**Status:** Design complete
**Audience:** Library authors, maintainers, and advanced integrators (Python/WASM)

---

## Executive Summary

The **Analysis crate** provides a flexible, extensible plugin architecture for financial analysis operations on top of the core finstack infrastructure. It enables sophisticated model analysis, sensitivity testing, scenario comparison, and diagnostic reporting through a unified interface that works seamlessly across Rust, Python, and WASM bindings.

---

## 1) Design Principles

1. **Reuse-First Orchestration**: Always delegate to `finstack-core`, `finstack-statements`, `finstack-valuations`, `finstack-scenarios` (and `finstack-portfolio` when needed) before adding new functionality. No reimplementation of pricing, cashflow generation, statement evaluation, scenario engines, or math kernels in this crate.
2. **Plugin Architecture**: Extensible analyzer registry with both compile-time and runtime registration
3. **Schema-Driven**: JSON Schema validation for analyzer parameters ensuring type safety across FFI boundaries
4. **Composability**: Analyzers can compose and delegate to other analyzers for complex workflows
5. **Performance**: Parallel execution where appropriate with deterministic aggregation
6. **Cross-Language**: Consistent behavior and results across Rust, Python, and WASM
7. **Auditability**: Full traceability of analysis inputs, parameters, and intermediate calculations

---

## 2) Architecture Overview

### 2.0 Overall Conformance & Boundaries

- Analysis is an orchestration layer. It reuses statements evaluation, valuations/pricing, and scenario execution from their respective crates rather than implementing them locally.
- No direct numeric kernels (discounting, accruals, interpolation), statement evaluators, or scenario engines are implemented here.
- No direct Polars/Arrow dependencies in Analysis; time-series handling occurs through dependent crates that standardize on Polars.

### 2.1 Crate Structure

```
/analysis
  ├── src
  │   ├── lib.rs                    // public API and re-exports
  │   ├── analyzer/
  │   │   ├── mod.rs                 // Analyzer trait and registry
  │   │   ├── meta.rs                // AnalyzerMeta and capabilities
  │   │   └── registry.rs            // Plugin registration system
  │   ├── builtin/
  │   │   ├── mod.rs                 // Built-in analyzer implementations
  │   │   ├── validation_report.rs   // Model validation analyzer
  │   │   ├── node_explainer.rs      // Node dependency explanations
  │   │   ├── sensitivity.rs         // Single-variable sensitivity
  │   │   ├── grid.rs                 // Multi-dimensional grid analysis
  │   │   ├── waterfall.rs            // Period-over-period waterfalls
  │   │   ├── waterfall_grid.rs      // Multi-node waterfall grids
  │   │   ├── recovery.rs            // Recovery rate analysis
  │   │   └── implied_ratings.rs     // Credit rating implications
  │   ├── compose/
  │   │   ├── mod.rs                 // Analyzer composition utilities
  │   │   ├── pipeline.rs            // Analysis pipeline builder
  │   │   └── aggregator.rs          // Result aggregation patterns
  │   ├── schemas/
  │   │   ├── mod.rs                 // Schema generation and validation
  │   │   ├── params.rs              // Parameter schema definitions
  │   │   └── results.rs             // Result schema definitions
  │   ├── parallel/
  │   │   ├── mod.rs                 // Parallel execution framework
  │   │   ├── scheduler.rs           // Work scheduling and distribution
  │   │   └── aggregation.rs         // Deterministic parallel aggregation
  │   ├── cache/
  │   │   ├── mod.rs                 // Analysis caching layer
  │   │   └── key.rs                 // Cache key generation
  │   ├── error.rs                   // Error types and handling
  │   └── config.rs                  // Configuration and feature flags
  ├── benches/                       // Performance benchmarks
  └── tests/                         // Integration and property tests
```

### 2.2 Dependencies

Per the overall architecture (Section 5 in `overall.md`):

```rust
// Cargo.toml
[dependencies]
finstack-core = { workspace = true }
finstack-statements = { workspace = true }
finstack-valuations = { workspace = true }
finstack-scenarios = { workspace = true }
finstack-portfolio = { workspace = true, optional = true }

serde = { workspace = true }
serde_json = { workspace = true }
schemars = "0.8"                   // JSON Schema generation
thiserror = { workspace = true }
indexmap = { workspace = true }
rayon = { workspace = true, optional = true }
tracing = { workspace = true }

# Optional link-time registration
linkme = { version = "0.3", optional = true }
inventory = { version = "0.3", optional = true }
ctor = { version = "0.2", optional = true }

[features]
default = ["parallel"]
parallel = ["rayon"]
link-registration = ["linkme", "ctor"]
distributed-registration = ["inventory"]
portfolio = ["dep:finstack-portfolio"]

// Note: Analysis declares no direct Polars/Arrow dependency; reuse through dependent crates.
```

---

## 3) Core Analyzer Interface

### 3.1 Analyzer Trait

As specified in `overall.md` Section 5.1:

```rust
pub trait Analyzer: Send + Sync {
    /// Metadata describing the analyzer's capabilities
    fn meta(&self) -> AnalyzerMeta;
    
    /// Execute the analysis with validated parameters
    fn analyze(
        &self,
        model: &FinancialModel,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, FinstackError>;
    
    /// JSON Schema for parameter validation
    fn param_schema(&self) -> serde_json::Value;
    
    /// Optional: schema for result structure
    fn result_schema(&self) -> Option<serde_json::Value> {
        None
    }
    
    /// Optional: supports incremental/streaming analysis
    fn supports_streaming(&self) -> bool {
        false
    }
    
    /// Optional: estimate computational complexity
    fn complexity_hint(&self, model: &FinancialModel) -> ComplexityHint {
        ComplexityHint::default()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalyzerMeta {
    pub id: String,
    pub name: String,
    pub version: semver::Version,
    pub description: String,
    pub category: AnalyzerCategory,
    pub tags: Vec<String>,
    pub capabilities: AnalyzerCapabilities,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AnalyzerCategory {
    Validation,
    Sensitivity,
    Diagnostic,
    Waterfall,
    Credit,
    Performance,
    Custom(String),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AnalyzerCapabilities {
    pub supports_partial_models: bool,
    pub supports_multi_entity: bool,
    pub supports_scenarios: bool,
    pub is_deterministic: bool,
    pub is_cacheable: bool,
    pub max_parallel_instances: Option<usize>,
}

#[derive(Clone, Debug)]
pub enum ComplexityHint {
    Constant,
    Linear { factor: f64 },
    Quadratic { factor: f64 },
    Exponential { base: f64 },
    Custom(Box<dyn Fn(usize) -> f64 + Send + Sync>),
}
```

### 3.2 Registration System

Per `overall.md` Section 5.2:

```rust
pub struct AnalyzerRegistry {
    analyzers: RwLock<IndexMap<String, Arc<dyn Analyzer>>>,
    metadata_cache: RwLock<HashMap<String, AnalyzerMeta>>,
}

impl AnalyzerRegistry {
    /// Manual registration (always supported)
    pub fn register(
        &self,
        name: &str,
        analyzer: Box<dyn Analyzer>,
    ) -> Result<(), RegistrationError> {
        let meta = analyzer.meta();
        if self.analyzers.read().contains_key(name) {
            return Err(RegistrationError::DuplicateName(name.to_string()));
        }
        
        self.analyzers.write().insert(name.to_string(), Arc::from(analyzer));
        self.metadata_cache.write().insert(name.to_string(), meta);
        Ok(())
    }
    
    /// Get analyzer by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Analyzer>> {
        self.analyzers.read().get(name).cloned()
    }
    
    /// List all registered analyzers
    pub fn list(&self) -> Vec<AnalyzerMeta> {
        self.metadata_cache.read().values().cloned().collect()
    }
    
    /// Bulk registration from a module
    pub fn register_module(&self, module: AnalyzerModule) -> Result<(), RegistrationError> {
        for (name, analyzer) in module.analyzers() {
            self.register(name, analyzer)?;
        }
        Ok(())
    }
}

// Link-time registration (optional feature)
#[cfg(feature = "link-registration")]
#[linkme::distributed_slice]
pub static BUILTIN_ANALYZERS: [fn() -> (&'static str, Box<dyn Analyzer>)];

#[cfg(feature = "link-registration")]
pub fn auto_register_builtins(registry: &AnalyzerRegistry) {
    for initializer in BUILTIN_ANALYZERS {
        let (name, analyzer) = initializer();
        registry.register(name, analyzer).expect("builtin registration failed");
    }
}
```

---

## 4) Built-in Analyzers

### 4.1 Validation Report

Comprehensive model validation with articulation tests:

```rust
pub struct ValidationReportAnalyzer;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ValidationParams {
    pub tolerance: Option<Decimal>,
    pub check_articulation: bool,
    pub check_formulas: bool,
    pub check_periods: bool,
    pub check_currency_consistency: bool,
    pub strict_mode: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationReport {
    pub passed: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub coverage: ValidationCoverage,
    pub articulation_results: Option<ArticulationResults>,
}

impl Analyzer for ValidationReportAnalyzer {
    fn analyze(
        &self,
        model: &FinancialModel,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, FinstackError> {
        let params: ValidationParams = serde_json::from_value(args)?;
        
        // Use core validation framework
        let validator = finstack_core::validation::ModelValidator::new(params.into());
        let result = validator.validate(model)?;
        
        Ok(serde_json::to_value(result)?)
    }
    
    fn param_schema(&self) -> serde_json::Value {
        schemars::schema_for!(ValidationParams)
    }
}
```

### 4.2 Node Explainer

Explains node dependencies and calculation paths:

```rust
pub struct NodeExplainerAnalyzer;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NodeExplainerParams {
    pub node_id: String,
    pub period_id: Option<String>,
    pub max_depth: Option<usize>,
    pub include_formulas: bool,
    pub include_values: bool,
    pub trace_path: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeExplanation {
    pub node_id: String,
    pub formula: Option<String>,
    pub dependencies: Vec<NodeDependency>,
    pub calculation_path: Option<Vec<CalculationStep>>,
    pub final_value: Option<AmountOrScalar>,
    pub metadata: IndexMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeDependency {
    pub node_id: String,
    pub dependency_type: DependencyType,
    pub periods_affected: Vec<String>,
    pub is_circular: bool,
}

impl Analyzer for NodeExplainerAnalyzer {
    fn analyze(
        &self,
        model: &FinancialModel,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, FinstackError> {
        let params: NodeExplainerParams = serde_json::from_value(args)?;
        
        // Build dependency graph using core expression engine
        let dag = finstack_core::expr::ExprBuilder::from_model(model)?;
        let explanation = self.explain_node(&dag, &params)?;
        
        Ok(serde_json::to_value(explanation)?)
    }
}
```

### 4.3 Sensitivity Analysis

Single and multi-variable sensitivity testing:

```rust
pub struct SensitivityAnalyzer;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SensitivityParams {
    pub target_nodes: Vec<String>,
    pub variables: Vec<SensitivityVariable>,
    pub output_nodes: Vec<String>,
    pub parallel: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SensitivityVariable {
    pub node_id: String,
    pub range: SensitivityRange,
    pub base_case_override: Option<Decimal>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum SensitivityRange {
    Percentage { min: f64, max: f64, steps: usize },
    Absolute { min: Decimal, max: Decimal, steps: usize },
    BasisPoints { min: i32, max: i32, steps: usize },
    Custom { values: Vec<Decimal> },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SensitivityResults {
    pub base_case: IndexMap<String, Decimal>,
    pub sensitivities: Vec<SensitivityResult>,
    pub statistics: SensitivityStatistics,
}

impl Analyzer for SensitivityAnalyzer {
    fn analyze(
        &self,
        model: &FinancialModel,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, FinstackError> {
        let params: SensitivityParams = serde_json::from_value(args)?;

        // Build scenarios via scenarios crate and execute using its engine.
        let grid = finstack_scenarios::builders::SensitivityGrid::from_params(&params)?;
        let scenarios = grid.to_scenarios();

        let mut engine = finstack_scenarios::ScenarioEngine::new();
        let results = engine.execute_many(model, &scenarios, |ctx| {
            // Delegate evaluation to statements (and valuations where applicable)
            let evaluator = finstack_statements::Evaluator::new();
            evaluator.evaluate(ctx.model(), params.parallel.unwrap_or(false))
        })?;

        Ok(serde_json::to_value(results)?)
    }
}
```

Notes (reuse-first):
- Scenario construction must use `finstack-scenarios` builders/DSL.
- Output calculation must use `finstack-statements::Evaluator` and/or `finstack-valuations` when pricing is needed.
- Parallelism follows workspace policy; no bespoke numeric kernels in Analysis.

### 4.4 Grid Analysis

Multi-dimensional parameter sweep:

```rust
pub struct GridAnalyzer;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GridParams {
    pub dimensions: Vec<GridDimension>,
    pub output_nodes: Vec<String>,
    pub aggregations: Vec<AggregationType>,
    pub parallel: Option<bool>,
    pub cache_intermediate: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GridDimension {
    pub name: String,
    pub node_id: String,
    pub values: Vec<Decimal>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GridResults {
    pub dimensions: Vec<GridDimension>,
    // Represent grid via stable maps to avoid introducing new numeric backends here
    pub grid: IndexMap<Vec<usize>, GridCell>,
    pub aggregates: IndexMap<String, AggregateResult>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GridCell {
    pub coordinates: Vec<usize>,
    pub inputs: IndexMap<String, Decimal>,
    pub outputs: IndexMap<String, Decimal>,
    pub valid: bool,
    pub errors: Vec<String>,
}
```

Notes (reuse-first):
- Build the Cartesian product using `finstack-scenarios` and execute via its engine.
- For each point, evaluate with `statements::Evaluator` and/or `valuations`.
- Avoid adding `ndarray` or similar numeric crates to Analysis.

### 4.5 Waterfall Analysis

Period-over-period change attribution:

```rust
pub struct WaterfallAnalyzer;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaterfallParams {
    pub node_id: String,
    pub start_period: String,
    pub end_period: String,
    pub breakdown_nodes: Option<Vec<String>>,
    pub include_fx_impact: bool,
    pub include_volume_price: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WaterfallResults {
    pub start_value: Decimal,
    pub end_value: Decimal,
    pub total_change: Decimal,
    pub components: Vec<WaterfallComponent>,
    pub bridge_chart_data: BridgeChartData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WaterfallComponent {
    pub name: String,
    pub category: ComponentCategory,
    pub impact: Decimal,
    pub percentage_of_change: Decimal,
    pub sub_components: Option<Vec<WaterfallComponent>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ComponentCategory {
    Organic,
    Price,
    Volume,
    Mix,
    FX,
    Acquisition,
    Divestiture,
    Other(String),
}
```

Note: Parallelism in Analysis is a thin orchestration over `rayon` and callee crates' controls. Prefer delegating work to `scenarios`, `statements`, and `valuations` rather than introducing additional scheduling here.

### 4.6 Recovery Analysis

Recovery rate and loss given default analysis:

```rust
pub struct RecoveryAnalyzer;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RecoveryParams {
    pub entity_id: String,
    pub recovery_scenarios: Vec<RecoveryScenario>,
    pub valuation_date: String,
    pub include_waterfalls: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RecoveryScenario {
    pub name: String,
    pub recovery_rate: Decimal,
    pub time_to_recovery_months: u32,
    pub costs: RecoveryCosts,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecoveryResults {
    pub scenarios: IndexMap<String, ScenarioResult>,
    pub expected_recovery: Decimal,
    pub loss_given_default: Decimal,
    pub recovery_waterfalls: Option<IndexMap<String, RecoveryWaterfall>>,
}
```

### 4.7 Implied Ratings

Credit rating implications from financial metrics:

```rust
pub struct ImpliedRatingsAnalyzer;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ImpliedRatingsParams {
    pub entity_id: String,
    pub rating_methodology: RatingMethodology,
    pub peer_group: Option<Vec<String>>,
    pub custom_weights: Option<IndexMap<String, f64>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub enum RatingMethodology {
    MoodysKmv,
    StandardAndPoors,
    Fitch,
    Internal(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImpliedRatingsResults {
    pub implied_rating: String,
    pub rating_score: Decimal,
    pub confidence_interval: (String, String),
    pub key_metrics: IndexMap<String, MetricAssessment>,
    pub peer_comparison: Option<PeerComparison>,
}
```

### 4.8 Scenario Explainer (Introspection & Debuggability)

Purpose: user‑facing introspection of scenarios that explains selector expansion, composition/conflict resolution, the final ordered execution plan, and a before/after impact preview. This analyzer is a thin wrapper over the scenarios engine "preview" capability and adapter previews (§8.1 in Scenarios TDD).

```rust
pub struct ScenarioExplainerAnalyzer;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ScenarioExplainerParams {
    /// Scenario as DSL text (mutually exclusive with `scenario`)
    #[serde(default)]
    pub scenario_dsl: Option<String>,

    /// Scenario as structured spec (mutually exclusive with `scenario_dsl`)
    #[serde(default)]
    pub scenario: Option<finstack_scenarios::ScenarioSpec>,

    /// Valuation/evaluation as-of date for time-window filtering
    pub as_of: time::Date,

    /// Strict/Lenient/Preview; defaults to Preview
    #[serde(default)]
    pub mode: Option<finstack_scenarios::ExecutionMode>,

    /// Include before/after impact preview per operation target
    #[serde(default)]
    pub include_impact_preview: bool,

    /// Override global limit for glob expansion during preview
    #[serde(default)]
    pub glob_max_matches: Option<usize>,

    /// Optional override for conflict strategy during composition
    #[serde(default)]
    pub conflict_strategy: Option<finstack_scenarios::ConflictStrategy>,

    /// Optional market data required for market/valuation previews
    #[serde(default)]
    pub market: Option<finstack_valuations::MarketData>,

    /// Optional portfolio required for portfolio previews
    #[serde(default)]
    pub portfolio: Option<finstack_portfolio::Portfolio>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ExplainedOperation {
    pub operation_id: String,
    pub normalized_path: String,
    pub phase: finstack_scenarios::Phase,
    pub priority: i32,
    pub declaration_index: usize,
    #[serde(default)]
    pub effective: Option<time::Date>,
    #[serde(default)]
    pub expires: Option<time::Date>,
    /// Canonical identifiers of concrete targets (e.g., curve ids, instrument ids, node ids)
    pub target_summary: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CompositionConflict {
    pub path: String,
    pub kept_operation_id: String,
    pub dropped_operation_ids: Vec<String>,
    pub strategy: finstack_scenarios::ConflictStrategy,
    pub reason: String, // e.g., "priority lower wins", "LastWins at same priority"
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ScenarioExplainerReport {
    /// Copy of effective composition rules used during planning
    pub composition: finstack_scenarios::CompositionRules,

    /// Final ordered list of concrete operations to be applied
    pub operations_final: Vec<ExplainedOperation>,

    /// Per original op, the glob/selector expansion list and truncation flag
    pub expansions: indexmap::IndexMap<String, finstack_scenarios::GlobExpansion>,

    /// Conflicts encountered and how they were resolved
    #[serde(default)]
    pub conflicts: Vec<CompositionConflict>,

    /// Optional before/after previews per target (only when include_impact_preview=true)
    #[serde(default)]
    pub impacts: Vec<finstack_scenarios::ImpactPreview>,
}

impl Analyzer for ScenarioExplainerAnalyzer {
    fn meta(&self) -> AnalyzerMeta { /* id: "scenario_explainer", category: Diagnostic */ }

    fn analyze(
        &self,
        model: &FinancialModel,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, FinstackError> {
        use finstack_scenarios as scn;

        let params: ScenarioExplainerParams = serde_json::from_value(args)?;
        let mut engine = scn::ScenarioEngine::new();

        // Build scenario from DSL or spec
        let spec = if let Some(dsl) = params.scenario_dsl.as_ref() {
            scn::ScenarioParser::new().parse(dsl)?
        } else {
            params.scenario.ok_or_else(|| FinstackError::InvalidInput("missing scenario".into()))?
        };

        // Compile to runtime Scenario
        let builder = scn::ScenarioBuilder::new(&finstack_core::expr::ExprCompiler::default());
        let mut scenario = builder.compile(spec)?;

        // Apply optional overrides
        if let Some(cs) = params.conflict_strategy { /* set on scenario/rules */ }
        let mode = params.mode.unwrap_or(scn::ExecutionMode::Preview);
        scenario.mode = mode;

        // Construct execution context (market/portfolio optional)
        let mut ctx = scn::ExecutionContext::from_model(model.clone())
            .with_as_of(params.as_of)
            .with_optional_market(params.market)
            .with_optional_portfolio(params.portfolio);

        if let Some(limit) = params.glob_max_matches { engine.set_glob_limit(limit); }

        // Obtain preview (plan + adapter-level previews)
        let preview = engine.preview(&scenario, &ctx)?;

        // Normalize into ScenarioExplainerReport
        let report = ScenarioExplainerReport {
            composition: preview.composition.clone(),
            operations_final: preview.plan.phases.iter()
                .flat_map(|(_, ops)| ops)
                .map(|ro| ExplainedOperation {
                    operation_id: ro.operation.id.clone(),
                    normalized_path: scn::normalize_path(&ro.operation.path),
                    phase: ro.phase,
                    priority: ro.operation.priority,
                    declaration_index: ro.operation.declaration_index,
                    effective: ro.operation.effective,
                    expires: ro.operation.expires,
                    target_summary: ro.target.describe_canonical_keys(),
                })
                .collect(),
            expansions: preview.expansions.clone(),
            conflicts: preview.metadata.composition_conflicts.clone(),
            impacts: if params.include_impact_preview { preview.impacts } else { vec![] },
        };

        Ok(serde_json::to_value(report)?)
    }

    fn param_schema(&self) -> serde_json::Value {
        schemars::schema_for!(ScenarioExplainerParams)
    }
}
```

Notes:
- Selector/glob expansion and truncation visibility reuse `GlobExpansion` from the scenarios preview API.
- Conflict narration relies on composition metadata captured during planning; reasons are normalized strings for auditability.
- Impact previews call adapter `preview` methods; providing `market`/`portfolio` enables full coverage beyond statements.

Python usage:

```python
from finstack.analysis import Analyzer

explainer = Analyzer.get("scenario_explainer")
report = explainer.analyze(
    model,
    {
        "scenario_dsl": 'market.fx.USD/EUR:+%2\nvaluations.instruments?{rating:"CCC"}.spread:+bp50',
        "as_of": "2025-03-31",
        "include_impact_preview": True,
        "market": market.to_dict(),      # or pydantic model .model_dump()
        # "portfolio": portfolio.to_dict(),
    },
)

# report["expansions"], report["composition"], report["operations_final"], report["impacts"]
```

WASM/TypeScript usage:

```typescript
import { createAnalyzer } from "@finstack/analysis-wasm";

const analyzer = createAnalyzer("scenario_explainer");
const report = await analyzer.analyze(model, {
  scenario_dsl: 'market.curves.USD_*:+bp25',
  as_of: "2025-03-31",
  include_impact_preview: true,
  market,
});

// report.expansions, report.composition, report.operations_final, report.impacts
```

---

## 5) Composition and Pipelines

### 5.1 Pipeline Builder

Fluent API for composing multiple analyses:

```rust
pub struct AnalysisPipeline {
    steps: Vec<PipelineStep>,
    context: PipelineContext,
}

pub struct PipelineStep {
    pub name: String,
    pub analyzer: Arc<dyn Analyzer>,
    pub params: serde_json::Value,
    pub dependencies: Vec<String>,
    pub on_error: ErrorStrategy,
}

pub enum ErrorStrategy {
    Fail,
    Skip,
    UseDefault(serde_json::Value),
    Retry { max_attempts: usize, backoff_ms: u64 },
}

impl AnalysisPipeline {
    pub fn builder() -> PipelineBuilder {
        PipelineBuilder::new()
    }
    
    pub async fn execute(
        &self,
        model: &FinancialModel,
    ) -> Result<PipelineResults, FinstackError> {
        let scheduler = PipelineScheduler::new(&self.steps);
        let execution_plan = scheduler.build_plan()?;
        
        let mut results = PipelineResults::new();
        for stage in execution_plan.stages {
            let stage_results = self.execute_stage(model, &stage).await?;
            results.merge(stage_results);
        }
        
        Ok(results)
    }
}

pub struct PipelineBuilder {
    steps: Vec<PipelineStep>,
}

impl PipelineBuilder {
    pub fn add<A: Analyzer + 'static>(
        mut self,
        name: &str,
        analyzer: A,
        params: impl Serialize,
    ) -> Result<Self, BuildError> {
        let step = PipelineStep {
            name: name.to_string(),
            analyzer: Arc::new(analyzer),
            params: serde_json::to_value(params)?,
            dependencies: vec![],
            on_error: ErrorStrategy::Fail,
        };
        self.steps.push(step);
        Ok(self)
    }
    
    pub fn depends_on(mut self, step: &str, deps: Vec<&str>) -> Self {
        if let Some(s) = self.steps.iter_mut().find(|s| s.name == step) {
            s.dependencies = deps.iter().map(|d| d.to_string()).collect();
        }
        self
    }
    
    pub fn on_error(mut self, step: &str, strategy: ErrorStrategy) -> Self {
        if let Some(s) = self.steps.iter_mut().find(|s| s.name == step) {
            s.on_error = strategy;
        }
        self
    }
    
    pub fn build(self) -> Result<AnalysisPipeline, BuildError> {
        // Validate DAG, check for cycles
        let dag = self.validate_dependencies()?;
        
        Ok(AnalysisPipeline {
            steps: self.steps,
            context: PipelineContext::new(),
        })
    }
}
```

### 5.2 Result Aggregation

Patterns for combining analyzer outputs:

```rust
pub trait ResultAggregator {
    type Input;
    type Output;
    
    fn aggregate(&self, results: Vec<Self::Input>) -> Result<Self::Output, AggregationError>;
}

pub struct WeightedAggregator {
    weights: IndexMap<String, f64>,
}

pub struct HierarchicalAggregator {
    hierarchy: TreeStructure,
    leaf_aggregator: Box<dyn ResultAggregator>,
}

pub struct ConditionalAggregator {
    conditions: Vec<(Predicate, Box<dyn ResultAggregator>)>,
    default: Box<dyn ResultAggregator>,
}
```

---

## 6) Parallel Execution

### 6.1 Parallel Framework

Deterministic parallel execution with stable aggregation:

```rust
pub struct ParallelExecutor {
    thread_pool: Option<rayon::ThreadPool>,
    deterministic: bool,
}

impl ParallelExecutor {
    pub fn execute_batch<T, F>(
        &self,
        items: Vec<T>,
        analyzer: F,
    ) -> Result<Vec<AnalysisResult>, FinstackError>
    where
        T: Send + 'static,
        F: Fn(T) -> Result<AnalysisResult, FinstackError> + Sync,
    {
        if self.deterministic {
            // Use stable parallel iteration
            items
                .par_iter()
                .enumerate()
                .map(|(idx, item)| (idx, analyzer(item)))
                .collect::<Vec<_>>()
                .into_iter()
                .sorted_by_key(|(idx, _)| *idx)
                .map(|(_, result)| result)
                .collect()
        } else {
            // Regular parallel execution
            items.par_iter().map(analyzer).collect()
        }
    }
}
```

### 6.2 Work Distribution

Smart scheduling based on complexity hints:

```rust
pub struct WorkScheduler {
    complexity_threshold: f64,
    max_parallel_jobs: usize,
}

impl WorkScheduler {
    pub fn schedule(
        &self,
        jobs: Vec<AnalysisJob>,
    ) -> Vec<ExecutionBatch> {
        let mut batches = vec![];
        let mut current_batch = ExecutionBatch::new();
        let mut current_complexity = 0.0;
        
        for job in jobs {
            let complexity = job.complexity_hint.estimate(job.input_size);
            
            if complexity > self.complexity_threshold {
                // Execute heavy jobs separately
                if !current_batch.is_empty() {
                    batches.push(current_batch);
                    current_batch = ExecutionBatch::new();
                }
                batches.push(ExecutionBatch::single(job));
            } else if current_complexity + complexity > self.complexity_threshold {
                // Start new batch
                batches.push(current_batch);
                current_batch = ExecutionBatch::new();
                current_batch.add(job);
                current_complexity = complexity;
            } else {
                // Add to current batch
                current_batch.add(job);
                current_complexity += complexity;
            }
        }
        
        if !current_batch.is_empty() {
            batches.push(current_batch);
        }
        
        batches
    }
}
```

---

## 7) Caching Layer

### 7.1 Cache Implementation

Content-addressed caching with TTL:

```rust
pub struct AnalysisCache {
    store: Arc<RwLock<HashMap<CacheKey, CachedResult>>>,
    ttl: Duration,
    max_size: usize,
}

#[derive(Hash, Eq, PartialEq)]
pub struct CacheKey {
    analyzer_id: String,
    model_hash: u64,
    params_hash: u64,
    version: semver::Version,
}

pub struct CachedResult {
    pub value: serde_json::Value,
    pub created_at: Instant,
    pub access_count: AtomicUsize,
    pub computation_time_ms: u64,
}

impl AnalysisCache {
    pub fn get_or_compute<F>(
        &self,
        key: &CacheKey,
        compute: F,
    ) -> Result<serde_json::Value, FinstackError>
    where
        F: FnOnce() -> Result<serde_json::Value, FinstackError>,
    {
        // Check cache
        if let Some(cached) = self.get(key) {
            if cached.created_at.elapsed() < self.ttl {
                cached.access_count.fetch_add(1, Ordering::Relaxed);
                return Ok(cached.value.clone());
            }
        }
        
        // Compute and cache
        let start = Instant::now();
        let result = compute()?;
        let computation_time_ms = start.elapsed().as_millis() as u64;
        
        self.put(key.clone(), CachedResult {
            value: result.clone(),
            created_at: Instant::now(),
            access_count: AtomicUsize::new(1),
            computation_time_ms,
        });
        
        Ok(result)
    }
}
```

### 7.2 Cache Key Generation

Stable hashing for cache keys:

```rust
pub fn generate_cache_key(
    analyzer: &dyn Analyzer,
    model: &FinancialModel,
    params: &serde_json::Value,
) -> CacheKey {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    
    let mut hasher = DefaultHasher::new();
    
    // Hash model (stable serialization)
    let model_bytes = bincode::serialize(model).unwrap();
    model_bytes.hash(&mut hasher);
    let model_hash = hasher.finish();
    
    // Hash params (canonical JSON)
    let mut hasher = DefaultHasher::new();
    let canonical_params = canonicalize_json(params);
    canonical_params.hash(&mut hasher);
    let params_hash = hasher.finish();
    
    CacheKey {
        analyzer_id: analyzer.meta().id,
        model_hash,
        params_hash,
        version: analyzer.meta().version,
    }
}

fn canonicalize_json(value: &serde_json::Value) -> String {
    // Sort keys, normalize numbers, etc.
    serde_json::to_string(&sort_json_keys(value)).unwrap()
}
```

---

## 8) Schema Management

### 8.1 Schema Generation

Automatic schema generation from Rust types:

```rust
pub trait SchemaProvider {
    fn param_schema(&self) -> JsonSchema;
    fn result_schema(&self) -> Option<JsonSchema>;
}

#[macro_export]
macro_rules! impl_schema_provider {
    ($analyzer:ty, $params:ty, $results:ty) => {
        impl SchemaProvider for $analyzer {
            fn param_schema(&self) -> JsonSchema {
                schemars::schema_for!($params)
            }
            
            fn result_schema(&self) -> Option<JsonSchema> {
                Some(schemars::schema_for!($results))
            }
        }
    };
}

// Usage
impl_schema_provider!(ValidationReportAnalyzer, ValidationParams, ValidationReport);
```

### 8.2 Schema Validation

Runtime validation with helpful errors:

```rust
pub struct SchemaValidator {
    compiled_schemas: HashMap<String, jsonschema::JSONSchema>,
}

impl SchemaValidator {
    pub fn validate(
        &self,
        schema_id: &str,
        instance: &serde_json::Value,
    ) -> Result<(), ValidationError> {
        let schema = self.compiled_schemas.get(schema_id)
            .ok_or_else(|| ValidationError::UnknownSchema(schema_id.to_string()))?;
        
        match schema.validate(instance) {
            Ok(_) => Ok(()),
            Err(errors) => {
                let messages: Vec<String> = errors
                    .map(|e| format!("{}: {}", e.instance_path, e))
                    .collect();
                Err(ValidationError::SchemaViolation {
                    schema_id: schema_id.to_string(),
                    errors: messages,
                })
            }
        }
    }
}
```

---

## 9) Python Bindings

### 9.1 PyO3 Integration

Seamless Python interface maintaining type safety:

```rust
#[pymodule]
fn analysis(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyAnalyzer>()?;
    m.add_class::<PyAnalysisPipeline>()?;
    m.add_function(wrap_pyfunction!(py_register_analyzer, m)?)?;
    m.add_function(wrap_pyfunction!(py_list_analyzers, m)?)?;
    Ok(())
}

#[pyclass]
struct PyAnalyzer {
    inner: Arc<dyn Analyzer>,
}

#[pymethods]
impl PyAnalyzer {
    fn analyze(
        &self,
        py: Python,
        model: &PyFinancialModel,
        params: &PyDict,
    ) -> PyResult<PyObject> {
        py.allow_threads(|| {
            let params_json = python_dict_to_json(params)?;
            let result = self.inner.analyze(model.as_ref(), params_json)?;
            Ok(json_to_python(py, result)?)
        })
    }
    
    fn get_param_schema(&self, py: Python) -> PyResult<PyObject> {
        let schema = self.inner.param_schema();
        json_to_python(py, schema)
    }
}

// Custom analyzer from Python
#[pyclass]
struct PythonAnalyzer {
    callback: PyObject,
    meta: AnalyzerMeta,
    param_schema: serde_json::Value,
}

impl Analyzer for PythonAnalyzer {
    fn analyze(
        &self,
        model: &FinancialModel,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, FinstackError> {
        Python::with_gil(|py| {
            let model_py = model_to_python(py, model)?;
            let args_py = json_to_python(py, args)?;
            
            let result = self.callback.call1(py, (model_py, args_py))?;
            python_to_json(result.as_ref(py))
        })
    }
}
```

### 9.2 Python API

Pythonic interface with type hints:

```python
from finstack.analysis import (
    Analyzer,
    Pipeline,
    register_analyzer,
    list_analyzers,
    ValidationReport,
    SensitivityAnalysis,
    WaterfallAnalysis,
)
from finstack.statements import FinancialModel
from typing import Dict, Any, Optional

# Using built-in analyzer
def analyze_model(model: FinancialModel) -> ValidationReport:
    analyzer = Analyzer.get("validation_report")
    result = analyzer.analyze(
        model,
        params={
            "tolerance": 0.01,
            "check_articulation": True,
            "strict_mode": False
        }
    )
    return ValidationReport.from_dict(result)

# Custom analyzer
class MyCustomAnalyzer(Analyzer):
    def meta(self) -> Dict[str, Any]:
        return {
            "id": "my_custom",
            "name": "My Custom Analyzer",
            "version": "1.0.0",
            "description": "Custom analysis logic"
        }
    
    def analyze(
        self,
        model: FinancialModel,
        params: Dict[str, Any]
    ) -> Dict[str, Any]:
        # Custom analysis logic
        return {"result": "custom"}
    
    def param_schema(self) -> Dict[str, Any]:
        return {
            "type": "object",
            "properties": {
                "threshold": {"type": "number"}
            }
        }

# Register custom analyzer
register_analyzer("my_custom", MyCustomAnalyzer())

# Pipeline example
pipeline = (
    Pipeline.builder()
    .add("validate", "validation_report", {"strict_mode": True})
    .add("sensitivity", "sensitivity", {
        "variables": [{"node_id": "revenue", "range": {"type": "percentage", "min": -10, "max": 10}}]
    })
    .depends_on("sensitivity", ["validate"])
    .build()
)

results = pipeline.execute(model)
```

---

## 10) WASM Bindings

### 10.1 wasm-bindgen Integration

Browser-compatible analysis with minimal overhead:

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmAnalyzer {
    inner: Arc<dyn Analyzer>,
}

#[wasm_bindgen]
impl WasmAnalyzer {
    pub fn analyze(
        &self,
        model: &WasmFinancialModel,
        params: JsValue,
    ) -> Result<JsValue, JsError> {
        let params_json: serde_json::Value = serde_wasm_bindgen::from_value(params)?;
        let result = self.inner.analyze(model.as_ref(), params_json)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(serde_wasm_bindgen::to_value(&result)?)
    }
    
    pub fn param_schema(&self) -> Result<JsValue, JsError> {
        Ok(serde_wasm_bindgen::to_value(&self.inner.param_schema())?)
    }
}

#[wasm_bindgen]
pub fn create_analyzer(name: &str) -> Result<WasmAnalyzer, JsError> {
    let registry = get_global_registry();
    let analyzer = registry.get(name)
        .ok_or_else(|| JsError::new(&format!("Unknown analyzer: {}", name)))?;
    Ok(WasmAnalyzer { inner: analyzer })
}

#[wasm_bindgen]
pub fn list_available_analyzers() -> Result<JsValue, JsError> {
    let registry = get_global_registry();
    let analyzers = registry.list();
    Ok(serde_wasm_bindgen::to_value(&analyzers)?)
}
```

### 10.2 JavaScript/TypeScript API

Type-safe TypeScript definitions:

```typescript
// finstack-analysis.d.ts
export interface Analyzer {
    analyze(model: FinancialModel, params: any): Promise<any>;
    getParamSchema(): any;
    getResultSchema(): any | null;
}

export interface AnalyzerMeta {
    id: string;
    name: string;
    version: string;
    description: string;
    category: AnalyzerCategory;
    tags: string[];
    capabilities: AnalyzerCapabilities;
}

export type AnalyzerCategory = 
    | "Validation"
    | "Sensitivity"
    | "Diagnostic"
    | "Waterfall"
    | "Credit"
    | "Performance"
    | { Custom: string };

export interface Pipeline {
    add(name: string, analyzerId: string, params: any): Pipeline;
    dependsOn(step: string, deps: string[]): Pipeline;
    onError(step: string, strategy: ErrorStrategy): Pipeline;
    execute(model: FinancialModel): Promise<PipelineResults>;
}

export function createAnalyzer(name: string): Analyzer;
export function listAnalyzers(): AnalyzerMeta[];
export function createPipeline(): Pipeline;
```

---

## 11) Testing Strategy

### 11.1 Unit Tests

Core functionality testing:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_analyzer_registration() {
        let registry = AnalyzerRegistry::new();
        let analyzer = Box::new(ValidationReportAnalyzer);
        
        assert!(registry.register("test", analyzer).is_ok());
        assert!(registry.get("test").is_some());
        assert_eq!(registry.list().len(), 1);
    }
    
    #[test]
    fn test_schema_validation() {
        let analyzer = ValidationReportAnalyzer;
        let schema = analyzer.param_schema();
        
        let valid_params = json!({
            "tolerance": 0.01,
            "check_articulation": true
        });
        
        assert!(validate_against_schema(&schema, &valid_params).is_ok());
    }
    
    #[test]
    fn test_cache_key_stability() {
        let model1 = create_test_model();
        let model2 = create_test_model();
        let params = json!({"test": true});
        
        let key1 = generate_cache_key(&analyzer, &model1, &params);
        let key2 = generate_cache_key(&analyzer, &model2, &params);
        
        assert_eq!(key1, key2);
    }
}
```

### 11.2 Property Tests

Using proptest for invariant checking:

```rust
#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn test_deterministic_parallel_execution(
            model in arb_financial_model(),
            params in arb_analysis_params(),
        ) {
            let analyzer = SensitivityAnalyzer;
            
            let sequential_result = analyzer.analyze(&model, params.clone()).unwrap();
            let parallel_result = analyzer.analyze_parallel(&model, params).unwrap();
            
            prop_assert_eq!(sequential_result, parallel_result);
        }
        
        #[test]
        fn test_pipeline_dag_properties(
            steps in prop::collection::vec(arb_pipeline_step(), 1..20)
        ) {
            let pipeline = build_pipeline_from_steps(steps);
            
            prop_assert!(is_valid_dag(&pipeline));
            prop_assert!(has_deterministic_execution_order(&pipeline));
        }
    }
}
```

### 11.3 Integration Tests

End-to-end analysis workflows:

```rust
#[test]
fn test_full_analysis_workflow() {
    // Setup
    let model = load_test_model("sample_company.json");
    let registry = create_registry_with_builtins();
    
    // Validation
    let validator = registry.get("validation_report").unwrap();
    let validation_result = validator.analyze(&model, json!({
        "check_articulation": true
    })).unwrap();
    assert!(validation_result["passed"].as_bool().unwrap());
    
    // Sensitivity
    let sensitivity = registry.get("sensitivity").unwrap();
    let sensitivity_result = sensitivity.analyze(&model, json!({
        "variables": [{
            "node_id": "revenue",
            "range": {"type": "percentage", "min": -10, "max": 10, "steps": 5}
        }],
        "output_nodes": ["ebitda", "free_cash_flow"]
    })).unwrap();
    
    // Waterfall
    let waterfall = registry.get("waterfall").unwrap();
    let waterfall_result = waterfall.analyze(&model, json!({
        "node_id": "ebitda",
        "start_period": "2024Q1",
        "end_period": "2024Q4"
    })).unwrap();
    
    // Assert results are consistent
    assert_results_consistent(&validation_result, &sensitivity_result, &waterfall_result);
}
```

### 11.4 Benchmark Tests

Performance regression testing:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_sensitivity_analysis(c: &mut Criterion) {
    let model = create_large_test_model(1000); // 1000 nodes
    let analyzer = SensitivityAnalyzer;
    
    c.bench_function("sensitivity_10_variables", |b| {
        b.iter(|| {
            analyzer.analyze(
                black_box(&model),
                black_box(create_sensitivity_params(10))
            )
        })
    });
}

fn benchmark_parallel_execution(c: &mut Criterion) {
    let models: Vec<_> = (0..100).map(|_| create_test_model()).collect();
    
    c.bench_function("parallel_100_models", |b| {
        b.iter(|| {
            execute_parallel(black_box(&models))
        })
    });
}

criterion_group!(benches, benchmark_sensitivity_analysis, benchmark_parallel_execution);
criterion_main!(benches);
```

---

## 12) Error Handling

### 12.1 Error Types

Comprehensive error taxonomy (orchestration-first; surface callee-crate errors and add minimal context):

```rust
#[derive(thiserror::Error, Debug)]
pub enum AnalysisError {
    #[error("Analyzer not found: {0}")]
    AnalyzerNotFound(String),
    
    #[error("Invalid parameters: {0}")]
    InvalidParameters(#[from] serde_json::Error),
    
    #[error("Schema validation failed: {errors:?}")]
    SchemaValidation { errors: Vec<String> },
    
    #[error("Pipeline error at step {step}: {source}")]
    Pipeline { step: String, #[source] source: FinstackError },
    
    #[error(transparent)]
    Core(#[from] finstack_core::FinstackError),
    
    #[error(transparent)]
    Statements(#[from] finstack_statements::StatementsError),
    
    #[error(transparent)]
    Valuations(#[from] finstack_valuations::ValuationsError),
}
```

---

## 13) Configuration

### 13.1 Runtime Configuration

Flexible configuration system:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub parallel: ParallelConfig,
    pub cache: CacheConfig,
    pub limits: LimitConfig,
    pub tracing: TracingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelConfig {
    pub enabled: bool,
    pub max_threads: Option<usize>,
    pub chunk_size: usize,
    pub deterministic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl_seconds: u64,
    pub max_entries: usize,
    pub max_size_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitConfig {
    pub max_execution_time_seconds: u64,
    pub max_memory_mb: usize,
    pub max_pipeline_depth: usize,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            parallel: ParallelConfig {
                enabled: true,
                max_threads: None,
                chunk_size: 100,
                deterministic: true,
            },
            cache: CacheConfig {
                enabled: true,
                ttl_seconds: 3600,
                max_entries: 1000,
                max_size_mb: 100,
            },
            limits: LimitConfig {
                max_execution_time_seconds: 300,
                max_memory_mb: 1024,
                max_pipeline_depth: 10,
            },
            tracing: TracingConfig::default(),
        }
    }
}
```

---

## 14) Performance Considerations

### 14.1 Optimization Strategies

1. **Lazy Evaluation**: Defer computation until results are needed
2. **Memoization**: Cache intermediate results within analysis runs
3. **Vectorization**: Use SIMD operations where applicable
4. **Smart Scheduling**: Prioritize light computations, batch heavy ones
5. **Memory Pooling**: Reuse allocations across analysis runs

### 14.2 Performance Targets

Per `overall.md` Section 11.1:

- Single analysis on 10k-node model: < 500ms
- Pipeline with 5 analyzers: < 2s
- Sensitivity grid (10x10): < 5s
- Parallel analysis of 100 models: < 10s

---

## 15) Migration and Compatibility

### 15.1 Version Migration

Supporting evolution of analyzer interfaces:

```rust
pub trait AnalyzerMigration {
    fn migrate_params(
        &self,
        from_version: &semver::Version,
        to_version: &semver::Version,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MigrationError>;
    
    fn migrate_results(
        &self,
        from_version: &semver::Version,
        to_version: &semver::Version,
        results: serde_json::Value,
    ) -> Result<serde_json::Value, MigrationError>;
}

// Automatic migration on registration
impl AnalyzerRegistry {
    pub fn register_with_migration(
        &self,
        name: &str,
        analyzer: Box<dyn Analyzer>,
        migration: Option<Box<dyn AnalyzerMigration>>,
    ) -> Result<(), RegistrationError> {
        // Store migration handler alongside analyzer
        self.register(name, analyzer)?;
        if let Some(m) = migration {
            self.migrations.write().insert(name.to_string(), m);
        }
        Ok(())
    }
}
```

---

## 16) Security Considerations

### 16.1 Input Validation

- All analyzer parameters validated against schemas before execution
- Model integrity checked before analysis
- Resource limits enforced (memory, CPU time)
- No arbitrary code execution from parameters

### 16.2 Isolation

- Python custom analyzers run in restricted environment
- WASM analyzers sandboxed by browser
- File system access limited to configured directories
- Network access disabled by default

---

## 17) Future Enhancements

### 17.1 Advanced Features (Feature-Gated)

1. **Machine Learning Integration**: ML-based anomaly detection and forecasting
2. **Distributed Execution**: Cluster-based analysis for massive portfolios
3. **Real-time Streaming**: Incremental analysis on streaming data
4. **GPU Acceleration**: CUDA/OpenCL for intensive computations
5. **Natural Language Interface**: Query analysis via LLM integration

### 17.2 Extensibility Points

1. **Custom Aggregators**: User-defined result combination strategies
2. **Analysis Hooks**: Pre/post-processing interceptors
3. **Custom Schedulers**: Alternative work distribution algorithms
4. **External Storage**: S3/GCS/Azure blob storage for cache
5. **Metrics Export**: Prometheus/OpenTelemetry integration

---

## 18) Conclusion

The Analysis crate provides a robust, extensible foundation for financial model analysis that integrates seamlessly with the finstack ecosystem. Through its plugin architecture, schema-driven validation, and cross-language support, it enables sophisticated analysis workflows while maintaining the core principles of determinism, performance, and auditability established in the overall architecture.

Key achievements:

- **Unified Interface**: Single Analyzer trait powers all analysis types
- **Type Safety**: Schema validation across language boundaries
- **Performance**: Parallel execution with deterministic results
- **Extensibility**: Plugin system supports custom analyzers
- **Cross-Platform**: Consistent behavior in Rust, Python, and WASM

This design ensures the Analysis crate can evolve to meet future requirements while maintaining backward compatibility and stable interfaces for existing consumers.
