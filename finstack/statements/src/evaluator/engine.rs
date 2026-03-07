//! Main evaluator implementation.

use crate::analysis::{MonteCarloConfig, MonteCarloResults};
use crate::dsl;
use crate::error::{Error, Result};
use crate::evaluator::context::EvaluationContext;
use crate::evaluator::dag::{evaluate_order, DependencyGraph};
use crate::evaluator::forecast_eval;
use crate::evaluator::formula::evaluate_formula;
use crate::evaluator::precedence::{resolve_node_value, NodeValueSource};
use crate::evaluator::results::{EvalWarning, ResultsMeta, StatementResult};
use crate::types::{FinancialModelSpec, NodeValueType};
use finstack_core::dates::PeriodId;
use finstack_core::expr::Expr;
use finstack_core::money::Money;
use indexmap::IndexMap;
use std::collections::HashSet;
use std::sync::Arc;
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
    /// Cached compiled expressions
    compiled_cache: IndexMap<String, Expr>,

    /// Cached forecast results: node_id → (period_id → value)
    forecast_cache: IndexMap<String, IndexMap<PeriodId, f64>>,
}

impl Evaluator {
    /// Create a new evaluator.
    #[must_use = "creating an evaluator has no effect without calling evaluate()"]
    pub fn new() -> Self {
        Self {
            compiled_cache: IndexMap::new(),
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
        #[cfg(not(target_arch = "wasm32"))]
        let start = Instant::now();

        // Clear caches for new evaluation to avoid stale formula/forecast reuse
        self.compiled_cache.clear();
        self.forecast_cache.clear();

        // Build dependency graph and check for cycles
        let dag = DependencyGraph::from_model(model)?;
        dag.detect_cycles()?;

        // Compute evaluation order
        let eval_order = evaluate_order(&dag)?;

        // Compile all formulas upfront
        self.compile_formulas(model)?;

        // Build node-to-column index
        let node_to_column: IndexMap<String, usize> = eval_order
            .iter()
            .enumerate()
            .map(|(i, node_id)| (node_id.clone(), i))
            .collect();

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
                    Some(node_id.clone())
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
                let schedule = instrument.build_full_schedule(market_ctx, as_of_date)?;
                let outstanding_path = schedule.outstanding_by_date()?;
                let opening_balance = outstanding_path
                    .iter()
                    .filter(|(d, _)| *d <= first_period.start)
                    .map(|(_, outstanding)| {
                        if outstanding.amount() < 0.0 {
                            Money::new(-outstanding.amount(), outstanding.currency())
                        } else {
                            *outstanding
                        }
                    })
                    .next_back()
                    .unwrap_or_else(|| {
                        outstanding_path
                            .first()
                            .map(|(_, outstanding)| {
                                if outstanding.amount() < 0.0 {
                                    Money::new(-outstanding.amount(), outstanding.currency())
                                } else {
                                    *outstanding
                                }
                            })
                            .unwrap_or_else(|| {
                                schedule
                                    .flows
                                    .first()
                                    .map(|cf| Money::new(0.0, cf.amount.currency()))
                                    .unwrap_or_else(|| {
                                        Money::new(0.0, finstack_core::currency::Currency::USD)
                                    })
                            })
                    });

                state
                    .opening_balances
                    .insert(instrument_id.clone(), opening_balance);
            }
        }

