"""Seeded stochastic parity tests.

Tests that leverage explicit seed support in ForecastSpec to verify
that stochastic operations produce identical results when given the same seed.
"""

from typing import Any

from finstack.core.dates import PeriodId
from hypothesis import assume, given, settings, strategies as st
import pytest
from tests.fixtures.strategies import (
    TOLERANCE_DETERMINISTIC,
)

from finstack.statements import AmountOrScalar, Evaluator, ForecastSpec, ModelBuilder


@pytest.mark.parity
@pytest.mark.seeded
class TestSeededForecastParity:
    """Test seeded forecast reproducibility."""

    def test_normal_forecast_same_seed_identical(self) -> None:
        """Normal forecasts with same seed produce identical results."""
        seed = 42
        mean = 100000.0
        std = 10000.0

        def create_model() -> Any:
            builder = ModelBuilder.new("seeded_test")
            builder.periods("2024Q1..Q4", "2024Q1")
            builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0))])
            builder.forecast("revenue", ForecastSpec.normal(mean, std, seed=seed))
            return builder.build()

        # Create and evaluate twice
        model1 = create_model()
        model2 = create_model()

        evaluator1 = Evaluator.new()
        evaluator2 = Evaluator.new()

        results1 = evaluator1.evaluate(model1)
        results2 = evaluator2.evaluate(model2)

        # Compare Q2-Q4 forecast values
        for q in [2, 3, 4]:
            period = PeriodId.quarter(2024, q)
            val1 = results1.get("revenue", period)
            val2 = results2.get("revenue", period)
            assert abs(val1 - val2) < TOLERANCE_DETERMINISTIC, f"Q{q}: {val1} != {val2} (diff: {abs(val1 - val2)})"

    def test_lognormal_forecast_same_seed_identical(self) -> None:
        """Lognormal forecasts with same seed produce identical results."""
        seed = 123
        mean = 0.05  # 5% mean growth
        std = 0.02

        def create_model() -> Any:
            builder = ModelBuilder.new("lognormal_test")
            builder.periods("2024Q1..Q4", "2024Q1")
            builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0))])
            builder.forecast("revenue", ForecastSpec.lognormal(mean, std, seed=seed))
            return builder.build()

        model1 = create_model()
        model2 = create_model()

        results1 = Evaluator.new().evaluate(model1)
        results2 = Evaluator.new().evaluate(model2)

        for q in [2, 3, 4]:
            period = PeriodId.quarter(2024, q)
            val1 = results1.get("revenue", period)
            val2 = results2.get("revenue", period)
            assert abs(val1 - val2) < TOLERANCE_DETERMINISTIC, f"Q{q}: {val1} != {val2}"

    def test_different_seeds_different_results(self) -> None:
        """Different seeds produce different forecast values."""
        seed1 = 42
        seed2 = 43
        mean = 100000.0
        std = 10000.0

        def create_model(seed: int) -> Any:
            builder = ModelBuilder.new("diff_seed_test")
            builder.periods("2024Q1..Q4", "2024Q1")
            builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0))])
            builder.forecast("revenue", ForecastSpec.normal(mean, std, seed=seed))
            return builder.build()

        model1 = create_model(seed1)
        model2 = create_model(seed2)

        results1 = Evaluator.new().evaluate(model1)
        results2 = Evaluator.new().evaluate(model2)

        # At least one period should differ significantly
        any_different = False
        for q in [2, 3, 4]:
            period = PeriodId.quarter(2024, q)
            val1 = results1.get("revenue", period)
            val2 = results2.get("revenue", period)
            if abs(val1 - val2) > 1.0:  # More than $1 difference
                any_different = True
                break

        assert any_different, "Different seeds should produce different results"

    def test_seeded_forecast_stable_across_evaluations(self) -> None:
        """Same seeded model produces stable results across multiple evaluations."""
        seed = 999
        mean = 100000.0
        std = 10000.0

        builder = ModelBuilder.new("stability_test")
        builder.periods("2024Q1..Q4", "2024Q1")
        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0))])
        builder.forecast("revenue", ForecastSpec.normal(mean, std, seed=seed))
        model = builder.build()

        # Evaluate 5 times
        results_list = []
        for _ in range(5):
            evaluator = Evaluator.new()
            result = evaluator.evaluate(model)
            results_list.append([result.get("revenue", PeriodId.quarter(2024, q)) for q in [2, 3, 4]])

        # All evaluations should produce identical results
        for i in range(1, len(results_list)):
            for j in range(3):
                assert abs(results_list[0][j] - results_list[i][j]) < TOLERANCE_DETERMINISTIC, (
                    f"Evaluation {i}, Q{j + 2}: {results_list[0][j]} != {results_list[i][j]}"
                )

    def test_multiple_nodes_with_different_seeds(self) -> None:
        """Multiple forecast nodes with different seeds are independent."""
        builder = ModelBuilder.new("multi_node_test")
        builder.periods("2024Q1..Q4", "2024Q1")

        # Two nodes with different seeds
        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0))])
        builder.forecast("revenue", ForecastSpec.normal(100000.0, 10000.0, seed=42))

        builder.value("expenses", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(60000.0))])
        builder.forecast("expenses", ForecastSpec.normal(60000.0, 5000.0, seed=43))

        model = builder.build()

        results1 = Evaluator.new().evaluate(model)
        results2 = Evaluator.new().evaluate(model)

        # Both nodes should be reproducible
        for q in [2, 3, 4]:
            period = PeriodId.quarter(2024, q)

            rev1 = results1.get("revenue", period)
            rev2 = results2.get("revenue", period)
            assert abs(rev1 - rev2) < TOLERANCE_DETERMINISTIC

            exp1 = results1.get("expenses", period)
            exp2 = results2.get("expenses", period)
            assert abs(exp1 - exp2) < TOLERANCE_DETERMINISTIC


