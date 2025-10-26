//! Tests for expression engine serialization/deserialization.

use finstack_core::config::{NumericMode, ResultsMeta, RoundingContext, RoundingMode};
use finstack_core::expr::dag::{BoundaryType, CacheStrategy, DagNode, ExecutionPlan};
use finstack_core::expr::{
    CompiledExpr, EvalOpts, EvaluationResult, Expr, ExprNode, Function, SimpleContext,
};
use hashbrown::HashMap;
use std::collections::HashSet;

#[test]
fn test_expr_ast_serde_roundtrip() {
    // Test basic expression AST serialization
    let expr = Expr::call(
        Function::RollingMean,
        vec![Expr::column("x"), Expr::literal(3.0)],
    )
    .with_id(42);

    let json = serde_json::to_string(&expr).expect("Failed to serialize Expr");
    let deserialized: Expr = serde_json::from_str(&json).expect("Failed to deserialize Expr");

    assert_eq!(expr.id, deserialized.id);
    match (&expr.node, &deserialized.node) {
        (ExprNode::Call(f1, args1), ExprNode::Call(f2, args2)) => {
            assert_eq!(f1, f2);
            assert_eq!(args1.len(), args2.len());
        }
        _ => panic!("Node type mismatch"),
    }
}

#[test]
fn test_expr_node_types_serde() {
    // Test Column node
    let col_node = ExprNode::Column("price".to_string());
    let json = serde_json::to_string(&col_node).expect("Failed to serialize Column");
    let deserialized: ExprNode = serde_json::from_str(&json).expect("Failed to deserialize Column");
    match deserialized {
        ExprNode::Column(name) => assert_eq!(name, "price"),
        _ => panic!("Expected Column node"),
    }

    // Test Literal node
    let lit_node = ExprNode::Literal(42.5);
    let json = serde_json::to_string(&lit_node).expect("Failed to serialize Literal");
    let deserialized: ExprNode =
        serde_json::from_str(&json).expect("Failed to deserialize Literal");
    match deserialized {
        ExprNode::Literal(val) => assert_eq!(val, 42.5),
        _ => panic!("Expected Literal node"),
    }

    // Test Call node
    let call_node = ExprNode::Call(
        Function::Lag,
        vec![Expr::column("value"), Expr::literal(1.0)],
    );
    let json = serde_json::to_string(&call_node).expect("Failed to serialize Call");
    let deserialized: ExprNode = serde_json::from_str(&json).expect("Failed to deserialize Call");
    match deserialized {
        ExprNode::Call(func, args) => {
            assert_eq!(func, Function::Lag);
            assert_eq!(args.len(), 2);
        }
        _ => panic!("Expected Call node"),
    }
}

#[test]
fn test_function_enum_serde() {
    // Test all function variants
    let functions = vec![
        Function::Lag,
        Function::Lead,
        Function::Diff,
        Function::PctChange,
        Function::CumSum,
        Function::CumProd,
        Function::CumMin,
        Function::CumMax,
        Function::RollingMean,
        Function::RollingSum,
        Function::EwmMean,
        Function::Std,
        Function::Var,
        Function::Median,
        Function::RollingStd,
        Function::RollingVar,
        Function::RollingMedian,
        Function::Shift,
        Function::Rank,
        Function::Quantile,
        Function::RollingMin,
        Function::RollingMax,
        Function::RollingCount,
        Function::EwmStd,
        Function::EwmVar,
    ];

    for func in functions {
        let json = serde_json::to_string(&func)
            .unwrap_or_else(|_| panic!("Failed to serialize {:?}", func));
        let deserialized: Function = serde_json::from_str(&json)
            .unwrap_or_else(|_| panic!("Failed to deserialize {:?}", func));
        assert_eq!(func, deserialized);
    }
}

