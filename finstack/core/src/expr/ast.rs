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
    /// Binary operation (arithmetic, comparison, logical).
    BinOp {
        /// Operator
        op: BinOp,
        /// Left operand
        left: Box<Expr>,
        /// Right operand
        right: Box<Expr>,
    },
    /// Unary operation.
    UnaryOp {
        /// Operator
        op: UnaryOp,
        /// Operand
        operand: Box<Expr>,
    },
    /// If-then-else conditional.
    IfThenElse {
        /// Condition expression
        condition: Box<Expr>,
        /// Then branch
        then_expr: Box<Expr>,
        /// Else branch
        else_expr: Box<Expr>,
    },
}

/// Binary operators for expressions.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BinOp {
    // Arithmetic
    /// Addition (+)
    Add,
    /// Subtraction (-)
    Sub,
    /// Multiplication (*)
    Mul,
    /// Division (/)
    Div,
    /// Modulo (%)
    Mod,

    // Comparison
    /// Equal (==)
    Eq,
    /// Not equal (!=)
    Ne,
    /// Less than (<)
    Lt,
    /// Less than or equal (<=)
    Le,
    /// Greater than (>)
    Gt,
    /// Greater than or equal (>=)
    Ge,

    // Logical
    /// Logical AND
    And,
    /// Logical OR
    Or,
}

/// Unary operators for expressions.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UnaryOp {
    /// Negation (-)
    Neg,
    /// Logical NOT
    Not,
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

    /// Create a new binary operation.
    pub fn bin_op(op: BinOp, left: Expr, right: Expr) -> Self {
        Self {
            id: None,
            node: ExprNode::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            },
        }
    }

    /// Create a new unary operation.
    pub fn unary_op(op: UnaryOp, operand: Expr) -> Self {
        Self {
            id: None,
            node: ExprNode::UnaryOp {
                op,
                operand: Box::new(operand),
            },
        }
    }

    /// Create a new if-then-else conditional.
    pub fn if_then_else(condition: Expr, then_expr: Expr, else_expr: Expr) -> Self {
        Self {
            id: None,
            node: ExprNode::IfThenElse {
                condition: Box::new(condition),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            },
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
            ExprNode::BinOp { op, left, right } => {
                3u8.hash(state);
                op.hash(state);
                left.hash(state);
                right.hash(state);
            }
            ExprNode::UnaryOp { op, operand } => {
                4u8.hash(state);
                op.hash(state);
                operand.hash(state);
            }
            ExprNode::IfThenElse {
                condition,
                then_expr,
                else_expr,
            } => {
                5u8.hash(state);
                condition.hash(state);
                then_expr.hash(state);
                else_expr.hash(state);
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
            (
                ExprNode::BinOp {
                    op: op1,
                    left: l1,
                    right: r1,
                },
                ExprNode::BinOp {
                    op: op2,
                    left: l2,
                    right: r2,
                },
            ) => op1 == op2 && l1 == l2 && r1 == r2,
            (
                ExprNode::UnaryOp {
                    op: op1,
                    operand: o1,
                },
                ExprNode::UnaryOp {
                    op: op2,
                    operand: o2,
                },
            ) => op1 == op2 && o1 == o2,
            (
                ExprNode::IfThenElse {
                    condition: c1,
                    then_expr: t1,
                    else_expr: e1,
                },
                ExprNode::IfThenElse {
                    condition: c2,
                    then_expr: t2,
                    else_expr: e2,
                },
            ) => c1 == c2 && t1 == t2 && e1 == e2,
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
