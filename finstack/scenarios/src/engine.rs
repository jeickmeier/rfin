//! Deterministic scenario execution engine.
//!
//! The engine glues together adapters from this crate to compose multiple
//! [`ScenarioSpec`](crate::spec::ScenarioSpec) definitions and apply them to
//! a mutable [`ExecutionContext`]. Its responsibilities are:
//! - enforce a repeatable ordering of operations
//! - delegate each `OperationSpec` variant to the appropriate adapter module
//! - collect reporting metadata about how many operations ran and whether any
//!   warnings were produced during execution

use crate::error::Result;
use crate::spec::{OperationSpec, RateBindingSpec, ScenarioSpec, VolSurfaceKind};
use finstack_core::market_data::hierarchy::{
    HierarchyNode, HierarchyTarget, MarketDataHierarchy, ResolutionMode, TagFilter,
};
use finstack_core::types::CurveId;
use finstack_core::{HashMap, HashSet};
use finstack_statements::types::NodeId;
use finstack_valuations::instruments::DynInstrument;
use indexmap::IndexMap;

fn rounding_stamp() -> Option<String> {
    Some(format!(
        "{:?}",
        finstack_core::config::RoundingMode::default()
    ))
}

/// Execution context for scenario application.
///
/// The context pins all mutable state that a scenario can touch — market data,
/// statement models, instrument inventories, and rate bindings — together with
/// the current valuation date.
///
/// # Fields
/// - `market`: Shared market data collection that stores curves, surfaces,
///   FX matrices, and spot prices.
/// - `model`: Financial statement model being shocked.
/// - `instruments`: Optional set of instruments to receive price/spread shocks
///   and to calculate carry/theta for time rolls.
/// - `rate_bindings`: Optional mapping from statement node identifiers to
///   detailed rate binding specs; used to sync statement rates after curve shocks.
/// - `calendar`: Optional holiday calendar for calendar-aware tenor calculations.
/// - `as_of`: Valuation date that operations reference.
///
/// # Examples
/// ```rust,no_run
/// use finstack_scenarios::ExecutionContext;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_statements::FinancialModelSpec;
/// use time::macros::date;
///
/// let mut market = MarketContext::new();
/// let mut model = FinancialModelSpec::new("demo", vec![]);
/// let as_of = date!(2025 - 01 - 01);
/// let ctx = ExecutionContext {
///     market: &mut market,
///     model: &mut model,
///     instruments: None,
///     rate_bindings: None,
///     calendar: None,
///     as_of,
/// };
///
/// assert_eq!(ctx.as_of, as_of);
/// ```
pub struct ExecutionContext<'a> {
    /// Market data context (curves, surfaces, FX, etc.).
    pub market: &'a mut finstack_core::market_data::context::MarketContext,

    /// Financial statements model.
    pub model: &'a mut finstack_statements::FinancialModelSpec,

    /// Optional vector of instruments for price/spread shocks and carry calculations.
    pub instruments: Option<&'a mut Vec<Box<DynInstrument>>>,

    /// Optional mapping from statement node IDs to binding specs for automatic rate updates.
    pub rate_bindings: Option<IndexMap<NodeId, RateBindingSpec>>,

    /// Optional holiday calendar for calendar-aware tenor calculations.
    pub calendar: Option<&'a dyn finstack_core::dates::HolidayCalendar>,

    /// Valuation date for context.
    pub as_of: time::Date,
}

/// Report describing what happened during [`ScenarioEngine::apply`].
///
/// # Examples
/// ```rust
/// use finstack_scenarios::engine::ApplicationReport;
///
/// let report = ApplicationReport {
///     operations_applied: 3,
///     user_operations: 1,
///     expanded_operations: 3,
///     warnings: vec!["fallback curve used".into()],
///     rounding_context: Some("default".into()),
/// };
///
/// assert_eq!(report.operations_applied, 3);
/// assert_eq!(report.user_operations, 1);
/// assert_eq!(report.expanded_operations, 3);
/// assert_eq!(report.warnings.len(), 1);
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApplicationReport {
    /// Number of effects successfully applied to the execution context.
    ///
    /// One user-level `OperationSpec` can produce multiple effects after
    /// hierarchy expansion (e.g. a single `CurveParallelBp` targeting the
    /// `USD` group may expand to one effect per USD-denominated discount or
    /// forward curve). Prefer `user_operations` for scenario-level reporting
    /// and this field for low-level audit.
    pub operations_applied: usize,
    /// Number of user-provided `OperationSpec` entries in the scenario
    /// (before hierarchy expansion and deduplication).
    pub user_operations: usize,
    /// Number of direct (non-hierarchy) operations produced after hierarchy
    /// expansion and resolution-mode deduplication. This is the count of
    /// operations that the engine actually tried to execute; it is always
    /// `>= user_operations` and is what should be compared to
    /// `operations_applied` when assessing scenario coverage.
    pub expanded_operations: usize,

    /// Warnings generated during application (non-fatal).
    pub warnings: Vec<String>,

    /// Rounding context stamp (for determinism tracking).
    pub rounding_context: Option<String>,
}

