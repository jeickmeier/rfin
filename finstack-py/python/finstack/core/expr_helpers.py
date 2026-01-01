"""Helper functions and extensions for ergonomic expression construction.

This module provides Pythonic shortcuts for building expression ASTs without
using verbose static method calls.

Examples
--------
>>> from finstack.core.expr import Expr, Function
>>> from finstack.core.expr_helpers import col, lit
>>> 
>>> # Instead of: Expr.bin_op(BinOp.ADD, Expr.column("x"), Expr.literal(10))
>>> # Use: col("x") + 10
>>> expr = col("x") + 10
>>> 
>>> # Complex expressions
>>> expr = (col("revenue") * 1.1 - col("cogs")) / col("periods")
>>> 
>>> # Function calls
>>> expr = rolling_mean(col("x"), 3)
"""

from typing import Union, List
from finstack.core.expr import Expr, Function, BinOp, UnaryOp

# Type alias for values that can be converted to Expr
ExprLike = Union[Expr, float, int]


def _to_expr(value: ExprLike) -> Expr:
    """Convert a value to an Expr if needed."""
    if isinstance(value, Expr):
        return value
    elif isinstance(value, (int, float)):
        return Expr.literal(float(value))
    else:
        raise TypeError(f"Cannot convert {type(value)} to Expr")


def col(name: str) -> Expr:
    """Create a column reference expression.
    
    Parameters
    ----------
    name : str
        Column name to reference
        
    Returns
    -------
    Expr
        Column reference expression
        
    Examples
    --------
    >>> expr = col("revenue")
    >>> expr = col("x") + col("y")
    """
    return Expr.column(name)


def lit(value: float) -> Expr:
    """Create a literal value expression.
    
    Parameters
    ----------
    value : float
        Literal value
        
    Returns
    -------
    Expr
        Literal expression
        
    Examples
    --------
    >>> expr = lit(100.0)
    >>> expr = col("x") + lit(50)
    """
    return Expr.literal(float(value))


# Monkey-patch arithmetic operators onto Expr
def _expr_add(self: Expr, other: ExprLike) -> Expr:
    """Add two expressions: self + other"""
    return Expr.bin_op(BinOp.ADD, self, _to_expr(other))


def _expr_radd(self: Expr, other: ExprLike) -> Expr:
    """Reverse add: other + self"""
    return Expr.bin_op(BinOp.ADD, _to_expr(other), self)


def _expr_sub(self: Expr, other: ExprLike) -> Expr:
    """Subtract: self - other"""
    return Expr.bin_op(BinOp.SUB, self, _to_expr(other))


def _expr_rsub(self: Expr, other: ExprLike) -> Expr:
    """Reverse subtract: other - self"""
    return Expr.bin_op(BinOp.SUB, _to_expr(other), self)


def _expr_mul(self: Expr, other: ExprLike) -> Expr:
    """Multiply: self * other"""
    return Expr.bin_op(BinOp.MUL, self, _to_expr(other))


def _expr_rmul(self: Expr, other: ExprLike) -> Expr:
    """Reverse multiply: other * self"""
    return Expr.bin_op(BinOp.MUL, _to_expr(other), self)


def _expr_truediv(self: Expr, other: ExprLike) -> Expr:
    """Divide: self / other"""
    return Expr.bin_op(BinOp.DIV, self, _to_expr(other))


def _expr_rtruediv(self: Expr, other: ExprLike) -> Expr:
    """Reverse divide: other / self"""
    return Expr.bin_op(BinOp.DIV, _to_expr(other), self)


def _expr_mod(self: Expr, other: ExprLike) -> Expr:
    """Modulo: self % other"""
    return Expr.bin_op(BinOp.MOD, self, _to_expr(other))


def _expr_rmod(self: Expr, other: ExprLike) -> Expr:
    """Reverse modulo: other % self"""
    return Expr.bin_op(BinOp.MOD, _to_expr(other), self)


def _expr_neg(self: Expr) -> Expr:
    """Negate: -self"""
    return Expr.unary_op(UnaryOp.NEG, self)


# Comparison operators
def _expr_eq(self: Expr, other: ExprLike) -> Expr:
    """Equal: self == other"""
    return Expr.bin_op(BinOp.EQ, self, _to_expr(other))


def _expr_ne(self: Expr, other: ExprLike) -> Expr:
    """Not equal: self != other"""
    return Expr.bin_op(BinOp.NE, self, _to_expr(other))


def _expr_lt(self: Expr, other: ExprLike) -> Expr:
    """Less than: self < other"""
    return Expr.bin_op(BinOp.LT, self, _to_expr(other))


def _expr_le(self: Expr, other: ExprLike) -> Expr:
    """Less than or equal: self <= other"""
    return Expr.bin_op(BinOp.LE, self, _to_expr(other))


def _expr_gt(self: Expr, other: ExprLike) -> Expr:
    """Greater than: self > other"""
    return Expr.bin_op(BinOp.GT, self, _to_expr(other))