#[test]
fn test_evaluation_result_serde() {
    let result = EvaluationResult {
        values: vec![1.0, 2.0, 3.0, 4.0, 5.0],
        metadata: ResultsMeta {
            numeric_mode: NumericMode::F64,
            rounding: RoundingContext {
                mode: RoundingMode::Bankers,
                ingest_scale_by_ccy: HashMap::new(),
                output_scale_by_ccy: HashMap::new(),
                version: 1,
            },
            fx_policy_applied: None,
            timestamp: None,
            version: None,
        },
    };

    let json = serde_json::to_string(&result).expect("Failed to serialize EvaluationResult");
    let deserialized: EvaluationResult =
        serde_json::from_str(&json).expect("Failed to deserialize EvaluationResult");

    assert_eq!(result.values, deserialized.values);
    assert_eq!(
        result.metadata.numeric_mode,
        deserialized.metadata.numeric_mode
    );
    assert_eq!(
        result.metadata.rounding.version,
        deserialized.metadata.rounding.version
    );
}

#[test]
fn test_dag_node_serde() {
    let node = DagNode {
        id: 1,
        expr: Expr::column("x"),
        dependencies: vec![2, 3],
        ref_count: 2,
        cost: 10,
    };

    let json = serde_json::to_string(&node).expect("Failed to serialize DagNode");
    let deserialized: DagNode = serde_json::from_str(&json).expect("Failed to deserialize DagNode");

    assert_eq!(node.id, deserialized.id);
    assert_eq!(node.dependencies, deserialized.dependencies);
    assert_eq!(node.ref_count, deserialized.ref_count);
    assert_eq!(node.cost, deserialized.cost);
}

#[test]
fn test_execution_plan_serde() {
    let nodes = vec![
        DagNode {
            id: 1,
            expr: Expr::column("x"),
            dependencies: vec![],
            ref_count: 1,
            cost: 1,
        },
        DagNode {
            id: 2,
            expr: Expr::literal(5.0),
            dependencies: vec![],
            ref_count: 1,
            cost: 1,
        },
    ];

    let mut cache_nodes = HashSet::new();
    cache_nodes.insert(1);

    let plan = ExecutionPlan {
        nodes: nodes.clone(),
        roots: vec![1, 2],
        meta: ResultsMeta {
            numeric_mode: NumericMode::F64,
            rounding: RoundingContext {
                mode: RoundingMode::Bankers,
                ingest_scale_by_ccy: HashMap::new(),
                output_scale_by_ccy: HashMap::new(),
                version: 1,
            },
            fx_policy_applied: None,
            timestamp: None,
            version: None,
        },
        cache_strategy: CacheStrategy {
            cache_nodes,
            expected_hit_rate: 0.75,
            memory_budget: 1000,
        },
    };

    let json = serde_json::to_string(&plan).expect("Failed to serialize ExecutionPlan");
    let deserialized: ExecutionPlan =
        serde_json::from_str(&json).expect("Failed to deserialize ExecutionPlan");

    assert_eq!(plan.nodes.len(), deserialized.nodes.len());
    assert_eq!(plan.roots, deserialized.roots);
    assert_eq!(
        plan.cache_strategy.expected_hit_rate,
        deserialized.cache_strategy.expected_hit_rate
    );
    assert_eq!(
        plan.cache_strategy.memory_budget,
        deserialized.cache_strategy.memory_budget
    );
}

#[test]
fn test_eval_opts_serde() {
    let opts = EvalOpts {
        plan: None,
        cache_budget_mb: Some(256),
    };

    let json = serde_json::to_string(&opts).expect("Failed to serialize EvalOpts");
    let deserialized: EvalOpts =
        serde_json::from_str(&json).expect("Failed to deserialize EvalOpts");

    assert_eq!(opts.cache_budget_mb, deserialized.cache_budget_mb);
    assert!(deserialized.plan.is_none());
}

