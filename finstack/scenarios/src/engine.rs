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
use crate::spec::{OperationSpec, ScenarioSpec};
use finstack_core::market_data::bumps::MarketBump;
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
///   market curve identifiers; used to sync statement rates after curve shocks.
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
    pub instruments:
        Option<&'a mut Vec<Box<dyn finstack_valuations::instruments::common::traits::Instrument>>>,

    /// Optional mapping from statement node IDs to curve IDs for automatic rate updates.
    pub rate_bindings: Option<IndexMap<String, String>>,

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

/// Orchestrates the deterministic application of a [`ScenarioSpec`](crate::spec::ScenarioSpec).
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
    /// Combined [`ScenarioSpec`](crate::spec::ScenarioSpec) containing all
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

        // In Phase A, we use simple last-wins for same target
        // (No deduplication; engine applies in order and last application wins)

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

        // Phase 0: Time roll-forward (if present)
        for op in &spec.operations {
            if let OperationSpec::TimeRollForward {
                period,
                apply_shocks,
            } = op
            {
                let _roll_report =
                    crate::adapters::time_roll::apply_time_roll_forward(ctx, period)?;
                applied += 1;

                // If apply_shocks is false, skip remaining operations
                if !apply_shocks {
                    return Ok(ApplicationReport {
                        operations_applied: applied,
                        warnings,
                        rounding_context: Some("default".into()),
                    });
                }
            }
        }

        // Phase 1: Market data operations (order: FX → Equities → Vol → Curves)
        let mut market_bumps: Vec<MarketBump> = Vec::new();

        for op in &spec.operations {
            match op {
                OperationSpec::MarketFxPct { base, quote, pct } => {
                    applied += crate::adapters::fx::apply_fx_shock(
                        *base,
                        *quote,
                        *pct,
                        ctx.as_of,
                        &mut market_bumps,
                    )?;
                }
                OperationSpec::EquityPricePct { ids, pct } => {
                    applied += crate::adapters::equity::apply_equity_price_shock(
                        ctx.market,
                        ids,
                        *pct,
                        &mut market_bumps,
                        &mut warnings,
                    )?;
                }
                OperationSpec::InstrumentPricePctByAttr { attrs, pct } => {
                    let mut empty_instruments: Vec<
                        Box<dyn finstack_valuations::instruments::common::traits::Instrument>,
                    > = Vec::new();
                    let instruments_slice = ctx
                        .instruments
                        .as_deref_mut()
                        .unwrap_or(&mut empty_instruments);

                    let w = crate::adapters::instruments::apply_instrument_attr_price_shock(
                        instruments_slice,
                        attrs,
                        *pct,
                    )?;
                    warnings.extend(w);
                }
                OperationSpec::InstrumentPricePctByType {
                    instrument_types,
                    pct,
                } => {
                    if let Some(instruments) = ctx.instruments.as_mut() {
                        match crate::adapters::instruments::apply_instrument_type_price_shock(
                            instruments,
                            instrument_types,
                            *pct,
                        ) {
                            Ok(count) => applied += count,
                            Err(e) => warnings.push(format!("Instrument type price shock: {}", e)),
                        }
                    } else {
                        warnings.push(
                            "Instrument type shock requested but no instruments provided"
                                .to_string(),
                        );
                    }
                }
                OperationSpec::VolSurfaceParallelPct {
                    surface_kind: _,
                    surface_id,
                    pct,
                } => {
                    crate::adapters::vol::apply_vol_parallel_shock(
                        surface_id,
                        *pct,
                        &mut market_bumps,
                    )?;
                    applied += 1;
                }
                OperationSpec::VolSurfaceBucketPct {
                    surface_kind: _,
                    surface_id,
                    tenors,
                    strikes,
                    pct,
                } => {
                    crate::adapters::vol::apply_vol_bucket_shock(
                        surface_id,
                        tenors.as_deref(),
                        strikes.as_deref(),
                        *pct,
                        &mut market_bumps,
                        &mut warnings,
                    )?;
                    applied += 1;
                }
                OperationSpec::CurveParallelBp {
                    curve_kind,
                    curve_id,
                    bp,
                } => {
                    crate::adapters::curves::apply_curve_parallel_shock(
                        *curve_kind,
                        curve_id,
                        *bp,
                        &mut market_bumps,
                    )?;
                    applied += 1;
                }
                OperationSpec::CurveNodeBp {
                    curve_kind,
                    curve_id,
                    nodes,
                    match_mode,
                } => {
                    // Apply node shocks directly using the adapter function
                    match crate::adapters::curves::apply_curve_node_shock(
                        ctx.market,
                        *curve_kind,
                        curve_id,
                        nodes,
                        *match_mode,
                    ) {
                        Ok(()) => applied += 1,
                        Err(e) => {
                            // Return errors for hazard curves and tenor not found (exact mode failures)
                            match &e {
                                crate::error::Error::UnsupportedOperation { .. }
                                | crate::error::Error::TenorNotFound { .. } => return Err(e),
                                _ => {
                                    // For other errors, convert to warning unless it's a hazard curve
                                    if *curve_kind == crate::spec::CurveKind::ParCDS {
                                        return Err(e);
                                    }
                                    warnings.push(format!(
                                        "Curve node shock failed for {}: {}",
                                        curve_id, e
                                    ));
                                }
                            }
                        }
                    }
                }
                OperationSpec::BaseCorrParallelPts { surface_id, points } => {
                    crate::adapters::basecorr::apply_basecorr_parallel_shock(
                        surface_id,
                        *points,
                        &mut market_bumps,
                    )?;
                    applied += 1;
                }
                OperationSpec::BaseCorrBucketPts {
                    surface_id,
                    detachment_bps,
                    maturities,
                    points,
                } => {
                    crate::adapters::basecorr::apply_basecorr_bucket_shock(
                        surface_id,
                        detachment_bps.as_deref(),
                        maturities.as_deref(),
                        *points,
                        &mut market_bumps,
                        &mut warnings,
                    )?;
                    applied += 1;
                }
                OperationSpec::InstrumentSpreadBpByAttr { attrs, bp } => {
                    let mut empty_instruments: Vec<
                        Box<dyn finstack_valuations::instruments::common::traits::Instrument>,
                    > = Vec::new();
                    let instruments_slice = ctx
                        .instruments
                        .as_deref_mut()
                        .unwrap_or(&mut empty_instruments);

                    let w = crate::adapters::instruments::apply_instrument_attr_spread_shock(
                        instruments_slice,
                        attrs,
                        *bp,
                    )?;
                    warnings.extend(w);
                }
                OperationSpec::InstrumentSpreadBpByType {
                    instrument_types,
                    bp,
                } => {
                    if let Some(instruments) = ctx.instruments.as_mut() {
                        match crate::adapters::instruments::apply_instrument_type_spread_shock(
                            instruments,
                            instrument_types,
                            *bp,
                        ) {
                            Ok(count) => applied += count,
                            Err(e) => warnings.push(format!("Instrument type spread shock: {}", e)),
                        }
                    } else {
                        warnings.push(
                            "Instrument type shock requested but no instruments provided"
                                .to_string(),
                        );
                    }
                }
                OperationSpec::AssetCorrelationPts { delta_pts: _ }
                | OperationSpec::PrepayDefaultCorrelationPts { delta_pts: _ }
                | OperationSpec::RecoveryCorrelationPts { delta_pts: _ }
                | OperationSpec::PrepayFactorLoadingPts { delta_pts: _ } => {
                    // Note: Correlation operations for structured credit instruments
                    // require direct access to StructuredCredit instruments.
                    // These operations are applied via the asset_corr adapter
                    // when StructuredCredit instruments are in scope.
                    // The engine provides placeholders here; actual application
                    // happens in the valuations layer via:
                    //   crate::adapters::asset_corr::apply_asset_correlation_shock
                    //   crate::adapters::asset_corr::apply_prepay_default_correlation_shock
                    warnings.push(
                        "Correlation operation: requires StructuredCredit instruments (use adapters::asset_corr directly)".to_string(),
                    );
                }
                _ => {} // statements and time roll handled elsewhere
            }
        }

        if !market_bumps.is_empty() {
            *ctx.market = ctx.market.apply_bumps(&market_bumps)?;
        }

        // Phase 2: Rate bindings update (if configured)
        if let Some(bindings) = &ctx.rate_bindings {
            for (node_id, curve_id) in bindings {
                match crate::adapters::statements::update_1y_rate_from_curve(
                    ctx.model, node_id, ctx.market, curve_id,
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        warnings.push(format!("Rate binding {}->{}: {}", node_id, curve_id, e))
                    }
                }
            }
        }

        // Phase 3: Statement operations
        for op in &spec.operations {
            match op {
                OperationSpec::StmtForecastPercent { node_id, pct } => {
                    crate::adapters::statements::apply_forecast_percent(ctx.model, node_id, *pct)?;
                    applied += 1;
                }
                OperationSpec::StmtForecastAssign { node_id, value } => {
                    crate::adapters::statements::apply_forecast_assign(ctx.model, node_id, *value)?;
                    applied += 1;
                }
                _ => {} // already handled above
            }
        }

        // Phase 4: Re-evaluate statements to propagate changes
        let eval_result = crate::adapters::statements::reevaluate_model(ctx.model);
        if let Err(e) = eval_result {
            warnings.push(format!("Model re-evaluation: {}", e));
        }

        Ok(ApplicationReport {
            operations_applied: applied,
            warnings,
            rounding_context: Some("default".into()), // Phase A: simple stamp
        })
    }
}