/// Tracks a hierarchy-expanded operation with metadata needed for deduplication.
struct HierarchyExpansion {
    /// Depth of the matched hierarchy node (deeper = more specific).
    matched_depth: usize,
    /// The expanded direct operation.
    operation: OperationSpec,
    /// Operation family + identifier used for resolution-mode deduplication.
    key: HierarchyExpansionKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum HierarchyExpansionKey {
    Curve {
        curve_kind: crate::spec::CurveKind,
        curve_id: CurveId,
    },
    VolSurface {
        surface_kind: VolSurfaceKind,
        surface_id: CurveId,
    },
    EquityPrice {
        price_id: CurveId,
    },
    BaseCorrelation {
        surface_id: CurveId,
    },
}

#[derive(Debug, Clone)]
struct HierarchyResolvedMatch {
    curve_id: CurveId,
    matched_depth: usize,
}

fn collect_subtree_matches(
    node: &HierarchyNode,
    matched_depth: usize,
    matches: &mut Vec<HierarchyResolvedMatch>,
) {
    for curve_id in node.curve_ids() {
        matches.push(HierarchyResolvedMatch {
            curve_id: curve_id.clone(),
            matched_depth,
        });
    }
    for child in node.children().values() {
        collect_subtree_matches(child, matched_depth, matches);
    }
}

fn collect_filtered_matches(
    node: &HierarchyNode,
    filter: &TagFilter,
    depth: usize,
    matches: &mut Vec<HierarchyResolvedMatch>,
) {
    if filter.matches(node.tags()) {
        collect_subtree_matches(node, depth, matches);
    }
    for child in node.children().values() {
        collect_filtered_matches(child, filter, depth + 1, matches);
    }
}

fn resolve_hierarchy_matches(
    hierarchy: &MarketDataHierarchy,
    target: &HierarchyTarget,
) -> Vec<HierarchyResolvedMatch> {
    let Some(node) = hierarchy.get_node(&target.path) else {
        return Vec::new();
    };

    let mut matches = Vec::new();
    let start_depth = target.path.len();
    match &target.tag_filter {
        None => collect_subtree_matches(node, start_depth, &mut matches),
        Some(filter) => collect_filtered_matches(node, filter, start_depth, &mut matches),
    }
    dedup_matches_keep_deepest(matches)
}

/// Collapse duplicate curve hits to a single match per `curve_id`, keeping the
/// deepest `matched_depth` seen for each. Required when a tag filter matches
/// both an ancestor node and one of its descendants: `collect_filtered_matches`
/// pushes the curves in the overlap twice (once as part of the ancestor's
/// subtree, once via the descendant match), which under `ResolutionMode::Cumulative`
/// would apply the same shock multiple times to one curve. Retaining the deepest
/// depth preserves `MostSpecificWins` semantics for cross-operation dedup.
fn dedup_matches_keep_deepest(matches: Vec<HierarchyResolvedMatch>) -> Vec<HierarchyResolvedMatch> {
    let mut best: HashMap<CurveId, usize> = HashMap::default();
    for m in &matches {
        best.entry(m.curve_id.clone())
            .and_modify(|d| *d = (*d).max(m.matched_depth))
            .or_insert(m.matched_depth);
    }
    let mut seen: HashSet<CurveId> = HashSet::default();
    let mut out = Vec::with_capacity(best.len());
    for m in matches {
        if seen.insert(m.curve_id.clone()) {
            let depth = best[&m.curve_id];
            out.push(HierarchyResolvedMatch {
                curve_id: m.curve_id,
                matched_depth: depth,
            });
        }
    }
    out
}

/// Expand hierarchy-targeted operations into direct-targeted operations.
///
/// - `Cumulative`: All matching hierarchy operations expand independently.
/// - `MostSpecificWins`: For each curve, only the deepest (longest path) hierarchy
///   operation applies.
///
/// When no hierarchy is attached to the market context, returns a clone of the
/// original operations unchanged.
fn expand_hierarchy_operations(
    operations: &[OperationSpec],
    market: &finstack_core::market_data::context::MarketContext,
    mode: ResolutionMode,
) -> Vec<OperationSpec> {
    let hierarchy = match market.hierarchy() {
        Some(h) => h,
        None => return operations.to_vec(),
    };

    // Expand hierarchy ops in-place so non-hierarchy ops retain their
    // original position relative to hierarchy-expanded ops.  This avoids
    // re-ordering that would, e.g., move a TimeRollForward before shocks
    // that were originally specified first.
    //
    // Phase 1: collect per-position expansions and record which indices
    //          are hierarchy ops vs pass-through.
    enum Slot {
        Direct(OperationSpec),
        Expanded(Vec<HierarchyExpansion>),
    }

    let mut slots: Vec<Slot> = Vec::with_capacity(operations.len());

    for op in operations {
        match op {
            OperationSpec::HierarchyCurveParallelBp {
                curve_kind,
                target,
                bp,
                discount_curve_id,
            } => {
                let matches = resolve_hierarchy_matches(hierarchy, target);
                let exps: Vec<HierarchyExpansion> = matches
                    .into_iter()
                    .map(|matched| HierarchyExpansion {
                        matched_depth: matched.matched_depth,
                        key: HierarchyExpansionKey::Curve {
                            curve_kind: *curve_kind,
                            curve_id: matched.curve_id.clone(),
                        },
                        operation: OperationSpec::CurveParallelBp {
                            curve_kind: *curve_kind,
                            curve_id: matched.curve_id.as_str().to_string(),
                            discount_curve_id: discount_curve_id.clone(),
                            bp: *bp,
                        },
                    })
                    .collect();
                slots.push(Slot::Expanded(exps));
            }
            OperationSpec::HierarchyVolSurfaceParallelPct {
                surface_kind,
                target,
                pct,
            } => {
                let matches = resolve_hierarchy_matches(hierarchy, target);
                let exps: Vec<HierarchyExpansion> = matches
                    .into_iter()
                    .map(|matched| HierarchyExpansion {
                        matched_depth: matched.matched_depth,
                        key: HierarchyExpansionKey::VolSurface {
                            surface_kind: *surface_kind,
                            surface_id: matched.curve_id.clone(),
                        },
                        operation: OperationSpec::VolSurfaceParallelPct {
                            surface_kind: *surface_kind,
                            surface_id: matched.curve_id.as_str().to_string(),
                            pct: *pct,
                        },
                    })
                    .collect();
                slots.push(Slot::Expanded(exps));
            }
            OperationSpec::HierarchyEquityPricePct { target, pct } => {
                let matches = resolve_hierarchy_matches(hierarchy, target);
                let exps: Vec<HierarchyExpansion> = matches
                    .into_iter()
                    .map(|matched| HierarchyExpansion {
                        matched_depth: matched.matched_depth,
                        key: HierarchyExpansionKey::EquityPrice {
                            price_id: matched.curve_id.clone(),
                        },
                        operation: OperationSpec::EquityPricePct {
                            ids: vec![matched.curve_id.as_str().to_string()],
                            pct: *pct,
                        },
                    })
                    .collect();
                slots.push(Slot::Expanded(exps));
            }
            OperationSpec::HierarchyBaseCorrParallelPts { target, points } => {
                let matches = resolve_hierarchy_matches(hierarchy, target);
                let exps: Vec<HierarchyExpansion> = matches
                    .into_iter()
                    .map(|matched| HierarchyExpansion {
                        matched_depth: matched.matched_depth,
                        key: HierarchyExpansionKey::BaseCorrelation {
                            surface_id: matched.curve_id.clone(),
                        },
                        operation: OperationSpec::BaseCorrParallelPts {
                            surface_id: matched.curve_id.as_str().to_string(),
                            points: *points,
                        },
                    })
                    .collect();
                slots.push(Slot::Expanded(exps));
            }
            other => slots.push(Slot::Direct(other.clone())),
        }
    }

    // Phase 2: apply resolution mode for deduplication across ALL
    //          hierarchy expansions, then flatten in original order.
    let all_expansions: Vec<&HierarchyExpansion> = slots
        .iter()
        .filter_map(|s| match s {
            Slot::Expanded(exps) => Some(exps.iter()),
            Slot::Direct(_) => None,
        })
        .flatten()
        .collect();

    let max_depth: HashMap<HierarchyExpansionKey, usize> =
        if matches!(mode, ResolutionMode::MostSpecificWins) {
            let mut md: HashMap<HierarchyExpansionKey, usize> = HashMap::default();
            for exp in &all_expansions {
                md.entry(exp.key.clone())
                    .and_modify(|best| *best = (*best).max(exp.matched_depth))
                    .or_insert(exp.matched_depth);
            }
            md
        } else {
            HashMap::default()
        };

    let mut result = Vec::with_capacity(operations.len());
    for slot in slots {
        match slot {
            Slot::Direct(op) => result.push(op),
            Slot::Expanded(exps) => {
                for exp in exps {
                    let keep = match mode {
                        ResolutionMode::Cumulative => true,
                        ResolutionMode::MostSpecificWins => max_depth
                            .get(&exp.key)
                            .is_some_and(|&max| exp.matched_depth == max),
                    };
                    if keep {
                        result.push(exp.operation);
                    }
                }
            }
        }
    }

    result
}

/// Orchestrates the deterministic application of a [`ScenarioSpec`].
///
/// The engine is intentionally lightweight: it does not own any state and can
/// be cloned or reused freely. All mutable inputs are supplied via
/// [`ExecutionContext`].
#[derive(Debug, Default, Clone)]
pub struct ScenarioEngine {
    _private: (),
}

impl ScenarioEngine {
    /// Create a new scenario engine with default settings.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_scenarios::ScenarioEngine;
    ///
    /// let engine = ScenarioEngine::new();
    /// let other = ScenarioEngine::default();
    /// assert_eq!(format!("{:?}", engine), format!("{:?}", other));
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Compose multiple scenarios into a single deterministic spec.
    ///
    /// Operations are sorted by priority (lower = first); operations targeting the
    /// same curve stack additively (two +25bp shocks produce +50bp).
    ///
    /// # Arguments
    /// - `scenarios`: Collection of scenario specifications to combine. Lower
    ///   `ScenarioSpec::priority` values are treated as higher priority and their
    ///   operations appear first.
    ///
    /// # Returns
    /// Combined [`ScenarioSpec`] containing all
    /// operations with deterministic ordering.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_scenarios::{ScenarioEngine, ScenarioSpec, OperationSpec, CurveKind};
    ///
    /// let s1 = ScenarioSpec {
    ///     id: "base".into(),
    ///     name: None,
    ///     description: None,
    ///     operations: vec![
    ///         OperationSpec::CurveParallelBp {
    ///             curve_kind: CurveKind::Discount,
    ///             curve_id: "USD_SOFR".into(),
    ///             discount_curve_id: None,
    ///             bp: 25.0,
    ///         },
    ///     ],
    ///     priority: 0,
    ///     resolution_mode: Default::default(),
    /// };
    ///
    /// let s2 = ScenarioSpec {
    ///     id: "overlay".into(),
    ///     name: None,
    ///     description: None,
    ///     operations: vec![
    ///         OperationSpec::StmtForecastPercent {
    ///             node_id: "Revenue".into(),
    ///             pct: -5.0,
    ///         },
    ///     ],
    ///     priority: 1,
    ///     resolution_mode: Default::default(),
    /// };
    ///
    /// let engine = ScenarioEngine::new();
    /// let composed = engine.compose(vec![s1, s2]);
    /// assert_eq!(composed.operations.len(), 2);
    /// ```
    #[must_use]
    pub fn compose(&self, mut scenarios: Vec<ScenarioSpec>) -> ScenarioSpec {
        // Stable sort by priority (lower = higher priority)
        scenarios.sort_by_key(|s| s.priority);

        let composed_id = if scenarios.is_empty() {
            "composed".to_string()
        } else {
            scenarios
                .iter()
                .map(|scenario| scenario.id.as_str())
                .collect::<Vec<_>>()
                .join("+")
        };
        let composed_name = if scenarios.is_empty() {
            Some("Composed Scenario".to_string())
        } else {
            Some(
                scenarios
                    .iter()
                    .map(|scenario| scenario.name.as_deref().unwrap_or(scenario.id.as_str()))
                    .collect::<Vec<_>>()
                    .join(" + "),
            )
        };
        let mut all_operations = Vec::new();
        let resolution_mode = if scenarios.is_empty() {
            ResolutionMode::default()
        } else if scenarios
            .iter()
            .all(|scenario| scenario.resolution_mode == scenarios[0].resolution_mode)
        {
            scenarios[0].resolution_mode
        } else {
            ResolutionMode::Cumulative
        };

        for scenario in scenarios {
            all_operations.extend(scenario.operations);
        }

        // Operations from all scenarios are concatenated in priority order.
        // No deduplication: multiple operations targeting the same curve stack
        // additively (e.g., two +25bp shocks produce +50bp, NOT last-wins).

        ScenarioSpec {
            id: composed_id,
            name: composed_name,
            description: None,
            operations: all_operations,
            priority: 0,
            resolution_mode,
        }
    }