        // Evaluate period-by-period
        let mut historical: IndexMap<PeriodId, IndexMap<String, f64>> = IndexMap::new();
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
            // Check if node has explicit value_type
            if let Some(value_type) = &node_spec.value_type {
                results
                    .node_value_types
                    .insert(node_id.clone(), *value_type);

                // Populate monetary_nodes if this is a monetary type
                if let NodeValueType::Monetary { currency } = value_type {
                    if let Some(period_map) = results.nodes.get(node_id) {
                        let mut money_map = IndexMap::new();
                        for (period_id, &f64_value) in period_map {
                            money_map.insert(
                                *period_id,
                                finstack_core::money::Money::new(f64_value, *currency),
                            );
                        }
                        results.monetary_nodes.insert(node_id.clone(), money_map);
                    }
                }
            } else if let Some(values) = &node_spec.values {
                // Infer from first value
                if let Some((_, first_value)) = values.iter().next() {
                    if let Some(money) = first_value.as_money() {
                        results.node_value_types.insert(
                            node_id.clone(),
                            NodeValueType::Monetary {
                                currency: money.currency(),
                            },
                        );

                        // Populate monetary_nodes from Money values
                        if let Some(period_map) = results.nodes.get(node_id) {
                            let mut money_map = IndexMap::new();
                            for (period_id, &f64_value) in period_map {
                                money_map.insert(
                                    *period_id,
                                    finstack_core::money::Money::new(f64_value, money.currency()),
                                );
                            }
                            results.monetary_nodes.insert(node_id.clone(), money_map);
                        }
                    } else {
                        results
                            .node_value_types
                            .insert(node_id.clone(), NodeValueType::Scalar);
                    }
                } else {
                    // No values, default to scalar
                    results
                        .node_value_types
                        .insert(node_id.clone(), NodeValueType::Scalar);
                }
            } else {
                // No explicit value_type and no values, default to scalar
                results
                    .node_value_types
                    .insert(node_id.clone(), NodeValueType::Scalar);
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
            rounding_context: None, // Not implemented yet
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

        // Compile formulas once and reset caches for this MC run.
        self.compiled_cache.clear();
        self.forecast_cache.clear();
        self.compile_formulas(model)?;

        // Build node-to-column index
        let node_to_column: IndexMap<String, usize> = eval_order
            .iter()
            .enumerate()
            .map(|(i, node_id)| (node_id.clone(), i))
            .collect();

        // Collect per-path node/period results.
        let mut all_paths: Vec<IndexMap<String, IndexMap<PeriodId, f64>>> =
            Vec::with_capacity(config.n_paths);

        for path_idx in 0..config.n_paths {
            // Clear forecast cache for each path to avoid sharing simulated
            // values across paths.
            self.forecast_cache.clear();

            let seed_offset = config.seed.wrapping_add(path_idx as u64);
            let mut historical: IndexMap<PeriodId, IndexMap<String, f64>> = IndexMap::new();

            for period in &model.periods {
                let (period_results, _warnings) = self.evaluate_period_mc(
                    model,
                    &period.id,
                    period.is_actual,
                    &eval_order,
                    &node_to_column,
                    &historical,
                    seed_offset,
                )?;

                // Store in historical context for next period
                historical.insert(period.id, period_results.clone());
            }

            // Transpose historical (period-centric) into node-centric layout.
            let mut node_map: IndexMap<String, IndexMap<PeriodId, f64>> = IndexMap::new();
            for (period_id, values) in &historical {
                for (node_id, value) in values {
                    node_map
                        .entry(node_id.clone())
                        .or_default()
                        .insert(*period_id, *value);
                }
            }

            all_paths.push(node_map);
        }

        crate::analysis::monte_carlo::aggregate_monte_carlo_paths(model, config, &all_paths)
    }

    /// Compile all formulas in the model.
    fn compile_formulas(&mut self, model: &FinancialModelSpec) -> Result<()> {
        // Sequential compilation
        for (node_id, node_spec) in &model.nodes {
            // Compile formula if present
            if let Some(formula_text) = &node_spec.formula_text {
                if !self.compiled_cache.contains_key(node_id) {
                    let expr = dsl::parse_and_compile(formula_text)?;
                    self.compiled_cache.insert(node_id.clone(), expr);
                }
            }

            // Compile where clause if present
            if let Some(where_text) = &node_spec.where_text {
                let where_key = format!("__where__{}", node_id);
                if !self.compiled_cache.contains_key(&where_key) {
                    let expr = dsl::parse_and_compile(where_text)?;
                    self.compiled_cache.insert(where_key, expr);
                }
            }
        }
        Ok(())
    }