def _expr_ge(self: Expr, other: ExprLike) -> Expr:
    """Greater than or equal: self >= other"""
    return Expr.bin_op(BinOp.GE, self, _to_expr(other))


# Logical operators
def _expr_and(self: Expr, other: ExprLike) -> Expr:
    """Logical AND: self & other"""
    return Expr.bin_op(BinOp.AND, self, _to_expr(other))


def _expr_or(self: Expr, other: ExprLike) -> Expr:
    """Logical OR: self | other"""
    return Expr.bin_op(BinOp.OR, self, _to_expr(other))


def _expr_invert(self: Expr) -> Expr:
    """Logical NOT: ~self"""
    return Expr.unary_op(UnaryOp.NOT, self)


# Apply monkey-patches
Expr.__add__ = _expr_add
Expr.__radd__ = _expr_radd
Expr.__sub__ = _expr_sub
Expr.__rsub__ = _expr_rsub
Expr.__mul__ = _expr_mul
Expr.__rmul__ = _expr_rmul
Expr.__truediv__ = _expr_truediv
Expr.__rtruediv__ = _expr_rtruediv
Expr.__mod__ = _expr_mod
Expr.__rmod__ = _expr_rmod
Expr.__neg__ = _expr_neg
Expr.__eq__ = _expr_eq
Expr.__ne__ = _expr_ne
Expr.__lt__ = _expr_lt
Expr.__le__ = _expr_le
Expr.__gt__ = _expr_gt
Expr.__ge__ = _expr_ge
Expr.__and__ = _expr_and
Expr.__or__ = _expr_or
Expr.__invert__ = _expr_invert


# Function call helpers
def lag(expr: Expr, n: Union[int, Expr]) -> Expr:
    """Lag expression by n periods.
    
    Parameters
    ----------
    expr : Expr
        Expression to lag
    n : int or Expr
        Number of periods to lag
        
    Returns
    -------
    Expr
        Lagged expression
    """
    return Expr.call(Function.LAG, [expr, _to_expr(n)])


def lead(expr: Expr, n: Union[int, Expr]) -> Expr:
    """Lead expression by n periods."""
    return Expr.call(Function.LEAD, [expr, _to_expr(n)])


def diff(expr: Expr, n: Union[int, Expr] = 1) -> Expr:
    """Difference: expr[t] - expr[t-n]"""
    return Expr.call(Function.DIFF, [expr, _to_expr(n)])


def pct_change(expr: Expr, n: Union[int, Expr] = 1) -> Expr:
    """Percent change: (expr[t] - expr[t-n]) / expr[t-n]"""
    return Expr.call(Function.PCT_CHANGE, [expr, _to_expr(n)])


def cumsum(expr: Expr) -> Expr:
    """Cumulative sum"""
    return Expr.call(Function.CUM_SUM, [expr])


def cumprod(expr: Expr) -> Expr:
    """Cumulative product"""
    return Expr.call(Function.CUM_PROD, [expr])


def cummin(expr: Expr) -> Expr:
    """Cumulative minimum"""
    return Expr.call(Function.CUM_MIN, [expr])


def cummax(expr: Expr) -> Expr:
    """Cumulative maximum"""
    return Expr.call(Function.CUM_MAX, [expr])


def rolling_mean(expr: Expr, window: Union[int, Expr]) -> Expr:
    """Rolling mean over window"""
    return Expr.call(Function.ROLLING_MEAN, [expr, _to_expr(window)])


def rolling_sum(expr: Expr, window: Union[int, Expr]) -> Expr:
    """Rolling sum over window"""
    return Expr.call(Function.ROLLING_SUM, [expr, _to_expr(window)])


def rolling_std(expr: Expr, window: Union[int, Expr]) -> Expr:
    """Rolling standard deviation over window"""
    return Expr.call(Function.ROLLING_STD, [expr, _to_expr(window)])


def rolling_var(expr: Expr, window: Union[int, Expr]) -> Expr:
    """Rolling variance over window"""
    return Expr.call(Function.ROLLING_VAR, [expr, _to_expr(window)])


def rolling_median(expr: Expr, window: Union[int, Expr]) -> Expr:
    """Rolling median over window"""
    return Expr.call(Function.ROLLING_MEDIAN, [expr, _to_expr(window)])


def rolling_min(expr: Expr, window: Union[int, Expr]) -> Expr:
    """Rolling minimum over window"""
    return Expr.call(Function.ROLLING_MIN, [expr, _to_expr(window)])


def rolling_max(expr: Expr, window: Union[int, Expr]) -> Expr:
    """Rolling maximum over window"""
    return Expr.call(Function.ROLLING_MAX, [expr, _to_expr(window)])


def rolling_count(expr: Expr, window: Union[int, Expr]) -> Expr:
    """Rolling count of non-null values over window"""
    return Expr.call(Function.ROLLING_COUNT, [expr, _to_expr(window)])


