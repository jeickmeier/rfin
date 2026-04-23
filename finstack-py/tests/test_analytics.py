"""Tests for analytics functions: returns, risk metrics, drawdowns."""

from datetime import date
from pathlib import Path
from typing import cast

import pytest

from finstack.analytics import (
    RuinModel,
    classify_breaches,
    comp_sum,
    comp_total,
    compare_var_backtests,
    compute_multiple,
    fytd_select,
    max_drawdown,
    mean_return,
    mtd_select,
    percentile_rank,
    pnl_explanation,
    qtd_select,
    regression_fair_value,
    rolling_var_forecasts,
    score_relative_value,
    sharpe,
    simple_returns,
    sortino,
    to_drawdown_series,
    volatility,
    ytd_select,
)


def _max_dd(returns: list[float]) -> float:
    """Convenience: compose max_drawdown with to_drawdown_series."""
    return max_drawdown(to_drawdown_series(returns))


class TestSimpleReturns:
    """Validate simple return computation from price series."""

    def test_basic_price_series(self) -> None:
        """Simple returns from [100, 110, 99] include a leading zero."""
        prices = [100.0, 110.0, 99.0]
        rets = simple_returns(prices)
        assert len(rets) == 3
        assert rets[0] == pytest.approx(0.0)
        assert rets[1] == pytest.approx(0.1)
        assert rets[2] == pytest.approx(-0.1, abs=1e-10)

    def test_constant_prices(self) -> None:
        """Constant prices yield zero returns."""
        prices = [50.0, 50.0, 50.0, 50.0]
        rets = simple_returns(prices)
        assert all(r == pytest.approx(0.0) for r in rets)

    def test_single_price(self) -> None:
        """Single price yields a series with one zero entry."""
        rets = simple_returns([100.0])
        assert len(rets) == 1
        assert rets[0] == pytest.approx(0.0)

    def test_stub_matches_runtime_contract(self) -> None:
        """The stub documents the leading-zero, same-length runtime shape."""
        stub_path = Path(__file__).resolve().parents[1] / "finstack" / "analytics" / "__init__.pyi"
        stub_text = stub_path.read_text()

        assert simple_returns([100.0, 101.0]) == pytest.approx([0.0, 0.01])
        assert "Simple returns (same length as ``prices``)." in stub_text
        assert ">>> simple_returns([100.0, 101.0])" in stub_text
        assert "[0.0, 0.01]" in stub_text


class TestVolatility:
    """Validate annualised volatility."""

    def test_zero_returns(self) -> None:
        """Zero-variance returns yield zero volatility."""
        rets = [0.0, 0.0, 0.0, 0.0, 0.0]
        assert volatility(rets) == pytest.approx(0.0)

    def test_positive_for_nonzero_returns(self) -> None:
        """Nonzero returns produce positive volatility."""
        rets = [0.01, -0.02, 0.015, -0.005, 0.01]
        assert volatility(rets) > 0.0

    def test_annualisation_factor(self) -> None:
        """Halving the annualisation factor reduces volatility by sqrt(2)."""
        rets = [0.01, -0.02, 0.015, -0.005, 0.01]
        v252 = volatility(rets, annualize=True, ann_factor=252.0)
        v126 = volatility(rets, annualize=True, ann_factor=126.0)
        ratio = v252 / v126
        assert ratio == pytest.approx(2.0**0.5, rel=1e-6)


class TestSharpe:
    """Validate annualised Sharpe ratio."""

    def test_positive_returns(self) -> None:
        """Consistently positive returns should yield positive Sharpe."""
        assert sharpe(0.10, 0.15, 0.0) > 0.0

    def test_zero_vol(self) -> None:
        """Sharpe with zero vol is NaN or inf."""
        import math

        s = sharpe(0.05, 0.0, 0.0)
        assert math.isinf(s) or math.isnan(s) or s == 0.0

    def test_rf_reduces_sharpe(self) -> None:
        """Higher risk-free rate reduces the Sharpe ratio."""
        s_low = sharpe(0.10, 0.15, 0.0)
        s_high = sharpe(0.10, 0.15, 0.05)
        assert s_low > s_high


