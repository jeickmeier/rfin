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
use crate::spec::{OperationSpec, RateBindingSpec, ScenarioSpec};
use indexmap::IndexMap;

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
    pub instruments: Option<&'a mut Vec<Box<dyn finstack_valuations::instruments::Instrument>>>,

    /// Optional mapping from statement node IDs to binding specs for automatic rate updates.
    pub rate_bindings: Option<IndexMap<String, RateBindingSpec>>,

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
///     warnings: vec!["fallback curve used".into()],
///     rounding_context: Some("default".into()),
/// };
///
/// assert_eq!(report.operations_applied, 3);
/// assert_eq!(report.warnings.len(), 1);
/// ```
#[derive(Debug, Clone)]
pub struct ApplicationReport {
    /// Number of operations successfully applied.
    pub operations_applied: usize,

    /// Warnings generated during application (non-fatal).
    pub warnings: Vec<String>,

    /// Rounding context stamp (for determinism tracking).
    pub rounding_context: Option<String>,
}

/// Orchestrates the deterministic application of a [`ScenarioSpec`].
///
/// The engine is intentionally lightweight: it does not own any state and can
/// be cloned or reused freely. All mutable inputs are supplied via
/// [`ExecutionContext`].
#[derive(Debug, Default)]
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
    pub fn new() -> Self {
        Self::default()
    }

    /// Compose multiple scenarios into a single deterministic spec.
    ///
    /// Operations are sorted by (priority, declaration_index); conflicts use last-wins.
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
    ///             bp: 25.0,
    ///         },
    ///     ],
    ///     priority: 0,
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
    /// };
    ///
    /// let engine = ScenarioEngine::new();
    /// let composed = engine.compose(vec![s1, s2]);
    /// assert_eq!(composed.operations.len(), 2);
    /// ```
    pub fn compose(&self, mut scenarios: Vec<ScenarioSpec>) -> ScenarioSpec {
        // Stable sort by priority (lower = higher priority)
        scenarios.sort_by_key(|s| s.priority);

        let mut all_operations = Vec::new();
        for scenario in scenarios {
            all_operations.extend(scenario.operations);
        }

        // Operations from all scenarios are concatenated in priority order.
        // No deduplication: multiple operations targeting the same curve stack
        // additively (e.g., two +25bp shocks produce +50bp, NOT last-wins).

        ScenarioSpec {
            id: "composed".into(),
            name: Some("Composed Scenario".into()),
            description: None,
            operations: all_operations,
            priority: 0,
        }
    }

    /// Apply a scenario specification to the execution context.
    ///
    /// Operations are applied in this order:
    /// 1. Market data (FX, equities, vol surfaces, curves, base correlation)
    /// 2. Rate bindings update (if configured)
    /// 3. Statement forecast adjustments
    /// 4. Statement re-evaluation
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

        // Validate: at most one TimeRollForward per scenario
        let time_roll_count = spec
            .operations
            .iter()
            .filter(|op| matches!(op, OperationSpec::TimeRollForward { .. }))
            .count();
        if time_roll_count > 1 {
            return Err(crate::error::Error::Validation(format!(
                "Scenario contains {} TimeRollForward operations; at most one is allowed",
                time_roll_count
            )));
        }

        // Phase 0: Time Roll Forward
        for op in &spec.operations {
            if let OperationSpec::TimeRollForward {
                period,
                apply_shocks,
                roll_mode,
            } = op
            {
                // Use the adapter logic
                crate::adapters::time_roll::apply_time_roll_forward(ctx, period, *roll_mode)?;

                // If shocks should NOT be applied, we stop here.
                if !*apply_shocks {
                    return Ok(ApplicationReport {
                        operations_applied: 1,
                        warnings,
                        rounding_context: Some("default".into()),
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

        let mut deferred_stmts = Vec::new();

        // Phase 1: Market data operations & Instrument operations
        for op in &spec.operations {
            // Skip TimeRoll as it was handled (or skipped) in Phase 0
            if let OperationSpec::TimeRollForward { .. } = op {
                applied += 1;
                continue;
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
                        crate::adapters::traits::ScenarioEffect::UpdateDiscountCurve {
                            id: _id,
                            curve,
                        } => {
                            *ctx.market = std::mem::take(ctx.market).insert(curve.as_ref().clone());
                            applied += 1;
                        }
                        crate::adapters::traits::ScenarioEffect::UpdateForwardCurve {
                            id: _id,
                            curve,
                        } => {
                            *ctx.market = std::mem::take(ctx.market).insert(curve.as_ref().clone());
                            applied += 1;
                        }
                        crate::adapters::traits::ScenarioEffect::UpdateHazardCurve {
                            id: _id,
                            curve,
                        } => {
                            *ctx.market = std::mem::take(ctx.market).insert(curve.as_ref().clone());
                            applied += 1;
                        }
                        crate::adapters::traits::ScenarioEffect::UpdateInflationCurve {
                            id: _id,
                            curve,
                        } => {
                            *ctx.market = std::mem::take(ctx.market).insert(curve.as_ref().clone());
                            applied += 1;
                        }
                        crate::adapters::traits::ScenarioEffect::UpdateVolIndexCurve {
                            id: _id,
                            curve,
                        } => {
                            *ctx.market = std::mem::take(ctx.market).insert(curve.as_ref().clone());
                            applied += 1;
                        }
                        crate::adapters::traits::ScenarioEffect::InstrumentPriceShock {
                            types,
                            attrs,
                            pct,
                        } => {
                            // Handle Types: Requires instruments
                            if let Some(ts) = types {
                                if let Some(instruments) = &mut ctx.instruments {
                                    match crate::adapters::instruments::apply_instrument_type_price_shock(
                                        instruments,
                                        &ts,
                                        pct,
                                    ) {
                                        Ok(c) => applied += c,
                                        Err(e) => warnings.push(format!("Instrument price shock error: {}", e)),
                                    }
                                } else {
                                    warnings.push("Instrument type shock requested but no instruments provided".to_string());
                                }
                            }
                            // Handle Attrs: Supports empty (legacy fallback)
                            if let Some(ats) = attrs {
                                if let Some(instruments) = &mut ctx.instruments {
                                    match crate::adapters::instruments::apply_instrument_attr_price_shock(
                                        instruments,
                                        &ats,
                                        pct,
                                    ) {
                                        Ok((count, w)) => {
                                            applied += count;
                                            warnings.extend(w);
                                        }
                                        Err(e) => warnings.push(format!(
                                            "Instrument price shock error: {}",
                                            e
                                        )),
                                    }
                                } else {
                                    warnings.push(
                                        "Instrument attribute shock requested but no instruments provided"
                                            .to_string(),
                                    );
                                }
                            }
                        }
                        crate::adapters::traits::ScenarioEffect::InstrumentSpreadShock {
                            types,
                            attrs,
                            bp,
                        } => {
                            // Handle Types: Requires instruments
                            if let Some(ts) = types {
                                if let Some(instruments) = &mut ctx.instruments {
                                    match crate::adapters::instruments::apply_instrument_type_spread_shock(
                                        instruments,
                                        &ts,
                                        bp,
                                    ) {
                                        Ok(c) => applied += c,
                                        Err(e) => warnings.push(format!("Instrument spread shock error: {}", e)),
                                    }
                                } else {
                                    warnings.push("Instrument type shock requested but no instruments provided".to_string());
                                }
                            }
                            // Handle Attrs: Supports empty
                            if let Some(ats) = attrs {
                                if let Some(instruments) = &mut ctx.instruments {
                                    match crate::adapters::instruments::apply_instrument_attr_spread_shock(
                                        instruments,
                                        &ats,
                                        bp,
                                    ) {
                                        Ok((count, w)) => {
                                            applied += count;
                                            warnings.extend(w);
                                        }
                                        Err(e) => warnings.push(format!(
                                            "Instrument spread shock error: {}",
                                            e
                                        )),
                                    }
                                } else {
                                    warnings.push(
                                        "Instrument attribute shock requested but no instruments provided"
                                            .to_string(),
                                    );
                                }
                            }
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
        if let Some(bindings) = &ctx.rate_bindings {
            for (node_id, binding) in bindings {
                let mut binding_to_use = None;
                if binding.node_id != *node_id {
                    warnings.push(format!(
                        "Rate binding node_id mismatch: map key '{}' vs binding '{}'; using key",
                        node_id, binding.node_id
                    ));
                    let mut clone = binding.clone();
                    clone.node_id = node_id.clone();
                    binding_to_use = Some(clone);
                }
                let binding_ref = binding_to_use.as_ref().unwrap_or(binding);

                match crate::adapters::statements::update_rate_from_binding(
                    binding_ref,
                    ctx.model,
                    ctx.market,
                ) {
                    Ok(_) => {}
                    Err(e) => warnings.push(format!(
                        "Rate binding {}->{}: {}",
                        node_id, binding_ref.curve_id, e
                    )),
                }
            }
        }

        // Phase 3: Statement Operations (Deferred)
        for effect in deferred_stmts {
            match effect {
                crate::adapters::traits::ScenarioEffect::RateBinding { binding } => {
                    // Apply dynamic rate binding
                    if let Some(rb) = &mut ctx.rate_bindings {
                        rb.insert(binding.node_id.clone(), binding.clone());
                    }
                    // Update immediately
                    match crate::adapters::statements::update_rate_from_binding(
                        &binding, ctx.model, ctx.market,
                    ) {
                        Ok(_) => {}
                        Err(e) => warnings.push(format!(
                            "Dynamic Rate binding {}->{}: {}",
                            binding.node_id, binding.curve_id, e
                        )),
                    }
                }
                crate::adapters::traits::ScenarioEffect::StmtForecastPercent { node_id, pct } => {
                    crate::adapters::statements::apply_forecast_percent(ctx.model, &node_id, pct)?;
                    applied += 1;
                }
                crate::adapters::traits::ScenarioEffect::StmtForecastAssign { node_id, value } => {
                    crate::adapters::statements::apply_forecast_assign(ctx.model, &node_id, value)?;
                    applied += 1;
                }
                _ => {}
            }
        }

        // Phase 4: Re-evaluate statements to propagate changes
        match crate::adapters::statements::reevaluate_model(ctx.model) {
            Ok(eval_warnings) => warnings.extend(
                eval_warnings
                    .into_iter()
                    .map(|w| format!("Model evaluation: {}", w)),
            ),
            Err(e) => warnings.push(format!("Model re-evaluation: {}", e)),
        }

        Ok(ApplicationReport {
            operations_applied: applied,
            warnings,
            rounding_context: Some("default".into()),
        })
    }
}
