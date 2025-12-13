//! Tests for expression DAG planning and optimization.
//!
//! This module tests:
//! - DAG construction and node deduplication
//! - Structural deduplication (ignoring expression IDs)
//! - Topological ordering of dependencies
//! - Cost estimation and cache strategy
//! - Pushdown boundary analysis

use finstack_core::config::{
    NumericMode, ResultsMeta, RoundingContext, RoundingMode, ToleranceConfig,
};
use finstack_core::expr::dag::{DagBuilder, PushdownAnalyzer};
use finstack_core::expr::{Expr, ExprNode, Function};

#[test]
fn test_expr_structural_eq_hash_ignore_id() {
    use std::collections::HashSet;

    // Same structure, different ids
    let e1 = Expr::call(
        Function::RollingMean,
        vec![Expr::column("x"), Expr::literal(3.0)],
    )
    .with_id(1);
    let e2 = Expr::call(
        Function::RollingMean,
        vec![Expr::column("x"), Expr::literal(3.0)],
    )
    .with_id(999);

    assert_eq!(e1, e2, "Expr equality must ignore id");

    let mut set = HashSet::new();
    set.insert(e1);
    // Should be considered duplicate due to structural identity
    assert!(set.contains(&e2));
    let inserted = set.insert(e2);
    assert!(
        !inserted,
        "Hash must ignore id so structural duplicates do not insert twice"
    );

    // time_window removed from identity; structural identity depends only on node
    let base_a = Expr::call(
        Function::RollingMean,
        vec![Expr::column("x"), Expr::literal(3.0)],
    );
    let base_b = Expr::call(
        Function::RollingMean,
        vec![Expr::column("x"), Expr::literal(3.0)],
    );
    assert_eq!(
        base_a, base_b,
        "Structural identity must match for identical nodes"
    );
}

#[test]
fn test_dag_dedup_ignores_expr_id() {
    let mut builder = DagBuilder::new();

    // Same structure, different ids should dedup into one subnode
    let col_x = Expr::column("x");
    let lit_3 = Expr::literal(3.0);
    let rm_a = Expr::call(Function::RollingMean, vec![col_x.clone(), lit_3.clone()]).with_id(42);
    let rm_b = Expr::call(Function::RollingMean, vec![col_x.clone(), lit_3.clone()]).with_id(77);

    let meta = ResultsMeta {
        numeric_mode: NumericMode::F64,
        rounding: RoundingContext {
            mode: RoundingMode::Bankers,
            ingest_scale_by_ccy: Default::default(),
            output_scale_by_ccy: Default::default(),
            tolerances: ToleranceConfig::default(),
            version: 1,
        },
        fx_policy_applied: None,
        timestamp: None,
        version: None,
    };
    let plan = builder.build_plan(vec![rm_a, rm_b], meta);

    // Expect nodes: Column(x), Literal(3), RollingMean — deduped shared RollingMean root appears once
    // but roots vector will have two entries pointing to the same node id.
    assert_eq!(
        plan.nodes.len(),
        3,
        "Structural dedup should eliminate duplicate RollingMean"
    );

    // Validate roots both map to the same node id
    assert_eq!(plan.roots.len(), 2);
    assert_eq!(
        plan.roots[0], plan.roots[1],
        "Duplicate roots should dedup to same node id"
    );
}

#[test]
fn test_dag_builder_simple_expressions() {
    let mut builder = DagBuilder::new();

    // Create simple expressions: Column("x"), Literal(3.0), RollingMean(Column("x"), 3)
    let col_x = Expr::column("x");
    let lit_3 = Expr::literal(3.0);
    let rolling_mean = Expr::call(Function::RollingMean, vec![col_x.clone(), lit_3.clone()]);

    let meta = ResultsMeta {
        numeric_mode: NumericMode::F64,
        rounding: RoundingContext {
            mode: RoundingMode::Bankers,
            ingest_scale_by_ccy: Default::default(),
            output_scale_by_ccy: Default::default(),
            tolerances: ToleranceConfig::default(),
            version: 1,
        },
        fx_policy_applied: None,
        timestamp: None,
        version: None,
    };
    let plan = builder.build_plan(vec![rolling_mean], meta);

    // Should have nodes for: Column("x"), Literal(3.0), RollingMean
    assert_eq!(plan.nodes.len(), 3);
    assert_eq!(plan.roots.len(), 1);

    // Verify topological order: dependencies should come before dependents
    let root_node = plan.nodes.last().unwrap();
    assert_eq!(root_node.dependencies.len(), 2); // RollingMean depends on column and literal
}