class TestSortino:
    """Validate annualised Sortino ratio."""

    def test_all_positive(self) -> None:
        """No downside returns should yield a very large Sortino (or inf)."""
        rets = [0.01, 0.02, 0.03, 0.015]
        s = sortino(rets)
        assert s > 0.0

    def test_mixed_returns(self) -> None:
        """Mixed positive/negative returns should produce a finite Sortino."""
        rets = [0.01, -0.02, 0.015, -0.005, 0.01]
        s = sortino(rets)
        assert isinstance(s, float)

    def test_mar_changes_sortino(self) -> None:
        """Raising the minimum acceptable return should tighten the ratio."""
        rets = [0.01, 0.02, 0.03, 0.04]
        baseline = sortino(rets, annualize=False, ann_factor=252.0, mar=0.0)
        hurdle = sortino(rets, annualize=False, ann_factor=252.0, mar=0.02)
        assert hurdle < baseline


class TestMeanReturn:
    """Validate arithmetic mean return."""

    def test_known_series(self) -> None:
        """Mean of [0.10, -0.10, 0.20] is about 0.0667."""
        rets = [0.10, -0.10, 0.20]
        assert mean_return(rets) == pytest.approx(0.2 / 3.0, abs=1e-10)


class TestCagr:
    """Validate CAGR basis selection."""

    def test_factor_basis_matches_expected_growth(self) -> None:
        """A one-year factor basis should annualize a single-year return directly."""
        from finstack.analytics import CagrBasis, cagr

        value = cagr([0.10], CagrBasis.factor(1.0))
        assert value == pytest.approx(0.10, abs=1e-12)

    def test_date_basis_matches_calendar_year_growth(self) -> None:
        """A one-year date basis should match the same single-year return."""
        from finstack.analytics import CagrBasis, cagr

        value = cagr([0.10], CagrBasis.dates(date(2024, 1, 1), date(2025, 1, 1)))
        assert value == pytest.approx(0.10, abs=1e-3)


class TestMaxDrawdown:
    """Validate maximum drawdown from return series."""

    def test_no_drawdown(self) -> None:
        """Monotone positive returns yield zero drawdown."""
        rets = [0.01, 0.01, 0.01, 0.01]
        assert _max_dd(rets) == pytest.approx(0.0)

    def test_known_drawdown(self) -> None:
        """A large drop creates a measurable drawdown (negative by convention)."""
        rets = [0.10, -0.20, 0.05, -0.05]
        dd = _max_dd(rets)
        assert dd < 0.0
        assert dd >= -1.0

    def test_full_loss(self) -> None:
        """A -100% return produces a -1.0 drawdown."""
        rets = [0.0, -1.0]
        assert _max_dd(rets) == pytest.approx(-1.0)


class TestCompSum:
    """Validate cumulative compounded return series."""

    def test_basic(self) -> None:
        """comp_sum of [0.10, 0.10] = [0.10, 0.21]."""
        cs = comp_sum([0.10, 0.10])
        assert len(cs) == 2
        assert cs[0] == pytest.approx(0.10)
        assert cs[1] == pytest.approx(0.21)

    def test_zero_returns(self) -> None:
        """Zero returns yield zero cumulative returns."""
        cs = comp_sum([0.0, 0.0, 0.0])
        assert all(c == pytest.approx(0.0) for c in cs)


class TestCompTotal:
    """Validate total compounded return."""

    def test_basic(self) -> None:
        """Total compound of [0.10, 0.10] is 0.21."""
        assert comp_total([0.10, 0.10]) == pytest.approx(0.21)

    def test_negative(self) -> None:
        """Total compound of [-0.50, -0.50] is -0.75."""
        assert comp_total([-0.50, -0.50]) == pytest.approx(-0.75)

    def test_empty(self) -> None:
        """Empty series compounds to zero."""
        assert comp_total([]) == pytest.approx(0.0)


class TestDrawdownSeries:
    """Validate to_drawdown_series conversion."""

    def test_no_drawdown(self) -> None:
        """Monotone positive returns have all-zero drawdowns."""
        dd = to_drawdown_series([0.01, 0.01, 0.01])
        assert all(d == pytest.approx(0.0) for d in dd)

    def test_length_matches(self) -> None:
        """Drawdown series has the same length as the input."""
        rets = [0.01, -0.02, 0.015]
        assert len(to_drawdown_series(rets)) == len(rets)


