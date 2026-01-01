"""Expression Engine Examples - AST Construction and Evaluation

This file demonstrates the expression engine capabilities for building and
evaluating complex time-series expressions.

Features demonstrated:
1. Basic AST construction (columns, literals, operators)
2. Ergonomic helpers (operator overloading)
3. Time-series functions (lag, rolling windows, cumulative)
4. Financial metrics (revenue growth, margins, ratios)
5. Complex expressions (multi-factor signals, conditionals)
6. Expression compilation and evaluation

The expression engine provides a powerful way to define calculations once
and evaluate them efficiently across time-series data.
"""

import sys


def example_1_basic_ast():
    """Example 1: Basic AST Construction

    Demonstrates manual AST construction using static methods.
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 1: Basic AST Construction")
    print("=" * 70)

    from finstack.core.expr import Expr, BinOp, Function, CompiledExpr

    # Manual AST construction
    x = Expr.column("x")
    ten = Expr.literal(10.0)
    expr = Expr.bin_op(BinOp.ADD, x, ten)

    print(f"Expression: x + 10")
    print(f"AST: {repr(expr)}")

    # Compile and evaluate
    compiled = CompiledExpr(expr)
    columns = ["x"]
    data = [[1.0, 2.0, 3.0, 4.0, 5.0]]

    result = compiled.eval(columns, data)
    print(f"Input:  {data[0]}")
    print(f"Output: {result.values}")


def example_2_ergonomic_helpers():
    """Example 2: Ergonomic Helpers with Operator Overloading

    Demonstrates Pythonic expression construction using helpers.
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 2: Ergonomic Helpers with Operator Overloading")
    print("=" * 70)

    from finstack.core.expr import CompiledExpr
    from finstack.core.expr_helpers import col, lit

    # Instead of verbose static methods, use operator overloading
    expr = col("x") + 10

    print(f"Expression: col('x') + 10")
    print(f"AST: {repr(expr)}")

    # Complex expression
    expr2 = (col("revenue") * 1.1 - col("cogs")) / col("periods")

    print(f"\nComplex expression: (revenue * 1.1 - cogs) / periods")
    print(f"AST: {repr(expr2)}")

    # Compile and evaluate
    compiled = CompiledExpr(expr2)
    columns = ["revenue", "cogs", "periods"]
    data = [
        [1000.0, 2000.0, 3000.0],  # revenue
        [600.0, 1200.0, 1800.0],   # cogs
        [12.0, 12.0, 12.0],        # periods
    ]

    result = compiled.eval(columns, data)
    print(f"\nInput:")
    print(f"  revenue: {data[0]}")
    print(f"  cogs:    {data[1]}")
    print(f"  periods: {data[2]}")
    print(f"Output (monthly margin): {[round(v, 2) for v in result.values]}")


def example_3_time_series_functions():
    """Example 3: Time-Series Functions

    Demonstrates lag, diff, pct_change, and cumulative functions.
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 3: Time-Series Functions")
    print("=" * 70)

    from finstack.core.expr import CompiledExpr
    from finstack.core.expr_helpers import col, lag, diff, pct_change, cumsum

    data = [[100.0, 105.0, 110.0, 115.0, 120.0]]
    columns = ["price"]

    # Lag
    print("\nLag (shift by 1 period):")
    expr = lag(col("price"), 1)
    result = CompiledExpr(expr).eval(columns, data)
    print(f"  Input:  {data[0]}")
    print(f"  Output: {result.values}")

    # Diff
    print("\nDiff (price[t] - price[t-1]):")
    expr = diff(col("price"), 1)
    result = CompiledExpr(expr).eval(columns, data)
    print(f"  Input:  {data[0]}")
    print(f"  Output: {result.values}")

    # Percent change
    print("\nPercent Change:")
    expr = pct_change(col("price"), 1)
    result = CompiledExpr(expr).eval(columns, data)
    print(f"  Input:  {data[0]}")
    print(f"  Output: {[round(v * 100, 2) if v == v else None for v in result.values]}%")

    # Cumulative sum
    print("\nCumulative Sum:")
    data2 = [[10.0, 20.0, 30.0, 40.0, 50.0]]
    expr = cumsum(col("value"))
    result = CompiledExpr(expr).eval(["value"], data2)
    print(f"  Input:  {data2[0]}")
    print(f"  Output: {result.values}")


def example_4_rolling_windows():
    """Example 4: Rolling Window Functions

    Demonstrates rolling mean, sum, std, and other windowed operations.
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 4: Rolling Window Functions")
    print("=" * 70)

    from finstack.core.expr import CompiledExpr
    from finstack.core.expr_helpers import col, rolling_mean, rolling_sum, rolling_std

    data = [[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]]
    columns = ["x"]

    # Rolling mean
    print("\nRolling Mean (window=3):")
    expr = rolling_mean(col("x"), 3)
    result = CompiledExpr(expr).eval(columns, data)
    print(f"  Input:  {data[0]}")
    print(f"  Output: {[round(v, 2) if v == v else None for v in result.values]}")

    # Rolling sum
    print("\nRolling Sum (window=3):")
    expr = rolling_sum(col("x"), 3)
    result = CompiledExpr(expr).eval(columns, data)
    print(f"  Input:  {data[0]}")
    print(f"  Output: {[round(v, 2) if v == v else None for v in result.values]}")

    # Rolling std
    print("\nRolling Std (window=3):")
    expr = rolling_std(col("x"), 3)
    result = CompiledExpr(expr).eval(columns, data)
    print(f"  Input:  {data[0]}")
    print(f"  Output: {[round(v, 4) if v == v else None for v in result.values]}")