    /// Build instruments from model specifications.
    ///
    /// Returns a map of instrument IDs to CashflowProvider trait objects.
    fn build_instruments(
        &self,
        model: &FinancialModelSpec,
    ) -> Result<
        Option<
            IndexMap<
                String,
                std::sync::Arc<dyn finstack_valuations::cashflow::CashflowProvider + Send + Sync>,
            >,
        >,
    > {
        use crate::capital_structure::integration;
        use crate::types::DebtInstrumentSpec;
        use finstack_valuations::cashflow::CashflowProvider;

        let cs_spec = match &model.capital_structure {
            Some(cs) => cs,
            None => return Ok(None),
        };

        let mut instruments: IndexMap<String, Arc<dyn CashflowProvider + Send + Sync>> =
            IndexMap::new();

        for debt_spec in &cs_spec.debt_instruments {
            let (id, instrument) = match debt_spec {
                DebtInstrumentSpec::Bond { id, .. }
                | DebtInstrumentSpec::Swap { id, .. }
                | DebtInstrumentSpec::TermLoan { id, .. }
                | DebtInstrumentSpec::Generic { id, .. } => {
                    let instrument = integration::build_any_instrument_from_spec(debt_spec)?;
                    (id.clone(), instrument)
                }
            };
            instruments.insert(id, instrument);
        }

        Ok(Some(instruments))
    }

