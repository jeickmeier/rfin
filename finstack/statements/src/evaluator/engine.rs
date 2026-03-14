//! Main evaluator implementation.

use crate::dsl;
use crate::error::{Error, Result};
use crate::evaluator::context::EvaluationContext;
use crate::evaluator::dag::{evaluate_order, DependencyGraph};
use crate::evaluator::forecast_eval;
use crate::evaluator::formula::evaluate_formula;
use crate::evaluator::monte_carlo::{
    MonteCarloAccumulator, MonteCarloConfig, MonteCarloResults, PathResult,
};
use crate::evaluator::precedence::{resolve_node_value, NodeValueSource};
use crate::evaluator::results::{EvalWarning, ResultsMeta, StatementResult};
use crate::evaluator::{capital_structure_runtime, capital_structure_runtime::dependent_closure};
use crate::types::{FinancialModelSpec, NodeId, NodeValueType};
use finstack_core::dates::PeriodId;
use finstack_core::expr::Expr;
use indexmap::IndexMap;
use std::collections::HashSet;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

/// Evaluator for financial models.
///
/// The evaluator compiles formulas, resolves dependencies, and evaluates
/// nodes period-by-period according to precedence rules.
///
/// # Example
///
/// ```rust
/// # use finstack_statements::builder::ModelBuilder;
/// # use finstack_statements::evaluator::Evaluator;
/// # use finstack_core::dates::PeriodId;
/// let model = ModelBuilder::new("demo")
///     .periods("2025Q1..Q2", None)?
///     .value("revenue", &[
///         (PeriodId::quarter(2025, 1), 100_000.0.into()),
///         (PeriodId::quarter(2025, 2), 105_000.0.into()),
///     ])
///     .compute("gross_profit", "revenue * 0.6")?
///     .build()?;
///
/// let mut evaluator = Evaluator::new();
/// let results = evaluator.evaluate(&model)?;
/// assert_eq!(results.get("revenue", &PeriodId::quarter(2025, 1)), Some(100_000.0));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
pub struct Evaluator {
    /// Cached compiled expressions (Arc-shared across Monte Carlo path clones)
    compiled_cache: std::sync::Arc<IndexMap<NodeId, Expr>>,

    /// Cached forecast results: node_id → (period_id → value)
    forecast_cache: IndexMap<NodeId, IndexMap<PeriodId, f64>>,
}

impl Evaluator {
    /// Create a new evaluator.
    #[must_use = "creating an evaluator has no effect without calling evaluate()"]
    pub fn new() -> Self {
        Self {
            compiled_cache: std::sync::Arc::new(IndexMap::new()),
            forecast_cache: IndexMap::new(),
        }
    }

    /// Create a new evaluator with pre-configured market context.
    ///
    /// This is a convenience constructor that stores market context and as-of date
    /// for capital structure evaluation. When you call `.evaluate()` on this evaluator,
    /// it will automatically use the stored market context.
    ///
    /// # Arguments
    ///
    /// * `market_ctx` - Market context with discount/forward curves
    /// * `as_of` - Valuation date for pricing
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use finstack_statements::prelude::Evaluator;
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::term_structures::DiscountCurve;
    /// use finstack_statements::types::FinancialModelSpec;
    /// use time::macros::date;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let discount_curve: DiscountCurve = unimplemented!("build a discount curve");
    /// let market_ctx = MarketContext::new()
    ///     .insert(discount_curve);
    ///
    /// let as_of_date = date!(2025-01-31);
    /// # let model: FinancialModelSpec = unimplemented!("build or load a model");
    /// let mut evaluator = Evaluator::with_market_context(&market_ctx, as_of_date);
    /// let results = evaluator.evaluate(&model)?;  // Uses stored market context
    /// # let _ = results;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_market_context(
        market_ctx: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> EvaluatorWithContext {
        EvaluatorWithContext {
            evaluator: Self::new(),
            market_ctx: market_ctx.clone(),
            as_of,
        }
    }

