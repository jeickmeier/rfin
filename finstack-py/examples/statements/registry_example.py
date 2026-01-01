"""
Dynamic Metric Registry Example

Demonstrates how to use the metric registry to define, load, and reuse
financial metrics across statement models.

Key Features:
1. Built-in metrics (fin.* namespace) with standard financial calculations
2. Custom metric definitions via JSON
3. Batch loading of multiple metrics
4. Inter-metric dependencies (metrics referencing other metrics)
5. Model evaluation with registry metrics
"""

import json
import tempfile
from pathlib import Path

from finstack.statements.builder import ModelBuilder
from finstack.statements.evaluator import Evaluator
from finstack.statements.registry import (
    MetricDefinition,
    MetricRegistry,
    Registry,
    UnitType,
)
from finstack.statements.types import Period


def example_1_builtin_metrics():
    """Example 1: Using built-in financial metrics."""
    print("\n" + "=" * 80)
    print("Example 1: Built-in Financial Metrics (fin.* namespace)")
    print("=" * 80)

    # Create registry and load built-ins
    registry = Registry.new()
    registry.load_builtins()

    # List available metrics
    fin_metrics = registry.list_metrics("fin")
    print(f"\nLoaded {len(fin_metrics)} built-in metrics:")
    for metric_id in sorted(fin_metrics[:10]):  # Show first 10
        metric = registry.get(metric_id)
        print(f"  {metric_id:30s} - {metric.name}")
    if len(fin_metrics) > 10:
        print(f"  ... and {len(fin_metrics) - 10} more")

    # Inspect a specific metric
    print("\nDetailed view of fin.gross_margin:")
    gross_margin = registry.get("fin.gross_margin")
    print(f"  ID:          {gross_margin.id}")
    print(f"  Name:        {gross_margin.name}")
    print(f"  Formula:     {gross_margin.formula}")
    print(f"  Description: {gross_margin.description}")
    print(f"  Category:    {gross_margin.category}")
    print(f"  Unit Type:   {gross_margin.unit_type}")
    print(f"  Requires:    {gross_margin.requires}")


def example_2_custom_json_metrics():
    """Example 2: Loading custom metrics from JSON."""
    print("\n" + "=" * 80)
    print("Example 2: Custom Metrics from JSON")
    print("=" * 80)

    # Create custom metric definitions
    custom_json = {
        "namespace": "custom",
        "schema_version": 1,
        "metrics": [
            {
                "id": "gross_profit",
                "name": "Gross Profit",
                "formula": "revenue - cogs",
                "description": "Revenue minus cost of goods sold",
                "category": "profitability",
                "unit_type": "currency",
                "requires": ["revenue", "cogs"],
                "tags": ["profit", "core"],
            },
            {
                "id": "gross_margin",
                "name": "Gross Margin %",
                "formula": "gross_profit / revenue",
                "description": "Gross profit as percentage of revenue",
                "category": "margins",
                "unit_type": "percentage",
                "requires": ["gross_profit", "revenue"],
                "tags": ["margin", "profitability"],
            },
            {
                "id": "operating_profit",
                "name": "Operating Profit (EBIT)",
                "formula": "gross_profit - operating_expenses",
                "description": "Earnings before interest and tax",
                "category": "profitability",
                "unit_type": "currency",
                "requires": ["gross_profit", "operating_expenses"],
                "tags": ["profit", "operating"],
            },
        ],
    }

    # Save to temporary file
    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".json", delete=False
    ) as f:
        json.dump(custom_json, f, indent=2)
        temp_path = f.name

    try:
        # Load from file
        registry = Registry.new()
        registry.load_from_json(temp_path)

        print(f"\nLoaded custom metrics from {temp_path}")
        custom_metrics = registry.list_metrics("custom")
        print(f"Found {len(custom_metrics)} metrics:")
        for metric_id in custom_metrics:
            metric = registry.get(metric_id)
            print(f"  {metric_id:30s} - {metric.formula:40s}")

    finally:
        Path(temp_path).unlink()


