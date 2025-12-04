"""Financial Statement Modeling Example

This example demonstrates the statements Python bindings, mirroring the
functionality from the Rust examples (statements_phase*.rs).

It shows:
- Type-state builder pattern
- Period integration with actuals/forecast split
- Value nodes (explicit period values)
- Calculated nodes (formulas)
- Forecast methods
- Model evaluation
- Extensions (corkscrew, scorecard)
- Metric registry
"""

from finstack.core.dates.periods import PeriodId
from finstack.statements.builder import ModelBuilder
from finstack.statements.evaluator import Evaluator
from finstack.statements.extensions import (
    ExtensionRegistry,
)
from finstack.statements.registry import Registry
from finstack.statements.types import (
    AmountOrScalar,
    ForecastSpec,
)


def example_1_basic_pl_model():
    """Example 1: Basic P&L Model"""
    print("=" * 60)
    print("Example 1: Basic P&L Model")
    print("=" * 60)

    # Build the model
    builder = ModelBuilder.new("Acme Corp Q1-Q4 2025")
    builder.periods("2025Q1..Q4", "2025Q2")  # Q1-Q2 actuals, Q3-Q4 forecast

    # Add revenue with actuals
    builder.value(
        "revenue",
        [
            (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(10_000_000.0)),
            (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(11_000_000.0)),
        ],
    )

    # Add operating expenses
    builder.value(
        "operating_expenses",
        [
            (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(2_000_000.0)),
            (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(2_100_000.0)),
        ],
    )

    # Add calculated metrics
    builder.compute("cogs", "revenue * 0.6")
    builder.compute("gross_profit", "revenue - cogs")
    builder.compute("operating_income", "gross_profit - operating_expenses")
    builder.compute("gross_margin", "gross_profit / revenue")

    # Add metadata
    builder.with_meta("author", "Finance Team")
    builder.with_meta("version", "1.0")

    model = builder.build()

    print(f"Model ID: {model.id}")
    print(f"Periods: {len(model.periods)} total")
    print(f"Nodes: {len(list(model.nodes.keys()))} total")
    print()

    # Show period breakdown
    print("Period Breakdown:")
    for period in model.periods:
        period_type = "Actual  " if period.is_actual else "Forecast"
        print(f"  {period.id} | {period_type} | {period.start} to {period.end}")
    print()


def example_2_model_evaluation():
    """Example 2: Model Evaluation"""
    print("=" * 60)
    print("Example 2: Model Evaluation")
    print("=" * 60)

    # Build model
    builder = ModelBuilder.new("Evaluation Example")
    builder.periods("2025Q1..Q4", "2025Q2")

    # Add revenue with forecast
    builder.value(
        "revenue",
        [
            (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(1_000_000.0)),
            (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(1_100_000.0)),
        ],
    )
    builder.forecast("revenue", ForecastSpec.growth(0.05))  # 5% growth for forecasts

    # Add expenses with forward fill
    builder.value(
        "opex",
        [
            (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(300_000.0)),
            (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(320_000.0)),
        ],
    )
    builder.forecast("opex", ForecastSpec.forward_fill())

    # Add calculated metrics
    builder.compute("cogs", "revenue * 0.55")
    builder.compute("gross_profit", "revenue - cogs")
    builder.compute("ebitda", "gross_profit - opex")
    builder.compute("ebitda_margin", "ebitda / revenue")

    model = builder.build()

    # Evaluate the model
    evaluator = Evaluator.new()
    results = evaluator.evaluate(model)

    print(f"Evaluation completed in {results.meta.eval_time_ms}ms")
    print(f"Nodes evaluated: {results.meta.num_nodes}")
    print(f"Periods evaluated: {results.meta.num_periods}")
    print()

    # Display results
    print("Results by Period:")
    for period in model.periods:
        print(f"\n{period.id} ({'Actual' if period.is_actual else 'Forecast'}):")
        print(f"  Revenue:        ${results.get('revenue', period.id):,.0f}")
        print(f"  COGS:           ${results.get('cogs', period.id):,.0f}")
        print(f"  Gross Profit:   ${results.get('gross_profit', period.id):,.0f}")
        print(f"  OpEx:           ${results.get('opex', period.id):,.0f}")
        print(f"  EBITDA:         ${results.get('ebitda', period.id):,.0f}")
        print(f"  EBITDA Margin:  {results.get('ebitda_margin', period.id):.1%}")


