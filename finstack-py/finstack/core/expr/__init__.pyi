"""Expression engine bindings (AST, compilation, evaluation).

This module exposes the finstack-core expression engine for Python.

The expression engine supports:
- Arithmetic, comparison, and logical operations
- Time-series functions (lag, lead, diff, pct_change)
- Rolling window operations (mean, sum, std, var, median, min, max)
- Cumulative operations (cumsum, cumprod, cummin, cummax)
- Exponentially-weighted moving averages (ewm_mean, ewm_std, ewm_var)
- Conditional expressions (if-then-else)
- DAG planning with shared subexpression detection
- Intelligent caching for intermediate results

Examples
--------
    >>> from finstack.core.expr import Expr, Function, CompiledExpr, EvalOpts
    >>> # Create expression: rolling_mean(x, 3)
    >>> expr = Expr.call(Function.ROLLING_MEAN, [Expr.column("x"), Expr.literal(3.0)])
    >>> # Compile and evaluate
    >>> compiled = CompiledExpr(expr)
    >>> result = compiled.eval(["x"], [[1.0, 2.0, 3.0, 4.0, 5.0]])
    >>> result.values
    [nan, nan, 2.0, 3.0, 4.0]
"""

from typing import Optional
from ..config import ResultsMeta

class Function:
    """Expression function enumeration.
    
    Supported functions for time-series operations and aggregations.
    Use class attributes to access function variants (e.g., ``Function.LAG``).
    """

    LAG: "Function"
    """Shift values backward by n positions."""
    LEAD: "Function"
    """Shift values forward by n positions."""
    DIFF: "Function"
    """Difference between current and n-lagged value."""
    PCT_CHANGE: "Function"
    """Percentage change from n periods ago."""
    CUM_SUM: "Function"
    """Cumulative sum."""
    CUM_PROD: "Function"
    """Cumulative product."""
    CUM_MIN: "Function"
    """Cumulative minimum."""
    CUM_MAX: "Function"
    """Cumulative maximum."""
    ROLLING_MEAN: "Function"
    """Rolling mean over a window."""
    ROLLING_SUM: "Function"
    """Rolling sum over a window."""
    ROLLING_STD: "Function"
    """Rolling standard deviation over a window."""
    ROLLING_VAR: "Function"
    """Rolling variance over a window."""
    ROLLING_MEDIAN: "Function"
    """Rolling median over a window."""
    ROLLING_MIN: "Function"
    """Rolling minimum over a window."""
    ROLLING_MAX: "Function"
    """Rolling maximum over a window."""
    ROLLING_COUNT: "Function"
    """Rolling count of non-NaN values over a window."""
    EWM_MEAN: "Function"
    """Exponentially weighted moving average."""
    EWM_STD: "Function"
    """Exponentially weighted moving standard deviation."""
    EWM_VAR: "Function"
    """Exponentially weighted moving variance."""
    STD: "Function"
    """Standard deviation (global reducer)."""
    VAR: "Function"
    """Variance (global reducer)."""
    MEDIAN: "Function"
    """Median (global reducer)."""
    SHIFT: "Function"
    """Shift/translate values by n positions."""
    RANK: "Function"
    """Rank values."""
    QUANTILE: "Function"
    """Quantile (global reducer)."""
    SUM: "Function"
    """Sum (global reducer)."""
    MEAN: "Function"
    """Mean (global reducer)."""
    ANNUALIZE: "Function"
    """Annualize a value."""
    ANNUALIZE_RATE: "Function"
    """Annualize a rate."""
    TTM: "Function"
    """Trailing twelve months."""
    YTD: "Function"
    """Year to date."""
    QTD: "Function"
    """Quarter to date."""
    FISCAL_YTD: "Function"
    """Fiscal year to date."""
    COALESCE: "Function"
    """Return first non-null value."""
    ABS: "Function"
    """Absolute value."""
    SIGN: "Function"
    """Sign function (-1, 0, or 1)."""
    GROWTH_RATE: "Function"
    """Growth rate calculation."""

    @property
    def name(self) -> str:
        """Get the function name as a string."""
        ...

    def __repr__(self) -> str: ...