def example_3_programmatic_metric_creation():
    """Example 3: Creating metrics programmatically."""
    print("\n" + "=" * 80)
    print("Example 3: Programmatic Metric Creation")
    print("=" * 80)

    # Create individual metric definitions
    metrics = [
        MetricDefinition(
            id="revenue_growth",
            name="Revenue Growth Rate",
            formula="pct_change(revenue, 1)",
            description="Period-over-period revenue growth",
            category="growth",
            unit_type=UnitType.PERCENTAGE,
            requires=["revenue"],
            tags=["growth", "topline"],
        ),
        MetricDefinition(
            id="ebitda_margin",
            name="EBITDA Margin %",
            formula="ebitda / revenue",
            description="EBITDA as percentage of revenue",
            category="margins",
            unit_type=UnitType.PERCENTAGE,
            requires=["ebitda", "revenue"],
            tags=["margin", "profitability"],
        ),
    ]

    # Create registry
    registry_obj = MetricRegistry("analytics", metrics)

    # Load into dynamic registry
    registry = Registry.new()
    registry.load_from_json_str(registry_obj.to_json())

    print("\nCreated programmatic metrics:")
    for metric_id in registry.list_metrics("analytics"):
        metric = registry.get(metric_id)
        print(f"  {metric.id:20s} - {metric.name}")
        print(f"    Formula: {metric.formula}")
        print(f"    Unit:    {metric.unit_type}")


def example_4_model_with_builtin_metrics():
    """Example 4: Building and evaluating a model with built-in metrics."""
    print("\n" + "=" * 80)
    print("Example 4: Model Evaluation with Built-in Metrics")
    print("=" * 80)

    # Create model builder
    builder = ModelBuilder.new()

    # Define periods (4 quarters)
    builder.periods(
        [
            Period(2024, 1, "Q1 2024"),
            Period(2024, 2, "Q2 2024"),
            Period(2024, 3, "Q3 2024"),
            Period(2024, 4, "Q4 2024"),
        ]
    )

    # Add input data (all values in thousands)
    builder.value(
        "revenue",
        [
            (1, 10000.0),  # Q1: $10M
            (2, 11000.0),  # Q2: $11M
            (3, 12000.0),  # Q3: $12M
            (4, 13000.0),  # Q4: $13M
        ],
    )

    builder.value(
        "cogs",
        [
            (1, 6000.0),  # Q1: $6M
            (2, 6400.0),  # Q2: $6.4M
            (3, 7000.0),  # Q3: $7M
            (4, 7500.0),  # Q4: $7.5M
        ],
    )

    builder.value(
        "operating_expenses",
        [
            (1, 2000.0),  # Q1: $2M
            (2, 2100.0),  # Q2: $2.1M
            (3, 2200.0),  # Q3: $2.2M
            (4, 2300.0),  # Q4: $2.3M
        ],
    )

    # Load all built-in metrics
    print("\nLoading built-in financial metrics...")
    builder.with_builtin_metrics()

    # Build and evaluate
    spec = builder.build()
    evaluator = Evaluator.new(spec)
    results = evaluator.evaluate()

    # Display results for key metrics
    print("\nEvaluation Results (values in thousands):")
    print(f"{'Metric':<25s} {'Q1':<12s} {'Q2':<12s} {'Q3':<12s} {'Q4':<12s}")
    print("-" * 73)

    metrics_to_show = [
        "revenue",
        "cogs",
        "fin.gross_profit",
        "fin.gross_margin",
        "fin.ebitda",
    ]

    for metric_id in metrics_to_show:
        try:
            values = results.get_node_values(metric_id)
            if values:
                row = [metric_id]
                for period_id, value in values:
                    row.append(f"{value:11.2f}")
                print(f"{row[0]:<25s} {row[1]} {row[2]} {row[3]} {row[4]}")
        except Exception:
            pass


def example_5_batch_add_metrics():
    """Example 5: Batch-adding multiple metrics from registry."""
    print("\n" + "=" * 80)
    print("Example 5: Batch Adding Metrics")
    print("=" * 80)

    # Create registry with built-in metrics
    registry = Registry.new()
    registry.load_builtins()

    # Create model
    builder = ModelBuilder.new()
    builder.periods([Period(2024, 1, "Q1"), Period(2024, 2, "Q2")])

    # Add input data
    builder.value("revenue", [(1, 100000.0), (2, 110000.0)])
    builder.value("cogs", [(1, 60000.0), (2, 65000.0)])
    builder.value("operating_expenses", [(1, 20000.0), (2, 22000.0)])

    # Batch add multiple metrics at once
    metrics_to_add = [
        "fin.gross_profit",
        "fin.gross_margin",
        "fin.ebitda",
    ]

    print(f"\nBatch adding {len(metrics_to_add)} metrics:")
    for metric_id in metrics_to_add:
        metric = registry.get(metric_id)
        print(f"  - {metric_id:25s} ({metric.name})")

    builder.add_registry_metrics(metrics_to_add, registry)

    # Build and evaluate
    spec = builder.build()
    evaluator = Evaluator.new(spec)
    results = evaluator.evaluate()

    # Display results
    print("\nResults:")
    print(f"{'Metric':<25s} {'Q1':<15s} {'Q2':<15s}")
    print("-" * 55)

    for metric_id in metrics_to_add:
        values = results.get_node_values(metric_id)
        if values:
            q1_val = values[0][1]
            q2_val = values[1][1] if len(values) > 1 else 0
            print(f"{metric_id:<25s} {q1_val:14.2f} {q2_val:14.2f}")