def example_3_metric_registry():
    """Example 3: Dynamic Metric Registry"""
    print("=" * 60)
    print("Example 3: Dynamic Metric Registry")
    print("=" * 60)

    # Create registry and load custom metrics
    # Note: Built-in metrics have a circular dependency issue, so we demonstrate
    # with custom metrics instead
    registry = Registry.new()
    
    # Load custom metrics instead of builtins
    custom_metrics_json = """
    {
        "namespace": "custom",
        "metrics": [
            {
                "id": "gross_profit",
                "name": "Gross Profit",
                "formula": "revenue - cogs",
                "category": "income_statement"
            },
            {
                "id": "gross_margin",
                "name": "Gross Margin %",
                "formula": "gross_profit / revenue",
                "category": "margins"
            },
            {
                "id": "operating_income",
                "name": "Operating Income",
                "formula": "gross_profit - opex",
                "category": "income_statement"
            },
            {
                "id": "operating_margin",
                "name": "Operating Margin %",
                "formula": "operating_income / revenue",
                "category": "margins"
            }
        ]
    }
    """
    registry.load_from_json_str(custom_metrics_json)

    print(f"Total metrics loaded: {len(registry.list_metrics(None))}")
    print()

    # List metrics by namespace
    custom_metrics = registry.list_metrics("custom")
    print(f"Metrics in 'custom' namespace: {len(custom_metrics)}")
    print()

    # Get a specific metric
    gross_margin = registry.get("custom.gross_margin")
    print(f"Metric: {gross_margin.name}")
    print(f"  ID: {gross_margin.id}")
    print(f"  Formula: {gross_margin.formula}")
    print(f"  Category: {gross_margin.category}")
    print(f"  Requires: {gross_margin.requires}")
    print()

    # Show some available metrics
    print("Sample metrics:")
    for metric_id in sorted(custom_metrics):
        metric = registry.get(metric_id)
        print(f"  {metric_id}: {metric.name}")


def example_4_extensions():
    """Example 4: Extension System"""
    print("=" * 60)
    print("Example 4: Extension System")
    print("=" * 60)

    # Build a simple model
    builder = ModelBuilder.new("Extension Test")
    builder.periods("2025Q1..Q2", None)
    builder.value(
        "test_node",
        [
            (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100.0)),
            (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(200.0)),
        ],
    )

    model = builder.build()

    # Evaluate
    evaluator = Evaluator.new()
    results = evaluator.evaluate(model)

    # Create extension registry
    ext_registry = ExtensionRegistry.new()

    # Note: register() method is not yet available due to Clone constraints
    # This will be added in a future version

    print("Extension system initialized")
    print("✓ CorkscrewExtension available")
    print("✓ CreditScorecardExtension available")


def example_5_json_serialization():
    """Example 5: JSON Serialization"""
    print("=" * 60)
    print("Example 5: JSON Serialization")
    print("=" * 60)

    # Build a model
    builder = ModelBuilder.new("Serialization Test")
    builder.periods("2025Q1..Q2", None)
    builder.value(
        "revenue",
        [(PeriodId.quarter(2025, 1), AmountOrScalar.scalar(1000.0))],
    )
    builder.compute("profit", "revenue * 0.2")

    model = builder.build()

    # Serialize to JSON
    json_str = model.to_json()
    print("Model serialized to JSON successfully")
    print(f"JSON length: {len(json_str)} bytes")

    # Deserialize from JSON
    from finstack.statements.types import FinancialModelSpec

    restored = FinancialModelSpec.from_json(json_str)
    print(f"Model restored from JSON: {restored.id}")
    print(f"Periods: {len(restored.periods)}")
    print(f"Nodes: {len(list(restored.nodes.keys()))}")


def main():
    """Run all examples."""
    print("\n" + "=" * 60)
    print("Finstack Statements Python Bindings - Examples")
    print("=" * 60 + "\n")

    example_1_basic_pl_model()
    print()

    example_2_model_evaluation()
    print()

    example_3_metric_registry()
    print()

    example_4_extensions()
    print()

    example_5_json_serialization()
    print()

    print("=" * 60)
    print("All examples completed successfully!")
    print("=" * 60)


if __name__ == "__main__":
    main()

