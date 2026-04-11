"""Performance analytics: returns, drawdowns, risk metrics, and benchmarks.

This module mirrors the native bindings registered from Rust. Types hold
structured results; :class:`Performance` is a stateful engine over a price panel;
standalone functions implement the same metrics on raw return series.
"""

from __future__ import annotations

import datetime
from typing import Sequence

import pandas as pd

__all__ = [
    "PeriodStats",
    "BetaResult",
    "GreeksResult",
    "RollingGreeks",
    "MultiFactorResult",
    "DrawdownEpisode",
    "LookbackReturns",
    "RollingSharpe",
    "RollingSortino",
    "RollingVolatility",
    "RuinDefinition",
    "RuinModel",
    "RuinEstimate",
    "BenchmarkAlignmentPolicy",
    "Performance",
    "group_by_period",
    "period_stats",
    "align_benchmark",
    "align_benchmark_with_policy",
    "calc_beta",
    "greeks",
    "rolling_greeks",
    "tracking_error",
    "information_ratio",
    "r_squared",
    "up_capture",
    "down_capture",
    "capture_ratio",
    "batting_average",
    "multi_factor_greeks",
    "treynor",
    "m_squared",
    "m_squared_from_returns",
    "count_consecutive",
    "to_drawdown_series",
    "drawdown_details",
    "avg_drawdown",
    "average_drawdown",
    "max_drawdown",
    "max_drawdown_from_returns",
    "max_drawdown_duration",
    "cdar",
    "ulcer_index",
    "pain_index",
    "calmar",
    "calmar_from_returns",
    "recovery_factor",
    "recovery_factor_from_returns",
    "martin_ratio",
    "martin_ratio_from_returns",
    "sterling_ratio",
    "sterling_ratio_from_returns",
    "burke_ratio",
    "pain_ratio",
    "pain_ratio_from_returns",
    "simple_returns",
    "clean_returns",
    "excess_returns",
    "convert_to_prices",
    "rebase",
    "comp_sum",
    "comp_total",
    "cagr",
    "cagr_from_periods",
    "mean_return",
    "volatility",
    "sharpe",
    "downside_deviation",
    "sortino",
    "geometric_mean",
    "omega_ratio",
    "gain_to_pain",
    "modified_sharpe",
    "estimate_ruin",
    "rolling_sharpe",
    "rolling_sortino",
    "rolling_volatility",
    "rolling_sharpe_values",
    "rolling_sortino_values",
    "rolling_volatility_values",
    "value_at_risk",
    "expected_shortfall",
    "parametric_var",
    "cornish_fisher_var",
    "skewness",
    "kurtosis",
    "tail_ratio",
    "outlier_win_ratio",
    "outlier_loss_ratio",
]

# ---------------------------------------------------------------------------
# Result types
# ---------------------------------------------------------------------------

class PeriodStats:
    """Aggregated statistics for grouped periodic returns."""

    @property
    def best(self) -> float:
        """Best period return."""

    @property
    def worst(self) -> float:
        """Worst period return."""

    @property
    def consecutive_wins(self) -> int:
        """Longest consecutive winning streak."""

    @property
    def consecutive_losses(self) -> int:
        """Longest consecutive losing streak."""

    @property
    def win_rate(self) -> float:
        """Fraction of positive-return periods."""

    @property
    def avg_return(self) -> float:
        """Average return across all periods."""

    @property
    def avg_win(self) -> float:
        """Average return of positive periods."""

    @property
    def avg_loss(self) -> float:
        """Average return of negative periods."""

    @property
    def payoff_ratio(self) -> float:
        """Payoff ratio (avg win / |avg loss|)."""

    @property
    def profit_ratio(self) -> float:
        """Profit ratio (sum wins / |sum losses|)."""

    @property
    def profit_factor(self) -> float:
        """Profit factor (gross profits / gross losses)."""

    @property
    def cpc_ratio(self) -> float:
        """Common-sense ratio (CPC)."""

    @property
    def kelly_criterion(self) -> float:
        """Kelly criterion optimal fraction."""

    def __repr__(self) -> str: ...

class BetaResult:
    """Regression beta with confidence interval."""

    @property
    def beta(self) -> float:
        """Beta coefficient."""

    @property
    def std_err(self) -> float:
        """Standard error of the beta estimate."""

    @property
    def ci_lower(self) -> float:
        """Lower 95% confidence bound."""

    @property
    def ci_upper(self) -> float:
        """Upper 95% confidence bound."""

    def __repr__(self) -> str: ...

class GreeksResult:
    """Alpha, beta, and R-squared from a single-index regression."""

    @property
    def alpha(self) -> float:
        """Jensen's alpha (annualized)."""

    @property
    def beta(self) -> float:
        """Beta coefficient."""

    @property
    def r_squared(self) -> float:
        """R-squared."""

    def __repr__(self) -> str: ...

class RollingGreeks:
    """Rolling alpha and beta time series."""

    def dates(self) -> list[datetime.date]:
        """Date labels for each rolling window.

        Args:
            (none)

        Returns:
            Dates aligned with rolling windows.

        Example:
            >>> # RollingGreeks from rolling_greeks(...); dates match alphas/betas length
            >>> True  # doctest: +SKIP
        """

    @property
    def alphas(self) -> list[float]:
        """Rolling alpha values."""

    @property
    def betas(self) -> list[float]:
        """Rolling beta values."""

    def to_dataframe(self) -> pd.DataFrame:
        """Convert to a pandas DataFrame with date index and alpha/beta columns."""
        ...

    def __repr__(self) -> str: ...

class MultiFactorResult:
    """Multi-factor regression result."""

    @property
    def alpha(self) -> float:
        """Intercept (alpha)."""

    @property
    def betas(self) -> list[float]:
        """Factor betas."""

    @property
    def r_squared(self) -> float:
        """R-squared."""

    @property
    def adjusted_r_squared(self) -> float:
        """Adjusted R-squared."""

    @property
    def residual_vol(self) -> float:
        """Residual volatility."""

    def __repr__(self) -> str: ...

class DrawdownEpisode:
    """A single drawdown episode with timing and depth information."""

    def start(self) -> datetime.date:
        """Start date of the drawdown."""

    def valley(self) -> datetime.date:
        """Date of the maximum drawdown within this episode."""

    def end(self) -> datetime.date | None:
        """Recovery date (``None`` if still in drawdown)."""

    @property
    def duration_days(self) -> int:
        """Duration in calendar days."""

    @property
    def max_drawdown(self) -> float:
        """Maximum drawdown depth (negative)."""

    @property
    def near_recovery_threshold(self) -> float:
        """Near-recovery threshold."""

    def __repr__(self) -> str: ...

