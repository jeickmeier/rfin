"""Parity tests for the performance analytics module.

Tests that the Rust-backed Performance class produces correct results by
comparing against pre-computed golden values.
"""

from __future__ import annotations

from datetime import date, timedelta
import math
from typing import TYPE_CHECKING

import polars as pl
import pytest

import finstack

if TYPE_CHECKING:
    from finstack.analytics import Performance


def _build_price_df(n: int = 100) -> pl.DataFrame:
    """Generate a deterministic price DataFrame with two tickers and a benchmark."""
    base_date = date(2024, 1, 2)
    dates = [base_date + timedelta(days=i) for i in range(n)]

    # Deterministic price paths (no randomness)
    price_a = [100.0]
    price_b = [100.0]
    price_bench = [100.0]
    for i in range(1, n):
        # ticker A: uptrend with oscillation
        price_a.append(price_a[-1] * (1.0 + 0.001 * math.sin(i * 0.3) + 0.0005))
        # ticker B: downtrend then recovery
        factor = -0.0008 if i < n // 2 else 0.0012
        price_b.append(price_b[-1] * (1.0 + factor + 0.0002 * math.sin(i * 0.5)))
        # benchmark: slow steady growth
        price_bench.append(price_bench[-1] * (1.0 + 0.0003))

    return pl.DataFrame({
        "date": dates,
        "A": price_a,
        "B": price_b,
        "bench": price_bench,
    }).with_columns(pl.col("date").cast(pl.Date))


@pytest.fixture
def price_df() -> pl.DataFrame:
    return _build_price_df()


def _make_perf(price_df: pl.DataFrame) -> Performance:
    from finstack.analytics import Performance as _Perf

    return _Perf(price_df, benchmark_ticker="bench", freq="daily")