    /// Evaluate a financial model over all periods with optional market context.
    ///
    /// This method allows you to provide market context for pricing capital structure instruments.
    /// If capital structure is defined but market context is not provided, capital structure
    /// cashflows will not be computed (cs.* references will fail at runtime).
    ///
    /// # Arguments
    ///
    /// * `model` - The financial model specification
    /// * `market_ctx` - Optional market context for pricing instruments
    /// * `as_of` - Optional valuation date for pricing
    ///
    /// # Returns
    ///
    /// Returns `StatementResult` containing the evaluated values for all nodes and periods.
    pub fn evaluate_with_market_context(
        &mut self,
        model: &FinancialModelSpec,
        market_ctx: Option<&finstack_core::market_data::context::MarketContext>,
        as_of: Option<finstack_core::dates::Date>,
    ) -> Result<StatementResult> {
        let _span = tracing::info_span!(
            "statements.evaluate",
            model_id = model.id.as_str(),
            periods = model.periods.len(),
            nodes = model.nodes.len(),
            has_market = market_ctx.is_some(),
            has_as_of = as_of.is_some()
        )
        .entered();
        #[cfg(not(target_arch = "wasm32"))]
        let start = Instant::now();

        self.compiled_cache = std::sync::Arc::new(IndexMap::new());
        self.forecast_cache.clear();

        // Build dependency graph and check for cycles
        let dag = DependencyGraph::from_model(model)?;
        dag.detect_cycles()?;

        // Compute evaluation order
        let eval_order = evaluate_order(&dag)?;

        // Compile all formulas upfront
        self.compile_formulas(model)?;

        let node_to_column: std::sync::Arc<IndexMap<NodeId, usize>> = std::sync::Arc::new(
            eval_order
                .iter()
                .enumerate()
                .map(|(i, node_id)| (node_id.clone(), i))
                .collect(),
        );

        let cs_seed_nodes: HashSet<String> = model
            .nodes
            .iter()
            .filter_map(|(node_id, spec)| {
                if spec
                    .formula_text
                    .as_deref()
                    .is_some_and(|text| text.contains("cs."))
                    || spec
                        .where_text
                        .as_deref()
                        .is_some_and(|text| text.contains("cs."))
                {
                    Some(node_id.as_str().to_string())
                } else {
                    None
                }
            })
            .collect();

        let cs_affected_nodes = dependent_closure(&dag, &cs_seed_nodes);

        // Initialize capital structure state for dynamic evaluation
        let mut cs_state = if let (Some(_market_ctx), Some(_as_of)) = (market_ctx, as_of) {
            Some(crate::capital_structure::CapitalStructureState::new())
        } else {
            None
        };

        // Pre-compute instruments if market context is available
        let instruments = if let (Some(_market_ctx), Some(_as_of)) = (market_ctx, as_of) {
            self.build_instruments(model)?
        } else {
            None
        };

        if let (Some(state), Some(insts), Some(market_ctx), Some(as_of_date), Some(first_period)) = (
            cs_state.as_mut(),
            instruments.as_ref(),
            market_ctx,
            as_of,
            model.periods.first(),
        ) {
            for (instrument_id, instrument) in insts {
                let opening_balance = capital_structure_runtime::resolve_opening_balance(
                    instrument.as_ref(),
                    market_ctx,
                    as_of_date,
                    first_period.start,
                )?;

                state
                    .opening_balances
                    .insert(instrument_id.clone(), opening_balance);
            }
        }

        // Evaluate period-by-period
        let mut historical: IndexMap<PeriodId, IndexMap<String, f64>> = IndexMap::new();
        let mut historical_cs: IndexMap<
            PeriodId,
            crate::capital_structure::CapitalStructureCashflows,
        > = IndexMap::new();
        let mut all_warnings = Vec::new();
        let mut results = StatementResult::new();

        // Accumulator for capital structure cashflows across all periods
        let mut cs_cashflows_accum = crate::capital_structure::CapitalStructureCashflows::new();
        let mut has_cs = false;

        // Sequential evaluation for all models
        for period in &model.periods {
            let (period_results, period_warnings) =
                if let (Some(market_ctx), Some(as_of), Some(ref mut state), Some(insts)) =
                    (market_ctx, as_of, cs_state.as_mut(), instruments.as_ref())
                {
                    let (vals, warns, period_cs) = self.evaluate_period_dynamic(
                        model,
                        period,
                        period.is_actual,
                        &eval_order,
                        &node_to_column,
                        &historical,
                        &historical_cs,
                        market_ctx,
                        as_of,
                        insts,
                        state,
                        &cs_affected_nodes,
                    )?;

                    // Merge this period's cs cashflows into the accumulator
                    for (inst_id, period_map) in period_cs.by_instrument {
                        let accum_map =
                            cs_cashflows_accum.by_instrument.entry(inst_id).or_default();
                        for (pid, breakdown) in period_map {
                            accum_map.insert(pid, breakdown);
                        }
                    }
                    for (pid, breakdown) in period_cs.totals {
                        cs_cashflows_accum.totals.insert(pid, breakdown);
                    }
                    for (currency, period_map) in period_cs.totals_by_currency {
                        let accum_map = cs_cashflows_accum
                            .totals_by_currency
                            .entry(currency)
                            .or_default();
                        for (pid, breakdown) in period_map {
                            accum_map.insert(pid, breakdown);
                        }
                    }
                    if cs_cashflows_accum.reporting_currency.is_none() {
                        cs_cashflows_accum.reporting_currency = period_cs.reporting_currency;
                    }
                    has_cs = true;

                    (vals, warns)
                } else {
                    self.evaluate_period(
                        model,
                        &period.id,
                        period.is_actual,
                        &eval_order,
                        &node_to_column,
                        &historical,
                        &historical_cs,
                        None,
                    )?
                };

            all_warnings.extend(period_warnings.into_iter());

            // Store in results
            for (node_id, value) in &period_results {
                results
                    .nodes
                    .entry(node_id.to_owned())
                    .or_default()
                    .insert(period.id, *value);
            }

            // Add to historical context for next period
            historical.insert(period.id, period_results.clone());
            if has_cs {
                historical_cs.insert(period.id, cs_cashflows_accum.clone());
            }

            // Advance CS state for next period
            if let Some(ref mut state) = cs_state {
                state.advance_period();
            }
        }

        // Expose accumulated capital structure cashflows on the result
        if has_cs {
            results.cs_cashflows = Some(cs_cashflows_accum);
        }

        // Infer and populate node value types from model
        for (node_id, node_spec) in &model.nodes {
            let node_id_str = node_id.as_str();
            // Check if node has explicit value_type
            if let Some(value_type) = &node_spec.value_type {
                results
                    .node_value_types
                    .insert(node_id_str.to_string(), *value_type);

                // Populate monetary_nodes if this is a monetary type
                if let NodeValueType::Monetary { currency } = value_type {
                    if let Some(period_map) = results.nodes.get(node_id_str) {
                        let mut money_map = IndexMap::new();
                        for (period_id, &f64_value) in period_map {
                            money_map.insert(
                                *period_id,
                                finstack_core::money::Money::new(f64_value, *currency),
                            );
                        }
                        results
                            .monetary_nodes
                            .insert(node_id_str.to_string(), money_map);
                    }
                }
            } else if let Some(values) = &node_spec.values {
                if let Some(inferred_type) = crate::types::infer_series_value_type(values.values())?
                {
                    if let NodeValueType::Monetary { currency } = inferred_type {
                        results.node_value_types.insert(
                            node_id_str.to_string(),
                            NodeValueType::Monetary { currency },
                        );

                        // Populate monetary_nodes from Money values
                        if let Some(period_map) = results.nodes.get(node_id_str) {
                            let mut money_map = IndexMap::new();
                            for (period_id, &f64_value) in period_map {
                                money_map.insert(
                                    *period_id,
                                    finstack_core::money::Money::new(f64_value, currency),
                                );
                            }
                            results
                                .monetary_nodes
                                .insert(node_id_str.to_string(), money_map);
                        }
                    } else {
                        results
                            .node_value_types
                            .insert(node_id_str.to_string(), NodeValueType::Scalar);
                    }
                } else {
                    // No values, default to scalar
                    results
                        .node_value_types
                        .insert(node_id_str.to_string(), NodeValueType::Scalar);
                }
            } else {
                // No explicit value_type and no values, default to scalar
                results
                    .node_value_types
                    .insert(node_id_str.to_string(), NodeValueType::Scalar);
            }
        }

        // Set metadata
        results.meta = ResultsMeta {
            #[cfg(not(target_arch = "wasm32"))]
            eval_time_ms: Some(start.elapsed().as_millis() as u64),
            #[cfg(target_arch = "wasm32")]
            eval_time_ms: None,
            num_nodes: model.nodes.len(),
            num_periods: model.periods.len(),
            numeric_mode: crate::evaluator::NumericMode::Float64,
            rounding_context: None, // TODO(v0.5): wire through from FinstackConfig
            parallel: false,
            warnings: all_warnings,
        };

        Ok(results)
    }