class LookbackReturns:
    """Period-to-date returns for each ticker."""

    @property
    def mtd(self) -> list[float]:
        """Month-to-date returns per ticker."""

    @property
    def qtd(self) -> list[float]:
        """Quarter-to-date returns per ticker."""

    @property
    def ytd(self) -> list[float]:
        """Year-to-date returns per ticker."""

    @property
    def fytd(self) -> list[float] | None:
        """Fiscal-year-to-date returns (``None`` if no fiscal config)."""

    def to_dataframe(self, ticker_names: list[str]) -> pd.DataFrame:
        """Convert to a pandas DataFrame with ticker names as index.

        Columns: mtd, qtd, ytd (and fytd when available).
        """
        ...

    def __repr__(self) -> str: ...

class RollingSharpe:
    """Rolling Sharpe ratio time series."""

    @property
    def values(self) -> list[float]:
        """Rolling Sharpe values."""

    def dates(self) -> list[datetime.date]:
        """Corresponding dates."""

    def to_dataframe(self) -> pd.DataFrame:
        """Convert to a pandas DataFrame with date index and a ``sharpe`` column."""
        ...

    def __repr__(self) -> str: ...

class RollingSortino:
    """Rolling Sortino ratio time series."""

    @property
    def values(self) -> list[float]:
        """Rolling Sortino values."""

    def dates(self) -> list[datetime.date]:
        """Corresponding dates."""

    def to_dataframe(self) -> pd.DataFrame:
        """Convert to a pandas DataFrame with date index and a ``sortino`` column."""
        ...

    def __repr__(self) -> str: ...

class RollingVolatility:
    """Rolling volatility time series."""

    @property
    def values(self) -> list[float]:
        """Rolling volatility values."""

    def dates(self) -> list[datetime.date]:
        """Corresponding dates."""

    def to_dataframe(self) -> pd.DataFrame:
        """Convert to a pandas DataFrame with date index and a ``volatility`` column."""
        ...

    def __repr__(self) -> str: ...

class RuinDefinition:
    """Definition of a ruin event for Monte Carlo ruin estimation."""

    @classmethod
    def wealth_floor(cls, floor_fraction: float) -> RuinDefinition:
        """Ruin if wealth falls below ``floor_fraction`` of initial wealth.

        Args:
            floor_fraction: Wealth floor as a fraction of initial wealth.

        Returns:
            Definition instance.

        Example:
            >>> RuinDefinition.wealth_floor(0.1)
            RuinDefinition(...)
        """

    @classmethod
    def terminal_floor(cls, floor_fraction: float) -> RuinDefinition:
        """Ruin if terminal wealth is below ``floor_fraction`` of initial.

        Args:
            floor_fraction: Terminal floor as a fraction of initial wealth.

        Returns:
            Definition instance.

        Example:
            >>> RuinDefinition.terminal_floor(0.05)
            RuinDefinition(...)
        """

    @classmethod
    def drawdown_breach(cls, max_drawdown: float) -> RuinDefinition:
        """Ruin if drawdown exceeds ``max_drawdown`` (positive threshold).

        Args:
            max_drawdown: Maximum allowed drawdown (positive).

        Returns:
            Definition instance.

        Example:
            >>> RuinDefinition.drawdown_breach(0.2)
            RuinDefinition(...)
        """

    def __repr__(self) -> str: ...

class RuinModel:
    """Configuration for Monte Carlo ruin estimation."""

    def __init__(
        self,
        horizon_periods: int = 252,
        n_paths: int = 10_000,
        block_size: int = 63,
        seed: int = 42,
        confidence_level: float = 0.95,
    ) -> None:
        """Create a ruin simulation model.

        Args:
            horizon_periods: Forward periods to simulate.
            n_paths: Number of Monte Carlo paths.
            block_size: Bootstrap block size.
            seed: RNG seed.
            confidence_level: Confidence level for the CI.

        Returns:
            ``None`` (constructor).

        Example:
            >>> RuinModel()
            RuinModel(...)
        """

    @property
    def horizon_periods(self) -> int:
        """Number of forward periods to simulate."""

    @property
    def n_paths(self) -> int:
        """Number of Monte Carlo paths."""

    @property
    def block_size(self) -> int:
        """Bootstrap block size."""

    @property
    def seed(self) -> int:
        """RNG seed."""

    @property
    def confidence_level(self) -> float:
        """Confidence level for the CI."""

    def __repr__(self) -> str: ...

class RuinEstimate:
    """Monte Carlo ruin probability estimate with confidence interval."""

    @property
    def probability(self) -> float:
        """Estimated ruin probability."""

    @property
    def std_err(self) -> float:
        """Standard error of the estimate."""

    @property
    def ci_lower(self) -> float:
        """Lower confidence bound."""

    @property
    def ci_upper(self) -> float:
        """Upper confidence bound."""

    def __repr__(self) -> str: ...

class BenchmarkAlignmentPolicy:
    """Policy for handling missing dates during benchmark alignment."""

    @classmethod
    def zero_on_missing(cls) -> BenchmarkAlignmentPolicy:
        """Fill missing benchmark dates with zero returns.

        Args:
            (none)

        Returns:
            Policy instance.

        Example:
            >>> BenchmarkAlignmentPolicy.zero_on_missing()
            BenchmarkAlignmentPolicy(...)
        """

    @classmethod
    def error_on_missing(cls) -> BenchmarkAlignmentPolicy:
        """Raise if benchmark dates don't cover all target dates.

        Args:
            (none)

        Returns:
            Policy instance.

        Example:
            >>> BenchmarkAlignmentPolicy.error_on_missing()
            BenchmarkAlignmentPolicy(...)
        """

    def __repr__(self) -> str: ...

# ---------------------------------------------------------------------------
# Performance engine
# ---------------------------------------------------------------------------