    /// Apply a scenario specification to the execution context.
    ///
    /// Operations are applied in this order:
    /// 0. Time roll-forward, if present
    /// 1. Market data (FX, equities, vol surfaces, curves, base correlation)
    /// 2. Rate bindings update (if configured)
    /// 3. Statement forecast adjustments
    /// 4. Statement re-evaluation
    ///
    /// If a [`crate::spec::OperationSpec::TimeRollForward`] sets
    /// `apply_shocks = false`, the engine returns immediately after phase 0 and
    /// does not apply the remaining operations in `spec`.
    ///
    /// # Arguments
    /// - `spec`: Scenario specification to apply.
    /// - `ctx`: Mutable execution context that supplies market data, statements,
    ///   instruments, and rate bindings.
    ///
    /// # Returns
    /// [`ApplicationReport`] summarising how many operations were applied and
    /// any warnings that were recorded.
    ///
    /// # Errors
    /// Propagates any error returned by adapter modules when an operation cannot
    /// be completed (for example missing market data, unsupported operation, or
    /// invalid tenor strings).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_scenarios::{ScenarioEngine, ScenarioSpec, OperationSpec, CurveKind, ExecutionContext};
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_statements::FinancialModelSpec;
    /// use time::macros::date;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut market = MarketContext::new();
    /// let mut model = FinancialModelSpec::new("test", vec![]);
    /// let as_of = date!(2025-01-01);
    ///
    /// let scenario = ScenarioSpec {
    ///     id: "test".into(),
    ///     name: None,
    ///     description: None,
    ///     operations: vec![
    ///         OperationSpec::StmtForecastPercent {
    ///             node_id: "Revenue".into(),
    ///             pct: -5.0,
    ///         },
    ///     ],
    ///     priority: 0,
    ///     resolution_mode: Default::default(),
    /// };
    ///
    /// let engine = ScenarioEngine::new();
    /// let mut ctx = ExecutionContext {
    ///     market: &mut market,
    ///     model: &mut model,
    ///     instruments: None,
    ///     rate_bindings: None,
    ///     calendar: None,
    ///     as_of,
    /// };
    ///
    /// let report = engine.apply(&scenario, &mut ctx)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn apply(
        &self,
        spec: &ScenarioSpec,
        ctx: &mut ExecutionContext,
    ) -> Result<ApplicationReport> {
        let mut applied = 0;
        let mut warnings = Vec::new();
        // Track whether model was re-evaluated if we wanted to report it,
        // but ApplicationReport doesn't support it yet.
        // We focus on operations_applied and warnings.

        let user_operations = spec.operations.len();

        // Phase -1: Expand hierarchy-targeted operations to direct operations
        let expanded_ops =
            expand_hierarchy_operations(&spec.operations, ctx.market, spec.resolution_mode);
        let expanded_operations = expanded_ops.len();

        // Phase 0: Time Roll Forward
        let time_roll_count = expanded_ops
            .iter()
            .filter(|op| matches!(op, OperationSpec::TimeRollForward { .. }))
            .count();
        if time_roll_count > 1 {
            return Err(crate::error::Error::validation(format!(
                "Scenario contains {} TimeRollForward operations; only one is allowed per scenario. \
                 Compose multiple rolls into separate scenario specs.",
                time_roll_count,
            )));
        }

        for op in &expanded_ops {
            if let OperationSpec::TimeRollForward {
                period,
                apply_shocks,
                roll_mode,
            } = op
            {
                crate::adapters::time_roll::apply_time_roll_forward(ctx, period, *roll_mode)?;
                applied += 1;

                if !*apply_shocks {
                    return Ok(ApplicationReport {
                        operations_applied: applied,
                        user_operations,
                        expanded_operations,
                        warnings,
                        rounding_context: rounding_stamp(),
                    });
                }
            }
        }

        // Initialize adapters
        // Optimization: Use stack-allocated array of references instead of Vec<Box<dyn>>
        // to avoid heap allocation on every call.
        let vol_adapter = crate::adapters::vol::VolAdapter;
        let curve_adapter = crate::adapters::curves::CurveAdapter;
        let base_corr_adapter = crate::adapters::basecorr::BaseCorrAdapter;
        let fx_adapter = crate::adapters::fx::FxAdapter;
        let equity_adapter = crate::adapters::equity::EquityAdapter;
        let instrument_adapter = crate::adapters::instruments::InstrumentAdapter;
        let statement_adapter = crate::adapters::statements::StatementAdapter;
        let asset_corr_adapter = crate::adapters::asset_corr::AssetCorrAdapter;

        let adapters: [&dyn crate::adapters::traits::ScenarioAdapter; 8] = [
            &vol_adapter,
            &curve_adapter,
            &base_corr_adapter,
            &fx_adapter,
            &equity_adapter,
            &instrument_adapter,
            &statement_adapter,
            &asset_corr_adapter,
        ];

        let has_rate_bindings = ctx.rate_bindings.is_some();
        let mut deferred_stmts = Vec::new();

        // Phase 1: Market data operations & Instrument operations
        for op in &expanded_ops {
            if let OperationSpec::TimeRollForward { .. } = op {
                continue; // handled in Phase 0
            }

            let mut adapter_effects = None;
            for adapter in &adapters {
                if let Some(effects) = adapter.try_generate_effects(op, ctx)? {
                    adapter_effects = Some(effects);
                    break;
                }
            }

            if let Some(effects) = adapter_effects {
                for effect in effects {
                    match effect {
                        crate::adapters::traits::ScenarioEffect::MarketBump(b) => {
                            // Apply immediately
                            *ctx.market = ctx.market.bump([b])?;
                            applied += 1;
                        }
                        crate::adapters::traits::ScenarioEffect::Warning(w) => warnings.push(w),
                        crate::adapters::traits::ScenarioEffect::UpdateCurve(storage) => {
                            *ctx.market = std::mem::take(ctx.market).insert(storage);
                            applied += 1;
                        }
                        crate::adapters::traits::ScenarioEffect::InstrumentPriceShock {
                            types,
                            attrs,
                            pct,
                        } => {
                            let (c, w) = apply_instrument_shock(
                                types.as_deref(),
                                attrs.as_ref(),
                                pct,
                                "price",
                                &mut ctx.instruments,
                                crate::adapters::instruments::apply_instrument_type_price_shock,
                                crate::adapters::instruments::apply_instrument_attr_price_shock,
                            );
                            applied += c;
                            warnings.extend(w);
                        }
                        crate::adapters::traits::ScenarioEffect::InstrumentSpreadShock {
                            types,
                            attrs,
                            bp,
                        } => {
                            let (c, w) = apply_instrument_shock(
                                types.as_deref(),
                                attrs.as_ref(),
                                bp,
                                "spread",
                                &mut ctx.instruments,
                                crate::adapters::instruments::apply_instrument_type_spread_shock,
                                crate::adapters::instruments::apply_instrument_attr_spread_shock,
                            );
                            applied += c;
                            warnings.extend(w);
                        }
                        crate::adapters::traits::ScenarioEffect::AssetCorrelationShock {
                            delta_pts,
                        }
                        | crate::adapters::traits::ScenarioEffect::PrepayDefaultCorrelationShock {
                            delta_pts,
                        } => {
                            let (count, ws) = apply_correlation_effect(&effect, delta_pts, ctx);
                            applied += count;
                            warnings.extend(ws);
                        }
                        crate::adapters::traits::ScenarioEffect::StmtForecastPercent { .. }
                        | crate::adapters::traits::ScenarioEffect::StmtForecastAssign { .. }
                        | crate::adapters::traits::ScenarioEffect::RateBinding { .. } => {
                            // Defer statement operations
                            deferred_stmts.push(effect);
                        }
                    }
                }
            } else {
                // Warning: Operation not handled by any adapter
                warnings.push(format!("Operation not supported: {:?}", op));
            }
        }

        // Phase 2: Rate bindings update (from context configuration)
        //
        // A mismatch between the map key and `binding.node_id` used to be
        // silently patched by the engine (rewriting `binding.node_id` to
        // match the key and emitting a warning). That behaviour was
        // footgun-y: the binding spec the caller passed was not the one
        // applied, and any other field on the binding (curve_id, tenor,
        // compounding) was still trusted as-is. We now fail hard so the
        // caller fixes the mismatch upstream.
        if let Some(bindings) = &ctx.rate_bindings {
            for (node_id, binding) in bindings {
                if binding.node_id != *node_id {
                    return Err(crate::error::Error::Validation(format!(
                        "Rate binding node_id mismatch: map key '{}' does not equal \
                         binding.node_id '{}'. The map key is authoritative for routing; \
                         rebuild the binding with node_id set to the map key.",
                        node_id, binding.node_id
                    )));
                }

                match crate::adapters::statements::update_rate_from_binding(
                    binding,
                    ctx.model,
                    ctx.market,
                    ctx.calendar,
                ) {
                    Ok(true) => {}
                    Ok(false) => warnings.push(format!(
                        "Rate binding {}->{}: node has no forecast values to assign",
                        node_id, binding.curve_id
                    )),
                    Err(e) => warnings.push(format!(
                        "Rate binding {}->{}: {}",
                        node_id, binding.curve_id, e
                    )),
                }
            }
        }

        // Phase 3: Statement Operations (Deferred)
        let mut applied_stmt_ops = 0usize;
        for effect in deferred_stmts {
            match effect {
                crate::adapters::traits::ScenarioEffect::RateBinding { binding } => {
                    // Apply dynamic rate binding
                    if let Some(rb) = &mut ctx.rate_bindings {
                        rb.insert(binding.node_id.clone(), binding.clone());
                    }
                    // Update immediately
                    match crate::adapters::statements::update_rate_from_binding(
                        &binding,
                        ctx.model,
                        ctx.market,
                        ctx.calendar,
                    ) {
                        Ok(true) => {
                            applied += 1;
                            applied_stmt_ops += 1;
                        }
                        Ok(false) => {
                            applied += 1;
                            applied_stmt_ops += 1;
                            warnings.push(format!(
                                "Dynamic rate binding {}->{}: node has no forecast values to assign",
                                binding.node_id, binding.curve_id
                            ));
                        }
                        Err(e) => warnings.push(format!(
                            "Dynamic rate binding {}->{}: {}",
                            binding.node_id, binding.curve_id, e
                        )),
                    }
                }
                crate::adapters::traits::ScenarioEffect::StmtForecastPercent { node_id, pct } => {
                    match crate::adapters::statements::apply_forecast_percent(
                        ctx.model,
                        node_id.as_str(),
                        pct,
                    ) {
                        Ok(true) => {
                            applied += 1;
                            applied_stmt_ops += 1;
                        }
                        Ok(false) => {
                            warnings.push(format!(
                                "Statement node '{}' has no forecast values to modify",
                                node_id,
                            ));
                        }
                        Err(e) => warnings.push(format!(
                            "Statement forecast percent for node {}: {}",
                            node_id.as_str(),
                            e
                        )),
                    }
                }
                crate::adapters::traits::ScenarioEffect::StmtForecastAssign { node_id, value } => {
                    match crate::adapters::statements::apply_forecast_assign(
                        ctx.model,
                        node_id.as_str(),
                        value,
                    ) {
                        Ok(true) => {
                            applied += 1;
                            applied_stmt_ops += 1;
                        }
                        Ok(false) => {
                            warnings.push(format!(
                                "Statement node '{}' has no forecast values to modify",
                                node_id,
                            ));
                        }
                        Err(e) => warnings.push(format!(
                            "Statement forecast assign for node {}: {}",
                            node_id.as_str(),
                            e
                        )),
                    }
                }
                _ => {}
            }
        }

        // Phase 4: Re-evaluate statements only if statement work was performed
        if applied_stmt_ops > 0 || has_rate_bindings {
            match crate::adapters::statements::reevaluate_model(ctx.model) {
                Ok(eval_warnings) => warnings.extend(
                    eval_warnings
                        .into_iter()
                        .map(|w| format!("Model evaluation: {}", w)),
                ),
                Err(e) => warnings.push(format!("Model re-evaluation: {}", e)),
            }
        }

        Ok(ApplicationReport {
            operations_applied: applied,
            user_operations,
            expanded_operations,
            warnings,
            rounding_context: rounding_stamp(),
        })
    }
}