def example_5_financial_metrics():
    """Example 5: Financial Metrics

    Demonstrates common financial calculations using expressions.
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 5: Financial Metrics")
    print("=" * 70)

    from finstack.core.expr import CompiledExpr
    from finstack.core.expr_helpers import col, pct_change, rolling_mean

    # Sample data: quarterly revenue
    revenue = [1000.0, 1100.0, 1150.0, 1200.0, 1300.0, 1350.0]
    columns = ["revenue"]
    data = [revenue]

    # Revenue growth rate
    print("\nRevenue Growth Rate (QoQ):")
    expr = pct_change(col("revenue"), 1)
    result = CompiledExpr(expr).eval(columns, data)
    growth_rates = [round(v * 100, 2) if v == v else None for v in result.values]
    print(f"  Revenue:      {revenue}")
    print(f"  Growth Rates: {growth_rates}%")

    # Trailing 4-quarter average (TTM-like)
    print("\nTrailing 4-Quarter Average Revenue:")
    expr = rolling_mean(col("revenue"), 4)
    result = CompiledExpr(expr).eval(columns, data)
    trailing_avg = [round(v, 2) if v == v else None for v in result.values]
    print(f"  Revenue:      {revenue}")
    print(f"  Trailing Avg: {trailing_avg}")

    # Margin calculation
    print("\nGross Margin (Revenue - COGS) / Revenue:")
    cogs = [600.0, 640.0, 660.0, 700.0, 750.0, 780.0]
    data2 = [revenue, cogs]
    columns2 = ["revenue", "cogs"]

    expr = (col("revenue") - col("cogs")) / col("revenue")
    result = CompiledExpr(expr).eval(columns2, data2)
    margins = [round(v * 100, 2) for v in result.values]
    print(f"  Revenue: {revenue}")
    print(f"  COGS:    {cogs}")
    print(f"  Margin:  {margins}%")


def example_6_conditionals():
    """Example 6: Conditional Expressions

    Demonstrates if-then-else conditional logic.
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 6: Conditional Expressions")
    print("=" * 70)

    from finstack.core.expr import CompiledExpr
    from finstack.core.expr_helpers import col, if_then_else, pct_change

    # Simple conditional: if revenue > 1000 then revenue else 0
    print("\nSimple Conditional (threshold filter):")
    revenue = [800.0, 950.0, 1100.0, 1200.0, 900.0]
    expr = if_then_else(col("revenue") > 1000, col("revenue"), 0)

    result = CompiledExpr(expr).eval(["revenue"], [revenue])
    print(f"  Revenue: {revenue}")
    print(f"  Filtered (>1000): {result.values}")

    # Nested conditional: rating based on growth
    print("\nNested Conditional (rating based on growth):")
    # if growth > 0.1 then 'High' (3) elif growth > 0.05 then 'Medium' (2) else 'Low' (1)
    growth = [-0.05, 0.03, 0.07, 0.12, 0.08]
    expr = if_then_else(
        col("growth") > 0.1,
        3,  # High
        if_then_else(col("growth") > 0.05, 2, 1),  # Medium / Low
    )

    result = CompiledExpr(expr).eval(["growth"], [growth])
    ratings = ["Low", "Low", "Medium", "High", "Medium"]
    print(f"  Growth: {[f'{g*100:.1f}%' for g in growth]}")
    print(f"  Rating (numeric): {result.values}")
    print(f"  Rating (labels):  {[ratings[int(r)-1] for r in result.values]}")