class TestPerformanceConstruction:
    """Test basic construction and properties."""

    def test_construction(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        assert repr(perf).startswith("Performance(")

    def test_invalid_freq(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        with pytest.raises(ValueError, match="Unknown frequency"):
            Performance(price_df, freq="invalid")

    def test_null_prices_rejected(self) -> None:
        from finstack.analytics import Performance

        df = pl.DataFrame({
            "date": [date(2024, 1, 1), date(2024, 1, 2), date(2024, 1, 3)],
            "A": [100.0, None, 102.0],
        }).with_columns(pl.col("date").cast(pl.Date))
        with pytest.raises(ValueError, match="null values"):
            Performance(df, freq="daily")

    def test_negative_simple_return_prices_rejected(self) -> None:
        from finstack.analytics import Performance

        df = pl.DataFrame({
            "date": [date(2024, 1, 1), date(2024, 1, 2)],
            "A": [-100.0, -90.0],
        }).with_columns(pl.col("date").cast(pl.Date))
        with pytest.raises((ValueError, finstack.ParameterError), match="Invalid input data"):
            Performance(df, freq="daily", log_returns=False)

    def test_non_finite_simple_return_prices_rejected(self) -> None:
        from finstack.analytics import Performance

        df = pl.DataFrame({
            "date": [date(2024, 1, 1), date(2024, 1, 2)],
            "A": [float("inf"), 101.0],
        }).with_columns(pl.col("date").cast(pl.Date))
        with pytest.raises((ValueError, finstack.ParameterError), match="Invalid input data"):
            Performance(df, freq="daily", log_returns=False)


class TestAccessors:
    """Test property accessors."""

    def test_ticker_names(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        assert perf.ticker_names == ["A", "B", "bench"]

    def test_benchmark_idx(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        assert perf.benchmark_idx == 2

    def test_freq(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        assert perf.freq == "daily"

    def test_log_returns(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        assert perf.log_returns is False

    def test_dates(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        dates_df = perf.dates
        assert isinstance(dates_df, pl.DataFrame)
        assert "date" in dates_df.columns


class TestScalarMetrics:
    """Test scalar per-ticker metrics against golden values."""

    def test_cagr(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.cagr()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3
        assert "cagr" in result.columns
        a_cagr = result.filter(pl.col("ticker") == "A")["cagr"][0]
        assert a_cagr > 0

    def test_sharpe(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.sharpe()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_sortino(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.sortino()
        assert isinstance(result, pl.DataFrame)

    def test_max_drawdown(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.max_drawdown()
        assert isinstance(result, pl.DataFrame)
        for val in result["max_drawdown"].to_list():
            assert val <= 0.0 + 1e-12

    def test_volatility(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.volatility(annualize=True)
        assert isinstance(result, pl.DataFrame)
        for val in result["volatility"].to_list():
            assert val >= 0.0

    def test_var_and_es(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        var = perf.value_at_risk(confidence=0.95)
        es = perf.expected_shortfall(confidence=0.95)
        assert isinstance(var, pl.DataFrame)
        assert isinstance(es, pl.DataFrame)
        for v, e in zip(var["var"].to_list(), es["es"].to_list(), strict=True):
            assert e <= v + 1e-12

    def test_calmar(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.calmar()
        assert isinstance(result, pl.DataFrame)

    def test_ulcer_index(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.ulcer_index()
        assert isinstance(result, pl.DataFrame)
        for val in result["ulcer_index"].to_list():
            assert val >= 0.0

    def test_tail_ratio(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.tail_ratio()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_estimate_ruin(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.estimate_ruin(
            definition="drawdown_breach",
            threshold=0.2,
            horizon_periods=63,
            n_paths=512,
            block_size=5,
            seed=42,
        )
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3
        assert {"probability", "std_err", "ci_lower", "ci_upper"}.issubset(result.columns)

    def test_estimate_ruin_invalid_threshold_produces_nan(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.estimate_ruin(
            definition="drawdown_breach",
            threshold=-0.1,
            horizon_periods=63,
            n_paths=512,
            block_size=5,
            seed=42,
        )
        assert isinstance(result, pl.DataFrame)
        assert all(math.isnan(value) for value in result["probability"].to_list())

    def test_skewness(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.skewness()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_kurtosis(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.kurtosis()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_geometric_mean(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.geometric_mean()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_downside_deviation(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.downside_deviation(mar=0.0)
        assert isinstance(result, pl.DataFrame)
        for val in result["downside_deviation"].to_list():
            assert val >= 0.0

    def test_max_drawdown_duration(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.max_drawdown_duration()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_omega_ratio(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.omega_ratio()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_treynor(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.treynor()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_gain_to_pain(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.gain_to_pain()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_martin_ratio(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.martin_ratio()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_parametric_var(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.parametric_var()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_cornish_fisher_var(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.cornish_fisher_var()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_recovery_factor(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.recovery_factor()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_sterling_ratio(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.sterling_ratio()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_burke_ratio(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.burke_ratio()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_pain_index(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.pain_index()
        assert isinstance(result, pl.DataFrame)
        for val in result["pain_index"].to_list():
            assert val >= 0.0

    def test_pain_ratio(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.pain_ratio()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_cdar(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.cdar()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_m_squared(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.m_squared()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_modified_sharpe(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.modified_sharpe()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3


class TestBenchmarkRelative:
    """Test benchmark-relative metrics."""

    def test_tracking_error(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.tracking_error()
        assert isinstance(result, pl.DataFrame)
        bench_te = result.filter(pl.col("ticker") == "bench")["tracking_error"][0]
        assert abs(bench_te) < 1e-10

    def test_information_ratio(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.information_ratio()
        assert isinstance(result, pl.DataFrame)

    def test_r_squared(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.r_squared()
        assert isinstance(result, pl.DataFrame)
        for val in result["r_squared"].to_list():
            assert 0.0 <= val <= 1.0 + 1e-10

    def test_beta(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.beta()
        assert isinstance(result, dict)
        assert "A" in result
        assert "beta" in result["A"]

    def test_greeks(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.greeks()
        assert isinstance(result, dict)
        assert "A" in result
        assert "alpha" in result["A"]
        assert "beta" in result["A"]
        assert "r_squared" in result["A"]

    def test_up_capture(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.up_capture()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_down_capture(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.down_capture()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_capture_ratio(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.capture_ratio()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_batting_average(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.batting_average()
        assert isinstance(result, pl.DataFrame)
        for val in result["batting_average"].to_list():
            assert 0.0 <= val <= 1.0


class TestStandaloneBindings:
    """Test standalone analytics helpers exposed directly to Python."""

    def test_simple_returns_marks_invalid_price_steps_as_nan(self) -> None:
        from finstack.analytics import simple_returns

        result = simple_returns([100.0, 101.0, -50.0, 102.0, float("inf")])
        assert result[0] == pytest.approx(0.0)
        assert result[1] == pytest.approx(0.01)
        assert math.isnan(result[2])
        assert math.isnan(result[3])
        assert math.isnan(result[4])

    def test_parametric_var_rejects_invalid_ann_factor(self) -> None:
        from finstack.analytics import parametric_var

        returns = [-0.03, -0.01, 0.01, 0.02]
        assert math.isnan(parametric_var(returns, 0.95, 0.0))
        assert math.isnan(parametric_var(returns, 0.95, -12.0))
        assert math.isnan(parametric_var(returns, 0.95, float("inf")))

    def test_cornish_fisher_var_rejects_invalid_ann_factor(self) -> None:
        from finstack.analytics import cornish_fisher_var

        returns = [-0.03, -0.01, 0.01, 0.02]
        assert math.isnan(cornish_fisher_var(returns, 0.95, 0.0))
        assert math.isnan(cornish_fisher_var(returns, 0.95, -12.0))
        assert math.isnan(cornish_fisher_var(returns, 0.95, float("inf")))

    def test_up_capture_uses_geometric_subset_returns(self) -> None:
        from finstack.analytics import up_capture

        returns = [0.04, -0.10, 0.06]
        benchmark = [0.02, -0.05, 0.03]
        expected = ((1.04 * 1.06) ** 0.5 - 1.0) / ((1.02 * 1.03) ** 0.5 - 1.0)
        assert up_capture(returns, benchmark) == pytest.approx(expected, rel=1e-12)

    def test_down_capture_uses_geometric_subset_returns(self) -> None:
        from finstack.analytics import down_capture

        returns = [0.03, -0.10, -0.20]
        benchmark = [0.01, -0.05, -0.10]
        expected = ((0.90 * 0.80) ** 0.5 - 1.0) / ((0.95 * 0.90) ** 0.5 - 1.0)
        assert down_capture(returns, benchmark) == pytest.approx(expected, rel=1e-12)


class TestSeriesOutputs:
    """Test series-type outputs."""

    def test_cumulative_returns(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.cumulative_returns()
        assert isinstance(result, pl.DataFrame)
        assert "date" in result.columns
        assert set(result.columns) == {"date", "A", "B", "bench"}

    def test_drawdown_series(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.drawdown_series()
        assert isinstance(result, pl.DataFrame)
        assert "date" in result.columns
        for col in ["A", "B", "bench"]:
            vals = result[col].to_list()
            for v in vals:
                assert v <= 0.0 + 1e-12

    def test_correlation(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.correlation()
        assert isinstance(result, pl.DataFrame)
        assert "ticker" in result.columns
        assert result.shape == (3, 4)  # ticker col + 3 value cols

    def test_cumulative_returns_outperformance(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.cumulative_returns_outperformance()
        assert isinstance(result, pl.DataFrame)
        assert "date" in result.columns
        bench_col = result["bench"].to_list()
        for v in bench_col:
            assert abs(v) < 1e-10

    def test_drawdown_outperformance(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.drawdown_outperformance()
        assert isinstance(result, pl.DataFrame)
        assert "date" in result.columns

    def test_excess_returns(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        n = len(perf.dates)
        rf = [0.0001] * n
        result = perf.excess_returns(rf)
        assert isinstance(result, pl.DataFrame)
        assert "date" in result.columns


class TestRollingMetrics:
    """Test per-ticker rolling metrics."""

    def test_rolling_volatility(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.rolling_volatility("A", window=20)
        assert isinstance(result, pl.DataFrame)
        assert "date" in result.columns
        assert "volatility" in result.columns

    def test_rolling_sortino(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.rolling_sortino("A", window=20)
        assert isinstance(result, pl.DataFrame)
        assert "date" in result.columns
        assert "sortino" in result.columns

    def test_rolling_sharpe(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.rolling_sharpe("A", window=20)
        assert isinstance(result, pl.DataFrame)
        assert "date" in result.columns
        assert "sharpe" in result.columns

    def test_rolling_greeks(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.rolling_greeks("A", window=20)
        assert isinstance(result, pl.DataFrame)
        assert "date" in result.columns
        assert "alpha" in result.columns
        assert "beta" in result.columns

    def test_unknown_ticker_raises(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        with pytest.raises(ValueError, match="Unknown ticker"):
            perf.rolling_volatility("DOESNOTEXIST", window=20)


class TestDrawdownDetails:
    """Test drawdown episode extraction."""

    def test_drawdown_details(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        episodes = perf.drawdown_details("B", n=3)
        assert isinstance(episodes, list)
        if episodes:
            ep = episodes[0]
            assert "start" in ep
            assert "valley" in ep
            assert "max_drawdown" in ep
            assert ep["max_drawdown"] < 0.0

    def test_stats_during_bench_drawdowns(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        episodes = perf.stats_during_bench_drawdowns(n=3)
        assert isinstance(episodes, list)


class TestMultiFactorGreeks:
    """Test multi-factor regression."""

    def test_multi_factor_greeks(self, price_df: pl.DataFrame) -> None:
        import math

        perf = _make_perf(price_df)
        n = len(perf.dates)
        factor1 = [0.001 * math.sin(i * 0.2) for i in range(n)]
        factor2 = [0.0005 * math.cos(i * 0.3) for i in range(n)]
        factors_df = pl.DataFrame({"mkt": factor1, "smb": factor2})

        result = perf.multi_factor_greeks("A", factors_df)
        assert isinstance(result, dict)
        assert "alpha" in result
        assert "betas" in result
        assert "r_squared" in result
        assert "adjusted_r_squared" in result
        assert "residual_vol" in result
        assert len(result["betas"]) == 2


class TestLookbackAndAggregation:
    """Test lookback returns and period stats."""

    def test_lookback_returns(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.lookback_returns(date(2024, 3, 15))
        assert isinstance(result, dict)
        assert "mtd" in result
        assert "qtd" in result
        assert "ytd" in result
        assert len(result["mtd"]) == 3

    def test_period_stats(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        result = perf.period_stats("A", agg_freq="monthly")
        assert isinstance(result, dict)
        assert "win_rate" in result
        assert "kelly_criterion" in result
        assert 0.0 <= result["win_rate"] <= 1.0


class TestDateRange:
    """Test reset_date_range functionality."""

    def test_reset_date_range(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        perf.volatility()

        perf.reset_date_range(date(2024, 2, 1), date(2024, 3, 1))
        subset_vol = perf.volatility()
        assert isinstance(subset_vol, pl.DataFrame)

    def test_reset_bench_ticker(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        assert perf.benchmark_idx == 2
        perf.reset_bench_ticker("A")
        assert perf.benchmark_idx == 0

    def test_series_after_date_range_reset(self, price_df: pl.DataFrame) -> None:
        """Series outputs must align with the active date range."""
        perf = _make_perf(price_df)
        full_dates = perf.dates.shape[0]

        perf.reset_date_range(date(2024, 2, 1), date(2024, 3, 1))
        subset_dates = perf.dates.shape[0]
        assert subset_dates < full_dates

        cum = perf.cumulative_returns()
        assert cum.shape[0] == subset_dates

        dd = perf.drawdown_series()
        assert dd.shape[0] == subset_dates

        outperf = perf.cumulative_returns_outperformance()
        assert outperf.shape[0] == subset_dates

        dd_outperf = perf.drawdown_outperformance()
        assert dd_outperf.shape[0] == subset_dates

        rf = [0.0001] * subset_dates
        er = perf.excess_returns(rf)
        assert er.shape[0] == subset_dates

    def test_repr_after_date_range_reset(self, price_df: pl.DataFrame) -> None:
        """Repr n_dates should reflect the active window."""
        perf = _make_perf(price_df)
        full_dates = perf.dates.shape[0]
        assert f"n_dates={full_dates}" in repr(perf)

        perf.reset_date_range(date(2024, 2, 1), date(2024, 3, 1))
        subset_dates = perf.dates.shape[0]
        assert f"n_dates={subset_dates}" in repr(perf)
        assert subset_dates < full_dates


class TestRepr:
    """Test repr output."""

    def test_repr_contains_info(self, price_df: pl.DataFrame) -> None:
        perf = _make_perf(price_df)
        r = repr(perf)
        assert "Performance(" in r
        assert "bench" in r
        assert "daily" in r
        assert "n_dates=" in r
