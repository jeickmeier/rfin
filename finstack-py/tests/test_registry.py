"""Tests for dynamic metric registry functionality."""

import json
from pathlib import Path
import tempfile

from finstack.core.dates import Period
from finstack.statements.builder import ModelBuilder
from finstack.statements.registry import (
    MetricDefinition,
    MetricRegistry,
    Registry,
    UnitType,
)
import pytest


class TestRegistry:
    """Test Registry class."""

    def test_registry_creation(self) -> None:
        """Test creating an empty registry."""
        registry = Registry.new()
        assert registry is not None
        assert len(registry.list_metrics()) == 0

    def test_load_builtins(self) -> None:
        """Test loading built-in metrics (fin.* namespace)."""
        registry = Registry.new()
        registry.load_builtins()

        # Verify metrics are loaded
        metrics = registry.list_metrics()
        assert len(metrics) > 0

        # Check for specific built-in metrics
        assert registry.has_metric("fin.gross_margin")
        assert registry.has_metric("fin.ebitda")

    def test_get_metric(self) -> None:
        """Test retrieving a specific metric definition."""
        registry = Registry.new()
        registry.load_builtins()

        metric = registry.get("fin.gross_margin")
        assert metric.id == "gross_margin"
        assert metric.name is not None
        assert metric.formula is not None
        assert "gross_profit" in metric.formula or "revenue" in metric.formula

    def test_list_metrics_by_namespace(self) -> None:
        """Test filtering metrics by namespace."""
        registry = Registry.new()
        registry.load_builtins()

        # List all metrics
        all_metrics = registry.list_metrics(None)
        assert len(all_metrics) > 0

        # List only fin.* metrics
        fin_metrics = registry.list_metrics("fin")
        assert len(fin_metrics) > 0
        assert all(m.startswith("fin.") for m in fin_metrics)

        # Empty namespace should return empty list
        custom_metrics = registry.list_metrics("custom")
        assert len(custom_metrics) == 0

    def test_load_from_json_str(self) -> None:
        """Test loading metrics from JSON string."""
        json_str = """{
            "namespace": "test",
            "schema_version": 1,
            "metrics": [
                {
                    "id": "gross_margin",
                    "name": "Gross Margin",
                    "formula": "gross_profit / revenue",
                    "description": "Gross profit as a percentage of revenue",
                    "category": "margins",
                    "unit_type": "percentage"
                },
                {
                    "id": "operating_margin",
                    "name": "Operating Margin",
                    "formula": "ebit / revenue",
                    "description": "Operating profit margin",
                    "category": "margins",
                    "unit_type": "percentage"
                }
            ]
        }"""

        registry = Registry.new()
        loaded_registry = registry.load_from_json_str(json_str)

        assert loaded_registry.namespace == "test"
        assert len(loaded_registry.metrics) == 2

        # Verify metrics are available
        assert registry.has_metric("test.gross_margin")
        assert registry.has_metric("test.operating_margin")

    def test_load_from_json_file(self) -> None:
        """Test loading metrics from a JSON file."""
        json_content = {
            "namespace": "custom",
            "schema_version": 1,
            "metrics": [
                {
                    "id": "net_margin",
                    "name": "Net Margin",
                    "formula": "net_income / revenue",
                    "category": "margins",
                    "unit_type": "percentage",
                }
            ],
        }

        # Create temporary file
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            json.dump(json_content, f)
            temp_path = f.name

        try:
            registry = Registry.new()
            registry.load_from_json(temp_path)

            assert registry.has_metric("custom.net_margin")
            metric = registry.get("custom.net_margin")
            assert metric.id == "net_margin"
            assert metric.formula == "net_income / revenue"
        finally:
            Path(temp_path).unlink()