#[test]
fn test_dag_builder_shared_subexpressions() {
    let mut builder = DagBuilder::new();

    // Create expressions that share Column("x"): RollingMean(x, 3) and RollingSum(x, 3)
    let col_x = Expr::column("x");
    let lit_3 = Expr::literal(3.0);
    let rolling_mean = Expr::call(Function::RollingMean, vec![col_x.clone(), lit_3.clone()]);
    let rolling_sum = Expr::call(Function::RollingSum, vec![col_x.clone(), lit_3.clone()]);

    let meta = ResultsMeta {
        numeric_mode: NumericMode::F64,
        rounding: RoundingContext {
            mode: RoundingMode::Bankers,
            ingest_scale_by_ccy: Default::default(),
            output_scale_by_ccy: Default::default(),
            tolerances: ToleranceConfig::default(),
            version: 1,
        },
        fx_policy_applied: None,
        timestamp: None,
        version: None,
    };
    let plan = builder.build_plan(vec![rolling_mean, rolling_sum], meta);

    // Should deduplicate shared subexpressions
    // Nodes: Column("x"), Literal(3.0), RollingMean, RollingSum
    assert_eq!(plan.nodes.len(), 4);
    assert_eq!(plan.roots.len(), 2);

    // Find the column node and check its reference count
    let col_node = plan
        .nodes
        .iter()
        .find(|n| matches!(n.expr.node, ExprNode::Column(_)))
        .unwrap();
    assert!(
        col_node.ref_count > 1,
        "Column should be referenced multiple times"
    );

    // Cache strategy should exist (may or may not recommend caching this specific node)
    assert!(plan.cache_strategy.expected_hit_rate >= 0.0);
}

#[test]
fn test_dag_multiple_function_types() {
    let mut builder = DagBuilder::new();

    // Create expressions with different function types
    let col_x = Expr::column("x");
    let lit_2 = Expr::literal(2.0);

    // Test various function types in the DAG
    let rolling_mean = Expr::call(Function::RollingMean, vec![col_x.clone(), lit_2.clone()]);
    let cum_sum = Expr::call(Function::CumSum, vec![col_x.clone()]);

    let meta = ResultsMeta {
        numeric_mode: NumericMode::F64,
        rounding: RoundingContext {
            mode: RoundingMode::Bankers,
            ingest_scale_by_ccy: Default::default(),
            output_scale_by_ccy: Default::default(),
            tolerances: ToleranceConfig::default(),
            version: 1,
        },
        fx_policy_applied: None,
        timestamp: None,
        version: None,
    };
    let plan = builder.build_plan(vec![rolling_mean, cum_sum], meta);

    // Verify that the expected nodes exist in the plan
    plan.nodes
        .iter()
        .find(|n| matches!(n.expr.node, ExprNode::Call(Function::RollingMean, _)))
        .unwrap();
    plan.nodes
        .iter()
        .find(|n| matches!(n.expr.node, ExprNode::Call(Function::CumSum, _)))
        .unwrap();
}

#[test]
fn test_dag_cost_estimation() {
    let mut builder = DagBuilder::new();

    // Create expressions with different costs
    let col_x = Expr::column("x");
    let lit_5 = Expr::literal(5.0);

    // Different functions have different estimated costs
    let lag = Expr::call(Function::Lag, vec![col_x.clone(), lit_5.clone()]);
    let rolling_std = Expr::call(Function::RollingStd, vec![col_x.clone(), lit_5.clone()]);

    let meta = ResultsMeta {
        numeric_mode: NumericMode::F64,
        rounding: RoundingContext {
            mode: RoundingMode::Bankers,
            ingest_scale_by_ccy: Default::default(),
            output_scale_by_ccy: Default::default(),
            tolerances: ToleranceConfig::default(),
            version: 1,
        },
        fx_policy_applied: None,
        timestamp: None,
        version: None,
    };
    let plan = builder.build_plan(vec![lag, rolling_std], meta);

    // Find nodes and check costs
    let lag_node = plan
        .nodes
        .iter()
        .find(|n| matches!(n.expr.node, ExprNode::Call(Function::Lag, _)))
        .unwrap();
    let rolling_std_node = plan
        .nodes
        .iter()
        .find(|n| matches!(n.expr.node, ExprNode::Call(Function::RollingStd, _)))
        .unwrap();

    // RollingStd should have higher cost than Lag
    assert!(rolling_std_node.cost > lag_node.cost);
}