class Performance:
    """Stateful performance analytics engine over a panel of ticker price series."""

    def __init__(
        self,
        prices: pd.DataFrame,
        benchmark_ticker: str | None = None,
        freq: str = "daily",
        use_log_returns: bool = False,
    ) -> None:
        """Build from a pandas DataFrame of prices (date-like index, one column per ticker).

        Args:
            prices: Price panel.
            benchmark_ticker: Benchmark column name; first column if ``None``.
            freq: ``daily``, ``weekly``, ``monthly``, ``quarterly``, ``semiannual``, or ``annual``.
            use_log_returns: Use log returns internally when ``True``.

        Returns:
            ``None`` (constructor).

        Example:
            >>> import pandas as pd
            >>> from finstack.analytics import Performance
            >>> df = pd.DataFrame({"SPY": [100.0, 101.0]}, index=pd.date_range("2024-01-01", periods=2))
            >>> perf = Performance(df)
            >>> isinstance(perf.ticker_names, list)
            True
        """

    @staticmethod
    def from_arrays(
        dates: Sequence[object],
        prices: list[list[float]],
        ticker_names: list[str],
        benchmark_ticker: str | None = None,
        freq: str = "daily",
        use_log_returns: bool = False,
    ) -> Performance:
        """Construct from raw arrays (dates, prices matrix, ticker names).

        Args:
            dates: Observation dates (``datetime.date`` or ``pd.Timestamp``).
            prices: ``prices[i]`` is the series for ticker *i*.
            ticker_names: Names for each price series.
            benchmark_ticker: Benchmark name; first column if ``None``.
            freq: Observation frequency string.
            use_log_returns: Use log returns internally when ``True``.

        Returns:
            ``Performance`` instance.

        Example:
            >>> import datetime
            >>> from finstack.analytics import Performance
            >>> d = [datetime.date(2024, 1, 1), datetime.date(2024, 1, 2)]
            >>> Performance.from_arrays(d, [[100.0, 101.0]], ["SPY"])
            Performance(...)
        """

    def reset_date_range(self, start: object, end: object) -> None:
        """Restrict analytics to a date window.

        Args:
            start: Inclusive start (date-like).
            end: Inclusive end (date-like).

        Returns:
            ``None``.

        Example:
            >>> # perf.reset_date_range(datetime.date(2024,1,1), datetime.date(2024,6,30))
            >>> True  # doctest: +SKIP
        """

    def reset_bench_ticker(self, ticker: str) -> None:
        """Change the benchmark ticker.

        Args:
            ticker: Column name of the new benchmark.

        Returns:
            ``None``.

        Example:
            >>> # perf.reset_bench_ticker("SPY")
            >>> True  # doctest: +SKIP
        """

    @property
    def ticker_names(self) -> list[str]:
        """Ticker names in column order."""

    @property
    def benchmark_idx(self) -> int:
        """Benchmark column index."""

    @property
    def freq(self) -> str:
        """Observation frequency (debug string)."""

    @property
    def uses_log_returns(self) -> bool:
        """Whether log returns are used internally."""

    def dates(self) -> list[datetime.date]:
        """Active date grid after any window filter.

        Args:
            (none)

        Returns:
            Dates in the active window.

        Example:
            >>> # isinstance(perf.dates()[0], datetime.date)
            >>> True  # doctest: +SKIP
        """

    def cagr(self) -> list[float]:
        """CAGR for each ticker."""

    def mean_return(self, annualize: bool = True) -> list[float]:
        """Mean return for each ticker."""

    def volatility(self, annualize: bool = True) -> list[float]:
        """Volatility for each ticker."""

    def sharpe(self, risk_free_rate: float = 0.0) -> list[float]:
        """Sharpe ratio for each ticker."""

    def sortino(self) -> list[float]:
        """Sortino ratio for each ticker."""

    def calmar(self) -> list[float]:
        """Calmar ratio for each ticker."""

    def max_drawdown(self) -> list[float]:
        """Max drawdown for each ticker."""

    def value_at_risk(self, confidence: float = 0.95) -> list[float]:
        """Historical VaR for each ticker."""

    def expected_shortfall(self, confidence: float = 0.95) -> list[float]:
        """Expected Shortfall for each ticker."""

    def tracking_error(self) -> list[float]:
        """Tracking error for each ticker vs benchmark."""

    def information_ratio(self) -> list[float]:
        """Information ratio for each ticker vs benchmark."""

    def skewness(self) -> list[float]:
        """Skewness for each ticker."""

    def kurtosis(self) -> list[float]:
        """Kurtosis for each ticker."""

    def geometric_mean(self) -> list[float]:
        """Geometric mean for each ticker."""

    def downside_deviation(self, mar: float = 0.0) -> list[float]:
        """Downside deviation for each ticker."""

    def max_drawdown_duration(self) -> list[int]:
        """Max drawdown duration (calendar days) for each ticker."""

    def up_capture(self) -> list[float]:
        """Up-capture ratio for each ticker vs benchmark."""

    def down_capture(self) -> list[float]:
        """Down-capture ratio for each ticker vs benchmark."""

    def capture_ratio(self) -> list[float]:
        """Capture ratio for each ticker vs benchmark."""

    def omega_ratio(self, threshold: float = 0.0) -> list[float]:
        """Omega ratio for each ticker."""

    def treynor(self, risk_free_rate: float = 0.0) -> list[float]:
        """Treynor ratio for each ticker."""

    def gain_to_pain(self) -> list[float]:
        """Gain-to-pain ratio for each ticker."""

    def ulcer_index(self) -> list[float]:
        """Ulcer index for each ticker."""

    def martin_ratio(self) -> list[float]:
        """Martin ratio for each ticker."""

    def recovery_factor(self) -> list[float]:
        """Recovery factor for each ticker."""

    def pain_index(self) -> list[float]:
        """Pain index for each ticker."""

    def pain_ratio(self, risk_free_rate: float = 0.0) -> list[float]:
        """Pain ratio for each ticker."""

    def tail_ratio(self, confidence: float = 0.95) -> list[float]:
        """Tail ratio for each ticker."""

    def r_squared(self) -> list[float]:
        """R-squared for each ticker vs benchmark."""

    def batting_average(self) -> list[float]:
        """Batting average for each ticker vs benchmark."""

    def parametric_var(self, confidence: float = 0.95) -> list[float]:
        """Parametric VaR for each ticker."""

    def cornish_fisher_var(self, confidence: float = 0.95) -> list[float]:
        """Cornish-Fisher VaR for each ticker."""

    def cdar(self, confidence: float = 0.95) -> list[float]:
        """CDaR for each ticker."""

    def m_squared(self, risk_free_rate: float = 0.0) -> list[float]:
        """M-squared for each ticker."""

    def modified_sharpe(self, risk_free_rate: float = 0.0, confidence: float = 0.95) -> list[float]:
        """Modified Sharpe ratio for each ticker."""

    def sterling_ratio(self, risk_free_rate: float = 0.0, n: int = 5) -> list[float]:
        """Sterling ratio for each ticker."""

    def burke_ratio(self, risk_free_rate: float = 0.0, n: int = 5) -> list[float]:
        """Burke ratio for each ticker."""

    def cumulative_returns(self) -> list[list[float]]:
        """Cumulative returns for each ticker."""

    def drawdown_series(self) -> list[list[float]]:
        """Drawdown series for each ticker."""

    def correlation_matrix(self) -> list[list[float]]:
        """Correlation matrix across all tickers."""

    def cumulative_returns_outperformance(self) -> list[list[float]]:
        """Cumulative returns outperformance vs benchmark."""

    def drawdown_outperformance(self) -> list[list[float]]:
        """Drawdown outperformance vs benchmark."""

    def excess_returns(self, rf: list[float], nperiods: float | None = None) -> list[list[float]]:
        """Excess returns over a risk-free rate series (per ticker)."""

    def beta(self) -> list[BetaResult]:
        """Beta for each ticker vs benchmark."""

    def greeks(self) -> list[GreeksResult]:
        """Greeks (alpha, beta, R²) for each ticker vs benchmark."""

    def rolling_greeks(self, ticker_idx: int, window: int = 63) -> RollingGreeks:
        """Rolling greeks for a specific ticker."""

    def rolling_volatility(self, ticker_idx: int, window: int = 63) -> RollingVolatility:
        """Rolling volatility for a specific ticker."""

    def rolling_sortino(self, ticker_idx: int, window: int = 63) -> RollingSortino:
        """Rolling Sortino for a specific ticker."""

    def rolling_sharpe(
        self,
        ticker_idx: int,
        window: int = 63,
        risk_free_rate: float = 0.0,
    ) -> RollingSharpe:
        """Rolling Sharpe for a specific ticker."""

    def drawdown_details(self, ticker_idx: int, n: int = 5) -> list[DrawdownEpisode]:
        """Top-N drawdown episodes for a specific ticker."""

    def stats_during_bench_drawdowns(self, n: int = 5) -> list[DrawdownEpisode]:
        """Stats during benchmark drawdown episodes."""

    def multi_factor_greeks(
        self,
        ticker_idx: int,
        factor_returns: list[list[float]],
    ) -> MultiFactorResult:
        """Multi-factor regression for a specific ticker."""

    def estimate_ruin(self, definition: RuinDefinition, model: RuinModel) -> list[RuinEstimate]:
        """Ruin estimation for each ticker."""

    def lookback_returns(
        self,
        ref_date: object,
        fiscal_year_start_month: int | None = None,
    ) -> LookbackReturns:
        """Period-to-date lookback returns.

        Raises:
            ValueError: If *fiscal_year_start_month* is not in ``1..=12``.
        """

    def period_stats(
        self,
        ticker_idx: int,
        agg_freq: str = "monthly",
        fiscal_year_start_month: int | None = None,
    ) -> PeriodStats:
        """Period statistics for one ticker at a given aggregation frequency.

        Raises:
            ValueError: If *fiscal_year_start_month* is not in ``1..=12``.
        """

    def summary_to_dataframe(
        self,
        risk_free_rate: float = 0.0,
        confidence: float = 0.95,
    ) -> pd.DataFrame:
        """Summary statistics for all tickers as a pandas DataFrame.

        One row per ticker, columns for each scalar metric (CAGR, volatility,
        Sharpe, max drawdown, etc.).
        """
        ...

    def cumulative_returns_to_dataframe(self) -> pd.DataFrame:
        """Cumulative returns for all tickers as a pandas DataFrame.

        Date index, one column per ticker.
        """
        ...

    def drawdown_series_to_dataframe(self) -> pd.DataFrame:
        """Drawdown series for all tickers as a pandas DataFrame.

        Date index, one column per ticker.
        """
        ...

    def correlation_to_dataframe(self) -> pd.DataFrame:
        """Correlation matrix as a pandas DataFrame.

        Ticker x ticker matrix with ticker names as index and columns.
        """
        ...

    def drawdown_details_to_dataframe(
        self,
        ticker_idx: int,
        n: int = 5,
    ) -> pd.DataFrame:
        """Top-N drawdown episodes for a ticker as a pandas DataFrame.

        Columns: start, valley, end, duration_days, max_drawdown,
        near_recovery_threshold.
        """
        ...

    def lookback_returns_to_dataframe(
        self,
        ref_date: object,
        fiscal_year_start_month: int | None = None,
    ) -> pd.DataFrame:
        """Period-to-date lookback returns as a pandas DataFrame.

        Ticker names as index, columns: mtd, qtd, ytd (and fytd when
        a fiscal config is given).

        Raises:
            ValueError: If *fiscal_year_start_month* is not in ``1..=12``.
        """
        ...

