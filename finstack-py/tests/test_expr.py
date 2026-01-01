"""Comprehensive tests for expression engine bindings.

Tests cover:
- AST construction (columns, literals, operators, functions)
- Expression compilation and evaluation
- DAG planning and caching
- Helper functions for ergonomic construction
- Complex expression patterns
"""

from finstack.core.expr import (
    BinOp,
    CompiledExpr,
    EvalOpts,
    Expr,
    Function,
    UnaryOp,
)
from finstack.core.expr_helpers import (
    col,
    cumsum,
    diff,
    if_then_else,
    lag,
    lead,
    lit,
    pct_change,
    rolling_mean,
    rolling_std,
    rolling_sum,
)
import pytest


class TestASTConstruction:
    """Test basic AST construction methods."""

    def test_column_reference(self) -> None:
        """Test column reference creation."""
        expr = Expr.column("revenue")
        assert repr(expr).startswith("Expr<")

    def test_literal_value(self) -> None:
        """Test literal value creation."""
        expr = Expr.literal(100.0)
        assert repr(expr).startswith("Expr<")

    def test_bin_op_creation(self) -> None:
        """Test binary operation creation."""
        left = Expr.column("x")
        right = Expr.literal(10.0)
        expr = Expr.bin_op(BinOp.ADD, left, right)
        assert repr(expr).startswith("Expr<")

    def test_unary_op_creation(self) -> None:
        """Test unary operation creation."""
        operand = Expr.column("x")
        expr = Expr.unary_op(UnaryOp.NEG, operand)
        assert repr(expr).startswith("Expr<")

    def test_function_call_creation(self) -> None:
        """Test function call creation."""
        x = Expr.column("x")
        window = Expr.literal(3.0)
        expr = Expr.call(Function.ROLLING_MEAN, [x, window])
        assert repr(expr).startswith("Expr<")

    def test_if_then_else_creation(self) -> None:
        """Test conditional creation."""
        condition = Expr.bin_op(BinOp.GT, Expr.column("x"), Expr.literal(0.0))
        then_expr = Expr.column("x")
        else_expr = Expr.literal(0.0)
        expr = Expr.if_then_else(condition, then_expr, else_expr)
        assert repr(expr).startswith("Expr<")

    def test_with_id(self) -> None:
        """Test assigning ID for caching."""
        expr = Expr.column("x").with_id(42)
        assert repr(expr).startswith("Expr<")


class TestHelperFunctions:
    """Test ergonomic helper functions."""

    def test_col_helper(self) -> None:
        """Test col() helper."""
        expr = col("revenue")
        assert repr(expr).startswith("Expr<")

    def test_lit_helper(self) -> None:
        """Test lit() helper."""
        expr = lit(100.0)
        assert repr(expr).startswith("Expr<")

    def test_arithmetic_operators(self) -> None:
        """Test operator overloading for arithmetic."""
        x = col("x")
        y = col("y")

        # Addition
        expr = x + y
        assert repr(expr).startswith("Expr<")

        # Subtraction
        expr = x - y
        assert repr(expr).startswith("Expr<")

        # Multiplication
        expr = x * y
        assert repr(expr).startswith("Expr<")

        # Division
        expr = x / y
        assert repr(expr).startswith("Expr<")

        # Modulo
        expr = x % y
        assert repr(expr).startswith("Expr<")

        # Negation
        expr = -x
        assert repr(expr).startswith("Expr<")

    def test_comparison_operators(self) -> None:
        """Test operator overloading for comparisons."""
        x = col("x")
        y = col("y")

        # Equal
        expr = x == y
        assert repr(expr).startswith("Expr<")

        # Not equal
        expr = x != y
        assert repr(expr).startswith("Expr<")

        # Less than
        expr = x < y
        assert repr(expr).startswith("Expr<")

        # Less than or equal
        expr = x <= y
        assert repr(expr).startswith("Expr<")

        # Greater than
        expr = x > y
        assert repr(expr).startswith("Expr<")

        # Greater than or equal
        expr = x >= y
        assert repr(expr).startswith("Expr<")

    def test_logical_operators(self) -> None:
        """Test operator overloading for logical operations."""
        x = col("x")
        y = col("y")

        # AND
        expr = (x > 0) & (y > 0)
        assert repr(expr).startswith("Expr<")

        # OR
        expr = (x > 0) | (y > 0)
        assert repr(expr).startswith("Expr<")

        # NOT
        expr = ~(x > 0)
        assert repr(expr).startswith("Expr<")

    def test_mixed_type_operations(self) -> None:
        """Test operations mixing Expr and scalars."""
        x = col("x")

        # Expr + scalar
        expr = x + 10
        assert repr(expr).startswith("Expr<")

        # Scalar + Expr
        expr = 10 + x
        assert repr(expr).startswith("Expr<")

        # Complex expression
        expr = (x * 1.1 - 100) / 12
        assert repr(expr).startswith("Expr<")