class BinOp:
    """Binary operation enumeration.
    
    Supported binary operators for arithmetic, comparison, and logical operations.
    """

    ADD: "BinOp"
    """Addition."""
    SUB: "BinOp"
    """Subtraction."""
    MUL: "BinOp"
    """Multiplication."""
    DIV: "BinOp"
    """Division."""
    MOD: "BinOp"
    """Modulo."""
    EQ: "BinOp"
    """Equality comparison."""
    NE: "BinOp"
    """Not-equal comparison."""
    LT: "BinOp"
    """Less-than comparison."""
    LE: "BinOp"
    """Less-than-or-equal comparison."""
    GT: "BinOp"
    """Greater-than comparison."""
    GE: "BinOp"
    """Greater-than-or-equal comparison."""
    AND: "BinOp"
    """Logical AND."""
    OR: "BinOp"
    """Logical OR."""

    def __repr__(self) -> str: ...


class UnaryOp:
    """Unary operation enumeration."""

    NEG: "UnaryOp"
    """Negation."""
    NOT: "UnaryOp"
    """Logical NOT."""

    def __repr__(self) -> str: ...


class Expr:
    """Expression AST node.
    
    Build expressions using static constructors like :meth:`column`, :meth:`literal`,
    :meth:`call`, :meth:`bin_op`, and :meth:`if_then_else`.
    
    Examples
    --------
        >>> from finstack.core.expr import Expr, Function, BinOp
        >>> # Column reference
        >>> x = Expr.column("x")
        >>> # Literal value
        >>> two = Expr.literal(2.0)
        >>> # Function call: rolling_mean(x, 3)
        >>> rolling = Expr.call(Function.ROLLING_MEAN, [x, Expr.literal(3.0)])
        >>> # Binary operation: x + 2
        >>> add = Expr.bin_op(BinOp.ADD, x, two)
        >>> # Conditional: if x > 0 then x else -x
        >>> abs_x = Expr.if_then_else(
        ...     Expr.bin_op(BinOp.GT, x, Expr.literal(0.0)),
        ...     x,
        ...     Expr.bin_op(BinOp.MUL, x, Expr.literal(-1.0))
        ... )
    """

    @staticmethod
    def column(name: str) -> "Expr":
        """Create a column reference expression.
        
        Parameters
        ----------
        name : str
            Column name to reference.
            
        Returns
        -------
        Expr
            Column reference expression.
        """
        ...

    @staticmethod
    def literal(value: float) -> "Expr":
        """Create a literal value expression.
        
        Parameters
        ----------
        value : float
            Constant value.
            
        Returns
        -------
        Expr
            Literal expression.
        """
        ...

    @staticmethod
    def call(func: Function, args: list["Expr"]) -> "Expr":
        """Create a function call expression.
        
        Parameters
        ----------
        func : Function
            Function to call.
        args : list[Expr]
            Arguments to the function.
            
        Returns
        -------
        Expr
            Function call expression.
        """
        ...

    @staticmethod
    def bin_op(op: BinOp, left: "Expr", right: "Expr") -> "Expr":
        """Create a binary operation expression.
        
        Parameters
        ----------
        op : BinOp
            Binary operator.
        left : Expr
            Left operand.
        right : Expr
            Right operand.
            
        Returns
        -------
        Expr
            Binary operation expression.
        """
        ...

    @staticmethod
    def unary_op(op: UnaryOp, operand: "Expr") -> "Expr":
        """Create a unary operation expression.
        
        Parameters
        ----------
        op : UnaryOp
            Unary operator.
        operand : Expr
            Operand.
            
        Returns
        -------
        Expr
            Unary operation expression.
        """
        ...

    @staticmethod
    def if_then_else(
        condition: "Expr", then_expr: "Expr", else_expr: "Expr"
    ) -> "Expr":
        """Create a conditional expression.
        
        Parameters
        ----------
        condition : Expr
            Boolean condition.
        then_expr : Expr
            Expression if condition is true.
        else_expr : Expr
            Expression if condition is false.
            
        Returns
        -------
        Expr
            Conditional expression.
        """
        ...

    def with_id(self, id: int) -> "Expr":
        """Return a copy of this expression with a specific ID.
        
        Parameters
        ----------
        id : int
            Expression ID for DAG deduplication.
            
        Returns
        -------
        Expr
            Expression with ID.
        """
        ...

    def __repr__(self) -> str: ...