# ---------------------------------------------------------------------------
# Standalone functions
# ---------------------------------------------------------------------------

def group_by_period(
    dates: Sequence[object],
    returns: list[float],
    freq: str = "monthly",
) -> list[tuple[str, float]]:
    """Group returns by period and return ``(period_id_str, compounded_return)`` pairs.

    Args:
        dates: Observation dates (date-like).
        returns: Simple returns aligned with *dates*.
        freq: Aggregation frequency string.

    Returns:
        List of ``(period_id, compounded_return)``.

    Example:
        >>> import datetime
        >>> d = [datetime.date(2024, m, 1) for m in range(1, 4)]
        >>> group_by_period(d, [0.01, -0.02, 0.03], "monthly")  # doctest: +SKIP
        [...]
    """

def period_stats(returns: list[float]) -> PeriodStats:
    """Compute period statistics from a flat list of periodic returns.

    Args:
        returns: Return values (synthetic period labels are used internally).

    Returns:
        :class:`PeriodStats` aggregate.

    Example:
        >>> period_stats([0.01, -0.005, 0.02]).win_rate >= 0
        True
    """

def align_benchmark(
    bench_returns: list[float],
    bench_dates: Sequence[object],
    target_dates: Sequence[object],
) -> list[float]:
    """Align benchmark returns to target dates using zero-fill for missing.

    Args:
        bench_returns: Benchmark returns.
        bench_dates: Dates for benchmark series.
        target_dates: Dates to align onto.

    Returns:
        Aligned benchmark returns.

    Example:
        >>> import datetime
        >>> align_benchmark([0.01], [datetime.date(2024, 1, 1)], [datetime.date(2024, 1, 1)])
        [0.01]
    """

def align_benchmark_with_policy(
    bench_returns: list[float],
    bench_dates: Sequence[object],
    target_dates: Sequence[object],
    policy: BenchmarkAlignmentPolicy,
) -> list[float]:
    """Align benchmark returns using a missing-date policy.

    Args:
        bench_returns: Benchmark returns.
        bench_dates: Benchmark dates.
        target_dates: Target dates.
        policy: Alignment policy.

    Returns:
        Aligned returns.

    Example:
        >>> import datetime
        >>> p = BenchmarkAlignmentPolicy.zero_on_missing()
        >>> align_benchmark_with_policy([0.01], [datetime.date(2024, 1, 1)], [datetime.date(2024, 1, 1)], p)
        [0.01]
    """

def calc_beta(portfolio: list[float], benchmark: list[float]) -> BetaResult:
    """Beta regression of portfolio against benchmark.

    Args:
        portfolio: Portfolio returns.
        benchmark: Benchmark returns (same length).

    Returns:
        :class:`BetaResult`.

    Example:
        >>> calc_beta([0.01, 0.02], [0.005, 0.015]).beta > 0
        True
    """