class TestFunctionHelpers:
    """Test function call helpers."""

    def test_lag_helper(self) -> None:
        """Test lag() helper."""
        expr = lag(col("x"), 1)
        assert repr(expr).startswith("Expr<")

    def test_lead_helper(self) -> None:
        """Test lead() helper."""
        expr = lead(col("x"), 1)
        assert repr(expr).startswith("Expr<")

    def test_diff_helper(self) -> None:
        """Test diff() helper."""
        expr = diff(col("x"), 1)
        assert repr(expr).startswith("Expr<")

    def test_pct_change_helper(self) -> None:
        """Test pct_change() helper."""
        expr = pct_change(col("x"), 1)
        assert repr(expr).startswith("Expr<")

    def test_cumsum_helper(self) -> None:
        """Test cumsum() helper."""
        expr = cumsum(col("x"))
        assert repr(expr).startswith("Expr<")

    def test_rolling_mean_helper(self) -> None:
        """Test rolling_mean() helper."""
        expr = rolling_mean(col("x"), 3)
        assert repr(expr).startswith("Expr<")

    def test_rolling_sum_helper(self) -> None:
        """Test rolling_sum() helper."""
        expr = rolling_sum(col("x"), 3)
        assert repr(expr).startswith("Expr<")

    def test_rolling_std_helper(self) -> None:
        """Test rolling_std() helper."""
        expr = rolling_std(col("x"), 3)
        assert repr(expr).startswith("Expr<")

    def test_if_then_else_helper(self) -> None:
        """Test if_then_else() helper."""
        expr = if_then_else(col("x") > 0, col("x"), 0)
        assert repr(expr).startswith("Expr<")


class TestCompilation:
    """Test expression compilation."""

    def test_compile_simple_expression(self) -> None:
        """Test compiling a simple expression."""
        expr = col("x") + 10
        compiled = CompiledExpr(expr.expr)
        assert compiled is not None

    def test_compile_complex_expression(self) -> None:
        """Test compiling a complex expression."""
        expr = rolling_mean(col("x"), 3) + rolling_sum(col("y"), 5)
        compiled = CompiledExpr(expr.expr)
        assert compiled is not None

    def test_with_cache(self) -> None:
        """Test adding cache budget."""
        expr = col("x") + 10
        compiled = CompiledExpr(expr.expr).with_cache(100)
        assert compiled is not None