    /// Evaluate a period with dynamic capital structure support.
    ///
    /// This method:
    /// 1. Pre-Model: Calculate contractual CS flows based on opening balances
    /// 2. Model Eval: Evaluate standard model nodes
    /// 3. Post-Model: Run waterfall logic to calculate sweeps/prepayments
    #[allow(clippy::too_many_arguments)]
    fn evaluate_period_dynamic(
        &mut self,
        model: &FinancialModelSpec,
        period: &finstack_core::dates::Period,
        is_actual: bool,
        eval_order: &[String],
        node_to_column: &IndexMap<String, usize>,
        historical: &IndexMap<PeriodId, IndexMap<String, f64>>,
        market_ctx: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        instruments: &IndexMap<
            String,
            std::sync::Arc<dyn finstack_valuations::cashflow::CashflowProvider + Send + Sync>,
        >,
        cs_state: &mut crate::capital_structure::CapitalStructureState,
        cs_affected_nodes: &HashSet<String>,
    ) -> Result<(
        IndexMap<String, f64>,
        Vec<EvalWarning>,
        crate::capital_structure::CapitalStructureCashflows,
    )> {
        use crate::capital_structure::integration;
        use indexmap::IndexMap;

        // Step 1: Pre-Model - Calculate contractual flows based on opening balances
        let mut contractual_flows: IndexMap<String, crate::capital_structure::CashflowBreakdown> =
            IndexMap::new();

        for (instrument_id, instrument) in instruments {
            let opening_balance =
                if let Some(balance) = cs_state.opening_balances.get(instrument_id).copied() {
                    balance
                } else {
                    // Fallback: if state has no opening balance for this instrument, seed a
                    // zero balance in the instrument's currency.
                    let schedule = instrument.build_full_schedule(market_ctx, as_of)?;
                    Money::new(0.0, schedule.notional.initial.currency())
                };

            let (breakdown, closing_balance) = integration::calculate_period_flows(
                instrument.as_ref(),
                period,
                opening_balance,
                market_ctx,
                as_of,
            )?;

            contractual_flows.insert(instrument_id.to_string(), breakdown.clone());
            cs_state.set_closing_balance(instrument_id.to_string(), closing_balance);
        }

        // Helper to recompute capital structure totals for the current period
        let period_id = period.id;
        let recompute_totals =
            |cashflows: &mut crate::capital_structure::CapitalStructureCashflows| {
                let mut total_breakdown: Option<crate::capital_structure::CashflowBreakdown> = None;

                for breakdown in cashflows
                    .by_instrument
                    .values()
                    .filter_map(|period_map| period_map.get(&period_id))
                {
                    if let Some(total) = &mut total_breakdown {
                        total.interest_expense_cash += breakdown.interest_expense_cash;
                        total.interest_expense_pik += breakdown.interest_expense_pik;
                        total.principal_payment += breakdown.principal_payment;
                        total.fees += breakdown.fees;
                        total.debt_balance += breakdown.debt_balance;
                        total.accrued_interest += breakdown.accrued_interest;
                    } else {
                        total_breakdown = Some(breakdown.clone());
                    }
                }

                if let Some(total) = total_breakdown {
                    cashflows.totals.insert(period_id, total.clone());
                    cashflows.reporting_currency = Some(total.interest_expense_cash.currency());
                }
            };

        // Create initial context with contractual CS flows
        let mut cs_cashflows = crate::capital_structure::CapitalStructureCashflows::new();
        for (inst_id, breakdown) in &contractual_flows {
            let mut period_map: indexmap::IndexMap<
                finstack_core::dates::PeriodId,
                crate::capital_structure::CashflowBreakdown,
            > = indexmap::IndexMap::new();
            period_map.insert(period_id, breakdown.clone());
            cs_cashflows
                .by_instrument
                .insert(inst_id.clone(), period_map);
        }
        recompute_totals(&mut cs_cashflows);

        let mut context =
            EvaluationContext::new(period_id, node_to_column.clone(), historical.clone());
        context.capital_structure_cashflows = Some(cs_cashflows.clone());

        // Step 2: Model Eval - Evaluate standard model nodes
        for node_id in eval_order {
            let node_spec = model
                .get_node(node_id)
                .ok_or_else(|| Error::eval(format!("Node '{}' not found in model", node_id)))?;

            if node_spec.where_text.is_some() {
                let where_key = format!("__where__{}", node_id);
                if let Some(where_expr) = self.compiled_cache.get(&where_key) {
                    let where_result = evaluate_formula(where_expr, &mut context, None)?;
                    if where_result == 0.0 {
                        context.set_value(node_id, 0.0)?;
                        continue;
                    }
                }
            }

            let source =
                crate::evaluator::precedence::resolve_node_value(node_spec, &period.id, is_actual)?;

            let value = match source {
                crate::evaluator::precedence::NodeValueSource::Value(v) => v,
                crate::evaluator::precedence::NodeValueSource::Forecast => {
                    crate::evaluator::forecast_eval::evaluate_forecast(
                        node_spec,
                        model,
                        &period.id,
                        &context,
                        &mut self.forecast_cache,
                        None,
                    )?
                }
                crate::evaluator::precedence::NodeValueSource::Formula(_) => {
                    let expr = self.compiled_cache.get(node_id).ok_or_else(|| {
                        Error::eval(format!("No compiled formula for node '{}'", node_id))
                    })?;
                    evaluate_formula(expr, &mut context, Some(node_id))?
                }
            };

            context.set_value(node_id, value)?;
        }

        // Step 3: Post-Model - Run waterfall logic if configured
        if let Some(cs_spec) = &model.capital_structure {
            if let Some(waterfall_spec) = &cs_spec.waterfall {
                let updated_flows = crate::capital_structure::waterfall::execute_waterfall(
                    &period_id,
                    &context,
                    waterfall_spec,
                    cs_state,
                    &contractual_flows,
                )?;

                // Update CS cashflows with sweep amounts
                for (inst_id, breakdown) in &updated_flows {
                    if let Some(period_map) = cs_cashflows.by_instrument.get_mut(inst_id) {
                        period_map.insert(period_id, breakdown.clone());
                    } else {
                        let mut period_map: indexmap::IndexMap<
                            finstack_core::dates::PeriodId,
                            crate::capital_structure::CashflowBreakdown,
                        > = indexmap::IndexMap::new();
                        period_map.insert(period_id, breakdown.clone());
                        cs_cashflows
                            .by_instrument
                            .insert(inst_id.clone(), period_map);
                    }
                }
                recompute_totals(&mut cs_cashflows);
                context.capital_structure_cashflows = Some(cs_cashflows);
            }
        }

        if context.capital_structure_cashflows.is_some() && !cs_affected_nodes.is_empty() {
            // Re-evaluate any nodes that are downstream of CS references now that CS cashflows have
            // been updated by the waterfall.
            for node_id in eval_order {
                if !cs_affected_nodes.contains(node_id) {
                    continue;
                }

                let node_spec = model
                    .get_node(node_id)
                    .ok_or_else(|| Error::eval(format!("Node '{}' not found in model", node_id)))?;

                if node_spec.where_text.is_some() {
                    let where_key = format!("__where__{}", node_id);
                    if let Some(where_expr) = self.compiled_cache.get(&where_key) {
                        let where_result = evaluate_formula(where_expr, &mut context, None)?;
                        if where_result == 0.0 {
                            context.set_value(node_id, 0.0)?;
                            continue;
                        }
                    }
                }

                let source = resolve_node_value(node_spec, &period.id, is_actual)?;
                let value = match source {
                    NodeValueSource::Value(v) => v,
                    NodeValueSource::Forecast => forecast_eval::evaluate_forecast(
                        node_spec,
                        model,
                        &period.id,
                        &context,
                        &mut self.forecast_cache,
                        None,
                    )?,
                    NodeValueSource::Formula(_) => {
                        let expr = self.compiled_cache.get(node_id).ok_or_else(|| {
                            Error::eval(format!("No compiled formula for node '{}'", node_id))
                        })?;
                        evaluate_formula(expr, &mut context, Some(node_id))?
                    }
                };

                context.set_value(node_id, value)?;
            }
        }

        let period_cs_cashflows = context
            .capital_structure_cashflows
            .take()
            .unwrap_or_default();
        let (values, warnings) = context.into_results();
        Ok((values, warnings, period_cs_cashflows))
    }

