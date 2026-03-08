use super::constraints::{Constraint, Inequality};
use super::decision::{
    build_decision_space, DecisionFeatures, DecisionItem, OptimizationDenominators,
};
use super::problem::PortfolioOptimizationProblem;
use super::result::{OptimizationStatus, PortfolioOptimizationResult};
use super::types::{MetricExpr, MissingMetricPolicy, PerPositionMetric, WeightingScheme};
use super::universe::{PositionFilter, TradeUniverse};
use crate::error::{Error, Result};
use crate::portfolio::Portfolio;
use crate::types::PositionId;
use finstack_core::config::FinstackConfig;
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::summation::neumaier_sum;
use finstack_valuations::metrics::MetricId;
use good_lp::{constraint, default_solver, variable, Expression, Solution, SolverModel};
use indexmap::IndexMap;

/// Optimizer interface; allows swapping implementations (LP, QP, etc.).
pub trait PortfolioOptimizer: Send + Sync {
    /// Optimize the portfolio for the given problem and market/config context.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when:
    /// - Portfolio validation fails
    /// - Required metrics cannot be priced
    /// - The LP backend fails or returns an invalid solution
    fn optimize(
        &self,
        problem: &PortfolioOptimizationProblem,
        market: &MarketContext,
        config: &FinstackConfig,
    ) -> Result<PortfolioOptimizationResult>;
}

/// LP‑based optimizer using the `good_lp` crate as backend.
pub struct DefaultLpOptimizer {
    /// Solver tolerance for optimality.
    pub tolerance: f64,
    /// Maximum iterations (backend‑specific meaning).
    pub max_iterations: usize,
}

impl Default for DefaultLpOptimizer {
    fn default() -> Self {
        Self {
            tolerance: 1.0e-8,
            max_iterations: 10_000,
        }
    }
}

/// Relation for LP constraints.
#[derive(Clone, Copy, Debug)]
enum LpRelation {
    /// `lhs <= rhs`.
    Le,
    /// `lhs >= rhs`.
    Ge,
    /// `lhs == rhs`.
    Eq,
}

/// Linear constraint: `coefficients · w (<=,>=,=) rhs`.
#[derive(Clone, Debug)]
struct LpConstraint {
    coefficients: Vec<f64>,
    relation: LpRelation,
    rhs: f64,
    /// Optional name (constraint label) for diagnostics.
    name: Option<String>,
}

#[derive(Clone, Copy, Debug)]
struct WeightVarSpec {
    var: good_lp::Variable,
    offset: f64,
    min_weight: f64,
    max_weight: f64,
}

impl DefaultLpOptimizer {
    fn validate_supported_problem(_problem: &PortfolioOptimizationProblem) -> Result<()> {
        Ok(())
    }

    fn reconstruction_denominator(
        weighting: WeightingScheme,
        denominators: OptimizationDenominators,
    ) -> f64 {
        match weighting {
            WeightingScheme::ValueWeight => denominators.gross_pv_base,
            WeightingScheme::NotionalWeight => denominators.gross_notional,
            WeightingScheme::UnitScaling => 1.0,
        }
    }

    /// Collect all `MetricId`s required by the problem's `PerPositionMetric`s.
    fn required_metrics(problem: &PortfolioOptimizationProblem) -> Vec<MetricId> {
        let mut metrics = Vec::new();

        let mut add_metric = |ppm: &PerPositionMetric| {
            if let PerPositionMetric::Metric(id) = ppm {
                if !metrics.contains(id) {
                    metrics.push(id.clone());
                }
            }
        };

        // Scan objective
        match &problem.objective {
            super::types::Objective::Maximize(expr) | super::types::Objective::Minimize(expr) => {
                match expr {
                    MetricExpr::WeightedSum { metric }
                    | MetricExpr::ValueWeightedAverage { metric } => {
                        add_metric(metric);
                    }
                    MetricExpr::TagExposureShare { .. } => {}
                }
            }
        }

        // Scan constraints
        for constraint in &problem.constraints {
            match constraint {
                Constraint::MetricBound { metric, .. } => match metric {
                    MetricExpr::WeightedSum { metric }
                    | MetricExpr::ValueWeightedAverage { metric } => {
                        add_metric(metric);
                    }
                    MetricExpr::TagExposureShare { .. } => {}
                },
                Constraint::TagExposureLimit { .. } => {}
                Constraint::TagExposureMinimum { .. } => {}
                Constraint::WeightBounds { .. } => {}
                Constraint::MaxTurnover { .. } => {}
                Constraint::MaxPositionDelta { .. } => {}
                Constraint::Budget { .. } => {}
            }
        }

        metrics
    }

