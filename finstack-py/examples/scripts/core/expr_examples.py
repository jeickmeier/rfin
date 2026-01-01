"""Expression Engine Examples - AST Construction and Evaluation.

This file demonstrates the expression engine capabilities for building and
evaluating complex time-series expressions.

Features demonstrated:
1. Basic AST construction (columns, literals, operators)
2. Binary operations (arithmetic, comparison, logical)
3. Time-series functions (lag, rolling windows, cumulative)
4. Financial metrics (margins, ratios)
5. Conditional expressions (if-then-else)
6. DAG planning and caching

The expression engine provides a powerful way to define calculations once
and evaluate them efficiently across time-series data.
"""

import sys


def example_1_basic_ast():
    """Example 1: Basic AST Construction.

    Demonstrates manual AST construction using static methods.
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 1: Basic AST Construction")
    print("=" * 70)

    from finstack.core.expr import BinOp, CompiledExpr, Expr, Function

    # Manual AST construction
    x = Expr.column("x")
    ten = Expr.literal(10.0)
    expr = Expr.bin_op(BinOp.ADD, x, ten)

    print("Expression: x + 10")
    print(f"AST: {expr!r}")

    # Compile and evaluate
    compiled = CompiledExpr(expr)
    columns = ["x"]
    data = [[1.0, 2.0, 3.0, 4.0, 5.0]]

    result = compiled.eval(columns, data)
    print(f"Input:  {data[0]}")
    print(f"Output: {result.values}")


def example_2_complex_arithmetic():
    """Example 2: Complex Arithmetic Expressions.

    Demonstrates building multi-operation expressions.
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 2: Complex Arithmetic Expressions")
    print("=" * 70)

    from finstack.core.expr import BinOp, CompiledExpr, Expr

    # Build expression: (revenue * 1.1 - cogs) / periods
    revenue = Expr.column("revenue")
    cogs = Expr.column("cogs")
    periods = Expr.column("periods")

    # revenue * 1.1
    revenue_scaled = Expr.bin_op(BinOp.MUL, revenue, Expr.literal(1.1))
    # revenue * 1.1 - cogs
    margin = Expr.bin_op(BinOp.SUB, revenue_scaled, cogs)
    # (revenue * 1.1 - cogs) / periods
    expr = Expr.bin_op(BinOp.DIV, margin, periods)

    print("Expression: (revenue * 1.1 - cogs) / periods")
    print(f"AST: {expr!r}")

    # Compile and evaluate
    compiled = CompiledExpr(expr)
    columns = ["revenue", "cogs", "periods"]
    data = [
        [1000.0, 2000.0, 3000.0],  # revenue
        [600.0, 1200.0, 1800.0],  # cogs
        [12.0, 12.0, 12.0],  # periods
    ]

    result = compiled.eval(columns, data)
    print("\nInput:")
    print(f"  revenue: {data[0]}")
    print(f"  cogs:    {data[1]}")
    print(f"  periods: {data[2]}")
    print(f"Output (monthly margin): {[round(v, 2) for v in result.values]}")


