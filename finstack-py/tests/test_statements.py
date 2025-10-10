"""Comprehensive tests for the statements Python bindings."""

import pytest

from finstack.core.currency import Currency
from finstack.core.dates import PeriodId
from finstack.statements.builder import ModelBuilder
from finstack.statements.evaluator import Evaluator
from finstack.statements.extensions import (
    CorkscrewExtension,
    CreditScorecardExtension,
    ExtensionRegistry,
    ExtensionStatus,
)
from finstack.statements.registry import Registry
from finstack.statements.types import (
    AmountOrScalar,
    FinancialModelSpec,
    ForecastMethod,
    ForecastSpec,
    NodeSpec,
    NodeType,
)


class TestNodeTypes:
    """Test node type enumerations and specifications."""

    def test_node_type_constants(self) -> None:
        """Test NodeType enum constants."""
        assert NodeType.VALUE is not None
        assert NodeType.CALCULATED is not None
        assert NodeType.MIXED is not None

    def test_node_spec_creation(self) -> None:
        """Test creating a NodeSpec."""
        spec = NodeSpec("revenue", NodeType.VALUE)
        assert spec.node_id == "revenue"
        # Note: Enum comparison checks string representation due to PyO3 behavior
        assert str(spec.node_type) == str(NodeType.VALUE)

    def test_node_spec_builder_pattern(self) -> None:
        """Test NodeSpec builder methods."""
        spec = (
            NodeSpec("revenue", NodeType.CALCULATED)
            .with_name("Revenue")
            .with_formula("sales * price")
            .with_tags(["income_statement", "top_line"])
        )
        assert spec.name == "Revenue"
        assert spec.formula_text == "sales * price"
        assert "income_statement" in spec.tags

    def test_node_spec_json_roundtrip(self) -> None:
        """Test NodeSpec JSON serialization."""
        spec = NodeSpec("test", NodeType.VALUE).with_name("Test Node")
        json_str = spec.to_json()
        restored = NodeSpec.from_json(json_str)
        assert restored.node_id == "test"
        assert restored.name == "Test Node"


class TestAmountOrScalar:
    """Test AmountOrScalar union type."""

    def test_scalar_creation(self) -> None:
        """Test creating a scalar value."""
        val = AmountOrScalar.scalar(100.0)
        assert val.is_scalar
        assert val.value == 100.0
        assert val.currency is None

    def test_amount_creation(self) -> None:
        """Test creating a currency amount."""
        usd = Currency("USD")
        val = AmountOrScalar.amount(1000.0, usd)
        assert not val.is_scalar
        assert val.value == 1000.0
        assert val.currency.code == "USD"

    def test_json_roundtrip(self) -> None:
        """Test AmountOrScalar JSON serialization."""
        val1 = AmountOrScalar.scalar(42.0)
        json1 = val1.to_json()
        restored1 = AmountOrScalar.from_json(json1)
        assert restored1.is_scalar
        assert restored1.value == 42.0


class TestForecastSpec:
    """Test forecast specifications."""

    def test_forecast_method_constants(self) -> None:
        """Test ForecastMethod enum constants."""
        assert ForecastMethod.FORWARD_FILL is not None
        assert ForecastMethod.GROWTH_PCT is not None
        assert ForecastMethod.CURVE_PCT is not None
        assert ForecastMethod.NORMAL is not None
        assert ForecastMethod.LOG_NORMAL is not None

    def test_forecast_spec_forward_fill(self) -> None:
        """Test forward fill forecast."""
        spec = ForecastSpec.forward_fill()
        assert str(spec.method) == str(ForecastMethod.FORWARD_FILL)

    def test_forecast_spec_growth(self) -> None:
        """Test growth percentage forecast."""
        spec = ForecastSpec.growth(0.05)
        assert str(spec.method) == str(ForecastMethod.GROWTH_PCT)
        params = spec.params
        assert params["rate"] == 0.05

    def test_forecast_spec_curve(self) -> None:
        """Test curve percentage forecast."""
        spec = ForecastSpec.curve([0.05, 0.06, 0.07])
        assert str(spec.method) == str(ForecastMethod.CURVE_PCT)
        params = spec.params
        assert params["curve"] == [0.05, 0.06, 0.07]

    def test_forecast_spec_normal(self) -> None:
        """Test normal distribution forecast."""
        spec = ForecastSpec.normal(0.05, 0.01, 42)
        assert str(spec.method) == str(ForecastMethod.NORMAL)
        params = spec.params
        assert params["mean"] == 0.05
        assert params["std_dev"] == 0.01
        assert params["seed"] == 42