    /// Check if a position matches a filter.
    fn matches_filter(position: &crate::position::Position, filter: &PositionFilter) -> bool {
        match filter {
            PositionFilter::All => true,
            PositionFilter::ByEntityId(id) => position.entity_id == *id,
            PositionFilter::ByTag { key, value } => position.tags.get(key) == Some(value),
            PositionFilter::ByPositionIds(ids) => ids.contains(&position.position_id),
            PositionFilter::Not(inner) => !Self::matches_filter(position, inner),
        }
    }

    fn matches_candidate_filter(
        candidate: &super::universe::CandidatePosition,
        filter: &PositionFilter,
    ) -> bool {
        match filter {
            PositionFilter::All => true,
            PositionFilter::ByEntityId(id) => candidate.entity_id == *id,
            PositionFilter::ByTag { key, value } => candidate.tags.get(key) == Some(value),
            PositionFilter::ByPositionIds(ids) => ids.contains(&candidate.id),
            PositionFilter::Not(inner) => !Self::matches_candidate_filter(candidate, inner),
        }
    }

    /// Lower a `PerPositionMetric` to a per‑decision value `m_i`.
    fn per_position_metric_value(
        ppm: &PerPositionMetric,
        feat: &DecisionFeatures,
        missing_policy: MissingMetricPolicy,
    ) -> Result<f64> {
        let val = match ppm {
            PerPositionMetric::Metric(id) => feat.measures.get(id.as_str()).copied(),
            PerPositionMetric::CustomKey(key) => feat.measures.get(key).copied(),
            PerPositionMetric::PvBase => Some(feat.pv_base),
            PerPositionMetric::PvNative => {
                // We do not retain native PV separately in `DecisionFeatures`; for now
                // treat PvNative as PvBase which is already in base currency.
                Some(feat.pv_base)
            }
            PerPositionMetric::TagEquals { key, value } => {
                let matches = feat.tags.get(key) == Some(value);
                Some(if matches { 1.0 } else { 0.0 })
            }
            PerPositionMetric::Constant(c) => Some(*c),
        };

        match (val, missing_policy) {
            (Some(v), _) => Ok(v),
            (None, MissingMetricPolicy::Zero) => Ok(0.0),
            (None, MissingMetricPolicy::Exclude) => Ok(0.0),
            (None, MissingMetricPolicy::Strict) => {
                Err(Error::invalid_input("required metric missing for position"))
            }
        }
    }

    /// Build coefficient vector `a` for a `MetricExpr`.
    fn build_metric_coefficients(
        expr: &MetricExpr,
        feats: &[DecisionFeatures],
        missing_policy: MissingMetricPolicy,
        trade_universe: &TradeUniverse,
        items: &[DecisionItem],
        portfolio: &Portfolio,
    ) -> Result<Vec<f64>> {
        let mut coeffs = Vec::with_capacity(feats.len());
        match expr {
            MetricExpr::WeightedSum { metric } | MetricExpr::ValueWeightedAverage { metric } => {
                for feat in feats {
                    let m_i = Self::per_position_metric_value(metric, feat, missing_policy)?;
                    coeffs.push(m_i);
                }
            }
            MetricExpr::TagExposureShare { tag_key, tag_value } => {
                for (item, feat) in items.iter().zip(feats) {
                    let mut matches = feat.tags.get(tag_key) == Some(tag_value);
                    // Also consider portfolio‑level tags if any
                    if !matches {
                        if let Some(position) = portfolio.get_position(item.position_id.as_str()) {
                            matches = position.tags.get(tag_key) == Some(tag_value);
                        }
                    }
                    // Candidates already have tags in `feat.tags`
                    let weight = if matches { 1.0 } else { 0.0 };
                    coeffs.push(weight);
                }
            }
        }

        // For `MissingMetricPolicy::Exclude`, zero out coefficients for limits (not implemented explicitly yet)
        let _ = trade_universe;

        Ok(coeffs)
    }
}