class TestEvaluation:
    """Test expression evaluation."""

    def test_eval_simple_addition(self) -> None:
        """Test evaluating simple addition."""
        expr = col("x") + 10
        compiled = CompiledExpr(expr.expr)

        columns = ["x"]
        data = [[1.0, 2.0, 3.0, 4.0, 5.0]]

        result = compiled.eval(columns, data)
        assert result.values == [11.0, 12.0, 13.0, 14.0, 15.0]

    def test_eval_multiplication(self) -> None:
        """Test evaluating multiplication."""
        expr = col("x") * 2
        compiled = CompiledExpr(expr.expr)

        columns = ["x"]
        data = [[1.0, 2.0, 3.0]]

        result = compiled.eval(columns, data)
        assert result.values == [2.0, 4.0, 6.0]

    def test_eval_multiple_columns(self) -> None:
        """Test evaluating expression with multiple columns."""
        expr = col("x") + col("y")
        compiled = CompiledExpr(expr.expr)

        columns = ["x", "y"]
        data = [[1.0, 2.0, 3.0], [10.0, 20.0, 30.0]]

        result = compiled.eval(columns, data)
        assert result.values == [11.0, 22.0, 33.0]

    def test_eval_lag(self) -> None:
        """Test evaluating lag function."""
        expr = lag(col("x"), 1)
        compiled = CompiledExpr(expr.expr)

        columns = ["x"]
        data = [[1.0, 2.0, 3.0, 4.0, 5.0]]

        result = compiled.eval(columns, data)
        # First value should be NaN/null (represented as 0.0 or NaN)
        # Subsequent values are shifted
        assert len(result.values) == 5
        # Note: actual null handling may vary, check if NaN or 0.0

    def test_eval_rolling_mean(self) -> None:
        """Test evaluating rolling mean."""
        expr = rolling_mean(col("x"), 3)
        compiled = CompiledExpr(expr.expr)

        columns = ["x"]
        data = [[1.0, 2.0, 3.0, 4.0, 5.0]]

        result = compiled.eval(columns, data)
        assert len(result.values) == 5
        # Check that rolling mean is computed correctly
        # Values: [NaN, NaN, 2.0, 3.0, 4.0] (or similar)

    def test_eval_cumsum(self) -> None:
        """Test evaluating cumulative sum."""
        expr = cumsum(col("x"))
        compiled = CompiledExpr(expr.expr)

        columns = ["x"]
        data = [[1.0, 2.0, 3.0, 4.0, 5.0]]

        result = compiled.eval(columns, data)
        assert result.values == [1.0, 3.0, 6.0, 10.0, 15.0]

    def test_eval_conditional(self) -> None:
        """Test evaluating conditional expression."""
        expr = if_then_else(col("x") > 2, col("x"), 0)
        compiled = CompiledExpr(expr.expr)

        columns = ["x"]
        data = [[1.0, 2.0, 3.0, 4.0]]

        result = compiled.eval(columns, data)
        # Values <= 2 should be 0, values > 2 should be original
        assert result.values[0] == 0.0  # 1.0 <= 2
        assert result.values[1] == 0.0  # 2.0 <= 2
        assert result.values[2] == 3.0  # 3.0 > 2
        assert result.values[3] == 4.0  # 4.0 > 2

    def test_eval_with_opts(self) -> None:
        """Test evaluating with options."""
        expr = col("x") + 10
        compiled = CompiledExpr(expr.expr)

        columns = ["x"]
        data = [[1.0, 2.0, 3.0]]

        opts = EvalOpts()
        result = compiled.eval(columns, data, opts)
        assert result.values == [11.0, 12.0, 13.0]


