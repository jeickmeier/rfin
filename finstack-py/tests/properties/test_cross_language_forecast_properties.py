"""Cross-language property tests for forecast operations.

These tests verify that forecast operations in the statements module
produce consistent and mathematically correct results across random
parameter combinations.
"""

from typing import Any

from finstack.core.dates import PeriodId
from hypothesis import assume, given, settings, strategies as st
import pytest
from tests.fixtures.strategies import (
    TOLERANCE_DETERMINISTIC,
    seeds,
)

from finstack.statements import AmountOrScalar, Evaluator, ForecastSpec, ModelBuilder


@pytest.mark.properties
class TestForecastDeterminism:
    """Property tests for forecast determinism across random inputs."""

    @given(
        st.floats(min_value=1000.0, max_value=1e9, allow_nan=False, allow_infinity=False),
        st.floats(min_value=100.0, max_value=1e6, allow_nan=False, allow_infinity=False),
        seeds,
    )
    @settings(max_examples=50, deadline=None)
    def test_normal_forecast_determinism(self, mean: float, std: float, seed: int) -> None:
        """Normal forecast with same parameters produces identical results."""
        assume(std > 0)
        assume(mean > 0)

        def create_model() -> Any:
            builder = ModelBuilder.new("normal_det_test")
            builder.periods("2024Q1..Q4", "2024Q1")
            builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(mean))])
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
            assert abs(val1 - val2) < TOLERANCE_DETERMINISTIC, f"Q{q}: {val1} != {val2} for seed={seed}"

    @given(
        st.floats(min_value=0.01, max_value=0.10, allow_nan=False, allow_infinity=False),
        st.floats(min_value=0.001, max_value=0.05, allow_nan=False, allow_infinity=False),
        seeds,
    )
    @settings(max_examples=50, deadline=None)
    def test_lognormal_forecast_determinism(self, mean: float, std: float, seed: int) -> None:
        """Lognormal forecast with same parameters produces identical results."""
        assume(std > 0)

        def create_model() -> Any:
            builder = ModelBuilder.new("lognormal_det_test")
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
            assert abs(val1 - val2) < TOLERANCE_DETERMINISTIC