def example_7_complex_expressions():
    """Example 7: Complex Multi-Factor Expressions

    Demonstrates building sophisticated multi-factor signals.
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 7: Complex Multi-Factor Expressions")
    print("=" * 70)

    from finstack.core.expr import CompiledExpr
    from finstack.core.expr_helpers import (
        col,
        pct_change,
        rolling_mean,
        rolling_std,
        if_then_else,
    )

    # Multi-factor signal: (momentum > 0) & (price > MA20) & (volatility < 0.02)
    price = [100.0, 102.0, 101.0, 103.0, 105.0, 104.0, 106.0, 108.0, 107.0, 110.0,
             112.0, 111.0, 113.0, 115.0, 114.0, 116.0, 118.0, 117.0, 119.0, 120.0,
             122.0]

    # Components
    momentum = pct_change(col("price"), 1)
    ma20 = rolling_mean(col("price"), 20)
    volatility = rolling_std(col("price"), 20) / col("price")

    # Signal: buy if momentum > 0 AND price > MA20 AND volatility < threshold
    signal = (momentum > 0) & (col("price") > ma20) & (volatility < 0.02)

    # Final expression: if signal then 1 else 0
    expr = if_then_else(signal, 1, 0)

    result = CompiledExpr(expr).eval(["price"], [price])
    signals = result.values

    print("\nMulti-Factor Trading Signal:")
    print(f"  Price:  {price[-5:]}")  # Last 5 values
    print(f"  Signal: {signals[-5:]}")  # Last 5 values
    print(f"  Total signals: {sum(s for s in signals if s == s)} / {len(signals)}")


def example_8_dag_planning():
    """Example 8: DAG Planning and Caching

    Demonstrates compilation with DAG planning for optimization.
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 8: DAG Planning and Caching")
    print("=" * 70)

    from finstack.core.expr import CompiledExpr
    from finstack.core.expr_helpers import col, rolling_mean
    from finstack.core.config import ResultsMeta

    # Build expression with shared sub-expressions
    # Both use rolling_mean(x, 3)
    rm = rolling_mean(col("x"), 3)
    expr = rm + rm * 2

    print(f"Expression: rolling_mean(x, 3) + rolling_mean(x, 3) * 2")

    # Compile with planning
    meta = ResultsMeta()
    compiled = CompiledExpr.with_planning(expr, meta)

    print(f"DAG planning enabled: {compiled.plan is not None}")
    if compiled.plan:
        print(f"  Nodes in plan: {compiled.plan.node_count}")
        print(f"  Root nodes: {compiled.plan.roots}")

    # Add caching
    compiled_with_cache = compiled.with_cache(100)  # 100 MB cache budget
    print(f"Cache budget set: 100 MB")

    # Evaluate
    data = [[1.0, 2.0, 3.0, 4.0, 5.0]]
    result = compiled.eval(["x"], data)
    print(f"\nInput:  {data[0]}")
    print(f"Output: {result.values}")


def main():
    """Run all examples."""
    print("\n" + "=" * 70)
    print("FINSTACK EXPRESSION ENGINE EXAMPLES")
    print("=" * 70)
    print("\nDemonstrating AST construction, compilation, and evaluation")
    print("for time-series expressions with financial applications.")

    try:
        example_1_basic_ast()
        example_2_ergonomic_helpers()
        example_3_time_series_functions()
        example_4_rolling_windows()
        example_5_financial_metrics()
        example_6_conditionals()
        example_7_complex_expressions()
        example_8_dag_planning()

        print("\n" + "=" * 70)
        print("All examples completed successfully!")
        print("=" * 70 + "\n")

    except Exception as e:
        print(f"\n❌ Error in examples: {e}", file=sys.stderr)
        import traceback

        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
