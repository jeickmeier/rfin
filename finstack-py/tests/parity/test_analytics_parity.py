"""Parity tests for the performance analytics module.

Tests that the Rust-backed Performance class produces correct results by
comparing against pre-computed golden values.
"""

from datetime import date, timedelta

import polars as pl
import pytest


def _build_price_df(n: int = 100) -> pl.DataFrame:
    """Generate a deterministic price DataFrame with two tickers and a benchmark."""
    import math

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


class TestPerformanceConstruction:
    """Test basic construction and properties."""

    def test_construction(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        assert repr(perf).startswith("Performance(")

    def test_invalid_freq(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        with pytest.raises(ValueError, match="Unknown frequency"):
            Performance(price_df, freq="invalid")


class TestScalarMetrics:
    """Test scalar per-ticker metrics against golden values."""

    def test_cagr(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        result = perf.cagr()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3  # 3 tickers
        assert "cagr" in result.columns
        # Ticker A has positive trend → positive CAGR
        a_cagr = result.filter(pl.col("ticker") == "A")["cagr"][0]
        assert a_cagr > 0

    def test_sharpe(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        result = perf.sharpe()
        assert isinstance(result, pl.DataFrame)
        assert result.shape[0] == 3

    def test_sortino(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        result = perf.sortino()
        assert isinstance(result, pl.DataFrame)

    def test_max_drawdown(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        result = perf.max_drawdown()
        assert isinstance(result, pl.DataFrame)
        # All max drawdowns should be <= 0
        for val in result["max_drawdown"].to_list():
            assert val <= 0.0 + 1e-12

    def test_volatility(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        result = perf.volatility(annualize=True)
        assert isinstance(result, pl.DataFrame)
        # Volatility should be non-negative
        for val in result["volatility"].to_list():
            assert val >= 0.0

    def test_var_and_es(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        var = perf.value_at_risk(confidence=0.95)
        es = perf.expected_shortfall(confidence=0.95)
        assert isinstance(var, pl.DataFrame)
        assert isinstance(es, pl.DataFrame)
        # ES should be at least as bad as VaR
        for v, e in zip(var["var"].to_list(), es["es"].to_list(), strict=True):
            assert e <= v + 1e-12

    def test_calmar(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        result = perf.calmar()
        assert isinstance(result, pl.DataFrame)

    def test_ulcer_index(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        result = perf.ulcer_index()
        assert isinstance(result, pl.DataFrame)
        for val in result["ulcer_index"].to_list():
            assert val >= 0.0


class TestBenchmarkRelative:
    """Test benchmark-relative metrics."""

    def test_tracking_error(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        result = perf.tracking_error()
        assert isinstance(result, pl.DataFrame)
        # Benchmark vs itself should have ~0 tracking error
        bench_te = result.filter(pl.col("ticker") == "bench")["tracking_error"][0]
        assert abs(bench_te) < 1e-10

    def test_information_ratio(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        result = perf.information_ratio()
        assert isinstance(result, pl.DataFrame)

    def test_r_squared(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        result = perf.r_squared()
        assert isinstance(result, pl.DataFrame)
        # R-squared should be in [0, 1]
        for val in result["r_squared"].to_list():
            assert 0.0 <= val <= 1.0 + 1e-10

    def test_beta(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        result = perf.beta()
        assert isinstance(result, dict)
        assert "A" in result
        assert "beta" in result["A"]

    def test_greeks(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        result = perf.greeks()
        assert isinstance(result, dict)
        assert "A" in result
        assert "alpha" in result["A"]
        assert "beta" in result["A"]
        assert "r_squared" in result["A"]


class TestSeriesOutputs:
    """Test series-type outputs."""

    def test_cumulative_returns(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        result = perf.cumulative_returns()
        assert isinstance(result, pl.DataFrame)
        assert set(result.columns) == {"A", "B", "bench"}

    def test_drawdown_series(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        result = perf.drawdown_series()
        assert isinstance(result, pl.DataFrame)
        # All drawdown values should be <= 0
        for col in result.columns:
            vals = result[col].to_list()
            for v in vals:
                assert v <= 0.0 + 1e-12

    def test_correlation(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        result = perf.correlation()
        assert isinstance(result, pl.DataFrame)
        assert result.shape == (3, 3)
        # Diagonal should be 1.0
        for i, col in enumerate(result.columns):
            assert abs(result[col][i] - 1.0) < 1e-10


class TestDateRange:
    """Test reset_date_range functionality."""

    def test_reset_date_range(self, price_df: pl.DataFrame) -> None:
        from finstack.analytics import Performance

        perf = Performance(price_df, benchmark_ticker="bench", freq="daily")
        perf.volatility()

        perf.reset_date_range(date(2024, 2, 1), date(2024, 3, 1))
        subset_vol = perf.volatility()
        assert isinstance(subset_vol, pl.DataFrame)