def greeks(
    returns: list[float],
    benchmark: list[float],
    ann_factor: float = 252.0,
) -> GreeksResult:
    """Single-index greeks (alpha, beta, R²).

    Args:
        returns: Asset returns.
        benchmark: Benchmark returns.
        ann_factor: Annualization factor.

    Returns:
        :class:`GreeksResult`.

    Example:
        >>> greeks([0.01, 0.0], [0.005, 0.0]).r_squared
        1.0
    """

def rolling_greeks(
    returns: list[float],
    benchmark: list[float],
    dates: Sequence[object],
    window: int = 63,
    ann_factor: float = 252.0,
) -> RollingGreeks:
    """Rolling greeks over a window.

    Args:
        returns: Asset returns.
        benchmark: Benchmark returns.
        dates: Observation dates.
        window: Rolling window length.
        ann_factor: Annualization factor.

    Returns:
        :class:`RollingGreeks`.

    Example:
        >>> import datetime
        >>> d = [datetime.date(2024, 1, i) for i in range(1, 70)]
        >>> rg = rolling_greeks([0.0] * 69, [0.0] * 69, d)
        >>> len(rg.alphas) > 0
        True
    """

def tracking_error(
    returns: list[float],
    benchmark: list[float],
    annualize: bool = True,
    ann_factor: float = 252.0,
) -> float:
    """Annualized tracking error.

    Args:
        returns: Asset returns.
        benchmark: Benchmark returns.
        annualize: Annualize when ``True``.
        ann_factor: Annualization factor.

    Returns:
        Tracking error.

    Example:
        >>> tracking_error([0.01, 0.02], [0.01, 0.02]) == 0.0
        True
    """

def information_ratio(
    returns: list[float],
    benchmark: list[float],
    annualize: bool = True,
    ann_factor: float = 252.0,
) -> float:
    """Information ratio.

    Args:
        returns: Active returns (or asset returns).
        benchmark: Benchmark returns.
        annualize: Annualize when ``True``.
        ann_factor: Annualization factor.

    Returns:
        Information ratio.

    Example:
        >>> isinstance(information_ratio([0.01, -0.01], [0.0, 0.0]), float)
        True
    """

def r_squared(returns: list[float], benchmark: list[float]) -> float:
    """R-squared against benchmark.

    Args:
        returns: Asset returns.
        benchmark: Benchmark returns.

    Returns:
        R².

    Example:
        >>> r_squared([0.01, 0.02], [0.005, 0.015])
        1.0
    """

def up_capture(returns: list[float], benchmark: list[float]) -> float:
    """Up-capture ratio.

    Args:
        returns: Asset returns.
        benchmark: Benchmark returns.

    Returns:
        Up-capture.

    Example:
        >>> up_capture([0.02, 0.0], [0.01, -0.01]) > 0
        True
    """

def down_capture(returns: list[float], benchmark: list[float]) -> float:
    """Down-capture ratio.

    Args:
        returns: Asset returns.
        benchmark: Benchmark returns.

    Returns:
        Down-capture.

    Example:
        >>> isinstance(down_capture([0.0, -0.01], [0.0, -0.02]), float)
        True
    """

def capture_ratio(returns: list[float], benchmark: list[float]) -> float:
    """Capture ratio (up/down).

    Args:
        returns: Asset returns.
        benchmark: Benchmark returns.

    Returns:
        Capture ratio.

    Example:
        >>> isinstance(capture_ratio([0.01, -0.01], [0.005, -0.005]), float)
        True
    """

def batting_average(returns: list[float], benchmark: list[float]) -> float:
    """Batting average vs benchmark.

    Args:
        returns: Asset returns.
        benchmark: Benchmark returns.

    Returns:
        Batting average.

    Example:
        >>> batting_average([0.01, -0.01], [0.0, 0.0]) in (0.0, 0.5, 1.0)
        True
    """

def multi_factor_greeks(
    returns: list[float],
    factors: list[list[float]],
    ann_factor: float = 252.0,
) -> MultiFactorResult:
    """Multi-factor regression.

    Args:
        returns: Asset returns.
        factors: One series per factor (same length as *returns*).
        ann_factor: Annualization factor.

    Returns:
        :class:`MultiFactorResult`.

    Example:
        >>> multi_factor_greeks([0.01, 0.0], [[0.005, 0.0], [0.0, 0.0]]).r_squared >= 0
        True
    """

def treynor(ann_return: float, risk_free_rate: float, beta: float) -> float:
    """Treynor ratio from pre-computed values.

    Args:
        ann_return: Annualized return.
        risk_free_rate: Risk-free rate.
        beta: Beta.

    Returns:
        Treynor ratio.

    Example:
        >>> treynor(0.1, 0.02, 1.0)
        0.08
    """

def m_squared(ann_return: float, ann_vol: float, bench_vol: float, risk_free_rate: float) -> float:
    """M-squared from pre-computed values.

    Args:
        ann_return: Annualized portfolio return.
        ann_vol: Annualized portfolio volatility.
        bench_vol: Annualized benchmark volatility.
        risk_free_rate: Risk-free rate.

    Returns:
        M².

    Example:
        >>> isinstance(m_squared(0.1, 0.2, 0.15, 0.0), float)
        True
    """

def m_squared_from_returns(
    portfolio: list[float],
    benchmark: list[float],
    ann_factor: float = 252.0,
    risk_free_rate: float = 0.0,
) -> float:
    """M-squared from return series.

    Args:
        portfolio: Portfolio returns.
        benchmark: Benchmark returns.
        ann_factor: Annualization factor.
        risk_free_rate: Risk-free rate.

    Returns:
        M².

    Example:
        >>> isinstance(m_squared_from_returns([0.01, 0.0], [0.005, 0.0]), float)
        True
    """

def count_consecutive(values: list[float]) -> int:
    """Count longest consecutive run of strictly positive values.

    Args:
        values: Numeric series.

    Returns:
        Longest positive run length.

    Example:
        >>> count_consecutive([1.0, 2.0, -1.0, 3.0])
        2
    """

def to_drawdown_series(returns: list[float]) -> list[float]:
    """Drawdown series from returns.

    Args:
        returns: Simple returns.

    Returns:
        Drawdown path (non-positive).

    Example:
        >>> min(to_drawdown_series([0.1, -0.5, 0.2])) < 0
        True
    """

def drawdown_details(
    drawdown: list[float],
    dates: Sequence[object],
    n: int = 5,
) -> list[DrawdownEpisode]:
    """Top-N drawdown episodes with date information.

    Args:
        drawdown: Drawdown series.
        dates: Dates aligned with *drawdown*.
        n: Number of episodes.

    Returns:
        Episode list.

    Example:
        >>> import datetime
        >>> dd = to_drawdown_series([0.05, -0.1, 0.02])
        >>> ds = [datetime.date(2024, 1, 1), datetime.date(2024, 1, 2), datetime.date(2024, 1, 3)]
        >>> isinstance(drawdown_details(dd, ds, 1), list)
        True
    """

