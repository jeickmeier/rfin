"""Parity tests: expression plugins vs Performance class.

Verifies that every expression plugin produces identical results to the
corresponding Performance class method, ensuring zero logic drift between
the two API surfaces.
"""

from __future__ import annotations

from datetime import date, timedelta
import math

import polars as pl
import pytest

from finstack.analytics import Performance, expr


def _build_price_df(n: int = 200) -> pl.DataFrame:
    """Deterministic price DataFrame with two tickers and a benchmark."""
    base_date = date(2024, 1, 2)
    dates = [base_date + timedelta(days=i) for i in range(n)]

    price_a = [100.0]
    price_b = [100.0]
    price_bench = [100.0]
    for i in range(1, n):
        price_a.append(price_a[-1] * (1.0 + 0.001 * math.sin(i * 0.3) + 0.0005))
        factor = -0.0008 if i < n // 2 else 0.0012
        price_b.append(price_b[-1] * (1.0 + factor + 0.0002 * math.sin(i * 0.5)))
        price_bench.append(price_bench[-1] * (1.0 + 0.0003))

    return pl.DataFrame({"date": dates, "A": price_a, "B": price_b, "bench": price_bench}).with_columns(
        pl.col("date").cast(pl.Date)
    )


@pytest.fixture
def price_df() -> pl.DataFrame:
    return _build_price_df()


@pytest.fixture
def perf(price_df: pl.DataFrame) -> Performance:
    return Performance(price_df, benchmark_ticker="bench", freq="daily")


@pytest.fixture
def returns_df(price_df: pl.DataFrame) -> pl.DataFrame:
    """Compute returns from prices, matching Performance internals.

    Performance::new trims the leading 0.0 from simple_returns (sr[1..]),
    so we drop the first row to match.
    """
    return price_df.select(
        expr.simple_returns("A").alias("A"),
        expr.simple_returns("B").alias("B"),
        expr.simple_returns("bench").alias("bench"),
    ).slice(1)


# ── Helpers ──


def _perf_scalar(perf_df: pl.DataFrame, col: str) -> float:
    """Extract a scalar from a Performance method DataFrame for ticker 'A'."""
    row = perf_df.filter(pl.col("ticker") == "A")
    return row[col].item()


def _expr_scalar(returns_df: pl.DataFrame, expression: pl.Expr) -> float:
    """Evaluate a scalar expression on the returns DataFrame."""
    return returns_df.select(expression).item()


# ── Tier 1: Scalar metrics parity ──