class ExecutionPlan:
    """Execution plan from DAG analysis.
    
    Contains the optimized execution order and metadata for evaluating
    an expression tree with shared subexpression detection.
    """

    @property
    def roots(self) -> list[int]:
        """Root node IDs in the execution plan."""
        ...

    @property
    def node_count(self) -> int:
        """Total number of nodes in the plan."""
        ...

    @property
    def metadata(self) -> ResultsMeta:
        """Execution metadata."""
        ...

    def __repr__(self) -> str: ...


class EvalOpts:
    """Evaluation options for expression execution.
    
    Parameters
    ----------
    plan : ExecutionPlan, optional
        Pre-computed execution plan.
    cache_budget_mb : int, optional
        Memory budget for intermediate result caching (in MB).
    """

    def __init__(
        self,
        *,
        plan: Optional[ExecutionPlan] = None,
        cache_budget_mb: Optional[int] = None,
    ) -> None: ...

    @property
    def cache_budget_mb(self) -> Optional[int]:
        """Memory budget for caching in MB."""
        ...

    @cache_budget_mb.setter
    def cache_budget_mb(self, value: Optional[int]) -> None: ...

    @property
    def plan(self) -> Optional[ExecutionPlan]:
        """Pre-computed execution plan, if any."""
        ...


class EvaluationResult:
    """Result of expression evaluation.
    
    Contains the computed values and metadata about the evaluation.
    """

    @property
    def values(self) -> list[float]:
        """Computed values."""
        ...

    @property
    def metadata(self) -> ResultsMeta:
        """Execution metadata."""
        ...


class CompiledExpr:
    """Compiled expression ready for evaluation.
    
    Compilation performs DAG analysis and prepares the expression for
    efficient evaluation over tabular data.
    
    Parameters
    ----------
    expr : Expr
        Expression to compile.
        
    Examples
    --------
        >>> from finstack.core.expr import Expr, Function, CompiledExpr
        >>> expr = Expr.call(Function.ROLLING_MEAN, [Expr.column("x"), Expr.literal(3.0)])
        >>> compiled = CompiledExpr(expr)
        >>> result = compiled.eval(["x"], [[1.0, 2.0, 3.0, 4.0, 5.0]])
        >>> result.values  # Rolling mean with window 3
        [nan, nan, 2.0, 3.0, 4.0]
    """

    def __init__(self, expr: Expr) -> None: ...

    @classmethod
    def with_planning(cls, expr: Expr, results_meta: ResultsMeta) -> "CompiledExpr":
        """Create a compiled expression with explicit planning metadata.
        
        Parameters
        ----------
        expr : Expr
            Expression to compile.
        results_meta : ResultsMeta
            Metadata for result stamping.
            
        Returns
        -------
        CompiledExpr
            Compiled expression.
        """
        ...

    def with_cache(self, budget_mb: int) -> "CompiledExpr":
        """Return a copy with caching enabled at the given budget.
        
        Parameters
        ----------
        budget_mb : int
            Memory budget for caching in MB.
            
        Returns
        -------
        CompiledExpr
            Compiled expression with caching.
        """
        ...

    @property
    def plan(self) -> Optional[ExecutionPlan]:
        """Execution plan, if computed."""
        ...

    def eval(
        self,
        columns: list[str],
        data: list[list[float]],
        opts: Optional[EvalOpts] = None,
    ) -> EvaluationResult:
        """Evaluate the expression over tabular data.
        
        Parameters
        ----------
        columns : list[str]
            Column names corresponding to the data.
        data : list[list[float]]
            Data columns (each inner list is a column).
        opts : EvalOpts, optional
            Evaluation options.
            
        Returns
        -------
        EvaluationResult
            Evaluation result with values and metadata.
            
        Raises
        ------
        ValueError
            If columns and data have mismatched lengths or
            data columns have inconsistent lengths.
        """
        ...


__all__ = [
    "Function",
    "BinOp",
    "UnaryOp",
    "Expr",
    "ExecutionPlan",
    "EvalOpts",
    "CompiledExpr",
    "EvaluationResult",
]