@pytest.mark.properties
class TestForecastMathematicalProperties:
    """Property tests for mathematical correctness of forecasts."""

    @given(
        st.floats(min_value=1000.0, max_value=1e9, allow_nan=False, allow_infinity=False),
        st.floats(min_value=0.01, max_value=0.05, allow_nan=False, allow_infinity=False),
        seeds,
    )
    @settings(max_examples=30, deadline=None)
    def test_lognormal_always_positive(self, base_value: float, std: float, seed: int) -> None:
        """Lognormal forecasts always produce positive values regardless of parameters."""
        assume(base_value > 0)
        assume(std > 0)

        builder = ModelBuilder.new("lognormal_pos_test")
        builder.periods("2024Q1..Q4", "2024Q1")
        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(base_value))])
        # Using small mean to test that even negative drift still produces positive values
        builder.forecast("revenue", ForecastSpec.lognormal(mean=0.0, std=std, seed=seed))
        model = builder.build()

        results = Evaluator.new().evaluate(model)

        for q in [2, 3, 4]:
            period = PeriodId.quarter(2024, q)
            value = results.get("revenue", period)
            assert value > 0, f"Q{q}: lognormal forecast should be positive, got {value}"

    @given(
        st.floats(min_value=1000.0, max_value=1e6, allow_nan=False, allow_infinity=False),
        st.floats(min_value=0.01, max_value=0.30, allow_nan=False, allow_infinity=False),
    )
    @settings(max_examples=30, deadline=None)
    def test_growth_forecast_monotonic(self, base_value: float, growth_rate: float) -> None:
        """Growth forecast with positive rate produces monotonically increasing values."""
        assume(base_value > 0)
        assume(growth_rate > 0)

        builder = ModelBuilder.new("growth_mono_test")
        builder.periods("2024Q1..Q4", "2024Q1")
        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(base_value))])
        builder.forecast("revenue", ForecastSpec.growth(growth_rate))
        model = builder.build()

        results = Evaluator.new().evaluate(model)

        values = [results.get("revenue", PeriodId.quarter(2024, q)) for q in [1, 2, 3, 4]]

        for i in range(len(values) - 1):
            assert values[i + 1] > values[i], (
                f"Growth forecast should be monotonic: Q{i + 1}={values[i]} -> Q{i + 2}={values[i + 1]}"
            )

    @given(
        st.floats(min_value=1000.0, max_value=1e6, allow_nan=False, allow_infinity=False),
        st.floats(min_value=-0.20, max_value=-0.01, allow_nan=False, allow_infinity=False),
    )
    @settings(max_examples=30, deadline=None)
    def test_negative_growth_forecast_monotonic_decreasing(self, base_value: float, growth_rate: float) -> None:
        """Growth forecast with negative rate produces monotonically decreasing values."""
        assume(base_value > 0)

        builder = ModelBuilder.new("neg_growth_test")
        builder.periods("2024Q1..Q4", "2024Q1")
        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(base_value))])
        builder.forecast("revenue", ForecastSpec.growth(growth_rate))
        model = builder.build()

        results = Evaluator.new().evaluate(model)

        values = [results.get("revenue", PeriodId.quarter(2024, q)) for q in [1, 2, 3, 4]]

        for i in range(len(values) - 1):
            assert values[i + 1] < values[i], (
                f"Negative growth should be monotonic decreasing: Q{i + 1}={values[i]} -> Q{i + 2}={values[i + 1]}"
            )

    @given(st.floats(min_value=1000.0, max_value=1e6, allow_nan=False, allow_infinity=False))
    @settings(max_examples=30, deadline=None)
    def test_forward_fill_constant(self, base_value: float) -> None:
        """Forward fill forecast produces constant values equal to the initial value."""
        assume(base_value > 0)

        builder = ModelBuilder.new("ff_const_test")
        builder.periods("2024Q1..Q4", "2024Q1")
        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(base_value))])
        builder.forecast("revenue", ForecastSpec.forward_fill())
        model = builder.build()

        results = Evaluator.new().evaluate(model)

        for q in [1, 2, 3, 4]:
            period = PeriodId.quarter(2024, q)
            value = results.get("revenue", period)
            assert abs(value - base_value) < TOLERANCE_DETERMINISTIC, (
                f"Q{q}: forward fill should equal base value {base_value}, got {value}"
            )