    /// Evaluate a single period.
    #[allow(clippy::too_many_arguments)]
    fn evaluate_period(
        &mut self,
        model: &FinancialModelSpec,
        period_id: &PeriodId,
        is_actual: bool,
        eval_order: &[String],
        node_to_column: &IndexMap<String, usize>,
        historical: &IndexMap<PeriodId, IndexMap<String, f64>>,
        cs_cashflows: Option<&crate::capital_structure::CapitalStructureCashflows>,
    ) -> Result<(IndexMap<String, f64>, Vec<EvalWarning>)> {
        // Create evaluation context
        let mut context =
            EvaluationContext::new(*period_id, node_to_column.clone(), historical.clone());

        // Add capital structure cashflows if available
        if let Some(cs) = cs_cashflows {
            context.capital_structure_cashflows = Some(cs.clone());
        }

        // Evaluate nodes in topological order
        for node_id in eval_order {
            let node_spec = model
                .get_node(node_id)
                .ok_or_else(|| Error::eval(format!("Node '{}' not found in model", node_id)))?;

            // Check where clause if present
            if node_spec.where_text.is_some() {
                let where_key = format!("__where__{}", node_id);
                if let Some(where_expr) = self.compiled_cache.get(&where_key) {
                    let where_result = evaluate_formula(where_expr, &mut context, None)?;
                    // If where clause evaluates to false (0.0), skip this node
                    if where_result == 0.0 {
                        // Set value to 0.0 or NaN for masked nodes
                        context.set_value(node_id, 0.0)?;
                        continue;
                    }
                }
            }

            // Resolve value using precedence: Value > Forecast > Formula
            let source = resolve_node_value(node_spec, period_id, is_actual)?;

            let value = match source {
                NodeValueSource::Value(v) => v,
                NodeValueSource::Forecast => {
                    // Evaluate forecast
                    forecast_eval::evaluate_forecast(
                        node_spec,
                        model,
                        period_id,
                        &context,
                        &mut self.forecast_cache,
                        None,
                    )?
                }
                NodeValueSource::Formula(_) => {
                    // Evaluate formula
                    let expr = self.compiled_cache.get(node_id).ok_or_else(|| {
                        Error::eval(format!("No compiled formula for node '{}'", node_id))
                    })?;
                    evaluate_formula(expr, &mut context, Some(node_id))?
                }
            };

            // Store in context for dependent nodes
            context.set_value(node_id, value)?;
        }

        // Return results for this period
        let (values, warnings) = context.into_results();
        Ok((values, warnings))
    }