/// Function that applies an instrument shock filtered by instrument type.
type TypeShockFn = fn(
    &mut [Box<DynInstrument>],
    &[finstack_valuations::pricer::InstrumentType],
    f64,
) -> crate::error::Result<(usize, Vec<String>)>;

/// Function that applies an instrument shock filtered by attributes.
type AttrShockFn = fn(
    &mut [Box<DynInstrument>],
    &indexmap::IndexMap<String, String>,
    f64,
) -> crate::error::Result<(usize, Vec<String>)>;

/// Apply an instrument shock (price or spread) dispatching by type and attribute filters.
fn apply_instrument_shock(
    types: Option<&[finstack_valuations::pricer::InstrumentType]>,
    attrs: Option<&indexmap::IndexMap<String, String>>,
    value: f64,
    kind: &str,
    instruments: &mut Option<&mut Vec<Box<DynInstrument>>>,
    type_fn: TypeShockFn,
    attr_fn: AttrShockFn,
) -> (usize, Vec<String>) {
    let mut applied = 0;
    let mut warnings = Vec::new();

    if let Some(ts) = types {
        if let Some(instruments) = instruments.as_mut() {
            match type_fn(instruments, ts, value) {
                Ok((c, w)) => {
                    applied += c;
                    warnings.extend(w);
                }
                Err(e) => warnings.push(format!("Instrument {} shock error: {}", kind, e)),
            }
        } else {
            warnings.push(format!(
                "Instrument type {} shock requested but no instruments provided",
                kind
            ));
        }
    }

    if let Some(ats) = attrs {
        if let Some(instruments) = instruments.as_mut() {
            match attr_fn(instruments, ats, value) {
                Ok((count, w)) => {
                    applied += count;
                    warnings.extend(w);
                }
                Err(e) => warnings.push(format!("Instrument {} shock error: {}", kind, e)),
            }
        } else {
            warnings.push(format!(
                "Instrument attribute {} shock requested but no instruments provided",
                kind
            ));
        }
    }

    (applied, warnings)
}

