//! Main evaluator for financial models.

use crate::dsl::{compile, parse_formula};
use crate::error::{Error, Result};
use crate::evaluator::{
    context::StatementContext,
    dag::{evaluate_order, DependencyGraph},
    precedence::{resolve_node_value, NodeValueSource},
};
use crate::types::FinancialModelSpec;
use finstack_core::dates::PeriodId;
use finstack_core::expr::CompiledExpr;
use indexmap::IndexMap;

/// Results from model evaluation.
#[derive(Debug, Clone)]
pub struct Results {
    /// Map of node_id → (period_id → value)
    pub nodes: IndexMap<String, IndexMap<PeriodId, f64>>,

    /// Metadata about the evaluation
    pub meta: ResultsMeta,
}

/// Metadata about evaluation results.
#[derive(Debug, Clone, Default)]
pub struct ResultsMeta {
    /// Evaluation time in milliseconds
    pub eval_time_ms: Option<u64>,

    /// Number of nodes evaluated
    pub num_nodes: usize,

    /// Number of periods evaluated
    pub num_periods: usize,

    /// Was evaluation parallel?
    pub parallel: bool,
}

impl Results {
    /// Create empty results.
    pub fn new() -> Self {
        Self {
            nodes: IndexMap::new(),
            meta: ResultsMeta::default(),
        }
    }

    /// Get value for a specific node and period.
    pub fn get(&self, node_id: &str, period_id: &PeriodId) -> Option<f64> {
        self.nodes.get(node_id)?.get(period_id).copied()
    }

    /// Export results to long-format Polars DataFrame.
    ///
    /// Schema: `(node_id, period_id, value)`
    ///
    /// # Example
    ///
    /// ```ignore
    /// let df = results.to_polars_long()?;
    /// ```
    #[cfg(feature = "polars_export")]
    pub fn to_polars_long(&self) -> Result<polars::prelude::DataFrame> {
        crate::results::export::to_polars_long(self)
    }

    /// Export results to long-format Polars DataFrame with node filtering.
    ///
    /// # Arguments
    ///
    /// * `node_filter` - List of node IDs to include
    ///
    /// # Example
    ///
    /// ```ignore
    /// let df = results.to_polars_long_filtered(&["revenue", "cogs"])?;
    /// ```
    #[cfg(feature = "polars_export")]
    pub fn to_polars_long_filtered(&self, node_filter: &[&str]) -> Result<polars::prelude::DataFrame> {
        crate::results::export::to_polars_long_filtered(self, node_filter)
    }

    /// Export results to wide-format Polars DataFrame.
    ///
    /// Schema: periods as rows, nodes as columns
    ///
    /// # Example
    ///
    /// ```ignore
    /// let df = results.to_polars_wide()?;
    /// ```
    #[cfg(feature = "polars_export")]
    pub fn to_polars_wide(&self) -> Result<polars::prelude::DataFrame> {
        crate::results::export::to_polars_wide(self)
    }
}

impl Default for Results {
    fn default() -> Self {
        Self::new()
    }
}

/// Evaluator for financial models.
#[derive(Debug)]
pub struct Evaluator {
    /// Cached compiled expressions
    compiled_cache: IndexMap<String, CompiledExpr>,

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

    /// Evaluate a financial model.
    ///
    /// # Arguments
    ///
    /// * `model` - The financial model to evaluate
    /// * `parallel` - Whether to use parallel evaluation (Phase 3: always false)
    ///
    /// # Returns
    ///
    /// Results containing evaluated values for all nodes and periods.
    pub fn evaluate(&mut self, model: &FinancialModelSpec, parallel: bool) -> Result<Results> {
        let start = std::time::Instant::now();

        // Clear forecast cache for new evaluation
        self.forecast_cache.clear();

        // Build dependency graph and check for cycles
        let graph = DependencyGraph::from_model(model)?;
        graph.detect_cycles()?;

        // Get evaluation order (topological sort)
        let eval_order = evaluate_order(&graph)?;

        // Compile all formulas upfront
        self.compile_formulas(model)?;

        // Create node-to-column mapping
        let node_to_column: IndexMap<String, usize> = eval_order
            .iter()
            .enumerate()
            .map(|(idx, node_id)| (node_id.clone(), idx))
            .collect();

        // Evaluate period by period
        let mut results = Results::new();
        let mut historical = IndexMap::new();

        for period in &model.periods {
            let period_results = self.evaluate_period(
                model,
                &period.id,
                period.is_actual,
                &eval_order,
                &node_to_column,
                &historical,
            )?;

            // Store results for this period
            for (node_id, value) in &period_results {
                results
                    .nodes
                    .entry(node_id.clone())
                    .or_default()
                    .insert(period.id, *value);
            }

            // Add to historical results
            historical.insert(period.id, period_results);
        }

        // Set metadata
        results.meta.eval_time_ms = Some(start.elapsed().as_millis() as u64);
        results.meta.num_nodes = model.nodes.len();
        results.meta.num_periods = model.periods.len();
        results.meta.parallel = parallel;

        Ok(results)
    }