impl PortfolioOptimizer for DefaultLpOptimizer {
    fn optimize(
        &self,
        problem: &PortfolioOptimizationProblem,
        market: &MarketContext,
        config: &FinstackConfig,
    ) -> Result<PortfolioOptimizationResult> {
        // Step 0: Validate portfolio and problem basics.
        problem.portfolio.validate()?;
        Self::validate_supported_problem(problem)?;

        match problem.weighting {
            WeightingScheme::ValueWeight
            | WeightingScheme::UnitScaling
            | WeightingScheme::NotionalWeight => {}
        }

        // Ensure there is at least one budget constraint, or we will add one with rhs=1.0.
        let has_budget = problem
            .constraints
            .iter()
            .any(|c| matches!(c, Constraint::Budget { .. }));

        // Step 1: Discover required metrics and value portfolio.
        let required_metrics = Self::required_metrics(problem);
        let options = crate::valuation::PortfolioValuationOptions {
            strict_risk: matches!(problem.missing_metric_policy, MissingMetricPolicy::Strict),
            additional_metrics: if required_metrics.is_empty() {
                None
            } else {
                Some(required_metrics.clone())
            },
            replace_standard_metrics: false,
        };

        let valuation = crate::valuation::value_portfolio_with_options(
            &problem.portfolio,
            market,
            config,
            &options,
        )?;

        // Step 2: Build decision space.
        let (decision_items, mut decision_features, current_weights, denominators) =
            build_decision_space(problem, &valuation, &required_metrics, market, config)?;

        if decision_items.is_empty() {
            return Err(Error::invalid_input(
                "no decision variables in optimization problem",
            ));
        }

        let n_vars = decision_items.len();

        // Step 3: Apply weight bounds from WeightBounds / MaxPositionDelta constraints.
        for constraint in &problem.constraints {
            match constraint {
                Constraint::WeightBounds {
                    filter, min, max, ..
                } => {
                    for (item, feat) in decision_items.iter().zip(decision_features.iter_mut()) {
                        // Reuse matches_filter logic.
                        let is_match = if item.is_existing {
                            if let Some(position) =
                                problem.portfolio.get_position(item.position_id.as_str())
                            {
                                Self::matches_filter(position, filter)
                            } else {
                                false
                            }
                        } else {
                            problem
                                .trade_universe
                                .candidates
                                .iter()
                                .find(|candidate| candidate.id == item.position_id)
                                .is_some_and(|candidate| {
                                    Self::matches_candidate_filter(candidate, filter)
                                })
                        };

                        if is_match {
                            feat.min_weight = feat.min_weight.max(*min);
                            feat.max_weight = feat.max_weight.min(*max);
                        }
                    }
                }
                Constraint::MaxPositionDelta {
                    filter, max_delta, ..
                } => {
                    for (item, feat) in decision_items.iter().zip(decision_features.iter_mut()) {
                        if !item.is_existing {
                            continue;
                        }
                        let Some(position) =
                            problem.portfolio.get_position(item.position_id.as_str())
                        else {
                            continue;
                        };
                        if !Self::matches_filter(position, filter) {
                            continue;
                        }
                        let current_weight = current_weights
                            .get(&item.position_id)
                            .copied()
                            .unwrap_or(0.0);
                        feat.min_weight = feat.min_weight.max(current_weight - *max_delta);
                        feat.max_weight = feat.max_weight.min(current_weight + *max_delta);
                    }
                }
                _ => {}
            }
        }

        // Step 4: Build objective coefficients.
        let objective_expr = &problem.objective;
        let coeffs_objective = match objective_expr {
            super::types::Objective::Maximize(expr) | super::types::Objective::Minimize(expr) => {
                Self::build_metric_coefficients(
                    expr,
                    &decision_features,
                    problem.missing_metric_policy,
                    &problem.trade_universe,
                    &decision_items,
                    &problem.portfolio,
                )?
            }
        };

        // Step 5: Build constraints as LP rows.
        let mut lp_constraints: Vec<LpConstraint> = Vec::new();

        for constraint in &problem.constraints {
            match constraint {
                Constraint::MetricBound {
                    label,
                    metric,
                    op,
                    rhs,
                } => {
                    let a = Self::build_metric_coefficients(
                        metric,
                        &decision_features,
                        problem.missing_metric_policy,
                        &problem.trade_universe,
                        &decision_items,
                        &problem.portfolio,
                    )?;
                    let relation = match op {
                        Inequality::Le => LpRelation::Le,
                        Inequality::Ge => LpRelation::Ge,
                        Inequality::Eq => LpRelation::Eq,
                    };
                    lp_constraints.push(LpConstraint {
                        coefficients: a,
                        relation,
                        rhs: *rhs,
                        name: label.clone(),
                    });
                }
                Constraint::TagExposureLimit {
                    label,
                    tag_key,
                    tag_value,
                    max_share,
                } => {
                    let metric = MetricExpr::TagExposureShare {
                        tag_key: tag_key.clone(),
                        tag_value: tag_value.clone(),
                    };
                    let a = Self::build_metric_coefficients(
                        &metric,
                        &decision_features,
                        problem.missing_metric_policy,
                        &problem.trade_universe,
                        &decision_items,
                        &problem.portfolio,
                    )?;
                    lp_constraints.push(LpConstraint {
                        coefficients: a,
                        relation: LpRelation::Le,
                        rhs: *max_share,
                        name: label.clone(),
                    });
                }
                Constraint::TagExposureMinimum {
                    label,
                    tag_key,
                    tag_value,
                    min_share,
                } => {
                    let metric = MetricExpr::TagExposureShare {
                        tag_key: tag_key.clone(),
                        tag_value: tag_value.clone(),
                    };
                    let a = Self::build_metric_coefficients(
                        &metric,
                        &decision_features,
                        problem.missing_metric_policy,
                        &problem.trade_universe,
                        &decision_items,
                        &problem.portfolio,
                    )?;
                    lp_constraints.push(LpConstraint {
                        coefficients: a,
                        relation: LpRelation::Ge,
                        rhs: *min_share,
                        name: label.clone(),
                    });
                }
                Constraint::WeightBounds { .. } => {
                    // Already applied to `DecisionFeatures::min_weight/max_weight`.
                }
                Constraint::MaxTurnover {
                    label: _,
                    max_turnover,
                } => {
                    // Turnover handled later via auxiliary variables.
                    // We record a placeholder constraint row that will be skipped in
                    // the primary constraint loop. The actual turnover constraint is
                    // built with auxiliary variables |w_i - w0_i| below.
                    lp_constraints.push(LpConstraint {
                        coefficients: vec![0.0; n_vars], // placeholder; not used
                        relation: LpRelation::Le,
                        rhs: *max_turnover,
                        // Use internal marker name to identify this placeholder
                        name: Some("__turnover_placeholder__".to_string()),
                    });
                }
                Constraint::MaxPositionDelta { .. } => {
                    // Implemented later by additional bounds around current weights.
                }
                Constraint::Budget { rhs } => {
                    // Budget: sum_i w_i == rhs
                    let coefficients = vec![1.0; n_vars];
                    lp_constraints.push(LpConstraint {
                        coefficients,
                        relation: LpRelation::Eq,
                        rhs: *rhs,
                        name: Some("budget".to_string()),
                    });
                }
            }
        }

        if !has_budget {
            // Add implicit budget constraint sum_i w_i == 1.0 if none was provided.
            lp_constraints.push(LpConstraint {
                coefficients: vec![1.0; n_vars],
                relation: LpRelation::Eq,
                rhs: 1.0,
                name: Some("budget".to_string()),
            });
        }

        // Step 6: Assemble LP model using good_lp.
        let maximise = matches!(problem.objective, super::types::Objective::Maximize(_));
        let mut vars = good_lp::variables!();

        // Decision variables w_i
        let mut w_vars = Vec::with_capacity(n_vars);
        for (item, feat) in decision_items.iter().zip(&decision_features) {
            let current_weight = current_weights
                .get(&item.position_id)
                .copied()
                .unwrap_or(0.0);
            // Held positions: lock weight at current value by min = max = current_weight.
            let (min_w, max_w) = if item.is_held {
                (current_weight, current_weight)
            } else {
                (feat.min_weight, feat.max_weight)
            };
            if max_w < min_w {
                return Err(Error::invalid_input(format!(
                    "inconsistent weight bounds for '{}': min {} > max {}",
                    item.position_id, min_w, max_w
                )));
            }

            let (var_min, var_max, offset) = if min_w < 0.0 {
                (0.0, max_w - min_w, min_w)
            } else {
                (min_w, max_w, 0.0)
            };

            w_vars.push(WeightVarSpec {
                var: vars.add(variable().min(var_min).max(var_max)),
                offset,
                min_weight: min_w,
                max_weight: max_w,
            });
        }

        // Auxiliary variables for turnover t_i (|w_i - w0_i|) if needed.
        let has_turnover_constraint = problem
            .constraints
            .iter()
            .any(|c| matches!(c, Constraint::MaxTurnover { .. }));

        let mut t_vars: Vec<Option<good_lp::Variable>> = vec![None; n_vars];

        if has_turnover_constraint {
            for t_var in t_vars.iter_mut().take(n_vars) {
                *t_var = Some(vars.add(variable().min(0.0)));
            }
        }

        // Objective
        let mut objective_expr: Expression = 0.0.into();
        for (var, coef) in w_vars.iter().zip(&coeffs_objective) {
            objective_expr += (*coef) * var.var;
            if var.offset != 0.0 {
                objective_expr += (*coef) * var.offset;
            }
        }

        let mut problem_model = if maximise {
            vars.maximise(objective_expr)
        } else {
            vars.minimise(objective_expr)
        }
        .using(default_solver);

        // Add explicit effective-weight bounds so signed candidate limits are
        // enforced even when the backend internally re-parameterizes variables.
        for w_var in &w_vars {
            let mut effective_weight: Expression = w_var.var.into();
            if w_var.offset != 0.0 {
                effective_weight += w_var.offset;
            }
            problem_model =
                problem_model.with(constraint!(effective_weight.clone() >= w_var.min_weight));
            problem_model = problem_model.with(constraint!(effective_weight <= w_var.max_weight));
        }

        // Add primary constraints
        for lc in &lp_constraints {
            // Skip placeholder turnover row; actual turnover constraint is
            // built below with auxiliary variables for |w_i - w0_i|.
            if matches!(lc.name.as_deref(), Some("__turnover_placeholder__")) {
                continue;
            }

            let mut lhs: Expression = 0.0.into();
            for (var, coef) in w_vars.iter().zip(&lc.coefficients) {
                lhs += (*coef) * var.var;
                if var.offset != 0.0 {
                    lhs += (*coef) * var.offset;
                }
            }

            problem_model = match lc.relation {
                LpRelation::Le => problem_model.with(constraint!(lhs <= lc.rhs)),
                LpRelation::Ge => problem_model.with(constraint!(lhs >= lc.rhs)),
                LpRelation::Eq => problem_model.with(constraint!(lhs == lc.rhs)),
            };
        }

        // Turnover constraint with auxiliary variables: Σ t_i <= max_turnover.
        if let Some(Constraint::MaxTurnover { max_turnover, .. }) = problem
            .constraints
            .iter()
            .find(|c| matches!(c, Constraint::MaxTurnover { .. }))
        {
            // For each i: t_i >= w_i - w0_i and t_i >= w0_i - w_i
            for (idx, w_var) in w_vars.iter().enumerate() {
                let t_var = match t_vars[idx] {
                    Some(v) => v,
                    None => continue,
                };
                let w0 = current_weights
                    .get(&decision_items[idx].position_id)
                    .copied()
                    .unwrap_or(0.0);

                // t_i >= w_i - w0  ->  t_i - w_i >= -w0
                let lhs1: Expression = t_var - w_var.var;
                problem_model = problem_model.with(constraint!(lhs1 >= w_var.offset - w0));

                // t_i >= w0 - w_i  ->  t_i + w_i >= w0
                let lhs2: Expression = t_var + w_var.var;
                problem_model = problem_model.with(constraint!(lhs2 >= w0 - w_var.offset));
            }

            // Sum t_i <= max_turnover
            let mut lhs_turnover: Expression = 0.0.into();
            for tv in t_vars.iter().flatten() {
                lhs_turnover += *tv;
            }
            problem_model = problem_model.with(constraint!(lhs_turnover <= *max_turnover));
        }

        // Solve LP
        let solution = problem_model
            .solve()
            .map_err(|e| Error::optimization_error(e.to_string()))?;

        // Extract weights
        let mut optimal_weights: IndexMap<PositionId, f64> = IndexMap::new();
        let mut weight_deltas: IndexMap<PositionId, f64> = IndexMap::new();

        for (item, w_var) in decision_items.iter().zip(&w_vars) {
            let w_star = solution.value(w_var.var) + w_var.offset;
            let w0 = current_weights
                .get(&item.position_id)
                .copied()
                .unwrap_or(0.0);
            optimal_weights.insert(item.position_id.clone(), w_star);
            weight_deltas.insert(item.position_id.clone(), w_star - w0);
        }

        // Implied quantities
        let mut implied_quantities: IndexMap<PositionId, f64> = IndexMap::new();
        let reconstruction_denominator =
            Self::reconstruction_denominator(problem.weighting, denominators);
        for (item, feat) in decision_items.iter().zip(&decision_features) {
            let w_star = optimal_weights
                .get(&item.position_id)
                .copied()
                .unwrap_or(0.0);
            let qty = match problem.weighting {
                WeightingScheme::NotionalWeight => w_star * reconstruction_denominator,
                WeightingScheme::ValueWeight => {
                    if feat.pv_per_unit.abs() > 1e-12 {
                        (w_star * reconstruction_denominator) / feat.pv_per_unit
                    } else {
                        0.0
                    }
                }
                WeightingScheme::UnitScaling => {
                    if item.is_existing {
                        item.current_quantity * w_star
                    } else {
                        w_star
                    }
                }
            };

            implied_quantities.insert(item.position_id.clone(), qty);
        }

        // Objective value at solution: a · w*
        let mut objective_value = 0.0_f64;
        for (coef, w_var) in coeffs_objective.iter().zip(&w_vars) {
            let w_star = solution.value(w_var.var) + w_var.offset;
            objective_value = neumaier_sum([objective_value, *coef * w_star].into_iter());
        }

        // Evaluate additional metric expressions of interest (for now: just objective).
        let mut metric_values: IndexMap<String, f64> = IndexMap::new();
        metric_values.insert("objective".to_string(), objective_value);

        // Constraint slacks
        let mut constraint_slacks: IndexMap<String, f64> = IndexMap::new();
        for lc in &lp_constraints {
            if let Some(name) = &lc.name {
                let mut lhs_val = 0.0;
                for (var, coef) in w_vars.iter().zip(&lc.coefficients) {
                    lhs_val += *coef * (solution.value(var.var) + var.offset);
                }

                let slack = match lc.relation {
                    LpRelation::Le => lc.rhs - lhs_val,
                    LpRelation::Ge => lhs_val - lc.rhs,
                    LpRelation::Eq => (lhs_val - lc.rhs).abs(),
                };
                constraint_slacks.insert(name.clone(), slack);
            }
        }

        // Dual values are backend-specific and good_lp typically doesn't expose them
        // in the generic interface. We leave them empty for now.
        let dual_values: IndexMap<String, f64> = IndexMap::new();

        // Optimization status – assume optimal if solve() succeeded.
        let status = OptimizationStatus::Optimal;

        // Reuse results_meta from config.
        let meta = finstack_core::config::results_meta_now(config);

        Ok(PortfolioOptimizationResult {
            problem: problem.clone(),
            current_weights,
            optimal_weights,
            weight_deltas,
            implied_quantities,
            objective_value,
            metric_values,
            status,
            dual_values,
            constraint_slacks,
            meta,
        })
    }
}