    /// Evaluate a single period for Monte Carlo paths.
    ///
    /// This is similar to [`evaluate_period`] but accepts a seed offset used to
    /// perturb stochastic forecast methods while keeping all other behavior
    /// identical.
    #[allow(clippy::too_many_arguments)]
    fn evaluate_period_mc(
        &mut self,
        model: &FinancialModelSpec,
        period_id: &PeriodId,
        is_actual: bool,
        eval_order: &[String],
        node_to_column: &IndexMap<String, usize>,
        historical: &IndexMap<PeriodId, IndexMap<String, f64>>,
        seed_offset: u64,
    ) -> Result<(IndexMap<String, f64>, Vec<EvalWarning>)> {
        // Create evaluation context
        let mut context =
            EvaluationContext::new(*period_id, node_to_column.clone(), historical.clone());

        // Evaluate nodes in topological order
        for node_id in eval_order {
            let node_spec = model
                .get_node(node_id)
                .ok_or_else(|| Error::eval(format!("Node '{}' not found in model", node_id)))?;

            // Check where clause if present
            if node_spec.where_text.is_some() {
                let where_key = format!("__where__{}", node_id);
                if let Some(where_expr) = self.compiled_cache.get(&where_key) {
                    let where_result = evaluate_formula(where_expr, &mut context, None)?;
                    // If where clause evaluates to false (0.0), skip this node
                    if where_result == 0.0 {
                        context.set_value(node_id, 0.0)?;
                        continue;
                    }
                }
            }

            // Resolve value using precedence: Value > Forecast > Formula
            let source = resolve_node_value(node_spec, period_id, is_actual)?;

            let value = match source {
                NodeValueSource::Value(v) => v,
                NodeValueSource::Forecast => {
                    // Evaluate forecast with per-path seed offset
                    forecast_eval::evaluate_forecast(
                        node_spec,
                        model,
                        period_id,
                        &context,
                        &mut self.forecast_cache,
                        Some(seed_offset),
                    )?
                }
                NodeValueSource::Formula(_) => {
                    // Evaluate formula
                    let expr = self.compiled_cache.get(node_id).ok_or_else(|| {
                        Error::eval(format!("No compiled formula for node '{}'", node_id))
                    })?;
                    evaluate_formula(expr, &mut context, Some(node_id))?
                }
            };

            // Store in context for dependent nodes
            context.set_value(node_id, value)?;
        }

        // Return results for this period
        let (values, warnings) = context.into_results();
        Ok((values, warnings))
    }
}

fn dependent_closure(graph: &DependencyGraph, seeds: &HashSet<String>) -> HashSet<String> {
    let mut visited: HashSet<String> = seeds.iter().cloned().collect();
    let mut stack: Vec<String> = seeds.iter().cloned().collect();

    while let Some(node) = stack.pop() {
        if let Some(dependents) = graph.dependents.get(&node) {
            for dependent in dependents {
                if visited.insert(dependent.clone()) {
                    stack.push(dependent.clone());
                }
            }
        }
    }

    visited
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