class TestModelBuilder:
    """Test ModelBuilder functionality."""

    def test_builder_basic_flow(self) -> None:
        """Test basic builder flow."""
        builder = ModelBuilder.new("test_model")
        builder.periods("2025Q1..Q4", None)

        # Add a value node
        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(110000.0)),
            ],
        )

        # Add a calculated node
        builder.compute("gross_profit", "revenue * 0.4")

        model = builder.build()
        assert model.id == "test_model"
        assert len(model.periods) == 4
        assert model.has_node("revenue")
        assert model.has_node("gross_profit")

    def test_builder_with_forecast(self) -> None:
        """Test builder with forecast specification."""
        builder = ModelBuilder.new("forecast_model")
        builder.periods("2025Q1..Q4", "2025Q2")

        builder.value(
            "revenue",
            [(PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100.0))],
        )

        # Add forecast for future periods
        builder.forecast("revenue", ForecastSpec.growth(0.05))

        model = builder.build()
        node = model.get_node("revenue")
        assert node is not None
        assert node.forecast is not None

    def test_builder_with_metadata(self) -> None:
        """Test builder with metadata."""
        builder = ModelBuilder.new("meta_model")
        builder.periods("2025Q1..Q2", None)
        builder.with_meta("author", "Test User")
        builder.with_meta("version", 1)

        model = builder.build()
        meta = model.meta
        assert meta["author"] == "Test User"
        assert meta["version"] == 1


class TestEvaluator:
    """Test model evaluation."""

    def test_evaluator_basic_evaluation(self) -> None:
        """Test basic model evaluation."""
        # Build a simple model
        builder = ModelBuilder.new("eval_test")
        builder.periods("2025Q1..Q2", None)
        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(110.0)),
            ],
        )
        builder.compute("cogs", "revenue * 0.6")
        builder.compute("gross_profit", "revenue - cogs")

        model = builder.build()

        # Evaluate the model
        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        # Check results
        assert results is not None
        assert results.meta.num_nodes == 3
        assert results.meta.num_periods == 2

        # Check computed values
        q1 = PeriodId.quarter(2025, 1)

        revenue_q1 = results.get("revenue", q1)
        assert revenue_q1 == 100.0

        cogs_q1 = results.get("cogs", q1)
        assert cogs_q1 == 60.0  # 100 * 0.6

        gross_profit_q1 = results.get("gross_profit", q1)
        assert gross_profit_q1 == 40.0  # 100 - 60

    def test_evaluator_with_forecast(self) -> None:
        """Test evaluation with forecast."""
        builder = ModelBuilder.new("forecast_eval")
        builder.periods("2025Q1..Q4", "2025Q2")

        # Actual values for Q1-Q2
        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(105.0)),
            ],
        )

        # Forecast for Q3-Q4
        builder.forecast("revenue", ForecastSpec.forward_fill())

        model = builder.build()
        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        # Check that Q3 and Q4 have forecast values (forward fill)
        q3 = PeriodId.quarter(2025, 3)
        q4 = PeriodId.quarter(2025, 4)

        revenue_q3 = results.get("revenue", q3)
        revenue_q4 = results.get("revenue", q4)

        assert revenue_q3 == 105.0  # Forward filled from Q2
        assert revenue_q4 == 105.0

    def test_results_accessors(self) -> None:
        """Test Results accessor methods."""
        builder = ModelBuilder.new("results_test")
        builder.periods("2025Q1..Q2", None)
        builder.value(
            "test_metric",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(10.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(20.0)),
            ],
        )

        model = builder.build()
        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        # Test get_node
        node_results = results.get_node("test_metric")
        assert node_results is not None
        assert len(node_results) == 2

        # Test get_or
        q1 = PeriodId.quarter(2025, 1)
        q3 = PeriodId.quarter(2025, 3)
        assert results.get_or("test_metric", q1, 0.0) == 10.0
        assert results.get_or("test_metric", q3, 99.0) == 99.0  # Default value