@pytest.mark.parity
@pytest.mark.seeded
@pytest.mark.properties
class TestSeededForecastProperties:
    """Property tests for seeded forecast behavior."""

    @given(st.integers(min_value=1, max_value=2**32 - 1))
    @settings(max_examples=30, deadline=None)
    def test_seed_determinism_property(self, seed: int) -> None:
        """Any seed value produces deterministic results."""
        mean = 100000.0
        std = 10000.0

        def create_model() -> Any:
            builder = ModelBuilder.new("seed_prop_test")
            builder.periods("2024Q1..Q4", "2024Q1")
            builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0))])
            builder.forecast("revenue", ForecastSpec.normal(mean, std, seed=seed))
            return builder.build()

        model1 = create_model()
        model2 = create_model()

        results1 = Evaluator.new().evaluate(model1)
        results2 = Evaluator.new().evaluate(model2)

        for q in [2, 3, 4]:
            period = PeriodId.quarter(2024, q)
            val1 = results1.get("revenue", period)
            val2 = results2.get("revenue", period)
            assert abs(val1 - val2) < TOLERANCE_DETERMINISTIC

    @given(
        st.floats(min_value=1000.0, max_value=1e9, allow_nan=False, allow_infinity=False),
        st.floats(min_value=100.0, max_value=1e6, allow_nan=False, allow_infinity=False),
        st.integers(min_value=1, max_value=1000000),
    )
    @settings(max_examples=30, deadline=None)
    def test_normal_forecast_reasonable_bounds(self, mean: float, std: float, seed: int) -> None:
        """Normal forecasts produce values within reasonable bounds (5 sigma).

        The normal forecast is a random walk: value[t] = value[t-1] + N(mean, std).
        After k steps the expected value is base + k*mean with std sqrt(k)*std.
        """
        assume(std > 0)

        base_value = mean
        builder = ModelBuilder.new("bounds_test")
        builder.periods("2024Q1..Q4", "2024Q1")
        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(base_value))])
        builder.forecast("revenue", ForecastSpec.normal(mean, std, seed=seed))
        model = builder.build()

        results = Evaluator.new().evaluate(model)

        for q in [2, 3, 4]:
            steps = q - 1
            period = PeriodId.quarter(2024, q)
            value = results.get("revenue", period)
            expected = base_value + steps * mean
            band = 5 * (steps**0.5) * std
            assert value > expected - band, f"Q{q}: {value} < {expected - band}"
            assert value < expected + band, f"Q{q}: {value} > {expected + band}"

    @given(
        st.floats(min_value=0.01, max_value=0.10, allow_nan=False, allow_infinity=False),
        st.floats(min_value=0.001, max_value=0.05, allow_nan=False, allow_infinity=False),
        st.integers(min_value=1, max_value=1000000),
    )
    @settings(max_examples=30, deadline=None)
    def test_lognormal_always_positive(self, mean: float, std: float, seed: int) -> None:
        """Lognormal forecasts always produce positive values."""
        assume(std > 0)

        builder = ModelBuilder.new("lognormal_pos_test")
        builder.periods("2024Q1..Q4", "2024Q1")
        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0))])
        builder.forecast("revenue", ForecastSpec.lognormal(mean, std, seed=seed))
        model = builder.build()

        results = Evaluator.new().evaluate(model)

        for q in [2, 3, 4]:
            period = PeriodId.quarter(2024, q)
            value = results.get("revenue", period)
            assert value > 0, f"Q{q}: lognormal forecast produced non-positive value {value}"

    @given(st.floats(min_value=0.01, max_value=0.20, allow_nan=False, allow_infinity=False))
    @settings(max_examples=20, deadline=None)
    def test_growth_forecast_correct_rate(self, growth_rate: float) -> None:
        """Growth forecast applies correct rate period-over-period."""
        base_value = 100000.0

        builder = ModelBuilder.new("growth_test")
        builder.periods("2024Q1..Q4", "2024Q1")
        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(base_value))])
        builder.forecast("revenue", ForecastSpec.growth(growth_rate))
        model = builder.build()

        results = Evaluator.new().evaluate(model)

        # Q1 is actual
        q1_value = results.get("revenue", PeriodId.quarter(2024, 1))
        assert abs(q1_value - base_value) < TOLERANCE_DETERMINISTIC

        # Q2 should be Q1 * (1 + rate)
        expected_q2 = base_value * (1 + growth_rate)
        actual_q2 = results.get("revenue", PeriodId.quarter(2024, 2))
        assert abs(actual_q2 - expected_q2) / expected_q2 < 0.001, f"Q2: expected {expected_q2}, got {actual_q2}"

        # Q3 should be Q2 * (1 + rate)
        expected_q3 = expected_q2 * (1 + growth_rate)
        actual_q3 = results.get("revenue", PeriodId.quarter(2024, 3))
        assert abs(actual_q3 - expected_q3) / expected_q3 < 0.001, f"Q3: expected {expected_q3}, got {actual_q3}"