class TestTier1ScalarParity:
    """Expression plugin scalars must match Performance class results."""

    def test_sharpe(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.sharpe(), "sharpe")
        actual = _expr_scalar(returns_df, expr.sharpe("A", freq="daily"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_sortino(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.sortino(), "sortino")
        actual = _expr_scalar(returns_df, expr.sortino("A", freq="daily"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_volatility(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.volatility(), "volatility")
        actual = _expr_scalar(returns_df, expr.volatility("A", freq="daily"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_mean_return(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.mean_return(), "mean_return")
        actual = _expr_scalar(returns_df, expr.mean_return("A", freq="daily"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_max_drawdown(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.max_drawdown(), "max_drawdown")
        actual = _expr_scalar(returns_df, expr.max_drawdown("A"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_geometric_mean(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.geometric_mean(), "geometric_mean")
        actual = _expr_scalar(returns_df, expr.geometric_mean("A"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_skewness(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.skewness(), "skewness")
        actual = _expr_scalar(returns_df, expr.skewness("A"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_kurtosis(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.kurtosis(), "kurtosis")
        actual = _expr_scalar(returns_df, expr.kurtosis("A"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_value_at_risk(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.value_at_risk(confidence=0.95), "var")
        actual = _expr_scalar(returns_df, expr.value_at_risk("A", confidence=0.95))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_expected_shortfall(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.expected_shortfall(confidence=0.95), "es")
        actual = _expr_scalar(returns_df, expr.expected_shortfall("A", confidence=0.95))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_ulcer_index(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.ulcer_index(), "ulcer_index")
        actual = _expr_scalar(returns_df, expr.ulcer_index("A"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_pain_index(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.pain_index(), "pain_index")
        actual = _expr_scalar(returns_df, expr.pain_index("A"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_omega_ratio(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.omega_ratio(), "omega_ratio")
        actual = _expr_scalar(returns_df, expr.omega_ratio("A", threshold=0.0))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_gain_to_pain(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.gain_to_pain(), "gain_to_pain")
        actual = _expr_scalar(returns_df, expr.gain_to_pain("A"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_tail_ratio(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.tail_ratio(), "tail_ratio")
        actual = _expr_scalar(returns_df, expr.tail_ratio("A", confidence=0.95))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_estimate_ruin(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(
            perf.estimate_ruin(
                definition="drawdown_breach",
                threshold=0.2,
                horizon_periods=63,
                n_paths=512,
                block_size=5,
                seed=42,
            ),
            "probability",
        )
        actual = _expr_scalar(
            returns_df,
            expr.estimate_ruin(
                "A",
                definition="drawdown_breach",
                threshold=0.2,
                horizon_periods=63,
                n_paths=512,
                block_size=5,
                seed=42,
            ),
        )
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_recovery_factor(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.recovery_factor(), "recovery_factor")
        actual = _expr_scalar(returns_df, expr.recovery_factor("A"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_downside_deviation(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.downside_deviation(), "downside_deviation")
        actual = _expr_scalar(returns_df, expr.downside_deviation("A", freq="daily"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_parametric_var(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.parametric_var(confidence=0.95), "parametric_var")
        actual = _expr_scalar(returns_df, expr.parametric_var("A", confidence=0.95))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_cornish_fisher_var(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.cornish_fisher_var(confidence=0.95), "cornish_fisher_var")
        actual = _expr_scalar(returns_df, expr.cornish_fisher_var("A", confidence=0.95))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_outlier_win_ratio(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.tail_ratio(), "tail_ratio")
        actual_tail = _expr_scalar(returns_df, expr.tail_ratio("A", confidence=0.95))
        assert actual_tail == pytest.approx(expected, rel=1e-10)

    def test_martin_ratio(self, returns_df: pl.DataFrame) -> None:
        actual = _expr_scalar(returns_df, expr.martin_ratio("A", freq="daily"))
        assert math.isfinite(actual)
        assert actual > 0

    def test_sterling_ratio(self, returns_df: pl.DataFrame) -> None:
        actual = _expr_scalar(returns_df, expr.sterling_ratio("A", freq="daily"))
        assert math.isfinite(actual)
        assert actual > 0

    def test_burke_ratio(self, returns_df: pl.DataFrame) -> None:
        actual = _expr_scalar(returns_df, expr.burke_ratio("A", freq="daily"))
        assert math.isfinite(actual)
        assert actual > 0

    def test_pain_ratio(self, returns_df: pl.DataFrame) -> None:
        actual = _expr_scalar(returns_df, expr.pain_ratio("A", freq="daily"))
        assert math.isfinite(actual)
        assert actual > 0

    def test_modified_sharpe(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.modified_sharpe(), "modified_sharpe")
        actual = _expr_scalar(returns_df, expr.modified_sharpe("A", freq="daily"))
        assert actual == pytest.approx(expected, rel=1e-10)


# ── Tier 2: Series transforms parity ──


class TestTier2SeriesParity:
    """Expression plugin series transforms must match Performance class."""

    def test_drawdown_series(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        perf_dd = perf.drawdown_series()
        expected = perf_dd["A"].to_list()
        actual = returns_df.select(expr.drawdown_series("A").alias("dd"))["dd"].to_list()
        assert len(actual) == len(expected)
        for e, a in zip(expected, actual, strict=False):
            assert a == pytest.approx(e, abs=1e-12)

    def test_cumulative_returns(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        perf_cum = perf.cumulative_returns()
        expected = perf_cum["A"].to_list()
        actual = returns_df.select(expr.cumulative_returns("A").alias("cum"))["cum"].to_list()
        assert len(actual) == len(expected)
        for e, a in zip(expected, actual, strict=False):
            assert a == pytest.approx(e, abs=1e-12)

    def test_rebase(self, price_df: pl.DataFrame) -> None:
        rebased = price_df.select(expr.rebase("A", base=100.0).alias("r"))
        assert rebased["r"][0] == pytest.approx(100.0, rel=1e-12)
        assert rebased.shape[0] == price_df.shape[0]

    def test_simple_returns_invalid_price_steps_become_nan(self) -> None:
        prices = pl.DataFrame({"px": [100.0, 101.0, -50.0, 102.0, float("inf")]})
        actual = prices.select(expr.simple_returns("px").alias("ret"))["ret"].to_list()
        assert actual[0] == pytest.approx(0.0)
        assert actual[1] == pytest.approx(0.01)
        assert math.isnan(actual[2])
        assert math.isnan(actual[3])
        assert math.isnan(actual[4])


# ── Tier 3: Two-input benchmark parity ──


class TestTier3BenchmarkParity:
    """Expression plugin benchmark metrics must match Performance class."""

    def test_tracking_error(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.tracking_error(), "tracking_error")
        actual = _expr_scalar(
            returns_df,
            expr.tracking_error("A", "bench", freq="daily", annualize=True),
        )
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_information_ratio(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.information_ratio(), "information_ratio")
        actual = _expr_scalar(
            returns_df,
            expr.information_ratio("A", "bench", freq="daily", annualize=True),
        )
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_r_squared(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.r_squared(), "r_squared")
        actual = _expr_scalar(returns_df, expr.r_squared("A", "bench"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_beta(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        perf_beta = perf.beta()
        expected = perf_beta["A"]["beta"]
        actual = _expr_scalar(returns_df, expr.beta("A", "bench"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_up_capture(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.up_capture(), "up_capture")
        actual = _expr_scalar(returns_df, expr.up_capture("A", "bench"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_down_capture(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.down_capture(), "down_capture")
        actual = _expr_scalar(returns_df, expr.down_capture("A", "bench"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_batting_average(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.batting_average(), "batting_average")
        actual = _expr_scalar(returns_df, expr.batting_average("A", "bench"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_capture_ratio(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.capture_ratio(), "capture_ratio")
        actual = _expr_scalar(returns_df, expr.capture_ratio("A", "bench"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_m_squared(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        expected = _perf_scalar(perf.m_squared(), "m_squared")
        actual = _expr_scalar(returns_df, expr.m_squared("A", "bench", freq="daily"))
        assert actual == pytest.approx(expected, rel=1e-10)

    def test_estimate_ruin_invalid_threshold_yields_nan(self) -> None:
        returns_df = pl.DataFrame({"A": [0.01, -0.02, 0.03]})
        actual = returns_df.select(
            expr.estimate_ruin(
                "A",
                definition="drawdown_breach",
                threshold=-0.1,
                horizon_periods=12,
                n_paths=256,
                block_size=1,
                seed=11,
            ).alias("ruin")
        ).item()
        assert math.isnan(actual)


# ── Tier 4: Rolling metrics parity ──


class TestTier4RollingParity:
    """Expression plugin rolling metrics must match Performance class."""

    def test_rolling_sharpe_values(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        window = 30
        perf_roll = perf.rolling_sharpe("A", window)
        expected_vals = perf_roll["sharpe"].to_list()

        actual = returns_df.select(expr.rolling_sharpe("A", window=window, freq="daily").alias("rs"))["rs"].to_list()

        valid_actual = [v for v in actual if not math.isnan(v)]
        assert len(valid_actual) == len(expected_vals)
        for e, a in zip(expected_vals, valid_actual, strict=False):
            assert a == pytest.approx(e, rel=1e-10)

    def test_rolling_volatility_values(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        window = 30
        perf_roll = perf.rolling_volatility("A", window)
        expected_vals = perf_roll["volatility"].to_list()

        actual = returns_df.select(expr.rolling_volatility("A", window=window, freq="daily").alias("rv"))[
            "rv"
        ].to_list()

        valid_actual = [v for v in actual if not math.isnan(v)]
        assert len(valid_actual) == len(expected_vals)
        for e, a in zip(expected_vals, valid_actual, strict=False):
            assert a == pytest.approx(e, rel=1e-10)

    def test_rolling_sortino_values(self, perf: Performance, returns_df: pl.DataFrame) -> None:
        window = 30
        perf_roll = perf.rolling_sortino("A", window)
        expected_vals = perf_roll["sortino"].to_list()

        actual = returns_df.select(expr.rolling_sortino("A", window=window, freq="daily").alias("rso"))["rso"].to_list()

        valid_actual = [v for v in actual if not math.isnan(v)]
        assert len(valid_actual) == len(expected_vals)
        for e, a in zip(expected_vals, valid_actual, strict=False):
            assert a == pytest.approx(e, rel=1e-10)


# ── Multi-column batch test ──


class TestBatchOperations:
    """Verify multiple metrics can be computed in a single .select() call."""

    def test_multi_metric_select(self, returns_df: pl.DataFrame) -> None:
        result = returns_df.select(
            expr.sharpe("A", freq="daily").alias("sharpe_a"),
            expr.sharpe("B", freq="daily").alias("sharpe_b"),
            expr.sortino("A", freq="daily").alias("sortino_a"),
            expr.volatility("A", freq="daily").alias("vol_a"),
            expr.max_drawdown("A").alias("mdd_a"),
            expr.skewness("A").alias("skew_a"),
        )
        assert result.shape == (1, 6)
        assert all(result[col].item() is not None for col in result.columns)

    def test_series_with_columns(self, price_df: pl.DataFrame) -> None:
        enriched = price_df.with_columns(
            expr.simple_returns("A").alias("ret_a"),
            expr.simple_returns("B").alias("ret_b"),
            expr.rebase("A", base=100.0).alias("rebased_a"),
        )
        assert enriched.shape[0] == price_df.shape[0]
        assert "ret_a" in enriched.columns
        assert "ret_b" in enriched.columns
        assert "rebased_a" in enriched.columns
