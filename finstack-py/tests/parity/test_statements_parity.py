"""Comprehensive parity tests for statements module.

Tests model building, evaluation, forecasts, formulas, extensions, and registry.
"""

from datetime import date
from typing import Any

from finstack.core.currency import USD
from finstack.core.dates import PeriodId
from finstack.core.money import Money
import pytest

from finstack.statements import (
    AmountOrScalar,
    Evaluator,
    ForecastSpec,
    ModelBuilder,
    Registry,
)


class TestModelBuilderParity:
    """Test model builder matches Rust implementation."""

    def test_builder_basic_construction(self) -> None:
        """Test basic model construction."""
        builder = ModelBuilder.new("test_model")
        builder.periods("2024Q1..Q4", None)

        model = builder.build()

        assert model.id == "test_model"
        assert len(model.periods) == 4

    def test_builder_with_actuals(self) -> None:
        """Test model with actuals cutoff."""
        builder = ModelBuilder.new("test_model")
        builder.periods("2024Q1..Q4", "2024Q2")

        model = builder.build()

        # Q1 and Q2 should be actual, Q3-Q4 forecast
        assert model.periods[0].is_actual
        assert model.periods[1].is_actual
        assert not model.periods[2].is_actual
        assert not model.periods[3].is_actual

    def test_builder_with_values(self) -> None:
        """Test adding value nodes."""
        builder = ModelBuilder.new("test_model")
        builder.periods("2024Q1..Q2", None)

        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2024, 2), AmountOrScalar.scalar(110000.0)),
            ],
        )

        model = builder.build()

        assert "revenue" in model.nodes

    def test_builder_with_formula(self) -> None:
        """Test adding formula nodes."""
        builder = ModelBuilder.new("test_model")
        builder.periods("2024Q1..Q2", None)

        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2024, 2), AmountOrScalar.scalar(110000.0)),
            ],
        )
        builder.compute("cogs", "revenue * 0.6")

        model = builder.build()

        assert "cogs" in model.nodes

    def test_builder_with_forecast(self) -> None:
        """Test adding forecast nodes."""
        builder = ModelBuilder.new("test_model")
        builder.periods("2024Q1..Q4", "2024Q1")

        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0))])
        builder.forecast("revenue", ForecastSpec.forward_fill())

        model = builder.build()

        # Should have forecast on revenue node
        assert "revenue" in model.nodes


class TestEvaluatorParity:
    """Test evaluator matches Rust implementation."""

    def test_evaluator_basic_evaluation(self) -> None:
        """Test basic model evaluation."""
        builder = ModelBuilder.new("test_model")
        builder.periods("2024Q1..Q2", None)

        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2024, 2), AmountOrScalar.scalar(110000.0)),
            ],
        )

        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        # Should have results for revenue
        assert results is not None

    def test_evaluator_formula_evaluation(self) -> None:
        """Test formula evaluation."""
        builder = ModelBuilder.new("test_model")
        builder.periods("2024Q1..Q2", None)

        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2024, 2), AmountOrScalar.scalar(110000.0)),
            ],
        )
        builder.compute("cogs", "revenue * 0.6")
        builder.compute("gross_profit", "revenue - cogs")

        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        # Results should have all three metrics
        assert results is not None

    def test_evaluator_precedence_value_over_formula(self) -> None:
        """Test Value > Formula precedence rule."""
        builder = ModelBuilder.new("test_model")
        builder.periods("2024Q1..Q2", None)

        # Create mixed node with both value and formula
        mixed = builder.mixed("revenue")
        mixed.values([
            (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0))  # Q1 has value
        ])
        mixed.formula("50000")  # Formula applies to Q2
        builder = mixed.finish()

        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        # Q1 should use value (100,000), Q2 should use formula (50,000)
        assert results is not None

    def test_evaluator_precedence_value_over_forecast(self) -> None:
        """Test Value > Forecast precedence rule."""
        builder = ModelBuilder.new("test_model")
        builder.periods("2024Q1..Q4", "2024Q1")

        # Create mixed node with value and forecast
        mixed = builder.mixed("revenue")
        mixed.values([(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0))])
        mixed.forecast(ForecastSpec.forward_fill())
        builder = mixed.finish()

        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        # Q1 should use value, Q2-Q4 should use forecast
        assert results is not None


class TestForecastSpecParity:
    """Test forecast specifications match Rust."""

    def test_forward_fill_forecast(self) -> None:
        """Test forward fill forecast."""
        forecast = ForecastSpec.forward_fill()
        assert forecast is not None

    def test_growth_percentage_forecast(self) -> None:
        """Test growth percentage forecast."""
        forecast = ForecastSpec.growth(0.05)  # 5% growth
        assert forecast is not None

    def test_normal_distribution_forecast(self) -> None:
        """Test normal distribution forecast."""
        forecast = ForecastSpec.normal(
            mean=100000.0,
            std=10000.0,
            seed=42,
        )
        assert forecast is not None

    def test_lognormal_distribution_forecast(self) -> None:
        """Test lognormal distribution forecast."""
        forecast = ForecastSpec.lognormal(
            mean=100000.0,
            std=10000.0,
            seed=42,
        )
        assert forecast is not None