@pytest.mark.properties
class TestForecastGrowthRateAccuracy:
    """Property tests for growth rate calculation accuracy."""

    @given(
        st.floats(min_value=10000.0, max_value=1e6, allow_nan=False, allow_infinity=False),
        st.floats(min_value=0.01, max_value=0.20, allow_nan=False, allow_infinity=False),
    )
    @settings(max_examples=30, deadline=None)
    def test_growth_rate_exact(self, base_value: float, growth_rate: float) -> None:
        """Growth forecast applies exact growth rate between periods."""
        assume(base_value > 0)
        assume(growth_rate > 0)

        builder = ModelBuilder.new("growth_exact_test")
        builder.periods("2024Q1..Q4", "2024Q1")
        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(base_value))])
        builder.forecast("revenue", ForecastSpec.growth(growth_rate))
        model = builder.build()

        results = Evaluator.new().evaluate(model)

        # Check Q1 -> Q2 growth
        q1_val = results.get("revenue", PeriodId.quarter(2024, 1))
        q2_val = results.get("revenue", PeriodId.quarter(2024, 2))
        expected_q2 = q1_val * (1 + growth_rate)

        assert abs(q2_val - expected_q2) / expected_q2 < 0.001, (
            f"Q2 should be Q1 * (1 + {growth_rate}): expected {expected_q2}, got {q2_val}"
        )

        # Check Q2 -> Q3 growth
        q3_val = results.get("revenue", PeriodId.quarter(2024, 3))
        expected_q3 = q2_val * (1 + growth_rate)

        assert abs(q3_val - expected_q3) / expected_q3 < 0.001, (
            f"Q3 should be Q2 * (1 + {growth_rate}): expected {expected_q3}, got {q3_val}"
        )

    @given(
        st.floats(min_value=10000.0, max_value=1e6, allow_nan=False, allow_infinity=False),
        st.lists(
            st.floats(min_value=-0.10, max_value=0.20, allow_nan=False, allow_infinity=False),
            min_size=3,
            max_size=3,
        ),
    )
    @settings(max_examples=30, deadline=None)
    def test_curve_rates_applied_correctly(self, base_value: float, rates: list[float]) -> None:
        """Curve forecast applies period-specific rates correctly."""
        assume(base_value > 0)
        assume(all(r > -0.5 for r in rates))  # Avoid values going negative

        builder = ModelBuilder.new("curve_exact_test")
        builder.periods("2024Q1..Q4", "2024Q1")
        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(base_value))])
        builder.forecast("revenue", ForecastSpec.curve(rates))
        model = builder.build()

        results = Evaluator.new().evaluate(model)

        q1_val = results.get("revenue", PeriodId.quarter(2024, 1))

        # Q2 uses rates[0]
        expected_q2 = q1_val * (1 + rates[0])
        actual_q2 = results.get("revenue", PeriodId.quarter(2024, 2))
        if abs(expected_q2) > 1:
            assert abs(actual_q2 - expected_q2) / abs(expected_q2) < 0.001, (
                f"Q2: expected {expected_q2}, got {actual_q2}"
            )

        # Q3 uses rates[1]
        expected_q3 = expected_q2 * (1 + rates[1])
        actual_q3 = results.get("revenue", PeriodId.quarter(2024, 3))
        if abs(expected_q3) > 1:
            assert abs(actual_q3 - expected_q3) / abs(expected_q3) < 0.001, (
                f"Q3: expected {expected_q3}, got {actual_q3}"
            )

        # Q4 uses rates[2]
        expected_q4 = expected_q3 * (1 + rates[2])
        actual_q4 = results.get("revenue", PeriodId.quarter(2024, 4))
        if abs(expected_q4) > 1:
            assert abs(actual_q4 - expected_q4) / abs(expected_q4) < 0.001, (
                f"Q4: expected {expected_q4}, got {actual_q4}"
            )