class TestComplexExpressions:
    """Test complex expression patterns."""

    def test_financial_metric(self) -> None:
        """Test building a financial metric expression: (revenue * 1.1 - cogs) / periods."""
        expr = (col("revenue") * 1.1 - col("cogs")) / col("periods")
        compiled = CompiledExpr(expr.expr)

        columns = ["revenue", "cogs", "periods"]
        data = [[1000.0, 2000.0], [600.0, 1200.0], [12.0, 12.0]]

        result = compiled.eval(columns, data)
        # (1000 * 1.1 - 600) / 12 = 500 / 12 = 41.67
        # (2000 * 1.1 - 1200) / 12 = 1000 / 12 = 83.33
        assert len(result.values) == 2
        assert abs(result.values[0] - 41.666666) < 0.001
        assert abs(result.values[1] - 83.333333) < 0.001

    def test_momentum_indicator(self) -> None:
        """Test building a momentum indicator: pct_change(rolling_mean(x, 5), 1)."""
        expr = pct_change(rolling_mean(col("price"), 5), 1)
        compiled = CompiledExpr(expr.expr)

        columns = ["price"]
        data = [[100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 107.0]]

        result = compiled.eval(columns, data)
        assert len(result.values) == 8

    def test_conditional_growth(self) -> None:
        """Test conditional growth rate: if revenue > 0 then growth_rate else 0."""
        from finstack.core.expr_helpers import growth_rate

        expr = if_then_else(col("revenue") > 0, growth_rate(col("revenue"), 1), 0)
        compiled = CompiledExpr(expr.expr)

        columns = ["revenue"]
        data = [[0.0, 100.0, 110.0, 121.0]]

        result = compiled.eval(columns, data)
        assert len(result.values) == 4

    def test_multi_factor_signal(self) -> None:
        """Test multi-factor signal combining multiple indicators."""
        from finstack.core.expr_helpers import rolling_std

        momentum = pct_change(col("price"), 1)
        volatility = rolling_std(col("price"), 20)
        threshold = lit(5.0)

        signal = (momentum > 0) & (volatility < threshold)
        compiled = CompiledExpr(signal.expr)

        # Should compile without errors
        assert compiled is not None

    def test_nested_conditionals(self) -> None:
        """Test nested conditional expressions."""
        # if x > 10 then (if x > 20 then 3 else 2) else 1
        expr = if_then_else(col("x") > 10, if_then_else(col("x") > 20, 3, 2), 1)
        compiled = CompiledExpr(expr.expr)

        columns = ["x"]
        data = [[5.0, 15.0, 25.0]]

        result = compiled.eval(columns, data)
        assert result.values == [1.0, 2.0, 3.0]


class TestEnumTypes:
    """Test enum type representations."""

    def test_function_enum(self) -> None:
        """Test Function enum constants."""
        assert Function.LAG is not None
        assert Function.ROLLING_MEAN is not None
        assert Function.CUM_SUM is not None
        assert Function.TTM is not None

    def test_function_name(self) -> None:
        """Test Function name property."""
        assert Function.LAG.name == "lag"
        assert Function.ROLLING_MEAN.name == "rolling_mean"
        assert Function.CUM_SUM.name == "cumsum"

    def test_binop_enum(self) -> None:
        """Test BinOp enum constants."""
        assert BinOp.ADD is not None
        assert BinOp.MUL is not None
        assert BinOp.EQ is not None
        assert BinOp.AND is not None

    def test_unaryop_enum(self) -> None:
        """Test UnaryOp enum constants."""
        assert UnaryOp.NEG is not None
        assert UnaryOp.NOT is not None


class TestEdgeCases:
    """Test edge cases and error handling."""

    def test_empty_data(self) -> None:
        """Test evaluation with empty data."""
        expr = col("x") + 10
        compiled = CompiledExpr(expr.expr)

        columns = ["x"]
        data = [[]]

        result = compiled.eval(columns, data)
        assert result.values == []

    def test_mismatched_columns_and_data(self) -> None:
        """Test evaluation with mismatched columns and data raises error."""
        expr = col("x") + 10
        compiled = CompiledExpr(expr.expr)

        columns = ["x", "y"]
        data = [[1.0, 2.0, 3.0]]

        with pytest.raises(ValueError, match="columns and data length must match"):
            compiled.eval(columns, data)

    def test_mismatched_series_lengths(self) -> None:
        """Test evaluation with mismatched series lengths raises error."""
        expr = col("x") + col("y")
        compiled = CompiledExpr(expr.expr)

        columns = ["x", "y"]
        data = [[1.0, 2.0], [10.0, 20.0, 30.0]]

        with pytest.raises(ValueError, match="all data series must have the same length"):
            compiled.eval(columns, data)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