    /// Compile all formulas in the model.
    fn compile_formulas(&mut self, model: &FinancialModelSpec) -> Result<()> {
        for (node_id, node_spec) in &model.nodes {
            if let Some(formula) = &node_spec.formula_text {
                if !self.compiled_cache.contains_key(node_id) {
                    let ast = parse_formula(formula)?;
                    let expr = compile(&ast)?;
                    // For Phase 3, we store the Expr directly wrapped in a CompiledExpr
                    let compiled = CompiledExpr::new(expr);
                    self.compiled_cache.insert(node_id.clone(), compiled);
                }
            }

            // Compile where clause if present
            if let Some(where_clause) = &node_spec.where_text {
                let where_key = format!("{}_where", node_id);
                if !self.compiled_cache.contains_key(&where_key) {
                    let ast = parse_formula(where_clause)?;
                    let expr = compile(&ast)?;
                    let compiled = CompiledExpr::new(expr);
                    self.compiled_cache.insert(where_key, compiled);
                }
            }
        }
        Ok(())
    }

    /// Evaluate all nodes for a single period.
    fn evaluate_period(
        &mut self,
        model: &FinancialModelSpec,
        period_id: &PeriodId,
        is_actual: bool,
        eval_order: &[String],
        node_to_column: &IndexMap<String, usize>,
        historical: &IndexMap<PeriodId, IndexMap<String, f64>>,
    ) -> Result<IndexMap<String, f64>> {
        let mut context =
            StatementContext::new(*period_id, node_to_column.clone(), historical.clone());

        // Evaluate each node in topological order
        for node_id in eval_order {
            let node_spec = model.get_node(node_id).ok_or_else(|| Error::NodeNotFound {
                node_id: node_id.clone(),
            })?;

            // Resolve node value using precedence
            let value_source = resolve_node_value(node_spec, period_id, is_actual)?;

            let value = match value_source {
                NodeValueSource::Value(v) => v,
                NodeValueSource::Forecast => {
                    // Evaluate forecast methods (Phase 4)
                    self.evaluate_forecast(node_spec, model, period_id, &context)?
                }
                NodeValueSource::Formula(_formula) => {
                    // Evaluate formula using compiled expression
                    let compiled = self.compiled_cache.get(node_id).ok_or_else(|| {
                        Error::eval(format!("No compiled formula for node '{}'", node_id))
                    })?;

                    // Evaluate the compiled expression
                    let result = self.evaluate_formula(compiled, &context)?;

                    // Apply where clause if present
                    if let Some(_where_text) = &node_spec.where_text {
                        let where_key = format!("{}_where", node_id);
                        if let Some(where_compiled) = self.compiled_cache.get(&where_key) {
                            let where_result = self.evaluate_formula(where_compiled, &context)?;
                            // If where clause is false (0.0), return NaN to mask the value
                            if where_result == 0.0 {
                                0.0 // For Phase 3, use 0.0 for masked values
                            } else {
                                result
                            }
                        } else {
                            result
                        }
                    } else {
                        result
                    }
                }
            };

            // Store the value in context
            context.set_value(node_id, value)?;
        }

        Ok(context.into_results())
    }

    /// Evaluate a compiled formula expression.
    ///
    /// This is a simplified evaluator for Phase 3 that handles basic arithmetic
    /// operations encoded as synthetic function calls from the compiler.
    fn evaluate_formula(&self, compiled: &CompiledExpr, context: &StatementContext) -> Result<f64> {
        self.evaluate_expr(&compiled.ast, context)
    }