@pytest.mark.properties
class TestMultiNodeForecasts:
    """Property tests for models with multiple forecast nodes."""

    @given(
        st.floats(min_value=10000.0, max_value=1e6, allow_nan=False, allow_infinity=False),
        st.floats(min_value=5000.0, max_value=5e5, allow_nan=False, allow_infinity=False),
        st.floats(min_value=0.01, max_value=0.15, allow_nan=False, allow_infinity=False),
        st.floats(min_value=0.01, max_value=0.10, allow_nan=False, allow_infinity=False),
    )
    @settings(max_examples=30, deadline=None)
    def test_multiple_growth_forecasts_independent(
        self, rev_base: float, exp_base: float, rev_growth: float, exp_growth: float
    ) -> None:
        """Multiple nodes with different growth rates are independent."""
        assume(rev_base > 0)
        assume(exp_base > 0)

        builder = ModelBuilder.new("multi_growth_test")
        builder.periods("2024Q1..Q4", "2024Q1")

        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(rev_base))])
        builder.forecast("revenue", ForecastSpec.growth(rev_growth))

        builder.value("expenses", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(exp_base))])
        builder.forecast("expenses", ForecastSpec.growth(exp_growth))

        model = builder.build()
        results = Evaluator.new().evaluate(model)

        # Check revenue follows its growth rate
        rev_q2 = results.get("revenue", PeriodId.quarter(2024, 2))
        expected_rev_q2 = rev_base * (1 + rev_growth)
        assert abs(rev_q2 - expected_rev_q2) / expected_rev_q2 < 0.001

        # Check expenses follows its growth rate
        exp_q2 = results.get("expenses", PeriodId.quarter(2024, 2))
        expected_exp_q2 = exp_base * (1 + exp_growth)
        assert abs(exp_q2 - expected_exp_q2) / expected_exp_q2 < 0.001

    @given(
        st.floats(min_value=10000.0, max_value=1e6, allow_nan=False, allow_infinity=False),
        seeds,
        seeds,
    )
    @settings(max_examples=30, deadline=None)
    def test_seeded_nodes_with_different_seeds_independent(self, base_value: float, seed1: int, seed2: int) -> None:
        """Nodes with different seeds produce independent random sequences."""
        assume(seed1 != seed2)

        builder = ModelBuilder.new("diff_seeds_test")
        builder.periods("2024Q1..Q4", "2024Q1")

        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(base_value))])
        builder.forecast("revenue", ForecastSpec.normal(base_value, base_value * 0.1, seed=seed1))

        builder.value("expenses", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(base_value * 0.6))])
        builder.forecast("expenses", ForecastSpec.normal(base_value * 0.6, base_value * 0.05, seed=seed2))

        model = builder.build()

        # Evaluate twice
        results1 = Evaluator.new().evaluate(model)
        results2 = Evaluator.new().evaluate(model)

        # Both nodes should be reproducible with their own seeds
        for q in [2, 3, 4]:
            period = PeriodId.quarter(2024, q)

            rev1 = results1.get("revenue", period)
            rev2 = results2.get("revenue", period)
            assert abs(rev1 - rev2) < TOLERANCE_DETERMINISTIC

            exp1 = results1.get("expenses", period)
            exp2 = results2.get("expenses", period)
            assert abs(exp1 - exp2) < TOLERANCE_DETERMINISTIC


@pytest.mark.properties
class TestFormulaWithForecast:
    """Property tests for formulas that depend on forecast nodes."""

    @given(
        st.floats(min_value=10000.0, max_value=1e6, allow_nan=False, allow_infinity=False),
        st.floats(min_value=0.01, max_value=0.20, allow_nan=False, allow_infinity=False),
        st.floats(min_value=0.4, max_value=0.8, allow_nan=False, allow_infinity=False),
    )
    @settings(max_examples=30, deadline=None)
    def test_formula_on_forecast_node(self, base_revenue: float, growth_rate: float, cogs_ratio: float) -> None:
        """Formulas correctly compute on forecast values."""
        assume(base_revenue > 0)

        builder = ModelBuilder.new("formula_forecast_test")
        builder.periods("2024Q1..Q4", "2024Q1")

        builder.value("revenue", [(PeriodId.quarter(2024, 1), AmountOrScalar.scalar(base_revenue))])
        builder.forecast("revenue", ForecastSpec.growth(growth_rate))

        builder.compute("cogs", f"revenue * {cogs_ratio}")
        builder.compute("gross_profit", "revenue - cogs")

        model = builder.build()
        results = Evaluator.new().evaluate(model)

        # Check Q2 (forecasted revenue)
        rev_q2 = results.get("revenue", PeriodId.quarter(2024, 2))
        expected_cogs_q2 = rev_q2 * cogs_ratio
        actual_cogs_q2 = results.get("cogs", PeriodId.quarter(2024, 2))

        assert abs(actual_cogs_q2 - expected_cogs_q2) / expected_cogs_q2 < 0.001, (
            f"COGS Q2: expected {expected_cogs_q2}, got {actual_cogs_q2}"
        )

        expected_gp_q2 = rev_q2 - expected_cogs_q2
        actual_gp_q2 = results.get("gross_profit", PeriodId.quarter(2024, 2))

        assert abs(actual_gp_q2 - expected_gp_q2) / abs(expected_gp_q2) < 0.001, (
            f"GP Q2: expected {expected_gp_q2}, got {actual_gp_q2}"
        )