    /// Evaluate a financial model over all periods.
    ///
    /// This is a convenience method that calls `evaluate_with_market_context` with no market context.
    /// If your model uses capital structure with cs.* references, use `evaluate_with_market_context`
    /// and provide market data.
    ///
    /// # Arguments
    ///
    /// * `model` - The financial model specification
    /// # Returns
    ///
    /// Returns `StatementResult` containing the evaluated values for all nodes and periods.
    pub fn evaluate(&mut self, model: &FinancialModelSpec) -> Result<StatementResult> {
        self.evaluate_with_market_context(model, None, None)
    }

    /// Evaluate a financial model in Monte Carlo mode.
    ///
    /// This method replays the model `n_paths` times with independent, but
    /// deterministic, seeds for stochastic forecast methods and aggregates
    /// the resulting distribution into percentile bands.
    ///
    /// Monte Carlo evaluation currently focuses on statement forecasts and
    /// does not support capital structure (`capital_structure`) integration.
    pub fn evaluate_monte_carlo(
        &mut self,
        model: &FinancialModelSpec,
        config: &MonteCarloConfig,
    ) -> Result<MonteCarloResults> {
        let _span = tracing::info_span!(
            "statements.evaluate_monte_carlo",
            model_id = model.id.as_str(),
            paths = config.n_paths,
            periods = model.periods.len(),
            nodes = model.nodes.len()
        )
        .entered();
        if config.n_paths == 0 {
            return Err(Error::eval(
                "Monte Carlo configuration requires n_paths > 0",
            ));
        }

        if model.capital_structure.is_some() {
            return Err(Error::eval(
                "Monte Carlo evaluation for statements does not yet support capital_structure. \
                 Run Monte Carlo on the underlying instruments using finstack-valuations \
                 for capital structure analysis.",
            ));
        }

        // Build dependency graph and evaluation order once.
        let dag = DependencyGraph::from_model(model)?;
        dag.detect_cycles()?;
        let eval_order = evaluate_order(&dag)?;

        self.compiled_cache = std::sync::Arc::new(IndexMap::new());
        self.forecast_cache.clear();
        self.compile_formulas(model)?;

        let node_to_column: std::sync::Arc<IndexMap<NodeId, usize>> = std::sync::Arc::new(
            eval_order
                .iter()
                .enumerate()
                .map(|(i, node_id)| (node_id.clone(), i))
                .collect(),
        );

        // Run paths — parallel when the `parallel` feature is enabled.
        let run_single_path = |path_idx: usize| -> Result<PathResult> {
            let mut path_eval = self.clone();
            path_eval.forecast_cache.clear();

            let seed_offset = config.seed.wrapping_add(path_idx as u64);
            let mut historical: IndexMap<PeriodId, IndexMap<String, f64>> = IndexMap::new();
            let mut all_warnings = Vec::new();

            for period in &model.periods {
                let (period_results, warnings) = path_eval.evaluate_period_mc(
                    model,
                    &period.id,
                    period.is_actual,
                    &eval_order,
                    &node_to_column,
                    &historical,
                    seed_offset,
                )?;
                all_warnings.extend(warnings);
                historical.insert(period.id, period_results.clone());
            }

            let mut node_map: IndexMap<String, IndexMap<PeriodId, f64>> = IndexMap::new();
            for (period_id, values) in &historical {
                for (node_id, value) in values {
                    node_map
                        .entry(node_id.clone())
                        .or_default()
                        .insert(*period_id, *value);
                }
            }
            Ok((node_map, all_warnings))
        };

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            let accumulator_seed = MonteCarloAccumulator::new(model, config)?;
            let accumulator = (0..config.n_paths)
                .into_par_iter()
                .try_fold(
                    || accumulator_seed.empty_like(),
                    |mut acc, path_idx| {
                        let (path_results, warnings) = run_single_path(path_idx)?;
                        acc.push_path(path_idx, path_results, warnings)?;
                        Ok(acc)
                    },
                )
                .try_reduce(
                    || accumulator_seed.empty_like(),
                    |left, right| left.merge(right),
                )?;
            accumulator.finish()
        }

