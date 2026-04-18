"""Tests for analytics functions: returns, risk metrics, drawdowns."""

from datetime import date

import pytest

from finstack.analytics import (
    comp_sum,
    comp_total,
    max_drawdown,
    mean_return,
    sharpe,
    simple_returns,
    sortino,
    to_drawdown_series,
    volatility,
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