#[test]
fn test_compiled_expr_serde() {
    // Create a compiled expression with a plan
    let meta = ResultsMeta {
        numeric_mode: NumericMode::F64,
        rounding: RoundingContext {
            mode: RoundingMode::Bankers,
            ingest_scale_by_ccy: HashMap::new(),
            output_scale_by_ccy: HashMap::new(),
            version: 1,
        },
        fx_policy_applied: None,
        timestamp: None,
        version: None,
    };

    let expr = Expr::call(
        Function::RollingSum,
        vec![Expr::column("values"), Expr::literal(5.0)],
    );

    let compiled = CompiledExpr::with_planning(expr.clone(), meta);

    let json = serde_json::to_string(&compiled).expect("Failed to serialize CompiledExpr");
    let deserialized: CompiledExpr =
        serde_json::from_str(&json).expect("Failed to deserialize CompiledExpr");

    // Verify AST is preserved
    assert_eq!(compiled.ast.id, deserialized.ast.id);

    // Verify plan is preserved if it existed
    assert_eq!(compiled.plan.is_some(), deserialized.plan.is_some());

    // Cache should be None after deserialization (it's skipped)
    assert!(deserialized.cache.is_none());
}

#[test]
fn test_simple_context_serde() {
    let context = SimpleContext::new(vec!["price", "volume", "timestamp"]);

    let json = serde_json::to_string(&context).expect("Failed to serialize SimpleContext");
    let deserialized: SimpleContext =
        serde_json::from_str(&json).expect("Failed to deserialize SimpleContext");

    // Verify indices are preserved
    assert_eq!(context.index_of("price"), deserialized.index_of("price"));
    assert_eq!(context.index_of("volume"), deserialized.index_of("volume"));
    assert_eq!(
        context.index_of("timestamp"),
        deserialized.index_of("timestamp")
    );
    assert_eq!(
        context.index_of("unknown"),
        deserialized.index_of("unknown")
    );
}

// Note: CachedResult and CacheStats are internal types (pub(crate))
// and cannot be tested from external tests. Their serialization is
// tested indirectly through the public API types that use them.

#[test]
fn test_boundary_type_serde() {
    let boundary1 = BoundaryType::OptimizedToScalar;
    let boundary2 = BoundaryType::ScalarToOptimized;

    let json1 = serde_json::to_string(&boundary1).expect("Failed to serialize BoundaryType");
    let deserialized1: BoundaryType =
        serde_json::from_str(&json1).expect("Failed to deserialize BoundaryType");

    let json2 = serde_json::to_string(&boundary2).expect("Failed to serialize BoundaryType");
    let deserialized2: BoundaryType =
        serde_json::from_str(&json2).expect("Failed to deserialize BoundaryType");

    match deserialized1 {
        BoundaryType::OptimizedToScalar => {}
        _ => panic!("Expected OptimizedToScalar"),
    }

    match deserialized2 {
        BoundaryType::ScalarToOptimized => {}
        _ => panic!("Expected ScalarToOptimized"),
    }
}

#[test]
fn test_complex_expression_tree_serde() {
    // Build a complex expression tree
    let expr = Expr::call(
        Function::RollingMean,
        vec![
            Expr::call(
                Function::Diff,
                vec![
                    Expr::call(
                        Function::Lag,
                        vec![Expr::column("price"), Expr::literal(1.0)],
                    ),
                    Expr::literal(1.0),
                ],
            ),
            Expr::literal(10.0),
        ],
    );

    let json = serde_json::to_string(&expr).expect("Failed to serialize complex expression");
    let deserialized: Expr =
        serde_json::from_str(&json).expect("Failed to deserialize complex expression");

    // Verify structure is preserved
    match &deserialized.node {
        ExprNode::Call(func, args) => {
            assert_eq!(*func, Function::RollingMean);
            assert_eq!(args.len(), 2);

            // Check nested structure
            match &args[0].node {
                ExprNode::Call(inner_func, inner_args) => {
                    assert_eq!(*inner_func, Function::Diff);
                    assert_eq!(inner_args.len(), 2);
                }
                _ => panic!("Expected nested Call node"),
            }
        }
        _ => panic!("Expected Call node at root"),
    }
}
