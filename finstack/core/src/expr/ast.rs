//! AST nodes and function registry for the expression engine.

use super::time_windows::DurationSpec;
use core::hash::{Hash, Hasher};

/// Expression AST with optional unique ID for DAG planning and caching.
#[derive(Clone, Debug)]
pub struct Expr {
    /// Unique identifier for this expression node (for caching and DAG planning).
    pub id: Option<u64>,
    /// The actual expression node.
    pub node: ExprNode,
    /// Optional time window specification for time-based operations.
    pub time_window: Option<TimeWindow>,
}

/// The core expression node types.
#[derive(Clone, Debug)]
pub enum ExprNode {
    /// Reference a column by name.
    Column(String),
    /// Literal scalar value using the crate's numeric type alias.
    Literal(crate::F),
    /// Call a registered function with positional arguments.
    Call(Function, Vec<Expr>),
}

impl Expr {
    /// Create a new column reference.
    pub fn column(name: impl Into<String>) -> Self {
        Self {
            id: None,
            node: ExprNode::Column(name.into()),
            time_window: None,
        }
    }

    /// Create a new literal value.
    pub fn literal(value: crate::F) -> Self {
        Self {
            id: None,
            node: ExprNode::Literal(value),
            time_window: None,
        }
    }

    /// Create a new function call.
    pub fn call(func: Function, args: Vec<Expr>) -> Self {
        Self {
            id: None,
            node: ExprNode::Call(func, args),
            time_window: None,
        }
    }

    /// Assign a unique ID to this expression for caching/DAG purposes.
    pub fn with_id(mut self, id: u64) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the time window for time-based operations.
    pub fn with_time_window(mut self, time_window: TimeWindow) -> Self {
        self.time_window = Some(time_window);
        self
    }
}

/// Hash implementation for Expr to support deduplication in DAG planning.
///
/// Note: Structural identity only. The opaque `id` field is intentionally
/// excluded from both `Hash` and `Eq` so that DAG deduplication and caches
/// consider two expressions identical if their `node` and `time_window`
/// match, regardless of their runtime-assigned ids.
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
        // Include time window in hash
        self.time_window.hash(state);
    }
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        let nodes_eq = match (&self.node, &other.node) {
            (ExprNode::Column(a), ExprNode::Column(b)) => a == b,
            (ExprNode::Literal(a), ExprNode::Literal(b)) => {
                // f64 equality via raw bits for deterministic NaN handling
                (*a).to_bits() == (*b).to_bits()
            }
            (ExprNode::Call(f1, a1), ExprNode::Call(f2, a2)) => f1 == f2 && a1 == a2,
            _ => false,
        };
        nodes_eq && self.time_window == other.time_window
    }
}

impl Eq for Expr {}

/// Time-based window specification for functions that support it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TimeWindow {
    /// Row-based window (traditional).
    Rows(usize),
    /// Time-based window with duration string (e.g., "30d", "1h").
    Duration {
        /// Parsed duration specification. String is kept only at IO boundaries.
        period: DurationSpec,
        /// Name of the time column to use for windowing.
        time_column: String,
    },
}

/// Built-in function identifiers.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
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
    /// Time-based rolling functions - row count in args, duration in TimeWindow
    RollingMeanTime,
    /// Rolling sum over time-based window.
    RollingSumTime,
    /// Rolling standard deviation over time-based window.
    RollingStdTime,
    /// Rolling variance over time-based window.
    RollingVarTime,
    /// Rolling median over time-based window.
    RollingMedianTime,
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

/// Window specification for rolling operations.
#[derive(Clone, Debug, PartialEq)]
pub enum WindowSpec {
    /// Simple row count.
    Rows(usize),
    /// Time-based window.
    Time(TimeWindow),
}

/// Execution plan metadata for determinism and caching.
#[derive(Clone, Debug)]
pub struct ExecMeta {
    /// Whether to use deterministic execution paths.
    pub deterministic: bool,
    /// Whether parallel execution is enabled.
    pub parallel: bool,
    /// Numeric mode being used.
    pub numeric_mode: crate::config::NumericMode,
    /// Schema version for forward compatibility.
    pub version: u32,
    /// Rounding mode for decimal calculations.
    pub rounding_mode: crate::config::RoundingMode,
    /// FX policy identifier for currency conversions.
    pub fx_policy: Option<String>,
}

/// Result envelope that includes execution metadata.
#[derive(Clone, Debug)]
pub struct EvaluationResult {
    /// The computed values.
    pub values: Vec<crate::F>,
    /// Execution metadata stamped into result.
    pub metadata: ResultMetadata,
}

/// Metadata stamped into evaluation results.
#[derive(Clone, Debug)]
pub struct ResultMetadata {
    /// Whether result was computed deterministically.
    pub deterministic: bool,
    /// Whether parallel execution was used.
    pub parallel_execution: bool,
    /// Numeric mode used for computation.
    pub numeric_mode: crate::config::NumericMode,
    /// Rounding context active during computation.
    pub rounding_context: crate::config::RoundingMode,
    /// FX policy applied (if any).
    pub fx_policy_applied: Option<String>,
    /// Execution time in nanoseconds.
    pub execution_time_ns: u64,
    /// Cache hit ratio during evaluation.
    pub cache_hit_ratio: Option<f64>,
}