class TestMetricDefinition:
    """Test MetricDefinition class."""

    def test_create_metric_definition(self) -> None:
        """Test creating a metric definition."""
        metric = MetricDefinition(
            id="test_margin",
            name="Test Margin",
            formula="profit / revenue",
            description="A test margin metric",
            category="margins",
            unit_type=UnitType.PERCENTAGE,
            requires=["profit", "revenue"],
            tags=["test", "margin"],
        )

        assert metric.id == "test_margin"
        assert metric.name == "Test Margin"
        assert metric.formula == "profit / revenue"
        assert metric.description == "A test margin metric"
        assert metric.category == "margins"
        assert metric.unit_type == UnitType.PERCENTAGE
        assert metric.requires == ["profit", "revenue"]
        assert metric.tags == ["test", "margin"]

    def test_metric_definition_json_roundtrip(self) -> None:
        """Test JSON serialization/deserialization of metric definitions."""
        metric = MetricDefinition(
            id="ebitda_margin",
            name="EBITDA Margin",
            formula="ebitda / revenue",
            unit_type=UnitType.PERCENTAGE,
        )

        # Serialize to JSON
        json_str = metric.to_json()
        assert json_str is not None

        # Deserialize back
        metric2 = MetricDefinition.from_json(json_str)
        assert metric2.id == metric.id
        assert metric2.name == metric.name
        assert metric2.formula == metric.formula


class TestMetricRegistry:
    """Test MetricRegistry class."""

    def test_create_metric_registry(self) -> None:
        """Test creating a metric registry."""
        metrics = [
            MetricDefinition(
                id="metric1",
                name="Metric 1",
                formula="a + b",
            ),
            MetricDefinition(
                id="metric2",
                name="Metric 2",
                formula="a * b",
            ),
        ]

        registry = MetricRegistry("test", metrics, schema_version=1)
        assert registry.namespace == "test"
        assert len(registry.metrics) == 2
        assert registry.schema_version == 1

    def test_metric_registry_json_roundtrip(self) -> None:
        """Test JSON serialization/deserialization of registry."""
        metrics = [
            MetricDefinition(
                id="gross_profit",
                name="Gross Profit",
                formula="revenue - cogs",
            )
        ]

        registry = MetricRegistry("custom", metrics)

        # Serialize to JSON
        json_str = registry.to_json()
        assert json_str is not None

        # Deserialize back
        registry2 = MetricRegistry.from_json(json_str)
        assert registry2.namespace == registry.namespace
        assert len(registry2.metrics) == len(registry.metrics)


