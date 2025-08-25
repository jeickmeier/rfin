//! AST nodes and function registry for the expression engine.

/// Expression AST.
#[derive(Clone, Debug)]
pub enum Expr {
    /// Reference a column by name.
    Column(String),
    /// Literal scalar value (f64 for now).
    Literal(f64),
    /// Call a registered function with positional arguments.
    Call(Function, Vec<Expr>),
}

/// Built-in function identifiers.
#[derive(Clone, Debug, Copy)]
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
}