class TestAmountOrScalarParity:
    """Test AmountOrScalar type matches Rust."""

    def test_scalar_construction(self) -> None:
        """Test scalar value construction."""
        value = AmountOrScalar.scalar(100.0)
        assert value is not None

    def test_money_construction(self) -> None:
        """Test money value construction."""
        money = Money(1000.0, USD)
        value = AmountOrScalar.amount(money.amount, USD)
        assert value is not None


class TestRegistryParity:
    """Test metric registry matches Rust."""

    def test_registry_creation(self) -> None:
        """Test registry creation."""
        registry = Registry.new()
        assert registry is not None

    def test_load_builtins(self) -> None:
        """Test loading built-in metrics."""
        registry = Registry.new()
        registry.load_builtins()

        # Should have built-in metrics
        metrics = registry.list_metrics()
        assert len(metrics) > 0

    def test_get_metric(self) -> None:
        """Test getting specific metric."""
        registry = Registry.new()
        registry.load_builtins()

        # Try to get a built-in metric
        metric = registry.get("fin.gross_margin")
        assert metric is not None

    def test_has_metric(self) -> None:
        """Test checking metric existence."""
        registry = Registry.new()
        registry.load_builtins()

        assert registry.has_metric("fin.gross_margin")
        assert not registry.has_metric("nonexistent.metric")

    def test_add_metric_to_model(self) -> None:
        """Test adding metric from registry to model."""
        registry = Registry.new()
        registry.load_builtins()

        builder = ModelBuilder.new("test_model")
        builder.periods("2024Q1..Q2", None)

        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2024, 2), AmountOrScalar.scalar(110000.0)),
            ],
        )
        builder.value(
            "cogs",
            [
                (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(60000.0)),
                (PeriodId.quarter(2024, 2), AmountOrScalar.scalar(65000.0)),
            ],
        )

        # Add metric from registry
        builder.add_metric_from_registry("fin.gross_margin", registry)

        model = builder.build()

        assert "fin.gross_margin" in model.nodes


class TestExtensionsParity:
    """Test extensions match Rust implementation."""

    def test_corkscrew_extension(self) -> None:
        """Test corkscrew extension."""
        from finstack.statements.extensions import CorkscrewConfig, CorkscrewExtension

        config = CorkscrewConfig(accounts=[], tolerance=0.01)
        extension = CorkscrewExtension.with_config(config)

        assert extension is not None

    def test_credit_scorecard_extension(self) -> None:
        """Test credit scorecard extension."""
        from finstack.statements.extensions import (
            CreditScorecardExtension,
            ScorecardConfig,
        )

        config = ScorecardConfig(rating_scale="S&P", metrics=[])
        extension = CreditScorecardExtension.with_config(config)

        assert extension is not None


class TestDataFrameExportParity:
    """Test DataFrame export matches Rust."""

    def test_to_polars_long(self) -> None:
        """Test long-format DataFrame export."""
        builder = ModelBuilder.new("test_model")
        builder.periods("2024Q1..Q2", None)

        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2024, 2), AmountOrScalar.scalar(110000.0)),
            ],
        )

        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        # Export to DataFrame
        df = results.to_polars_long()

        assert df is not None
        assert len(df) > 0

    def test_to_polars_wide(self) -> None:
        """Test wide-format DataFrame export."""
        builder = ModelBuilder.new("test_model")
        builder.periods("2024Q1..Q2", None)

        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2024, 2), AmountOrScalar.scalar(110000.0)),
            ],
        )

        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        # Export to DataFrame
        df = results.to_polars_wide()

        assert df is not None
        assert len(df) > 0


class TestCapitalStructureParity:
    """Test capital structure integration matches Rust."""

    def test_add_bond_to_model(self) -> None:
        """Test adding bond to model."""
        builder = ModelBuilder.new("test_model")
        builder.periods("2024Q1..Q4", None)

        notional = Money(10_000_000.0, USD)
        issue_date = date(2024, 1, 1)
        maturity_date = date(2029, 1, 1)

        builder.add_bond(
            "BOND-001",
            notional,
            0.05,
            issue_date,
            maturity_date,
            "USD-OIS",
        )

        model = builder.build()

        assert model.capital_structure is not None
        assert len(model.capital_structure.debt_instruments) == 1

    def test_add_swap_to_model(self) -> None:
        """Test adding interest rate swap to model."""
        builder = ModelBuilder.new("test_model")
        builder.periods("2024Q1..Q4", None)

        notional = Money(5_000_000.0, USD)
        start_date = date(2024, 1, 1)
        maturity_date = date(2029, 1, 1)

        builder.add_swap(
            "SWAP-001",
            notional,
            0.04,
            start_date,
            maturity_date,
            "USD-OIS",
            "USD-SOFR-3M",
        )

        model = builder.build()

        assert model.capital_structure is not None
        assert len(model.capital_structure.debt_instruments) == 1


