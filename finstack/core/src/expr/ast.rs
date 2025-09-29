//! AST nodes and function registry for the expression engine.

use core::hash::{Hash, Hasher};

// DurationSpec removed: time-window API was unused in evaluation

/// Expression AST with optional unique ID for DAG planning and caching.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Expr {
    /// Unique identifier for this expression node (for caching and DAG planning).
    pub id: Option<u64>,
    /// The actual expression node.
    pub node: ExprNode,
}

/// The core expression node types.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExprNode {
    /// Reference a column by name.
    Column(String),
    /// Literal scalar value using the crate's numeric type alias.
    Literal(f64),
    /// Call a registered function with positional arguments.
    Call(Function, Vec<Expr>),
}

impl Expr {
    /// Create a new column reference.
    pub fn column(name: impl Into<String>) -> Self {
        Self {
            id: None,
            node: ExprNode::Column(name.into()),
        }
    }

    /// Create a new literal value.
    pub fn literal(value: f64) -> Self {
        Self {
            id: None,
            node: ExprNode::Literal(value),
        }
    }

    /// Create a new function call.
    pub fn call(func: Function, args: Vec<Expr>) -> Self {
        Self {
            id: None,
            node: ExprNode::Call(func, args),
        }
    }

    /// Assign a unique ID to this expression for caching/DAG purposes.
    pub fn with_id(mut self, id: u64) -> Self {
        self.id = Some(id);
        self
    }
}

/// Hash implementation for Expr to support deduplication in DAG planning.
///
/// Note: Structural identity only. The opaque `id` field is intentionally
/// excluded from both `Hash` and `Eq` so that DAG deduplication and caches
/// consider two expressions identical if their `node` matches, regardless of
/// their runtime-assigned ids.
impl Hash for Expr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self.node {
            ExprNode::Column(name) => {
                0u8.hash(state);
                name.hash(state);
            }
            ExprNode::Literal(val) => {
                1u8.hash(state);
                // Hash via raw f64 bits for determinism (covers NaN payloads)
                (*val).to_bits().hash(state);
            }
            ExprNode::Call(func, args) => {
                2u8.hash(state);
                (*func as u8).hash(state);
                args.hash(state);
            }
        }
    }
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        match (&self.node, &other.node) {
            (ExprNode::Column(a), ExprNode::Column(b)) => a == b,
            (ExprNode::Literal(a), ExprNode::Literal(b)) => {
                // f64 equality via raw bits for deterministic NaN handling
                (*a).to_bits() == (*b).to_bits()
            }
            (ExprNode::Call(f1, a1), ExprNode::Call(f2, a2)) => f1 == f2 && a1 == a2,
            _ => false,
        }
    }
}

impl Eq for Expr {}

/// Built-in function identifiers.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Function {
    /// Previous N values (shift down).
    Lag,
    /// Next N values (shift up).
    Lead,
    /// First/lagged difference with step N (default 1).
    Diff,
    /// Percentage change over step N (default 1).
    PctChange,
    /// Cumulative sum.
    CumSum,
    /// Cumulative product.
    CumProd,
    /// Cumulative minimum.
    CumMin,
    /// Cumulative maximum.
    CumMax,
    /// Rolling arithmetic mean over a fixed row window size.
    RollingMean,
    /// Rolling sum over a fixed row window size.
    RollingSum,
    /// Exponentially weighted moving average with alpha and adjust flag.
    EwmMean,
    /// Population standard deviation.
    Std,
    /// Population variance.
    Var,
    /// Median.
    Median,
    /// Rolling standard deviation over a fixed row window size.
    RollingStd,
    /// Rolling variance over a fixed row window size.
    RollingVar,
    /// Rolling median over a fixed row window size.
    RollingMedian,

    /// Shift values by N positions (positive = shift down, negative = shift up).
    Shift,
    /// Rank values (dense ranking).
    Rank,
    /// Calculate quantile/percentile of values.
    Quantile,
    /// Rolling minimum over a fixed row window size.
    RollingMin,
    /// Rolling maximum over a fixed row window size.
    RollingMax,
    /// Count non-null values in rolling window.
    RollingCount,
    /// Exponentially weighted moving standard deviation.
    EwmStd,
    /// Exponentially weighted moving variance.
    EwmVar,
}

// WindowSpec removed with time-window API cleanup

// ExecMeta removed in favor of unified config::ResultsMeta

/// Result envelope that includes execution metadata.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EvaluationResult {
    /// The computed values.
    pub values: Vec<f64>,
    /// Execution metadata stamped into result.
    pub metadata: crate::config::ResultsMeta,
}

// ResultMetadata removed in favor of unified config::ResultsMeta