        #[cfg(not(feature = "parallel"))]
        {
            let mut accumulator = MonteCarloAccumulator::new(model, config)?;
            for path_idx in 0..config.n_paths {
                let (path_results, warnings) = run_single_path(path_idx)?;
                accumulator.push_path(path_idx, path_results, warnings)?;
            }
            accumulator.finish()
        }
    }

    /// Compile all formulas in the model.
    fn compile_formulas(&mut self, model: &FinancialModelSpec) -> Result<()> {
        let cache = std::sync::Arc::make_mut(&mut self.compiled_cache);
        for (node_id, node_spec) in &model.nodes {
            if let Some(formula_text) = &node_spec.formula_text {
                if !cache.contains_key(node_id) {
                    let expr = dsl::parse_and_compile(formula_text)?;
                    cache.insert(node_id.clone(), expr);
                }
            }

            if let Some(where_text) = &node_spec.where_text {
                let where_key = NodeId::new(format!("__where__{}", node_id));
                if !cache.contains_key(&where_key) {
                    let expr = dsl::parse_and_compile(where_text)?;
                    cache.insert(where_key, expr);
                }
            }
        }
        Ok(())
    }

    /// Evaluate nodes in topological order within a context.
    ///
    /// This is the shared inner loop used by all period evaluation methods.
    /// It handles where-clause masking, precedence resolution, forecast/formula
    /// dispatch, and storing results in the context.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn evaluate_nodes_in_order(
        &mut self,
        model: &FinancialModelSpec,
        period_id: &PeriodId,
        is_actual: bool,
        eval_order: &[NodeId],
        context: &mut EvaluationContext,
        seed_offset: Option<u64>,
        node_filter: Option<&HashSet<String>>,
    ) -> Result<()> {
        for node_id in eval_order {
            if let Some(filter) = node_filter {
                if !filter.contains(node_id.as_str()) {
                    continue;
                }
            }

            let node_spec = model
                .get_node(node_id.as_str())
                .ok_or_else(|| Error::eval(format!("Node '{}' not found in model", node_id)))?;

            if node_spec.where_text.is_some() {
                let where_key = NodeId::new(format!("__where__{}", node_id));
                if let Some(where_expr) = self.compiled_cache.get(&where_key) {
                    let where_result = evaluate_formula(where_expr, context, None)?;
                    if where_result == 0.0 {
                        context.set_value(node_id.as_str(), 0.0)?;
                        continue;
                    }
                }
            }

            let value = (|| -> Result<f64> {
                let source = resolve_node_value(node_spec, period_id, is_actual)?;
                match source {
                    NodeValueSource::Value(v) => Ok(v),
                    NodeValueSource::Forecast => forecast_eval::evaluate_forecast(
                        node_spec,
                        model,
                        period_id,
                        context,
                        &mut self.forecast_cache,
                        seed_offset,
                    ),
                    NodeValueSource::Formula(_) => {
                        let expr = self.compiled_cache.get(node_id).ok_or_else(|| {
                            Error::eval(format!("No compiled formula for node '{}'", node_id))
                        })?;
                        evaluate_formula(expr, context, Some(node_id.as_str()))
                    }
                }
            })()
            .map_err(|e| {
                tracing::error!(
                    node_id = node_id.as_str(),
                    period = %period_id,
                    error = %e,
                    "node evaluation failed"
                );
                e
            })?;

            context.set_value(node_id.as_str(), value)?;
        }

        Ok(())
    }

    /// Evaluate a single period.
    #[allow(clippy::too_many_arguments)]
    fn evaluate_period(
        &mut self,
        model: &FinancialModelSpec,
        period_id: &PeriodId,
        is_actual: bool,
        eval_order: &[NodeId],
        node_to_column: &std::sync::Arc<IndexMap<NodeId, usize>>,
        historical: &IndexMap<PeriodId, IndexMap<String, f64>>,
        historical_cs: &IndexMap<PeriodId, crate::capital_structure::CapitalStructureCashflows>,
        cs_cashflows: Option<&crate::capital_structure::CapitalStructureCashflows>,
    ) -> Result<(IndexMap<String, f64>, Vec<EvalWarning>)> {
        let mut context = EvaluationContext::new(
            *period_id,
            std::sync::Arc::clone(node_to_column),
            std::sync::Arc::new(historical.clone()),
        );
        context.historical_capital_structure_cashflows = std::sync::Arc::new(historical_cs.clone());

        if let Some(cs) = cs_cashflows {
            context.capital_structure_cashflows = Some(cs.clone());
        }

        self.evaluate_nodes_in_order(
            model,
            period_id,
            is_actual,
            eval_order,
            &mut context,
            None,
            None,
        )?;

        Ok(context.into_results())
    }

    /// Evaluate a single period for Monte Carlo paths.
    ///
    /// Identical to [`evaluate_period`] but passes a seed offset to perturb
    /// stochastic forecast methods.
    #[allow(clippy::too_many_arguments)]
    fn evaluate_period_mc(
        &mut self,
        model: &FinancialModelSpec,
        period_id: &PeriodId,
        is_actual: bool,
        eval_order: &[NodeId],
        node_to_column: &std::sync::Arc<IndexMap<NodeId, usize>>,
        historical: &IndexMap<PeriodId, IndexMap<String, f64>>,
        seed_offset: u64,
    ) -> Result<(IndexMap<String, f64>, Vec<EvalWarning>)> {
        let mut context = EvaluationContext::new(
            *period_id,
            std::sync::Arc::clone(node_to_column),
            std::sync::Arc::new(historical.clone()),
        );

        self.evaluate_nodes_in_order(
            model,
            period_id,
            is_actual,
            eval_order,
            &mut context,
            Some(seed_offset),
            None,
        )?;

        Ok(context.into_results())
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// Evaluator with pre-configured market context.
///
/// This is a convenience wrapper that stores market context and as-of date,
/// making it easier to evaluate models with capital structure.
///
/// # Example
///
/// ```rust,no_run
/// use finstack_statements::evaluator::Evaluator;
/// use finstack_core::{dates::Date, market_data::context::MarketContext};
/// use finstack_statements::types::FinancialModelSpec;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let market_ctx = MarketContext::new();
/// let as_of = Date::from_calendar_date(2025, time::Month::January, 31).expect("test should succeed");
/// # let model: FinancialModelSpec = unimplemented!("build or load a model");
/// let mut evaluator = Evaluator::with_market_context(&market_ctx, as_of);
/// let results = evaluator.evaluate(&model)?;
/// # let _ = results;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct EvaluatorWithContext {
    evaluator: Evaluator,
    market_ctx: finstack_core::market_data::context::MarketContext,
    as_of: finstack_core::dates::Date,
}

impl EvaluatorWithContext {
    /// Evaluate a financial model using the stored market context.
    ///
    /// # Arguments
    ///
    /// * `model` - The financial model specification
    ///
    /// # Returns
    ///
    /// Returns `StatementResult` containing the evaluated values for all nodes and periods.
    pub fn evaluate(&mut self, model: &FinancialModelSpec) -> Result<StatementResult> {
        self.evaluator
            .evaluate_with_market_context(model, Some(&self.market_ctx), Some(self.as_of))
    }
}