def example_6_selective_metric_loading():
    """Example 6: Selectively loading individual metrics."""
    print("\n" + "=" * 80)
    print("Example 6: Selective Metric Loading")
    print("=" * 80)

    # Create model
    builder = ModelBuilder.new()
    builder.periods([Period(2024, 1, "Q1")])

    # Add minimal input data
    builder.value("revenue", [(1, 100000.0)])
    builder.value("cogs", [(1, 60000.0)])

    # Add only specific metrics (this loads builtins internally)
    print("\nAdding only fin.gross_profit metric...")
    builder.add_metric("fin.gross_profit")

    # Build and verify
    spec = builder.build()
    node_ids = [node.node_id for node in spec.nodes]

    print("\nNodes in model:")
    for node_id in sorted(node_ids):
        print(f"  - {node_id}")

    # Evaluate
    evaluator = Evaluator.new(spec)
    results = evaluator.evaluate()

    gross_profit = results.get_node_values("fin.gross_profit")
    if gross_profit:
        print(f"\nGross Profit (Q1): ${gross_profit[0][1]:,.2f}")


def example_7_dependent_metrics():
    """Example 7: Inter-metric dependencies."""
    print("\n" + "=" * 80)
    print("Example 7: Metrics with Dependencies")
    print("=" * 80)

    # Create custom metrics with dependencies
    custom_json = """{
        "namespace": "chain",
        "schema_version": 1,
        "metrics": [
            {
                "id": "step1",
                "name": "Step 1",
                "formula": "a + b"
            },
            {
                "id": "step2",
                "name": "Step 2",
                "formula": "step1 * c"
            },
            {
                "id": "step3",
                "name": "Step 3",
                "formula": "step2 / d"
            }
        ]
    }"""

    registry = Registry.new()
    registry.load_from_json_str(custom_json)

    # Create model
    builder = ModelBuilder.new()
    builder.periods([Period(2024, 1, "Q1")])

    # Add inputs
    builder.value("a", [(1, 10.0)])
    builder.value("b", [(1, 20.0)])
    builder.value("c", [(1, 2.0)])
    builder.value("d", [(1, 5.0)])

    # Add only step3 - dependencies should be auto-included
    print("\nAdding chain.step3 (which depends on step2, which depends on step1)...")
    builder.add_metric_from_registry("chain.step3", registry)

    # Build and verify all dependencies were added
    spec = builder.build()
    node_ids = [node.node_id for node in spec.nodes]

    print("\nNodes added (showing metric dependencies):")
    for node_id in sorted([n for n in node_ids if n.startswith("chain.")]):
        print(f"  - {node_id}")

    # Evaluate
    evaluator = Evaluator.new(spec)
    results = evaluator.evaluate()

    # Show calculation chain
    print("\nCalculation results:")
    step1 = results.get_node_values("chain.step1")
    step2 = results.get_node_values("chain.step2")
    step3 = results.get_node_values("chain.step3")

    if step1 and step2 and step3:
        print(f"  step1 = a + b = 10 + 20 = {step1[0][1]}")
        print(f"  step2 = step1 * c = {step1[0][1]} * 2 = {step2[0][1]}")
        print(f"  step3 = step2 / d = {step2[0][1]} / 5 = {step3[0][1]}")


def main():
    """Run all examples."""
    print("\n" + "=" * 80)
    print("DYNAMIC METRIC REGISTRY EXAMPLES")
    print("=" * 80)

    example_1_builtin_metrics()
    example_2_custom_json_metrics()
    example_3_programmatic_metric_creation()
    example_4_model_with_builtin_metrics()
    example_5_batch_add_metrics()
    example_6_selective_metric_loading()
    example_7_dependent_metrics()

    print("\n" + "=" * 80)
    print("All examples completed successfully!")
    print("=" * 80 + "\n")


if __name__ == "__main__":
    main()
