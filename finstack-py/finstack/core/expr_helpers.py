"""Ergonomic helper functions for building expressions.

This module provides convenient functions and classes for building
expression ASTs without verbose constructor calls.

Examples:
--------
    >>> from finstack.core.expr_helpers import col, lit, lag, rolling_mean
    >>> expr = rolling_mean(col("price"), 5) > lag(col("price"), 1)
    >>> # Supports arithmetic operators
    >>> margin = (col("revenue") - col("cogs")) / col("revenue")
"""

from __future__ import annotations

from typing import Union

# Import the Rust Expr types
try:
    from finstack.finstack.core.expr import BinOp, Expr, Function, UnaryOp
except ImportError:
    # Fallback imports for different module structures
    try:
        from finstack.core.expr import BinOp, Expr, Function, UnaryOp
    except ImportError:
        # For type checking
        BinOp = None
        Expr = None
        Function = None
        UnaryOp = None

# Type alias for values that can be converted to expressions
ExprLike = Union["ExprWrapper", "Expr", float, int]


class ExprWrapper:
    """Wrapper around Expr that provides operator overloading.

    This class wraps the Rust Expr type and adds Python operator
    overloading for more ergonomic expression building.

    For use with CompiledExpr, call unwrap() or access the .expr property.
    """

    def __init__(self, expr: Expr) -> None:
        """Create a wrapper around an Expr.

        Parameters
        ----------
        expr : Expr
            The underlying Rust expression.
        """
        self._expr = expr

    @property
    def expr(self) -> Expr:
        """Get the underlying Expr for use with Rust functions like CompiledExpr."""
        return self._expr

    def unwrap(self) -> Expr:
        """Get the underlying Expr for use with Rust functions like CompiledExpr.

        This is an alias for the .expr property for more explicit unwrapping.
        """
        return self._expr

    def __repr__(self) -> str:
        """Return the representation of the underlying expression."""
        return repr(self._expr)

    __hash__ = None

    def _ensure_expr(self, other: ExprLike) -> Expr:
        """Convert a value to an Expr if needed."""
        if isinstance(other, ExprWrapper):
            return other._expr
        elif isinstance(other, Expr):
            return other
        elif isinstance(other, (int, float)):
            return Expr.literal(float(other))
        else:
            raise TypeError(f"Cannot convert {type(other)} to Expr")

    # Arithmetic operators
    def __add__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing addition."""
        return ExprWrapper(Expr.bin_op(BinOp.ADD, self._expr, self._ensure_expr(other)))

    def __radd__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing reflected addition."""
        return ExprWrapper(Expr.bin_op(BinOp.ADD, self._ensure_expr(other), self._expr))

    def __sub__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing subtraction."""
        return ExprWrapper(Expr.bin_op(BinOp.SUB, self._expr, self._ensure_expr(other)))

    def __rsub__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing reflected subtraction."""
        return ExprWrapper(Expr.bin_op(BinOp.SUB, self._ensure_expr(other), self._expr))

    def __mul__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing multiplication."""
        return ExprWrapper(Expr.bin_op(BinOp.MUL, self._expr, self._ensure_expr(other)))

    def __rmul__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing reflected multiplication."""
        return ExprWrapper(Expr.bin_op(BinOp.MUL, self._ensure_expr(other), self._expr))

    def __truediv__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing division."""
        return ExprWrapper(Expr.bin_op(BinOp.DIV, self._expr, self._ensure_expr(other)))

    def __rtruediv__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing reflected division."""
        return ExprWrapper(Expr.bin_op(BinOp.DIV, self._ensure_expr(other), self._expr))

    def __mod__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing modulo."""
        return ExprWrapper(Expr.bin_op(BinOp.MOD, self._expr, self._ensure_expr(other)))

    def __rmod__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing reflected modulo."""
        return ExprWrapper(Expr.bin_op(BinOp.MOD, self._ensure_expr(other), self._expr))

    def __neg__(self) -> ExprWrapper:
        """Return an expression representing negation."""
        return ExprWrapper(Expr.unary_op(UnaryOp.NEG, self._expr))

    # Comparison operators
    def __eq__(self, other: ExprLike) -> ExprWrapper:  # type: ignore[override]
        """Return an expression representing equality."""
        return ExprWrapper(Expr.bin_op(BinOp.EQ, self._expr, self._ensure_expr(other)))

    def __ne__(self, other: ExprLike) -> ExprWrapper:  # type: ignore[override]
        """Return an expression representing inequality."""
        return ExprWrapper(Expr.bin_op(BinOp.NE, self._expr, self._ensure_expr(other)))

    def __lt__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing less-than."""
        return ExprWrapper(Expr.bin_op(BinOp.LT, self._expr, self._ensure_expr(other)))

    def __le__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing less-than-or-equal."""
        return ExprWrapper(Expr.bin_op(BinOp.LE, self._expr, self._ensure_expr(other)))

    def __gt__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing greater-than."""
        return ExprWrapper(Expr.bin_op(BinOp.GT, self._expr, self._ensure_expr(other)))

    def __ge__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing greater-than-or-equal."""
        return ExprWrapper(Expr.bin_op(BinOp.GE, self._expr, self._ensure_expr(other)))

    # Logical operators
    def __and__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing logical AND."""
        return ExprWrapper(Expr.bin_op(BinOp.AND, self._expr, self._ensure_expr(other)))

    def __rand__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing reflected logical AND."""
        return ExprWrapper(Expr.bin_op(BinOp.AND, self._ensure_expr(other), self._expr))

    def __or__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing logical OR."""
        return ExprWrapper(Expr.bin_op(BinOp.OR, self._expr, self._ensure_expr(other)))

    def __ror__(self, other: ExprLike) -> ExprWrapper:
        """Return an expression representing reflected logical OR."""
        return ExprWrapper(Expr.bin_op(BinOp.OR, self._ensure_expr(other), self._expr))

    def __invert__(self) -> ExprWrapper:
        """Return an expression representing logical NOT."""
        return ExprWrapper(Expr.unary_op(UnaryOp.NOT, self._expr))

    def with_id(self, id: int) -> ExprWrapper:
        """Return a copy with a specific ID for DAG deduplication."""
        return ExprWrapper(self._expr.with_id(id))


def _to_expr(value: ExprLike) -> Expr:
    """Convert a value to an Expr."""
    if isinstance(value, ExprWrapper):
        return value._expr
    elif isinstance(value, Expr):
        return value
    elif isinstance(value, (int, float)):
        return Expr.literal(float(value))
    else:
        raise TypeError(f"Cannot convert {type(value)} to Expr")


# Column and literal constructors
def col(name: str) -> ExprWrapper:
    """Create a column reference expression.

    Parameters
    ----------
    name : str
        Column name to reference.

    Returns:
    -------
    ExprWrapper
        Column reference expression with operator overloading.

    Examples:
    --------
        >>> revenue = col("revenue")
        >>> expr = revenue * 1.1
    """
    return ExprWrapper(Expr.column(name))


def lit(value: float) -> ExprWrapper:
    """Create a literal value expression.

    Parameters
    ----------
    value : float
        Constant value.

    Returns:
    -------
    ExprWrapper
        Literal expression with operator overloading.

    Examples:
    --------
        >>> threshold = lit(100.0)
        >>> expr = col("price") > threshold
    """
    return ExprWrapper(Expr.literal(float(value)))


# Time-series functions
def lag(expr: ExprLike, periods: int = 1) -> ExprWrapper:
    """Shift values backward by n positions.

    Parameters
    ----------
    expr : ExprLike
        Input expression.
    periods : int, default 1
        Number of periods to shift.

    Returns:
    -------
    ExprWrapper
        Lagged expression.

    Examples:
    --------
        >>> prev_price = lag(col("price"), 1)
    """
    return ExprWrapper(Expr.call(Function.LAG, [_to_expr(expr), Expr.literal(float(periods))]))


def lead(expr: ExprLike, periods: int = 1) -> ExprWrapper:
    """Shift values forward by n positions.

    Parameters
    ----------
    expr : ExprLike
        Input expression.
    periods : int, default 1
        Number of periods to shift.

    Returns:
    -------
    ExprWrapper
        Lead expression.

    Examples:
    --------
        >>> next_price = lead(col("price"), 1)
    """
    return ExprWrapper(Expr.call(Function.LEAD, [_to_expr(expr), Expr.literal(float(periods))]))


def diff(expr: ExprLike, periods: int = 1) -> ExprWrapper:
    """Difference between current and n-lagged value.

    Parameters
    ----------
    expr : ExprLike
        Input expression.
    periods : int, default 1
        Number of periods for differencing.

    Returns:
    -------
    ExprWrapper
        Differenced expression.

    Examples:
    --------
        >>> price_change = diff(col("price"), 1)
    """
    return ExprWrapper(Expr.call(Function.DIFF, [_to_expr(expr), Expr.literal(float(periods))]))


def pct_change(expr: ExprLike, periods: int = 1) -> ExprWrapper:
    """Percentage change from n periods ago.

    Parameters
    ----------
    expr : ExprLike
        Input expression.
    periods : int, default 1
        Number of periods for comparison.

    Returns:
    -------
    ExprWrapper
        Percentage change expression.

    Examples:
    --------
        >>> returns = pct_change(col("price"), 1)
    """
    return ExprWrapper(Expr.call(Function.PCT_CHANGE, [_to_expr(expr), Expr.literal(float(periods))]))


def growth_rate(expr: ExprLike, periods: int = 1) -> ExprWrapper:
    """Growth rate calculation.

    Parameters
    ----------
    expr : ExprLike
        Input expression.
    periods : int, default 1
        Number of periods for comparison.

    Returns:
    -------
    ExprWrapper
        Growth rate expression.

    Examples:
    --------
        >>> growth = growth_rate(col("revenue"), 1)
    """
    return ExprWrapper(Expr.call(Function.GROWTH_RATE, [_to_expr(expr), Expr.literal(float(periods))]))


# Cumulative functions
def cumsum(expr: ExprLike) -> ExprWrapper:
    """Cumulative sum.

    Parameters
    ----------
    expr : ExprLike
        Input expression.

    Returns:
    -------
    ExprWrapper
        Cumulative sum expression.

    Examples:
    --------
        >>> running_total = cumsum(col("amount"))
    """
    return ExprWrapper(Expr.call(Function.CUM_SUM, [_to_expr(expr)]))


def cumprod(expr: ExprLike) -> ExprWrapper:
    """Cumulative product.

    Parameters
    ----------
    expr : ExprLike
        Input expression.

    Returns:
    -------
    ExprWrapper
        Cumulative product expression.
    """
    return ExprWrapper(Expr.call(Function.CUM_PROD, [_to_expr(expr)]))


def cummin(expr: ExprLike) -> ExprWrapper:
    """Cumulative minimum.

    Parameters
    ----------
    expr : ExprLike
        Input expression.

    Returns:
    -------
    ExprWrapper
        Cumulative minimum expression.
    """
    return ExprWrapper(Expr.call(Function.CUM_MIN, [_to_expr(expr)]))


def cummax(expr: ExprLike) -> ExprWrapper:
    """Cumulative maximum.

    Parameters
    ----------
    expr : ExprLike
        Input expression.

    Returns:
    -------
    ExprWrapper
        Cumulative maximum expression.
    """
    return ExprWrapper(Expr.call(Function.CUM_MAX, [_to_expr(expr)]))


# Rolling window functions
def rolling_mean(expr: ExprLike, window: int) -> ExprWrapper:
    """Rolling mean over a window.

    Parameters
    ----------
    expr : ExprLike
        Input expression.
    window : int
        Window size.

    Returns:
    -------
    ExprWrapper
        Rolling mean expression.

    Examples:
    --------
        >>> sma_20 = rolling_mean(col("price"), 20)
    """
    return ExprWrapper(Expr.call(Function.ROLLING_MEAN, [_to_expr(expr), Expr.literal(float(window))]))


def rolling_sum(expr: ExprLike, window: int) -> ExprWrapper:
    """Rolling sum over a window.

    Parameters
    ----------
    expr : ExprLike
        Input expression.
    window : int
        Window size.

    Returns:
    -------
    ExprWrapper
        Rolling sum expression.
    """
    return ExprWrapper(Expr.call(Function.ROLLING_SUM, [_to_expr(expr), Expr.literal(float(window))]))


def rolling_std(expr: ExprLike, window: int) -> ExprWrapper:
    """Rolling standard deviation over a window.

    Parameters
    ----------
    expr : ExprLike
        Input expression.
    window : int
        Window size.

    Returns:
    -------
    ExprWrapper
        Rolling standard deviation expression.

    Examples:
    --------
        >>> volatility = rolling_std(col("returns"), 20)
    """
    return ExprWrapper(Expr.call(Function.ROLLING_STD, [_to_expr(expr), Expr.literal(float(window))]))


def rolling_var(expr: ExprLike, window: int) -> ExprWrapper:
    """Rolling variance over a window.

    Parameters
    ----------
    expr : ExprLike
        Input expression.
    window : int
        Window size.

    Returns:
    -------
    ExprWrapper
        Rolling variance expression.
    """
    return ExprWrapper(Expr.call(Function.ROLLING_VAR, [_to_expr(expr), Expr.literal(float(window))]))


def rolling_min(expr: ExprLike, window: int) -> ExprWrapper:
    """Rolling minimum over a window.

    Parameters
    ----------
    expr : ExprLike
        Input expression.
    window : int
        Window size.

    Returns:
    -------
    ExprWrapper
        Rolling minimum expression.
    """
    return ExprWrapper(Expr.call(Function.ROLLING_MIN, [_to_expr(expr), Expr.literal(float(window))]))


def rolling_max(expr: ExprLike, window: int) -> ExprWrapper:
    """Rolling maximum over a window.

    Parameters
    ----------
    expr : ExprLike
        Input expression.
    window : int
        Window size.

    Returns:
    -------
    ExprWrapper
        Rolling maximum expression.
    """
    return ExprWrapper(Expr.call(Function.ROLLING_MAX, [_to_expr(expr), Expr.literal(float(window))]))


# Conditional
def if_then_else(condition: ExprLike, then_expr: ExprLike, else_expr: ExprLike) -> ExprWrapper:
    """Create a conditional expression.

    Parameters
    ----------
    condition : ExprLike
        Boolean condition.
    then_expr : ExprLike
        Expression if condition is true.
    else_expr : ExprLike
        Expression if condition is false.

    Returns:
    -------
    ExprWrapper
        Conditional expression.

    Examples:
    --------
        >>> abs_x = if_then_else(col("x") > 0, col("x"), -col("x"))
    """
    return ExprWrapper(Expr.if_then_else(_to_expr(condition), _to_expr(then_expr), _to_expr(else_expr)))


__all__ = [
    "ExprWrapper",
    "col",
    "cummax",
    "cummin",
    "cumprod",
    "cumsum",
    "diff",
    "growth_rate",
    "if_then_else",
    "lag",
    "lead",
    "lit",
    "pct_change",
    "rolling_max",
    "rolling_mean",
    "rolling_min",
    "rolling_std",
    "rolling_sum",
    "rolling_var",
]