class TestRegistry:
    """Test metric registry."""

    def test_registry_creation(self) -> None:
        """Test creating a registry."""
        registry = Registry.new()
        assert registry is not None

    def test_registry_load_builtins(self) -> None:
        """Test loading built-in metrics."""
        registry = Registry.new()
        registry.load_builtins()

        # Check that some built-in metrics are loaded
        assert registry.has_metric("fin.gross_margin")

    def test_registry_list_metrics(self) -> None:
        """Test listing metrics."""
        registry = Registry.new()
        registry.load_builtins()

        # List all metrics
        all_metrics = registry.list_metrics(None)
        assert len(all_metrics) > 0

        # List metrics in 'fin' namespace
        fin_metrics = registry.list_metrics("fin")
        assert len(fin_metrics) > 0
        assert all(m.startswith("fin.") for m in fin_metrics)

    def test_registry_get_metric(self) -> None:
        """Test getting a metric definition."""
        registry = Registry.new()
        registry.load_builtins()

        metric = registry.get("fin.gross_margin")
        assert metric is not None
        assert metric.id == "gross_margin"
        assert metric.formula is not None


class TestExtensions:
    """Test extension system."""

    def test_extension_registry_creation(self) -> None:
        """Test creating an extension registry."""
        registry = ExtensionRegistry.new()
        assert registry is not None

    def test_corkscrew_extension(self) -> None:
        """Test corkscrew extension."""
        ext = CorkscrewExtension.new()
        assert ext is not None

    def test_scorecard_extension(self) -> None:
        """Test credit scorecard extension."""
        ext = CreditScorecardExtension.new()
        assert ext is not None

    def test_extension_status_constants(self) -> None:
        """Test ExtensionStatus enum."""
        assert ExtensionStatus.SUCCESS is not None
        assert ExtensionStatus.FAILED is not None
        assert ExtensionStatus.SKIPPED is not None


class TestIntegration:
    """Integration tests for complete workflows."""

    def test_complete_pl_model(self) -> None:
        """Test complete P&L model build and evaluation."""
        # Build model
        builder = ModelBuilder.new("Acme Corp P&L")
        builder.periods("2025Q1..Q4", "2025Q2")

        # Add revenue with actuals and forecast
        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(1000000.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(1100000.0)),
            ],
        )
        builder.forecast("revenue", ForecastSpec.growth(0.05))

        # Add operating expenses
        builder.value(
            "opex",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(200000.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(210000.0)),
            ],
        )
        builder.forecast("opex", ForecastSpec.forward_fill())

        # Add calculated metrics
        builder.compute("cogs", "revenue * 0.6")
        builder.compute("gross_profit", "revenue - cogs")
        builder.compute("operating_income", "gross_profit - opex")
        builder.compute("gross_margin", "gross_profit / revenue")

        model = builder.build()

        # Evaluate
        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        # Validate results
        assert results.meta.num_nodes == 6
        assert results.meta.num_periods == 4

        # Check Q1 calculations
        q1 = PeriodId.quarter(2025, 1)
        assert results.get("revenue", q1) == 1000000.0
        assert results.get("cogs", q1) == 600000.0
        assert results.get("gross_profit", q1) == 400000.0
        assert results.get("gross_margin", q1) == 0.4

    def test_json_serialization(self) -> None:
        """Test full model JSON serialization."""
        builder = ModelBuilder.new("json_test")
        builder.periods("2025Q1..Q2", None)
        builder.value(
            "test",
            [(PeriodId.quarter(2025, 1), AmountOrScalar.scalar(10.0))],
        )

        model = builder.build()

        # Serialize to JSON
        json_str = model.to_json()
        assert json_str is not None
        assert "json_test" in json_str

        # Deserialize
        restored = FinancialModelSpec.from_json(json_str)
        assert restored.id == "json_test"
        assert len(restored.periods) == 2


class TestErrorHandling:
    """Test error handling and edge cases."""

    def test_invalid_formula(self) -> None:
        """Test that invalid formulas raise errors."""
        builder = ModelBuilder.new("error_test")
        builder.periods("2025Q1..Q2", None)

        # This should raise an error for invalid formula syntax
        with pytest.raises(Exception, match=".*"):
            builder.compute("bad_node", "revenue + + cogs")

    def test_circular_dependency_detection(self) -> None:
        """Test circular dependency detection."""
        builder = ModelBuilder.new("circular_test")
        builder.periods("2025Q1..Q2", None)
        builder.compute("a", "b + 1")
        builder.compute("b", "c + 1")
        builder.compute("c", "a + 1")  # Circular!

        model = builder.build()
        evaluator = Evaluator.new()

        # Should raise an error for circular dependencies
        with pytest.raises(Exception, match=".*"):
            evaluator.evaluate(model)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])