#[test]
fn test_pushdown_boundary_analysis() {
    let mut builder = DagBuilder::new();

    // Create a mixed scenario: Polars-eligible function depending on scalar-only function
    let col_x = Expr::column("x");
    let cum_sum = Expr::call(Function::CumSum, vec![col_x.clone()]); // Scalar-only
    let rolling_mean = Expr::call(Function::RollingMean, vec![cum_sum, Expr::literal(3.0)]); // Polars-eligible

    let meta = ResultsMeta {
        numeric_mode: NumericMode::F64,
        rounding: RoundingContext {
            mode: RoundingMode::Bankers,
            ingest_scale_by_ccy: Default::default(),
            output_scale_by_ccy: Default::default(),
            tolerances: ToleranceConfig::default(),
            version: 1,
        },
        fx_policy_applied: None,
        timestamp: None,
        version: None,
    };
    let plan = builder.build_plan(vec![rolling_mean], meta);

    // Analyze pushdown boundaries
    let boundaries = PushdownAnalyzer::analyze_boundaries(&plan);

    // Boundaries analysis should complete successfully
    assert!(
        boundaries.estimated_speedup >= 0.0,
        "Speedup should be non-negative"
    );

    // Should have some analysis result (may or may not find boundaries)
    // Just verify the analysis completes without error
}

#[test]
fn test_dag_cache_strategy() {
    let mut builder = DagBuilder::new();

    // Create expression with high-cost shared subexpression
    let col_x = Expr::column("x");
    let lit_10 = Expr::literal(10.0);

    // Create expensive operation that's used multiple times
    let rolling_std = Expr::call(Function::RollingStd, vec![col_x.clone(), lit_10.clone()]);
    let expr1 = Expr::call(
        Function::RollingMean,
        vec![rolling_std.clone(), Expr::literal(5.0)],
    );
    let expr2 = Expr::call(
        Function::RollingSum,
        vec![rolling_std.clone(), Expr::literal(3.0)],
    );

    let meta = ResultsMeta {
        numeric_mode: NumericMode::F64,
        rounding: RoundingContext {
            mode: RoundingMode::Bankers,
            ingest_scale_by_ccy: Default::default(),
            output_scale_by_ccy: Default::default(),
            tolerances: ToleranceConfig::default(),
            version: 1,
        },
        fx_policy_applied: None,
        timestamp: None,
        version: None,
    };
    let plan = builder.build_plan(vec![expr1, expr2], meta);

    // Cache strategy should recommend caching the expensive shared operation
    let rolling_std_node = plan
        .nodes
        .iter()
        .find(|n| matches!(n.expr.node, ExprNode::Call(Function::RollingStd, _)))
        .unwrap();

    assert!(
        rolling_std_node.ref_count > 1,
        "RollingStd should be shared"
    );

    // Expected hit rate should be positive
    assert!(plan.cache_strategy.expected_hit_rate > 0.0);
}

#[test]
fn test_dag_topological_ordering() {
    let mut builder = DagBuilder::new();

    // Create a chain of dependencies: x -> lag(x, 1) -> diff(lag(x, 1), 1)
    let col_x = Expr::column("x");
    let lag_x = Expr::call(Function::Lag, vec![col_x.clone(), Expr::literal(1.0)]);
    let diff_lag = Expr::call(Function::Diff, vec![lag_x.clone(), Expr::literal(1.0)]);

    let meta = ResultsMeta {
        numeric_mode: NumericMode::F64,
        rounding: RoundingContext {
            mode: RoundingMode::Bankers,
            ingest_scale_by_ccy: Default::default(),
            output_scale_by_ccy: Default::default(),
            tolerances: ToleranceConfig::default(),
            version: 1,
        },
        fx_policy_applied: None,
        timestamp: None,
        version: None,
    };
    let plan = builder.build_plan(vec![diff_lag], meta);

    // Verify topological order: Column should come first, then Lag, then Diff
    let mut found_column = false;
    let mut found_lag = false;

    for node in &plan.nodes {
        match &node.expr.node {
            ExprNode::Column(_) => {
                assert!(!found_lag && !found_column, "Column should come first");
                found_column = true;
            }
            ExprNode::Call(Function::Lag, _) => {
                assert!(found_column && !found_lag, "Lag should come after Column");
                found_lag = true;
            }
            ExprNode::Call(Function::Diff, _) => {
                assert!(found_column && found_lag, "Diff should come last");
            }
            _ => {}
        }
    }
}

#[test]
fn test_dag_empty_plan() {
    let mut builder = DagBuilder::new();
    let meta = ResultsMeta {
        numeric_mode: NumericMode::F64,
        rounding: RoundingContext {
            mode: RoundingMode::Bankers,
            ingest_scale_by_ccy: Default::default(),
            output_scale_by_ccy: Default::default(),
            tolerances: ToleranceConfig::default(),
            version: 1,
        },
        fx_policy_applied: None,
        timestamp: None,
        version: None,
    };
    let plan = builder.build_plan(vec![], meta);

    assert!(plan.nodes.is_empty());
    assert!(plan.roots.is_empty());
    assert_eq!(plan.cache_strategy.expected_hit_rate, 0.0);
}