class TestCompsBindings:
    """Validate canonical comps binding behavior."""

    def test_compute_multiple_uses_company_metrics_and_multiple(self) -> None:
        """compute_multiple mirrors the Rust CompanyMetrics + Multiple API."""
        metrics = {"enterprise_value": 8_500.0, "ebitda": 1_000.0}
        value = compute_multiple(metrics, "EvEbitda")
        assert value == pytest.approx(8.5)

    def test_compute_multiple_returns_none_for_missing_inputs(self) -> None:
        """Missing denominator fields yield None instead of NaN sentinel values."""
        metrics = {"enterprise_value": 8_500.0}
        assert compute_multiple(metrics, "EvEbitda") is None

    def test_regression_fair_value_uses_subject_y_for_residual(self) -> None:
        """Residual matches actual minus fitted, not a binding-invented placeholder."""
        result = regression_fair_value(
            [1.0, 2.0, 3.0, 4.0],
            [3.0, 5.0, 7.0, 9.0],
            3.0,
            10.0,
        )
        assert result["fitted_value"] == pytest.approx(7.0)
        assert result["residual"] == pytest.approx(3.0)

    def test_percentile_rank_uses_fraction_units(self) -> None:
        """Python keeps Rust/WASM percentile rank units in [0, 1]."""
        assert percentile_rank(250.0, [100.0, 200.0, 300.0, 400.0, 500.0]) == pytest.approx(0.4)

    def test_ruin_model_default_matches_rust(self) -> None:
        """Default bootstrap block size matches the canonical Rust model."""
        assert RuinModel().block_size == 5

    def test_score_relative_value_accepts_regression_dimensions(self) -> None:
        """Python can pass full regression dimensions through to Rust scoring."""
        subject = {"leverage": 2.0, "oas_bps": 250.0}
        peers = [
            {"leverage": 1.0, "oas_bps": 100.0},
            {"leverage": 2.0, "oas_bps": 200.0},
            {"leverage": 3.0, "oas_bps": 300.0},
        ]

        result = score_relative_value(
            subject,
            peers,
            [{"label": "Spread vs Leverage", "y": "oas_bps", "x": ["leverage"], "weight": 1.0}],
        )

        by_dimension = cast(dict[str, dict[str, float | None]], result["by_dimension"])
        dim = by_dimension["Spread vs Leverage"]
        assert dim["regression_residual"] == pytest.approx(50.0)
        assert dim["r_squared"] == pytest.approx(1.0)
        assert cast(float, result["composite_score"]) > 0.0


class TestLookbackBindings:
    """Validate Python lookback selector bindings."""

    @staticmethod
    def _dates() -> list[date]:
        return [
            date(2024, 12, 31),
            date(2025, 1, 1),
            date(2025, 1, 2),
            date(2025, 1, 31),
            date(2025, 2, 1),
            date(2025, 2, 15),
        ]

    def test_mtd_select_returns_index_range(self) -> None:
        assert mtd_select(self._dates(), date(2025, 2, 15)) == (4, 6)

    def test_qtd_select_returns_index_range(self) -> None:
        assert qtd_select(self._dates(), date(2025, 2, 15)) == (1, 6)

    def test_ytd_select_returns_index_range(self) -> None:
        assert ytd_select(self._dates(), date(2025, 2, 15)) == (1, 6)

    def test_fytd_select_returns_index_range(self) -> None:
        assert fytd_select(self._dates(), date(2025, 2, 15), 10, 1) == (0, 6)


class TestBacktestingBindings:
    """Validate extended Python backtesting bindings."""

    def test_classify_breaches_returns_dense_boolean_series(self) -> None:
        """The binding preserves one breach indicator per observation."""
        assert classify_breaches([-0.02, -0.02], [-0.01, -0.03]) == [False, True]

    def test_rolling_var_forecasts_historical(self) -> None:
        forecasts, realized = rolling_var_forecasts(
            [0.01, -0.02, 0.015, -0.01, 0.02, -0.03],
            3,
            method="Historical",
        )
        assert len(forecasts) == 3
        assert len(realized) == 3

    def test_compare_var_backtests_returns_model_labels(self) -> None:
        comparison = compare_var_backtests(
            [
                ("Historical", [-0.02, -0.02, -0.02]),
                ("Parametric", [-0.015, -0.015, -0.015]),
            ],
            [-0.01, -0.03, -0.01],
        )
        assert [label for label, _ in comparison.results] == ["Historical", "Parametric"]

    def test_pnl_explanation_wraps_struct(self) -> None:
        result = pnl_explanation(
            [100.0, 110.0, 105.0],
            [99.0, 109.0, 104.0],
            [10.0, 10.0, 10.0],
        )
        assert result.n == 3
        assert result.mean_abs_unexplained == pytest.approx(1.0)