@pytest.mark.parity
@pytest.mark.seeded
class TestForwardFillForecastParity:
    """Test forward fill forecast determinism."""

    def test_forward_fill_deterministic(self) -> None:
        """Forward fill forecast is deterministic."""
        builder = ModelBuilder.new("ff_test")
        builder.periods("2024Q1..Q4", "2024Q1")
        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0))])
        builder.forecast("revenue", ForecastSpec.forward_fill())
        model = builder.build()

        results1 = Evaluator.new().evaluate(model)
        results2 = Evaluator.new().evaluate(model)

        for q in [1, 2, 3, 4]:
            period = PeriodId.quarter(2024, q)
            val1 = results1.get("revenue", period)
            val2 = results2.get("revenue", period)
            assert abs(val1 - val2) < TOLERANCE_DETERMINISTIC
            # All values should equal the Q1 value
            assert abs(val1 - 100000.0) < TOLERANCE_DETERMINISTIC


@pytest.mark.parity
@pytest.mark.seeded
class TestCurveForecastParity:
    """Test curve forecast determinism."""

    def test_curve_forecast_deterministic(self) -> None:
        """Curve forecast with explicit rates is deterministic."""
        rates = [0.05, 0.10, 0.15]  # Growth rates for Q2, Q3, Q4

        builder = ModelBuilder.new("curve_test")
        builder.periods("2024Q1..Q4", "2024Q1")
        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(100000.0))])
        builder.forecast("revenue", ForecastSpec.curve(rates))
        model = builder.build()

        results1 = Evaluator.new().evaluate(model)
        results2 = Evaluator.new().evaluate(model)

        for q in [2, 3, 4]:
            period = PeriodId.quarter(2024, q)
            val1 = results1.get("revenue", period)
            val2 = results2.get("revenue", period)
            assert abs(val1 - val2) < TOLERANCE_DETERMINISTIC

    def test_curve_forecast_applies_correct_rates(self) -> None:
        """Curve forecast applies period-specific rates correctly."""
        rates = [0.05, 0.10, 0.15]
        base_value = 100000.0

        builder = ModelBuilder.new("curve_rates_test")
        builder.periods("2024Q1..Q4", "2024Q1")
        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(base_value))])
        builder.forecast("revenue", ForecastSpec.curve(rates))
        model = builder.build()

        results = Evaluator.new().evaluate(model)

        # Q2 should use rates[0] = 0.05
        expected_q2 = base_value * (1 + rates[0])
        actual_q2 = results.get("revenue", PeriodId.quarter(2024, 2))
        assert abs(actual_q2 - expected_q2) / expected_q2 < 0.001

        # Q3 should use rates[1] = 0.10
        expected_q3 = expected_q2 * (1 + rates[1])
        actual_q3 = results.get("revenue", PeriodId.quarter(2024, 3))
        assert abs(actual_q3 - expected_q3) / expected_q3 < 0.001

        # Q4 should use rates[2] = 0.15
        expected_q4 = expected_q3 * (1 + rates[2])
        actual_q4 = results.get("revenue", PeriodId.quarter(2024, 4))
        assert abs(actual_q4 - expected_q4) / expected_q4 < 0.001