def example_3_time_series_functions():
    """Example 3: Time-Series Functions.

    Demonstrates lag, diff, pct_change, and cumulative functions.
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 3: Time-Series Functions")
    print("=" * 70)

    from finstack.core.expr import CompiledExpr, Expr, Function

    data = [[100.0, 105.0, 110.0, 115.0, 120.0]]
    columns = ["price"]
    price = Expr.column("price")

    # Lag - shift by 1 period
    print("\nLag (shift by 1 period):")
    lag_expr = Expr.call(Function.LAG, [price, Expr.literal(1.0)])
    result = CompiledExpr(lag_expr).eval(columns, data)
    print(f"  Input:  {data[0]}")
    print(f"  Output: {result.values}")

    # Diff - difference from previous period
    print("\nDiff (price[t] - price[t-1]):")
    diff_expr = Expr.call(Function.DIFF, [price, Expr.literal(1.0)])
    result = CompiledExpr(diff_expr).eval(columns, data)
    print(f"  Input:  {data[0]}")
    print(f"  Output: {result.values}")

    # Percent change
    print("\nPercent Change:")
    pct_expr = Expr.call(Function.PCT_CHANGE, [price, Expr.literal(1.0)])
    result = CompiledExpr(pct_expr).eval(columns, data)
    print(f"  Input:  {data[0]}")
    print(f"  Output: {[round(v * 100, 2) if v == v else None for v in result.values]}%")

    # Cumulative sum
    print("\nCumulative Sum:")
    data2 = [[10.0, 20.0, 30.0, 40.0, 50.0]]
    value = Expr.column("value")
    cumsum_expr = Expr.call(Function.CUM_SUM, [value])
    result = CompiledExpr(cumsum_expr).eval(["value"], data2)
    print(f"  Input:  {data2[0]}")
    print(f"  Output: {result.values}")


def example_4_rolling_windows():
    """Example 4: Rolling Window Functions.

    Demonstrates rolling mean, sum, std, and other windowed operations.
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 4: Rolling Window Functions")
    print("=" * 70)

    from finstack.core.expr import CompiledExpr, Expr, Function

    data = [[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]]
    columns = ["x"]
    x = Expr.column("x")
    window = Expr.literal(3.0)

    # Rolling mean
    print("\nRolling Mean (window=3):")
    rolling_mean = Expr.call(Function.ROLLING_MEAN, [x, window])
    result = CompiledExpr(rolling_mean).eval(columns, data)
    print(f"  Input:  {data[0]}")
    print(f"  Output: {[round(v, 2) if v == v else None for v in result.values]}")

    # Rolling sum
    print("\nRolling Sum (window=3):")
    rolling_sum = Expr.call(Function.ROLLING_SUM, [x, window])
    result = CompiledExpr(rolling_sum).eval(columns, data)
    print(f"  Input:  {data[0]}")
    print(f"  Output: {[round(v, 2) if v == v else None for v in result.values]}")

    # Rolling std
    print("\nRolling Std (window=3):")
    rolling_std = Expr.call(Function.ROLLING_STD, [x, window])
    result = CompiledExpr(rolling_std).eval(columns, data)
    print(f"  Input:  {data[0]}")
    print(f"  Output: {[round(v, 4) if v == v else None for v in result.values]}")