def ewm_mean(expr: Expr, alpha: Union[float, Expr], adjust: Union[bool, Expr] = True) -> Expr:
    """Exponentially weighted moving average"""
    return Expr.call(Function.EWM_MEAN, [expr, _to_expr(alpha), _to_expr(1.0 if adjust else 0.0)])


def ewm_std(expr: Expr, alpha: Union[float, Expr], adjust: Union[bool, Expr] = True) -> Expr:
    """Exponentially weighted moving standard deviation"""
    return Expr.call(Function.EWM_STD, [expr, _to_expr(alpha), _to_expr(1.0 if adjust else 0.0)])


def ewm_var(expr: Expr, alpha: Union[float, Expr], adjust: Union[bool, Expr] = True) -> Expr:
    """Exponentially weighted moving variance"""
    return Expr.call(Function.EWM_VAR, [expr, _to_expr(alpha), _to_expr(1.0 if adjust else 0.0)])


def std(expr: Expr) -> Expr:
    """Standard deviation (global reducer)"""
    return Expr.call(Function.STD, [expr])


def var(expr: Expr) -> Expr:
    """Variance (global reducer)"""
    return Expr.call(Function.VAR, [expr])


def median(expr: Expr) -> Expr:
    """Median (global reducer)"""
    return Expr.call(Function.MEDIAN, [expr])


def shift(expr: Expr, n: Union[int, Expr]) -> Expr:
    """Shift expression by n periods (alias for lag)"""
    return Expr.call(Function.SHIFT, [expr, _to_expr(n)])


def rank(expr: Expr) -> Expr:
    """Rank values (global reducer, broadcasts scalar rank)"""
    return Expr.call(Function.RANK, [expr])


def quantile(expr: Expr, q: Union[float, Expr]) -> Expr:
    """Quantile (global reducer, broadcasts scalar)"""
    return Expr.call(Function.QUANTILE, [expr, _to_expr(q)])


def sum_(expr: Expr) -> Expr:
    """Sum (global reducer)"""
    return Expr.call(Function.SUM, [expr])


def mean(expr: Expr) -> Expr:
    """Mean (global reducer)"""
    return Expr.call(Function.MEAN, [expr])


def annualize(expr: Expr, periods_per_year: Union[int, Expr]) -> Expr:
    """Annualize a return or rate"""
    return Expr.call(Function.ANNUALIZE, [expr, _to_expr(periods_per_year)])


def annualize_rate(expr: Expr, periods_per_year: Union[int, Expr]) -> Expr:
    """Annualize a rate (compound)"""
    return Expr.call(Function.ANNUALIZE_RATE, [expr, _to_expr(periods_per_year)])


def ttm(expr: Expr) -> Expr:
    """Trailing twelve months sum"""
    return Expr.call(Function.TTM, [expr])


def ytd(expr: Expr) -> Expr:
    """Year-to-date sum"""
    return Expr.call(Function.YTD, [expr])


def qtd(expr: Expr) -> Expr:
    """Quarter-to-date sum"""
    return Expr.call(Function.QTD, [expr])


def fiscal_ytd(expr: Expr, fiscal_month: Union[int, Expr] = 1) -> Expr:
    """Fiscal year-to-date sum"""
    return Expr.call(Function.FISCAL_YTD, [expr, _to_expr(fiscal_month)])


def coalesce(*exprs: Expr) -> Expr:
    """Return first non-null value"""
    if not exprs:
        raise ValueError("coalesce requires at least one argument")
    return Expr.call(Function.COALESCE, list(exprs))


def abs_(expr: Expr) -> Expr:
    """Absolute value"""
    return Expr.call(Function.ABS, [expr])


def sign(expr: Expr) -> Expr:
    """Sign (-1, 0, or 1)"""
    return Expr.call(Function.SIGN, [expr])


def growth_rate(expr: Expr, n: Union[int, Expr] = 1) -> Expr:
    """Growth rate over n periods"""
    return Expr.call(Function.GROWTH_RATE, [expr, _to_expr(n)])


def if_then_else(condition: Expr, then_expr: ExprLike, else_expr: ExprLike) -> Expr:
    """Conditional expression: if condition then then_expr else else_expr"""
    return Expr.if_then_else(condition, _to_expr(then_expr), _to_expr(else_expr))


__all__ = [
    "col",
    "lit",
    "lag",
    "lead",
    "diff",
    "pct_change",
    "cumsum",
    "cumprod",
    "cummin",
    "cummax",
    "rolling_mean",
    "rolling_sum",
    "rolling_std",
    "rolling_var",
    "rolling_median",
    "rolling_min",
    "rolling_max",
    "rolling_count",
    "ewm_mean",
    "ewm_std",
    "ewm_var",
    "std",
    "var",
    "median",
    "shift",
    "rank",
    "quantile",
    "sum_",
    "mean",
    "annualize",
    "annualize_rate",
    "ttm",
    "ytd",
    "qtd",
    "fiscal_ytd",
    "coalesce",
    "abs_",
    "sign",
    "growth_rate",
    "if_then_else",
]
