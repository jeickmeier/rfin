//! Main evaluator implementation.

use crate::dsl;
use crate::error::{Error, Result};
use crate::evaluator::context::EvaluationContext;
use crate::evaluator::dag::{evaluate_order, DependencyGraph};
use crate::evaluator::forecast_eval;
use crate::evaluator::formula::evaluate_formula;
use crate::evaluator::precedence::{resolve_node_value, NodeValueSource};
use crate::evaluator::results::{Results, ResultsMeta};
use crate::types::FinancialModelSpec;
use finstack_core::dates::PeriodId;
use finstack_core::expr::Expr;
use indexmap::IndexMap;
use std::time::Instant;

/// Evaluator for financial models.
///
/// The evaluator compiles formulas, resolves dependencies, and evaluates
/// nodes period-by-period according to precedence rules.
#[derive(Clone)]
pub struct Evaluator {
    /// Cached compiled expressions
    compiled_cache: IndexMap<String, Expr>,

    /// Cached forecast results: node_id → (period_id → value)
    forecast_cache: IndexMap<String, IndexMap<PeriodId, f64>>,
}

impl Evaluator {
    /// Create a new evaluator.
    pub fn new() -> Self {
        Self {
            compiled_cache: IndexMap::new(),
            forecast_cache: IndexMap::new(),
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
    /// Returns `Results` containing the evaluated values for all nodes and periods.
    pub fn evaluate_with_market_context(
        &mut self,
        model: &FinancialModelSpec,
        market_ctx: Option<&finstack_core::market_data::MarketContext>,
        as_of: Option<finstack_core::dates::Date>,
    ) -> Result<Results> {
        let start = Instant::now();

        // Clear forecast cache for new evaluation
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

        // Compute capital structure cashflows if market context is provided
        let cs_cashflows = if let (Some(market_ctx), Some(as_of)) = (market_ctx, as_of) {
            self.compute_cs_cashflows(model, market_ctx, as_of)?
        } else {
            None
        };

        // Evaluate period-by-period
        let mut historical: IndexMap<PeriodId, IndexMap<String, f64>> = IndexMap::new();
        let mut results = Results::new();

        // Sequential evaluation for all models
        for period in &model.periods {
            let period_results = self.evaluate_period(
                model,
                &period.id,
                period.is_actual,
                &eval_order,
                &node_to_column,
                &historical,
                cs_cashflows.as_ref(),
            )?;

            // Store in results
            for (node_id, value) in &period_results {
                results
                    .nodes
                    .entry(node_id.clone())
                    .or_default()
                    .insert(period.id, *value);
            }

            // Add to historical context for next period
            historical.insert(period.id, period_results);
        }

        // Set metadata
        results.meta = ResultsMeta {
            eval_time_ms: Some(start.elapsed().as_millis() as u64),
            num_nodes: model.nodes.len(),
            num_periods: model.periods.len(),
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
    /// Returns `Results` containing the evaluated values for all nodes and periods.
    pub fn evaluate(&mut self, model: &FinancialModelSpec) -> Result<Results> {
        self.evaluate_with_market_context(model, None, None)
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

    /// Compute capital structure cashflows from model's instrument specifications.
    ///
    /// This is a private method that encapsulates all capital structure computation logic.
    /// It builds instruments from the model's specs and aggregates cashflows by period.
    ///
    /// # Arguments
    /// * `model` - The financial model containing capital structure specs
    /// * `market_ctx` - Market context with discount/forward curves
    /// * `as_of` - Valuation date for pricing
    ///
    /// # Returns
    /// Returns `Some(cashflows)` if capital structure is defined, `None` otherwise.
    fn compute_cs_cashflows(
        &self,
        model: &FinancialModelSpec,
        market_ctx: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<Option<crate::capital_structure::CapitalStructureCashflows>> {
        use crate::capital_structure::integration;
        use crate::types::DebtInstrumentSpec;
        use finstack_valuations::cashflow::traits::CashflowProvider;
        use std::sync::Arc;

        // Return None if no capital structure is defined
        let cs_spec = match &model.capital_structure {
            Some(cs) => cs,
            None => return Ok(None),
        };

        // Build instruments from specifications using valuations types directly
        let mut instruments: IndexMap<String, Arc<dyn CashflowProvider + Send + Sync>> =
            IndexMap::new();

        for debt_spec in &cs_spec.debt_instruments {
            // build_any_instrument_from_spec handles all variants (Bond, Swap, Generic)
            let (id, instrument) = match debt_spec {
                DebtInstrumentSpec::Bond { id, .. }
                | DebtInstrumentSpec::Swap { id, .. }
                | DebtInstrumentSpec::Generic { id, .. } => {
                    let instrument = integration::build_any_instrument_from_spec(debt_spec)?;
                    (id.clone(), instrument)
                }
            };
            instruments.insert(id, instrument);
        }

        // Aggregate cashflows by period using valuations cashflow aggregation
        let cashflows = integration::aggregate_instrument_cashflows(
            &instruments,
            &model.periods,
            market_ctx,
            as_of,
        )?;

        Ok(Some(cashflows))
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
    ) -> Result<IndexMap<String, f64>> {
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
                    let where_result = evaluate_formula(where_expr, &context)?;
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
                    )?
                }
                NodeValueSource::Formula(_) => {
                    // Evaluate formula
                    let expr = self.compiled_cache.get(node_id).ok_or_else(|| {
                        Error::eval(format!("No compiled formula for node '{}'", node_id))
                    })?;
                    evaluate_formula(expr, &context)?
                }
            };

            // Store in context for dependent nodes
            context.set_value(node_id, value)?;
        }

        // Return results for this period
        Ok(context.into_results())
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::ModelBuilder;
    use crate::types::AmountOrScalar;

    #[test]
    fn test_simple_evaluation() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .unwrap()
            .value(
                "revenue",
                &[
                    (
                        PeriodId::quarter(2025, 1),
                        AmountOrScalar::scalar(100_000.0),
                    ),
                    (
                        PeriodId::quarter(2025, 2),
                        AmountOrScalar::scalar(110_000.0),
                    ),
                ],
            )
            .build()
            .unwrap();

        let mut evaluator = Evaluator::new();
        let results = evaluator.evaluate(&model).unwrap();

        assert_eq!(
            results.get("revenue", &PeriodId::quarter(2025, 1)),
            Some(100_000.0)
        );
        assert_eq!(
            results.get("revenue", &PeriodId::quarter(2025, 2)),
            Some(110_000.0)
        );
    }

    #[test]
    fn test_formula_evaluation() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .unwrap()
            .value(
                "revenue",
                &[
                    (
                        PeriodId::quarter(2025, 1),
                        AmountOrScalar::scalar(100_000.0),
                    ),
                    (
                        PeriodId::quarter(2025, 2),
                        AmountOrScalar::scalar(110_000.0),
                    ),
                ],
            )
            .compute("cogs", "revenue * 0.6")
            .unwrap()
            .build()
            .unwrap();

        let mut evaluator = Evaluator::new();
        let results = evaluator.evaluate(&model).unwrap();

        assert_eq!(
            results.get("cogs", &PeriodId::quarter(2025, 1)),
            Some(60_000.0)
        );
        assert_eq!(
            results.get("cogs", &PeriodId::quarter(2025, 2)),
            Some(66_000.0)
        );
    }

    #[test]
    fn test_circular_dependency_error() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .unwrap()
            .compute("a", "b + 1")
            .unwrap()
            .compute("b", "a + 1")
            .unwrap()
            .build()
            .unwrap();

        let mut evaluator = Evaluator::new();
        let result = evaluator.evaluate(&model);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circular"));
    }
}