    /// Recursively evaluate an Expr.
    fn evaluate_expr(
        &self,
        expr: &finstack_core::expr::Expr,
        context: &StatementContext,
    ) -> Result<f64> {
        use finstack_core::expr::ExprNode;

        match &expr.node {
            ExprNode::Literal(v) => Ok(*v),

            ExprNode::Column(name) => context.get_value(name),

            ExprNode::Call(_func, args) => {
                // Check if this is a synthetic function call (encoded arithmetic operation)
                if let Some(first_arg) = args.first() {
                    if let ExprNode::Column(marker) = &first_arg.node {
                        if let Some(op_name) = marker.strip_prefix("__stmt_fn::") {
                            // This is a synthetic operation
                            return self.evaluate_synthetic_op(op_name, &args[1..], context);
                        }
                    }
                }

                // Otherwise, it's a real function - for Phase 3, we support limited functions
                Err(Error::eval(
                    "Function evaluation not fully implemented in Phase 3",
                ))
            }
        }
    }

    /// Evaluate synthetic operations (arithmetic, comparison, logical).
    fn evaluate_synthetic_op(
        &self,
        op_name: &str,
        args: &[finstack_core::expr::Expr],
        context: &StatementContext,
    ) -> Result<f64> {
        // Recursively evaluate arguments
        let eval_arg =
            |arg: &finstack_core::expr::Expr| -> Result<f64> { self.evaluate_expr(arg, context) };

        match op_name {
            "add" => {
                let left = eval_arg(&args[0])?;
                let right = eval_arg(&args[1])?;
                Ok(left + right)
            }
            "sub" => {
                let left = eval_arg(&args[0])?;
                let right = eval_arg(&args[1])?;
                Ok(left - right)
            }
            "mul" => {
                let left = eval_arg(&args[0])?;
                let right = eval_arg(&args[1])?;
                Ok(left * right)
            }
            "div" => {
                let left = eval_arg(&args[0])?;
                let right = eval_arg(&args[1])?;
                Ok(left / right)
            }
            "mod" => {
                let left = eval_arg(&args[0])?;
                let right = eval_arg(&args[1])?;
                Ok(left % right)
            }
            "eq" => {
                let left = eval_arg(&args[0])?;
                let right = eval_arg(&args[1])?;
                Ok(if (left - right).abs() < 1e-10 {
                    1.0
                } else {
                    0.0
                })
            }
            "ne" => {
                let left = eval_arg(&args[0])?;
                let right = eval_arg(&args[1])?;
                Ok(if (left - right).abs() >= 1e-10 {
                    1.0
                } else {
                    0.0
                })
            }
            "lt" => {
                let left = eval_arg(&args[0])?;
                let right = eval_arg(&args[1])?;
                Ok(if left < right { 1.0 } else { 0.0 })
            }
            "le" => {
                let left = eval_arg(&args[0])?;
                let right = eval_arg(&args[1])?;
                Ok(if left <= right { 1.0 } else { 0.0 })
            }
            "gt" => {
                let left = eval_arg(&args[0])?;
                let right = eval_arg(&args[1])?;
                Ok(if left > right { 1.0 } else { 0.0 })
            }
            "ge" => {
                let left = eval_arg(&args[0])?;
                let right = eval_arg(&args[1])?;
                Ok(if left >= right { 1.0 } else { 0.0 })
            }
            "and" => {
                let left = eval_arg(&args[0])?;
                let right = eval_arg(&args[1])?;
                Ok(if left != 0.0 && right != 0.0 {
                    1.0
                } else {
                    0.0
                })
            }
            "or" => {
                let left = eval_arg(&args[0])?;
                let right = eval_arg(&args[1])?;
                Ok(if left != 0.0 || right != 0.0 {
                    1.0
                } else {
                    0.0
                })
            }
            "not" => {
                let val = eval_arg(&args[0])?;
                Ok(if val == 0.0 { 1.0 } else { 0.0 })
            }
            "if" => {
                let condition = eval_arg(&args[0])?;
                if condition != 0.0 {
                    eval_arg(&args[1])
                } else {
                    eval_arg(&args[2])
                }
            }
            _ => Err(Error::eval(format!(
                "Unknown synthetic operation: {}",
                op_name
            ))),
        }
    }