def avg_drawdown(drawdown: list[float], n: int = 5) -> float:
    """Average of the N deepest drawdowns.

    Args:
        drawdown: Drawdown series.
        n: Number of drawdowns to average.

    Returns:
        Average depth.

    Example:
        >>> avg_drawdown(to_drawdown_series([0.1, -0.2]), 1) < 0
        True
    """

def average_drawdown(drawdowns: list[float]) -> float:
    """Simple arithmetic average of drawdown values.

    Args:
        drawdowns: Drawdown samples.

    Returns:
        Mean drawdown.

    Example:
        >>> average_drawdown([-0.1, -0.2]) < 0
        True
    """

def max_drawdown(drawdown: list[float]) -> float:
    """Maximum drawdown from an existing drawdown series.

    Args:
        drawdown: Drawdown series.

    Returns:
        Deepest drawdown (negative).

    Example:
        >>> max_drawdown(to_drawdown_series([0.5, -0.9])) < 0
        True
    """

def max_drawdown_from_returns(returns: list[float]) -> float:
    """Maximum drawdown computed directly from returns.

    Args:
        returns: Simple returns.

    Returns:
        Max drawdown.

    Example:
        >>> max_drawdown_from_returns([0.1, -0.5]) < 0
        True
    """

def max_drawdown_duration(drawdown: list[float], dates: Sequence[object]) -> int:
    """Maximum drawdown duration in calendar days.

    Args:
        drawdown: Drawdown series.
        dates: Aligned dates.

    Returns:
        Duration in days.

    Example:
        >>> import datetime
        >>> dd = to_drawdown_series([0.0, -0.1, 0.0])
        >>> ds = [datetime.date(2024, 1, 1), datetime.date(2024, 1, 2), datetime.date(2024, 1, 3)]
        >>> max_drawdown_duration(dd, ds) >= 0
        True
    """

def cdar(drawdown: list[float], confidence: float = 0.95) -> float:
    """Conditional Drawdown at Risk.

    Args:
        drawdown: Drawdown series.
        confidence: Confidence level.

    Returns:
        CDaR.

    Example:
        >>> isinstance(cdar(to_drawdown_series([0.01, -0.05])), float)
        True
    """

def ulcer_index(drawdown: list[float]) -> float:
    """Ulcer index.

    Args:
        drawdown: Drawdown series.

    Returns:
        Ulcer index.

    Example:
        >>> ulcer_index(to_drawdown_series([0.0, -0.1])) >= 0
        True
    """

def pain_index(drawdown: list[float]) -> float:
    """Pain index (average drawdown depth).

    Args:
        drawdown: Drawdown series.

    Returns:
        Pain index.

    Example:
        >>> pain_index(to_drawdown_series([0.0, -0.2])) < 0
        True
    """

def calmar(cagr_val: float, max_dd: float) -> float:
    """Calmar ratio from pre-computed CAGR and max drawdown.

    Args:
        cagr_val: CAGR.
        max_dd: Max drawdown (typically negative).

    Returns:
        Calmar ratio.

    Example:
        >>> isinstance(calmar(0.1, -0.2), float)
        True
    """

def calmar_from_returns(returns: list[float], ann_factor: float = 252.0) -> float:
    """Calmar ratio from returns.

    Args:
        returns: Simple returns.
        ann_factor: Annualization factor.

    Returns:
        Calmar ratio.

    Example:
        >>> isinstance(calmar_from_returns([0.001] * 252), float)
        True
    """

def recovery_factor(total_return: float, max_dd: float) -> float:
    """Recovery factor from pre-computed values.

    Args:
        total_return: Total return.
        max_dd: Max drawdown.

    Returns:
        Recovery factor.

    Example:
        >>> isinstance(recovery_factor(0.5, -0.2), float)
        True
    """

def recovery_factor_from_returns(returns: list[float]) -> float:
    """Recovery factor from returns.

    Args:
        returns: Simple returns.

    Returns:
        Recovery factor.

    Example:
        >>> isinstance(recovery_factor_from_returns([0.01, -0.02]), float)
        True
    """

def martin_ratio(cagr_val: float, ulcer: float) -> float:
    """Martin ratio from pre-computed values.

    Args:
        cagr_val: CAGR.
        ulcer: Ulcer index.

    Returns:
        Martin ratio.

    Example:
        >>> isinstance(martin_ratio(0.1, 0.05), float)
        True
    """

def martin_ratio_from_returns(returns: list[float], ann_factor: float = 252.0) -> float:
    """Martin ratio from returns.

    Args:
        returns: Simple returns.
        ann_factor: Annualization factor.

    Returns:
        Martin ratio.

    Example:
        >>> isinstance(martin_ratio_from_returns([0.001] * 100), float)
        True
    """

def sterling_ratio(cagr_val: float, avg_dd: float, risk_free_rate: float = 0.0) -> float:
    """Sterling ratio from pre-computed values.

    Args:
        cagr_val: CAGR.
        avg_dd: Average drawdown.
        risk_free_rate: Risk-free rate.

    Returns:
        Sterling ratio.

    Example:
        >>> isinstance(sterling_ratio(0.1, 0.05), float)
        True
    """

def sterling_ratio_from_returns(
    returns: list[float],
    ann_factor: float = 252.0,
    risk_free_rate: float = 0.0,
) -> float:
    """Sterling ratio from returns.

    Args:
        returns: Simple returns.
        ann_factor: Annualization factor.
        risk_free_rate: Risk-free rate.

    Returns:
        Sterling ratio.

    Example:
        >>> isinstance(sterling_ratio_from_returns([0.001] * 200), float)
        True
    """

def burke_ratio(cagr_val: float, dd_episodes: list[float], risk_free_rate: float = 0.0) -> float:
    """Burke ratio from pre-computed values.

    Args:
        cagr_val: CAGR.
        dd_episodes: Squared drawdown episode depths.
        risk_free_rate: Risk-free rate.

    Returns:
        Burke ratio.

    Example:
        >>> isinstance(burke_ratio(0.1, [0.04, 0.01]), float)
        True
    """

def pain_ratio(cagr_val: float, pain: float, risk_free_rate: float = 0.0) -> float:
    """Pain ratio from pre-computed values.

    Args:
        cagr_val: CAGR.
        pain: Pain index.
        risk_free_rate: Risk-free rate.

    Returns:
        Pain ratio.

    Example:
        >>> isinstance(pain_ratio(0.1, 0.05), float)
        True
    """