class TestModelBuilderWithRegistry:
    """Test ModelBuilder integration with registry."""

    def test_with_builtin_metrics(self) -> None:
        """Test loading built-in metrics into model."""
        builder = ModelBuilder.new("test_model")
        builder.periods([Period(2024, 1, "Q1"), Period(2024, 2, "Q2")])

        # Add some input nodes
        builder.value("revenue", [(1, 100000.0), (2, 110000.0)])
        builder.value("cogs", [(1, 60000.0), (2, 65000.0)])

        # Load built-in metrics
        builder.with_builtin_metrics()

        spec = builder.build()
        assert spec is not None

        # Built-in metrics should be in the spec
        node_ids = [node.node_id for node in spec.nodes]
        assert "fin.gross_profit" in node_ids
        assert "fin.gross_margin" in node_ids

    def test_add_single_metric(self) -> None:
        """Test adding a single metric from built-in registry."""
        builder = ModelBuilder.new("test_model")
        builder.periods([Period(2024, 1, "Q1"), Period(2024, 2, "Q2")])

        # Add dependencies
        builder.value("revenue", [(1, 100000.0), (2, 110000.0)])
        builder.value("cogs", [(1, 60000.0), (2, 65000.0)])

        # Add single metric (this will load builtins internally)
        builder.add_metric("fin.gross_profit")

        spec = builder.build()
        node_ids = [node.node_id for node in spec.nodes]
        assert "fin.gross_profit" in node_ids

    def test_add_metric_from_registry(self) -> None:
        """Test adding a metric from a custom registry."""
        # Create custom registry
        registry = Registry.new()
        json_str = """{
            "namespace": "custom",
            "schema_version": 1,
            "metrics": [
                {
                    "id": "margin",
                    "name": "Custom Margin",
                    "formula": "profit / revenue"
                }
            ]
        }"""
        registry.load_from_json_str(json_str)

        # Create model
        builder = ModelBuilder.new("test_model")
        builder.periods([Period(2024, 1, "Q1")])
        builder.value("profit", [(1, 40000.0)])
        builder.value("revenue", [(1, 100000.0)])

        # Add metric from custom registry
        builder.add_metric_from_registry("custom.margin", registry)

        spec = builder.build()
        node_ids = [node.node_id for node in spec.nodes]
        assert "custom.margin" in node_ids

    def test_add_registry_metrics_batch(self) -> None:
        """Test batch-adding multiple metrics from registry."""
        # Create registry
        registry = Registry.new()
        registry.load_builtins()

        # Create model with dependencies
        builder = ModelBuilder.new("test_model")
        builder.periods([Period(2024, 1, "Q1"), Period(2024, 2, "Q2")])
        builder.value("revenue", [(1, 100000.0), (2, 110000.0)])
        builder.value("cogs", [(1, 60000.0), (2, 65000.0)])
        builder.value("operating_expenses", [(1, 20000.0), (2, 22000.0)])
        builder.value("interest_expense", [(1, 2000.0), (2, 2100.0)])
        builder.value("tax_expense", [(1, 5000.0), (2, 5500.0)])

        # Batch add multiple metrics
        builder.add_registry_metrics(
            [
                "fin.gross_profit",
                "fin.gross_margin",
                "fin.ebitda",
            ],
            registry,
        )

        spec = builder.build()
        node_ids = [node.node_id for node in spec.nodes]
        assert "fin.gross_profit" in node_ids
        assert "fin.gross_margin" in node_ids
        assert "fin.ebitda" in node_ids

    def test_load_fin_ebitda_and_evaluate(self) -> None:
        """Test loading fin.ebitda metric and evaluating it (from task requirements)."""
        from finstack.statements.evaluator import Evaluator

        # Create registry and load built-ins
        registry = Registry.new()
        registry.load_builtins()

        # Verify fin.ebitda exists
        assert registry.has_metric("fin.ebitda")
        ebitda_metric = registry.get("fin.ebitda")
        assert ebitda_metric.formula is not None

        # Create model with required inputs for EBITDA
        builder = ModelBuilder.new("test_model")
        builder.periods([Period(2024, 1, "Q1"), Period(2024, 2, "Q2")])

        # EBITDA typically = revenue - cogs - operating_expenses
        builder.value("revenue", [(1, 100000.0), (2, 110000.0)])
        builder.value("cogs", [(1, 60000.0), (2, 65000.0)])
        builder.value("operating_expenses", [(1, 20000.0), (2, 22000.0)])

        # Add the EBITDA metric
        builder.add_metric("fin.ebitda")

        # Build and evaluate
        spec = builder.build()
        evaluator = Evaluator.new(spec)
        results = evaluator.evaluate()

        # Verify EBITDA was calculated
        ebitda_values = results.get_node_values("fin.ebitda")
        assert ebitda_values is not None
        assert len(ebitda_values) > 0

        # EBITDA for Q1 should be 100000 - 60000 - 20000 = 20000
        # (depends on actual formula in builtin registry)
        assert ebitda_values[0][1] > 0  # Should be positive


class TestInterMetricDependencies:
    """Test metrics that depend on other metrics."""

    def test_dependent_metrics(self) -> None:
        """Test loading metrics that depend on each other."""
        from finstack.statements.evaluator import Evaluator

        registry = Registry.new()
        json_str = """{
            "namespace": "test",
            "schema_version": 1,
            "metrics": [
                {
                    "id": "gross_profit",
                    "name": "Gross Profit",
                    "formula": "revenue - cogs"
                },
                {
                    "id": "gross_margin",
                    "name": "Gross Margin",
                    "formula": "gross_profit / revenue"
                }
            ]
        }"""
        registry.load_from_json_str(json_str)

        # Create model
        builder = ModelBuilder.new("test_model")
        builder.periods([Period(2024, 1, "Q1")])
        builder.value("revenue", [(1, 100000.0)])
        builder.value("cogs", [(1, 60000.0)])

        # Add dependent metric (should automatically include gross_profit)
        builder.add_metric_from_registry("test.gross_margin", registry)

        # Build and evaluate
        spec = builder.build()
        evaluator = Evaluator.new(spec)
        results = evaluator.evaluate()

        # Both metrics should be calculated
        gross_profit = results.get_node_values("test.gross_profit")
        gross_margin = results.get_node_values("test.gross_margin")

        assert gross_profit is not None
        assert gross_margin is not None

        # gross_profit = 100000 - 60000 = 40000
        assert abs(gross_profit[0][1] - 40000.0) < 0.01

        # gross_margin = 40000 / 100000 = 0.4
        assert abs(gross_margin[0][1] - 0.4) < 0.01


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