/// Apply a correlation shock effect to StructuredCredit instruments via downcast.
///
/// Handles [`ScenarioEffect::AssetCorrelationShock`] and
/// [`ScenarioEffect::PrepayDefaultCorrelationShock`]; any other variant is a
/// no-op so the caller can pass it through a generic dispatch loop.
///
/// Clamping is detected at the primitive field level (via
/// [`CorrelationStructure::bump_asset_with_clamp_info`]) so warnings fire only
/// when a correlation value is actually clamped, not whenever the aggregate
/// `asset_correlation()` accessor moves less than `delta` (which is normal for
/// sectored structures).
fn apply_correlation_effect(
    effect: &crate::adapters::traits::ScenarioEffect,
    delta_pts: f64,
    ctx: &mut ExecutionContext,
) -> (usize, Vec<String>) {
    use crate::adapters::traits::ScenarioEffect;
    use finstack_valuations::instruments::fixed_income::structured_credit::StructuredCredit;

    let instruments = match ctx.instruments.as_mut() {
        Some(insts) => insts,
        None => {
            return (
                0,
                vec!["Correlation shock requested but no instruments provided".to_string()],
            );
        }
    };

    let mut count = 0usize;
    let mut warnings = Vec::new();

    for inst in instruments.iter_mut() {
        let Some(sc) = inst.as_any_mut().downcast_mut::<StructuredCredit>() else {
            continue;
        };
        let Some(ref corr) = sc.credit_model.correlation_structure else {
            continue;
        };

        let (new_corr, clamp_info) = match effect {
            ScenarioEffect::AssetCorrelationShock { .. } => {
                corr.bump_asset_with_clamp_info(delta_pts)
            }
            ScenarioEffect::PrepayDefaultCorrelationShock { .. } => {
                corr.bump_prepay_default_with_clamp_info(delta_pts)
            }
            _ => continue,
        };

        if let Some(info) = clamp_info {
            warnings.push(format!("Correlation bump for '{}': {info}", sc.id));
        }
        sc.credit_model.correlation_structure = Some(new_corr);
        count += 1;
    }

    if count == 0 {
        warnings.push(
            "Correlation shock: no StructuredCredit instruments with correlation structure found"
                .to_string(),
        );
    }

    (count, warnings)
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::spec::OperationSpec;

    #[test]
    fn compose_preserves_source_ids_and_names() {
        let engine = ScenarioEngine::new();
        let composed = engine.compose(vec![
            ScenarioSpec {
                id: "rates_up".into(),
                name: Some("Rates Up".into()),
                description: None,
                operations: vec![OperationSpec::StmtForecastPercent {
                    node_id: "Revenue".into(),
                    pct: 1.0,
                }],
                priority: 2,
                resolution_mode: ResolutionMode::MostSpecificWins,
            },
            ScenarioSpec {
                id: "credit_down".into(),
                name: None,
                description: None,
                operations: vec![OperationSpec::StmtForecastPercent {
                    node_id: "Expenses".into(),
                    pct: -1.0,
                }],
                priority: 1,
                resolution_mode: ResolutionMode::Cumulative,
            },
        ]);

        assert_eq!(composed.id.as_str(), "credit_down+rates_up");
        assert_eq!(composed.name.as_deref(), Some("credit_down + Rates Up"));
        assert_eq!(composed.operations.len(), 2);
        assert_eq!(composed.resolution_mode, ResolutionMode::Cumulative);
    }
}