def pain_ratio_from_returns(
    returns: list[float],
    ann_factor: float = 252.0,
    risk_free_rate: float = 0.0,
) -> float:
    """Pain ratio from returns.

    Args:
        returns: Simple returns.
        ann_factor: Annualization factor.
        risk_free_rate: Risk-free rate.

    Returns:
        Pain ratio.

    Example:
        >>> isinstance(pain_ratio_from_returns([0.001] * 120), float)
        True
    """

def simple_returns(prices: list[float]) -> list[float]:
    """Simple returns from prices.

    Args:
        prices: Price series.

    Returns:
        Simple returns (length ``n-1``).

    Example:
        >>> simple_returns([100.0, 101.0])
        [0.01]
    """

def clean_returns(returns: list[float]) -> list[float]:
    """Replace NaN/Inf in returns with zero (copy).

    Args:
        returns: Return series.

    Returns:
        Cleaned returns.

    Example:
        >>> clean_returns([1.0, float("nan")])[1]
        0.0
    """

def excess_returns(returns: list[float], rf: list[float], nperiods: float | None = None) -> list[float]:
    """Excess returns over a risk-free series.

    Args:
        returns: Asset returns.
        rf: Risk-free rates per period.
        nperiods: Optional periods per year for conversion.

    Returns:
        Excess returns.

    Example:
        >>> excess_returns([0.02, 0.01], [0.0, 0.0])
        [0.02, 0.01]
    """

def convert_to_prices(returns: list[float], base: float = 100.0) -> list[float]:
    """Convert returns to a price path.

    Args:
        returns: Simple returns.
        base: Starting level.

    Returns:
        Price series (includes base).

    Example:
        >>> convert_to_prices([0.1, -0.1], 100.0)[0]
        100.0
    """

def rebase(prices: list[float], base: float = 100.0) -> list[float]:
    """Rebase a price series to start at ``base``.

    Args:
        prices: Prices.
        base: New initial level.

    Returns:
        Rebased prices.

    Example:
        >>> rebase([200.0, 220.0], 100.0)[0]
        100.0
    """

def comp_sum(returns: list[float]) -> list[float]:
    """Cumulative compounded returns.

    Args:
        returns: Simple returns.

    Returns:
        Cumulative compounded path.

    Example:
        >>> comp_sum([0.1, 0.1])[-1] > 0.1
        True
    """

def comp_total(returns: list[float]) -> float:
    """Total compounded return.

    Args:
        returns: Simple returns.

    Returns:
        Total return.

    Example:
        >>> comp_total([0.1, -0.1]) > -1
        True
    """

def cagr(returns: list[float], start: object, end: object) -> float:
    """CAGR between two calendar dates (index positions implied by series length).

    Args:
        returns: Simple returns.
        start: Start date (date-like).
        end: End date (date-like).

    Returns:
        CAGR over the window.

    Example:
        >>> import datetime
        >>> cagr([0.01] * 10, datetime.date(2024, 1, 1), datetime.date(2024, 1, 31))  # doctest: +SKIP
    """

def cagr_from_periods(returns: list[float], ann_factor: float) -> float:
    """CAGR from an annualization factor.

    Args:
        returns: Simple returns.
        ann_factor: Periods per year.

    Returns:
        CAGR.

    Example:
        >>> isinstance(cagr_from_periods([0.01] * 252, 252.0), float)
        True
    """

def mean_return(returns: list[float], annualize: bool = False, ann_factor: float = 1.0) -> float:
    """Arithmetic mean return.

    Args:
        returns: Simple returns.
        annualize: Annualize when ``True``.
        ann_factor: Annualization factor when annualizing.

    Returns:
        Mean return.

    Example:
        >>> mean_return([0.01, 0.03], False, 1.0)
        0.02
    """

def volatility(returns: list[float], annualize: bool = True, ann_factor: float = 252.0) -> float:
    """Volatility (standard deviation of returns).

    Args:
        returns: Simple returns.
        annualize: Annualize when ``True``.
        ann_factor: Annualization factor.

    Returns:
        Volatility.

    Example:
        >>> volatility([0.0, 0.0, 0.0]) == 0.0
        True
    """

def sharpe(ann_return: float, ann_vol: float, risk_free_rate: float = 0.0) -> float:
    """Sharpe ratio from pre-computed annualized return and vol.

    Args:
        ann_return: Annualized return.
        ann_vol: Annualized volatility.
        risk_free_rate: Risk-free rate.

    Returns:
        Sharpe ratio.

    Example:
        >>> sharpe(0.1, 0.2, 0.0)
        0.5
    """

def downside_deviation(
    returns: list[float],
    mar: float = 0.0,
    annualize: bool = True,
    ann_factor: float = 252.0,
) -> float:
    """Downside deviation.

    Args:
        returns: Simple returns.
        mar: Minimum acceptable return.
        annualize: Annualize when ``True``.
        ann_factor: Annualization factor.

    Returns:
        Downside deviation.

    Example:
        >>> downside_deviation([0.01, -0.05]) >= 0
        True
    """

def sortino(returns: list[float], annualize: bool = True, ann_factor: float = 252.0) -> float:
    """Sortino ratio.

    Args:
        returns: Simple returns.
        annualize: Annualize when ``True``.
        ann_factor: Annualization factor.

    Returns:
        Sortino ratio.

    Example:
        >>> isinstance(sortino([0.01, -0.02]), float)
        True
    """

def geometric_mean(returns: list[float]) -> float:
    """Geometric mean of returns.

    Args:
        returns: Simple returns.

    Returns:
        Geometric mean.

    Example:
        >>> isinstance(geometric_mean([0.01, 0.02]), float)
        True
    """

def omega_ratio(returns: list[float], threshold: float = 0.0) -> float:
    """Omega ratio.

    Args:
        returns: Simple returns.
        threshold: Return threshold.

    Returns:
        Omega.

    Example:
        >>> omega_ratio([0.01, -0.005]) > 0
        True
    """

def gain_to_pain(returns: list[float]) -> float:
    """Gain-to-pain ratio.

    Args:
        returns: Simple returns.

    Returns:
        Gain-to-pain.

    Example:
        >>> isinstance(gain_to_pain([0.02, -0.01]), float)
        True
    """

def modified_sharpe(
    returns: list[float],
    risk_free_rate: float = 0.0,
    confidence: float = 0.95,
    ann_factor: float = 252.0,
) -> float:
    """Modified Sharpe ratio.

    Args:
        returns: Simple returns.
        risk_free_rate: Risk-free rate.
        confidence: Confidence for modified VaR.
        ann_factor: Annualization factor.

    Returns:
        Modified Sharpe.

    Example:
        >>> isinstance(modified_sharpe([0.01, -0.02]), float)
        True
    """