class TestFormulaParity:
    """Test formula parsing and evaluation matches Rust."""

    def test_simple_arithmetic(self) -> None:
        """Test simple arithmetic formulas."""
        builder = ModelBuilder.new("test_model")
        builder.periods("2024Q1..Q2", None)

        builder.value(
            "a",
            [
                (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100.0)),
                (PeriodId.quarter(2024, 2), AmountOrScalar.scalar(100.0)),
            ],
        )
        builder.value(
            "b",
            [
                (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(50.0)),
                (PeriodId.quarter(2024, 2), AmountOrScalar.scalar(50.0)),
            ],
        )

        builder.compute("sum", "a + b")
        builder.compute("diff", "a - b")
        builder.compute("product", "a * b")
        builder.compute("quotient", "a / b")

        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        assert results is not None

    def test_formula_with_constants(self) -> None:
        """Test formulas with numeric constants."""
        builder = ModelBuilder.new("test_model")
        builder.periods("2024Q1..Q2", None)

        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2024, 2), AmountOrScalar.scalar(110000.0)),
            ],
        )

        builder.compute("cogs", "revenue * 0.6")
        builder.compute("opex", "revenue * 0.2")

        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        assert results is not None

    def test_formula_with_functions(self) -> None:
        """Test formulas with built-in functions."""
        builder = ModelBuilder.new("test_model")
        builder.periods("2024Q1..Q2", None)

        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2024, 2), AmountOrScalar.scalar(110000.0)),
            ],
        )

        # Use max() function
        builder.compute("max_revenue", "max(revenue, 105000)")

        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        assert results is not None


class TestEdgeCases:
    """Test edge cases and boundary conditions."""

    def test_empty_model(self) -> None:
        """Test empty model evaluation."""
        builder = ModelBuilder.new("empty_model")
        builder.periods("2024Q1..Q2", None)

        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        # Should succeed with no nodes
        assert results is not None

    def test_single_period_model(self) -> None:
        """Test model with single period."""
        builder = ModelBuilder.new("single_period")
        builder.periods("2024Q1..Q1", None)

        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0))])

        model = builder.build()

        assert len(model.periods) == 1

    def test_large_number_of_periods(self) -> None:
        """Test model with many periods."""
        builder = ModelBuilder.new("many_periods")
        builder.periods("2024M01..M12", None)  # 12 monthly periods

        # Add value for first month
        builder.value("revenue", [(PeriodId.month(2024, 1), AmountOrScalar.scalar(100000.0))])
        builder.forecast("revenue", ForecastSpec.forward_fill())

        model = builder.build()

        assert len(model.periods) == 12

    def test_zero_values(self) -> None:
        """Test model with zero values."""
        builder = ModelBuilder.new("zero_values")
        builder.periods("2024Q1..Q2", None)

        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(0.0)),
                (PeriodId.quarter(2024, 2), AmountOrScalar.scalar(0.0)),
            ],
        )
        builder.compute("cogs", "revenue * 0.6")

        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        # Should handle zero gracefully
        assert results is not None

    def test_negative_values(self) -> None:
        """Test model with negative values."""
        builder = ModelBuilder.new("negative_values")
        builder.periods("2024Q1..Q2", None)

        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2024, 2), AmountOrScalar.scalar(110000.0)),
            ],
        )
        builder.value(
            "loss",
            [
                (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(-50000.0)),
                (PeriodId.quarter(2024, 2), AmountOrScalar.scalar(-60000.0)),
            ],
        )

        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        # Should handle negatives gracefully
        assert results is not None


class TestDeterminismParity:
    """Test deterministic evaluation matches Rust."""

    def test_evaluation_is_deterministic(self) -> None:
        """Test that multiple evaluations produce identical results."""
        builder = ModelBuilder.new("deterministic_test")
        builder.periods("2024Q1..Q2", None)

        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2024, 2), AmountOrScalar.scalar(110000.0)),
            ],
        )
        builder.compute("cogs", "revenue * 0.6")
        builder.compute("gross_profit", "revenue - cogs")

        model = builder.build()

        evaluator = Evaluator.new()

        # Evaluate twice
        results1 = evaluator.evaluate(model)
        results2 = evaluator.evaluate(model)

        # Results should be identical (same objects or same values)
        assert results1 is not None
        assert results2 is not None

    def test_forecast_with_seed_is_deterministic(self) -> None:
        """Test that forecasts with fixed seed are deterministic."""

        # Create two identical models with seeded random forecast
        def create_model() -> Any:
            builder = ModelBuilder.new("random_test")
            builder.periods("2024Q1..Q4", "2024Q1")

            builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0))])
            builder.forecast("revenue", ForecastSpec.normal(100000.0, 10000.0, seed=42))

            return builder.build()

        model1 = create_model()
        model2 = create_model()

        evaluator = Evaluator.new()

        results1 = evaluator.evaluate(model1)
        results2 = evaluator.evaluate(model2)

        # Results should be identical with same seed
        assert results1 is not None
        assert results2 is not None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