def example_5_financial_metrics():
    """Example 5: Financial Metrics.

    Demonstrates common financial calculations using expressions.
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 5: Financial Metrics")
    print("=" * 70)

    from finstack.core.expr import BinOp, CompiledExpr, Expr, Function

    # Sample data: quarterly revenue
    revenue_data = [1000.0, 1100.0, 1150.0, 1200.0, 1300.0, 1350.0]
    columns = ["revenue"]
    data = [revenue_data]
    revenue = Expr.column("revenue")

    # Revenue growth rate (using pct_change)
    print("\nRevenue Growth Rate (QoQ):")
    growth_expr = Expr.call(Function.PCT_CHANGE, [revenue, Expr.literal(1.0)])
    result = CompiledExpr(growth_expr).eval(columns, data)
    growth_rates = [round(v * 100, 2) if v == v else None for v in result.values]
    print(f"  Revenue:      {revenue_data}")
    print(f"  Growth Rates: {growth_rates}%")

    # Trailing 4-quarter average (TTM-like)
    print("\nTrailing 4-Quarter Average Revenue:")
    trailing_expr = Expr.call(Function.ROLLING_MEAN, [revenue, Expr.literal(4.0)])
    result = CompiledExpr(trailing_expr).eval(columns, data)
    trailing_avg = [round(v, 2) if v == v else None for v in result.values]
    print(f"  Revenue:      {revenue_data}")
    print(f"  Trailing Avg: {trailing_avg}")

    # Margin calculation: (Revenue - COGS) / Revenue
    print("\nGross Margin (Revenue - COGS) / Revenue:")
    cogs_data = [600.0, 640.0, 660.0, 700.0, 750.0, 780.0]
    data2 = [revenue_data, cogs_data]
    columns2 = ["revenue", "cogs"]
    cogs = Expr.column("cogs")

    # (revenue - cogs) / revenue
    diff = Expr.bin_op(BinOp.SUB, revenue, cogs)
    margin_expr = Expr.bin_op(BinOp.DIV, diff, revenue)
    result = CompiledExpr(margin_expr).eval(columns2, data2)
    margins = [round(v * 100, 2) for v in result.values]
    print(f"  Revenue: {revenue_data}")
    print(f"  COGS:    {cogs_data}")
    print(f"  Margin:  {margins}%")


def example_6_conditionals():
    """Example 6: Conditional Expressions.

    Demonstrates if-then-else conditional logic.
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 6: Conditional Expressions")
    print("=" * 70)

    from finstack.core.expr import BinOp, CompiledExpr, Expr

    # Simple conditional: if revenue > 1000 then revenue else 0
    print("\nSimple Conditional (threshold filter):")
    revenue_data = [800.0, 950.0, 1100.0, 1200.0, 900.0]
    revenue = Expr.column("revenue")
    threshold = Expr.literal(1000.0)
    zero = Expr.literal(0.0)

    # revenue > 1000
    condition = Expr.bin_op(BinOp.GT, revenue, threshold)
    expr = Expr.if_then_else(condition, revenue, zero)

    result = CompiledExpr(expr).eval(["revenue"], [revenue_data])
    print(f"  Revenue: {revenue_data}")
    print(f"  Filtered (>1000): {result.values}")

    # Nested conditional: rating based on growth
    print("\nNested Conditional (rating based on growth):")
    # if growth > 0.1 then 3 (High) elif growth > 0.05 then 2 (Medium) else 1 (Low)
    growth_data = [-0.05, 0.03, 0.07, 0.12, 0.08]
    growth = Expr.column("growth")

    high_threshold = Expr.literal(0.1)
    med_threshold = Expr.literal(0.05)
    high_rating = Expr.literal(3.0)
    med_rating = Expr.literal(2.0)
    low_rating = Expr.literal(1.0)

    # growth > 0.05 ? 2 : 1
    inner = Expr.if_then_else(
        Expr.bin_op(BinOp.GT, growth, med_threshold),
        med_rating,
        low_rating,
    )
    # growth > 0.1 ? 3 : inner
    expr = Expr.if_then_else(
        Expr.bin_op(BinOp.GT, growth, high_threshold),
        high_rating,
        inner,
    )

    result = CompiledExpr(expr).eval(["growth"], [growth_data])
    rating_labels = ["Low", "Low", "Medium", "High", "Medium"]
    print(f"  Growth: {[f'{g * 100:.1f}%' for g in growth_data]}")
    print(f"  Rating (numeric): {result.values}")
    print(f"  Rating (labels):  {[rating_labels[int(r) - 1] for r in result.values]}")


def example_7_dag_planning():
    """Example 7: DAG Planning and Caching.

    Demonstrates compilation with caching for optimization.
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 7: DAG Planning and Caching")
    print("=" * 70)

    from finstack.core.expr import BinOp, CompiledExpr, Expr, Function

    x = Expr.column("x")
    window = Expr.literal(3.0)

    # Build expression with shared sub-expressions
    # Both use rolling_mean(x, 3)
    rm = Expr.call(Function.ROLLING_MEAN, [x, window])
    # rolling_mean(x, 3) * 2
    rm_times_2 = Expr.bin_op(BinOp.MUL, rm, Expr.literal(2.0))
    # rolling_mean(x, 3) + rolling_mean(x, 3) * 2
    expr = Expr.bin_op(BinOp.ADD, rm, rm_times_2)

    print("Expression: rolling_mean(x, 3) + rolling_mean(x, 3) * 2")

    # Compile expression
    compiled = CompiledExpr(expr)

    print(f"DAG planning enabled: {compiled.plan is not None}")
    if compiled.plan:
        print(f"  Nodes in plan: {compiled.plan.node_count}")
        print(f"  Root nodes: {compiled.plan.roots}")

    # Add caching
    compiled.with_cache(100)  # 100 MB cache budget
    print("Cache budget set: 100 MB")

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
        example_2_complex_arithmetic()
        example_3_time_series_functions()
        example_4_rolling_windows()
        example_5_financial_metrics()
        example_6_conditionals()
        example_7_dag_planning()

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