    /// Evaluate forecast for a node in a specific period.
    ///
    /// This finds the base value (last actual or computed value before forecast periods)
    /// and applies the forecast method to generate the forecasted value.
    /// Results are cached so the forecast is only computed once for all periods.
    fn evaluate_forecast(
        &mut self,
        node_spec: &crate::types::NodeSpec,
        model: &FinancialModelSpec,
        period_id: &PeriodId,
        context: &StatementContext,
    ) -> Result<f64> {
        // Check if we already have cached forecast for this node
        if let Some(cached) = self.forecast_cache.get(&node_spec.node_id) {
            if let Some(&value) = cached.get(period_id) {
                return Ok(value);
            }
        }

        // Get forecast specs for this node
        if node_spec.forecasts.is_empty() {
            return Err(Error::eval(format!(
                "Node '{}' has no forecast specifications",
                node_spec.node_id
            )));
        }

        // Find all forecast periods (periods where is_actual = false)
        let forecast_periods: Vec<PeriodId> = model
            .periods
            .iter()
            .filter(|p| !p.is_actual)
            .map(|p| p.id)
            .collect();

        if forecast_periods.is_empty() {
            return Err(Error::eval(format!(
                "No forecast periods found for node '{}'",
                node_spec.node_id
            )));
        }

        // Determine base value: last actual value or last computed value
        let base_value = self.determine_base_value(node_spec, period_id, model, context)?;

        // Apply first forecast spec (for Phase 4, we support one forecast spec per node)
        let forecast_spec = &node_spec.forecasts[0];
        let forecast_values =
            crate::forecast::apply_forecast(forecast_spec, base_value, &forecast_periods)?;

        // Cache the forecast results for all periods
        self.forecast_cache
            .insert(node_spec.node_id.clone(), forecast_values.clone());

        // Get the value for this specific period
        forecast_values.get(period_id).copied().ok_or_else(|| {
            Error::eval(format!(
                "Forecast did not generate value for period {}",
                period_id
            ))
        })
    }

    /// Determine the base value for forecast (last actual or last computed value).
    fn determine_base_value(
        &self,
        node_spec: &crate::types::NodeSpec,
        current_period_id: &PeriodId,
        model: &FinancialModelSpec,
        context: &StatementContext,
    ) -> Result<f64> {
        // Try to find the last actual value before current period
        // Scan backwards through periods
        for period in model.periods.iter().rev() {
            if period.id >= *current_period_id {
                continue; // Skip current and future periods
            }

            // Check if we have an explicit value for this period
            if let Some(ref values) = node_spec.values {
                if let Some(value) = values.get(&period.id) {
                    return Ok(value.value());
                }
            }

            // Check historical results
            if let Some(historical_value) =
                context.get_historical_value(&node_spec.node_id, &period.id)
            {
                return Ok(historical_value);
            }
        }

        // If no prior value found, use the first explicit value if available
        if let Some(ref values) = node_spec.values {
            if let Some(first_value) = values.values().next() {
                return Ok(first_value.value());
            }
        }

        Err(Error::eval(format!(
            "No base value found for forecast of node '{}'",
            node_spec.node_id
        )))
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
                    (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0)),
                    (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(110.0)),
                ],
            )
            .build()
            .unwrap();

        let mut evaluator = Evaluator::new();
        let results = evaluator.evaluate(&model, false).unwrap();

        assert_eq!(
            results.get("revenue", &PeriodId::quarter(2025, 1)),
            Some(100.0)
        );
        assert_eq!(
            results.get("revenue", &PeriodId::quarter(2025, 2)),
            Some(110.0)
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
                    (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0)),
                    (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(110.0)),
                ],
            )
            .compute("cogs", "revenue * 0.6")
            .unwrap()
            .compute("gross_profit", "revenue - cogs")
            .unwrap()
            .build()
            .unwrap();

        let mut evaluator = Evaluator::new();
        let results = evaluator.evaluate(&model, false).unwrap();

        // Check COGS (60% of revenue)
        assert_eq!(results.get("cogs", &PeriodId::quarter(2025, 1)), Some(60.0));
        assert_eq!(results.get("cogs", &PeriodId::quarter(2025, 2)), Some(66.0));

        // Check gross profit
        assert_eq!(
            results.get("gross_profit", &PeriodId::quarter(2025, 1)),
            Some(40.0)
        );
        assert_eq!(
            results.get("gross_profit", &PeriodId::quarter(2025, 2)),
            Some(44.0)
        );
    }

    #[test]
    fn test_circular_dependency_error() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .unwrap()
            .compute("a", "b + 1")
            .unwrap()
            .compute("b", "c + 1")
            .unwrap()
            .compute("c", "a + 1")
            .unwrap()
            .build()
            .unwrap();

        let mut evaluator = Evaluator::new();
        let result = evaluator.evaluate(&model, false);

        assert!(result.is_err());
    }
}