def estimate_ruin(returns: list[float], definition: RuinDefinition, model: RuinModel) -> RuinEstimate:
    """Monte Carlo ruin probability estimation.

    Args:
        returns: Historical returns to bootstrap.
        definition: Ruin event definition.
        model: Simulation parameters.

    Returns:
        :class:`RuinEstimate`.

    Example:
        >>> est = estimate_ruin([0.001] * 200, RuinDefinition.wealth_floor(0.5), RuinModel(n_paths=100))
        >>> 0 <= est.probability <= 1
        True
    """

def rolling_sharpe(
    returns: list[float],
    dates: Sequence[object],
    window: int = 63,
    ann_factor: float = 252.0,
    risk_free_rate: float = 0.0,
) -> RollingSharpe:
    """Rolling Sharpe with date labels.

    Args:
        returns: Simple returns.
        dates: Observation dates.
        window: Window length.
        ann_factor: Annualization factor.
        risk_free_rate: Risk-free rate.

    Returns:
        :class:`RollingSharpe`.

    Example:
        >>> import datetime
        >>> d = [datetime.date(2024, 1, i) for i in range(1, 70)]
        >>> rs = rolling_sharpe([0.0] * 69, d)
        >>> len(rs.values) > 0
        True
    """

def rolling_sortino(
    returns: list[float],
    dates: Sequence[object],
    window: int = 63,
    ann_factor: float = 252.0,
) -> RollingSortino:
    """Rolling Sortino with date labels.

    Args:
        returns: Simple returns.
        dates: Observation dates.
        window: Window length.
        ann_factor: Annualization factor.

    Returns:
        :class:`RollingSortino`.

    Example:
        >>> import datetime
        >>> d = [datetime.date(2024, 1, i) for i in range(1, 70)]
        >>> len(rolling_sortino([0.0] * 69, d).values) > 0
        True
    """

def rolling_volatility(
    returns: list[float],
    dates: Sequence[object],
    window: int = 63,
    ann_factor: float = 252.0,
) -> RollingVolatility:
    """Rolling volatility with date labels.

    Args:
        returns: Simple returns.
        dates: Observation dates.
        window: Window length.
        ann_factor: Annualization factor.

    Returns:
        :class:`RollingVolatility`.

    Example:
        >>> import datetime
        >>> d = [datetime.date(2024, 1, i) for i in range(1, 70)]
        >>> len(rolling_volatility([0.01] * 69, d).values) > 0
        True
    """

def rolling_sharpe_values(
    returns: list[float],
    window: int = 63,
    ann_factor: float = 252.0,
    risk_free_rate: float = 0.0,
) -> list[float]:
    """Rolling Sharpe values only (no dates).

    Args:
        returns: Simple returns.
        window: Window length.
        ann_factor: Annualization factor.
        risk_free_rate: Risk-free rate.

    Returns:
        Rolling Sharpe series.

    Example:
        >>> len(rolling_sharpe_values([0.0] * 100)) > 0
        True
    """

def rolling_sortino_values(returns: list[float], window: int = 63, ann_factor: float = 252.0) -> list[float]:
    """Rolling Sortino values only (no dates).

    Args:
        returns: Simple returns.
        window: Window length.
        ann_factor: Annualization factor.

    Returns:
        Rolling Sortino series.

    Example:
        >>> len(rolling_sortino_values([0.0] * 100)) > 0
        True
    """

def rolling_volatility_values(returns: list[float], window: int = 63, ann_factor: float = 252.0) -> list[float]:
    """Rolling volatility values only (no dates).

    Args:
        returns: Simple returns.
        window: Window length.
        ann_factor: Annualization factor.

    Returns:
        Rolling vol series.

    Example:
        >>> len(rolling_volatility_values([0.01] * 100)) > 0
        True
    """

def value_at_risk(returns: list[float], confidence: float = 0.95, ann_factor: float | None = None) -> float:
    """Historical Value-at-Risk.

    Args:
        returns: Simple returns.
        confidence: Confidence level.
        ann_factor: Optional annualization for reporting scale.

    Returns:
        VaR.

    Example:
        >>> value_at_risk([-0.5, -0.01, 0.02], 0.95) <= 0
        True
    """

def expected_shortfall(returns: list[float], confidence: float = 0.95, ann_factor: float | None = None) -> float:
    """Expected Shortfall (CVaR).

    Args:
        returns: Simple returns.
        confidence: Confidence level.
        ann_factor: Optional annualization.

    Returns:
        Expected shortfall.

    Example:
        >>> expected_shortfall([-0.5, -0.01, 0.02], 0.95) <= 0
        True
    """

def parametric_var(returns: list[float], confidence: float = 0.95, ann_factor: float | None = None) -> float:
    """Parametric VaR (Gaussian).

    Args:
        returns: Simple returns.
        confidence: Confidence level.
        ann_factor: Optional annualization.

    Returns:
        Parametric VaR.

    Example:
        >>> isinstance(parametric_var([0.0, 0.01, -0.01]), float)
        True
    """

def cornish_fisher_var(returns: list[float], confidence: float = 0.95, ann_factor: float | None = None) -> float:
    """Cornish-Fisher VaR.

    Args:
        returns: Simple returns.
        confidence: Confidence level.
        ann_factor: Optional annualization.

    Returns:
        Cornish-Fisher VaR.

    Example:
        >>> isinstance(cornish_fisher_var([0.01, -0.02, 0.0]), float)
        True
    """

def skewness(returns: list[float]) -> float:
    """Skewness of returns.

    Args:
        returns: Simple returns.

    Returns:
        Sample skewness.

    Example:
        >>> isinstance(skewness([0.01, 0.02, -0.05]), float)
        True
    """

def kurtosis(returns: list[float]) -> float:
    """Excess kurtosis of returns.

    Args:
        returns: Simple returns.

    Returns:
        Excess kurtosis.

    Example:
        >>> isinstance(kurtosis([0.0] * 20), float)
        True
    """

def tail_ratio(returns: list[float], confidence: float = 0.95) -> float:
    """Tail ratio (upper quantile / |lower quantile|).

    Args:
        returns: Simple returns.
        confidence: Confidence level.

    Returns:
        Tail ratio.

    Example:
        >>> isinstance(tail_ratio([0.05, -0.04, 0.0]), float)
        True
    """

def outlier_win_ratio(returns: list[float], confidence: float = 0.95) -> float:
    """Outlier win ratio.

    Args:
        returns: Simple returns.
        confidence: Confidence level.

    Returns:
        Ratio.

    Example:
        >>> isinstance(outlier_win_ratio([0.2, -0.01, 0.0]), float)
        True
    """

def outlier_loss_ratio(returns: list[float], confidence: float = 0.95) -> float:
    """Outlier loss ratio.

    Args:
        returns: Simple returns.
        confidence: Confidence level.

    Returns:
        Ratio.

    Example:
        >>> isinstance(outlier_loss_ratio([-0.2, 0.01, 0.0]), float)
        True
    """
