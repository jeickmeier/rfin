"""Calibrator classes for bootstrapping term structures and vol surfaces.

Each calibrator follows a builder pattern -- construct with required identifiers,
optionally configure interpolation / day-count / extrapolation settings, then
call :meth:`calibrate` with market quotes and context to produce a curve or
surface plus a :class:`CalibrationReport`.

Available calibrators
---------------------
- :class:`DiscountCurveCalibrator` -- OIS / LIBOR discount curves
- :class:`ForwardCurveCalibrator` -- Forward rate curves (e.g., 3M SOFR)
- :class:`HazardCurveCalibrator` -- Credit hazard-rate curves
- :class:`InflationCurveCalibrator` -- CPI / inflation curves
- :class:`VolSurfaceCalibrator` -- Implied-volatility surfaces (SABR)
- :class:`BaseCorrelationCalibrator` -- CDS tranche base-correlation curves
"""

from __future__ import annotations

import datetime
from typing import Any

from finstack.core.config import FinstackConfig
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import (
    BaseCorrelationCurve,
    DiscountCurve,
    ForwardCurve,
    HazardCurve,
    InflationCurve,
)
from finstack.core.market_data.surfaces import VolSurface
from finstack.valuations.calibration.config import CalibrationMethod, MultiCurveConfig
from finstack.valuations.calibration.quote import (
    CreditQuote,
    InflationQuote,
    RatesQuote,
    VolQuote,
)
from finstack.valuations.calibration.report import CalibrationReport


# ---------------------------------------------------------------------------
# DiscountCurveCalibrator
# ---------------------------------------------------------------------------

class DiscountCurveCalibrator:
    """Calibrate a discount curve from deposit, FRA, and swap quotes.

    Parameters
    ----------
    curve_id : str
        Identifier for the resulting curve (e.g., ``"USD-OIS"``).
    base_date : datetime.date
        Curve base date.
    currency : str or Currency
        Curve currency.

    Examples
    --------
    >>> from finstack.valuations.calibration.methods import DiscountCurveCalibrator
    >>> cal = DiscountCurveCalibrator("USD-OIS", date(2024, 1, 1), "USD")
    >>> cal = cal.with_extrapolation("flat_forward")
    >>> curve, report = cal.calibrate(quotes)
    """

    def __init__(
        self,
        curve_id: str,
        base_date: datetime.date,
        currency: str | Any,
    ) -> None:
        """Create a discount curve calibrator.

        Parameters
        ----------
        curve_id : str
            Curve identifier.
        base_date : datetime.date
            Base date for the curve.
        currency : str or Currency
            Currency of the curve.
        """
        ...

    def with_finstack_config(self, config: FinstackConfig) -> DiscountCurveCalibrator:
        """Apply configuration from a :class:`FinstackConfig`.

        Parameters
        ----------
        config : FinstackConfig
            Configuration with calibration extensions.

        Returns
        -------
        DiscountCurveCalibrator
            New calibrator with updated configuration.
        """
        ...

    def with_multi_curve_config(self, multi_curve: MultiCurveConfig) -> DiscountCurveCalibrator:
        """Set multi-curve configuration (e.g., dual-curve stripping).

        Parameters
        ----------
        multi_curve : MultiCurveConfig
            Multi-curve configuration.

        Returns
        -------
        DiscountCurveCalibrator
            New calibrator with multi-curve settings.
        """
        ...

    def with_solve_interp(self, interp: str | Any) -> DiscountCurveCalibrator:
        """Set the interpolation style for solving.

        Parameters
        ----------
        interp : str or InterpStyle
            Interpolation style (e.g., ``"linear"``, ``"log_linear"``).

        Returns
        -------
        DiscountCurveCalibrator
            New calibrator with updated interpolation.
        """
        ...

    def with_calibration_method(self, method: CalibrationMethod) -> DiscountCurveCalibrator:
        """Set the calibration method (bootstrap, global optimisation, etc.).

        Parameters
        ----------
        method : CalibrationMethod
            Calibration method.

        Returns
        -------
        DiscountCurveCalibrator
            New calibrator with updated method.
        """
        ...

    def with_extrapolation(self, policy: str | Any) -> DiscountCurveCalibrator:
        """Set the extrapolation policy for the final curve.

        Parameters
        ----------
        policy : str or ExtrapolationPolicy
            Extrapolation policy (``"flat_forward"`` or ``"flat_zero"``).

        Returns
        -------
        DiscountCurveCalibrator
            New calibrator with updated extrapolation policy.
        """
        ...

    def calibrate(
        self,
        quotes: list[RatesQuote],
        market: MarketContext | None = None,
    ) -> tuple[DiscountCurve, CalibrationReport]:
        """Calibrate a discount curve from rate quotes.

        Parameters
        ----------
        quotes : list[RatesQuote]
            Market quotes (deposits, FRAs, swaps, etc.).
        market : MarketContext or None, optional
            Base market context for multi-curve setups.

        Returns
        -------
        tuple[DiscountCurve, CalibrationReport]
            Calibrated curve and diagnostic report.

        Raises
        ------
        RuntimeError
            If calibration fails to converge.
        """
        ...


# ---------------------------------------------------------------------------
# ForwardCurveCalibrator
# ---------------------------------------------------------------------------

class ForwardCurveCalibrator:
    """Calibrate a forward rate curve (e.g., 3M SOFR forwards).

    Parameters
    ----------
    curve_id : str
        Identifier for the resulting curve.
    tenor_years : float
        Forward rate tenor in years (e.g., ``0.25`` for 3M).
    base_date : datetime.date
        Curve base date.
    currency : str or Currency
        Curve currency.
    discount_curve_id : str
        Identifier of the discount curve used for present-value calculations.

    Examples
    --------
    >>> from finstack.valuations.calibration.methods import ForwardCurveCalibrator
    >>> cal = ForwardCurveCalibrator("USD-3M", 0.25, date(2024, 1, 1), "USD", "USD-OIS")
    >>> curve, report = cal.calibrate(quotes, market)
    """

    def __init__(
        self,
        curve_id: str,
        tenor_years: float,
        base_date: datetime.date,
        currency: str | Any,
        discount_curve_id: str,
    ) -> None:
        """Create a forward curve calibrator.

        Parameters
        ----------
        curve_id : str
            Curve identifier.
        tenor_years : float
            Forward tenor in years.
        base_date : datetime.date
            Base date.
        currency : str or Currency
            Curve currency.
        discount_curve_id : str
            Discount curve identifier.
        """
        ...

    def with_finstack_config(self, config: FinstackConfig) -> ForwardCurveCalibrator:
        """Apply configuration from a :class:`FinstackConfig`.

        Parameters
        ----------
        config : FinstackConfig
            Configuration with calibration extensions.

        Returns
        -------
        ForwardCurveCalibrator
            New calibrator with updated configuration.
        """
        ...

    def with_solve_interp(self, interp: str | Any) -> ForwardCurveCalibrator:
        """Set the interpolation style for solving.

        Parameters
        ----------
        interp : str or InterpStyle
            Interpolation style.

        Returns
        -------
        ForwardCurveCalibrator
            New calibrator with updated interpolation.
        """
        ...

    def calibrate(
        self,
        quotes: list[RatesQuote],
        market: MarketContext,
    ) -> tuple[ForwardCurve, CalibrationReport]:
        """Calibrate a forward curve from rate quotes.

        Parameters
        ----------
        quotes : list[RatesQuote]
            Market quotes.
        market : MarketContext
            Market context containing the discount curve.

        Returns
        -------
        tuple[ForwardCurve, CalibrationReport]
            Calibrated forward curve and diagnostic report.

        Raises
        ------
        RuntimeError
            If calibration fails.
        """
        ...


# ---------------------------------------------------------------------------
# HazardCurveCalibrator
# ---------------------------------------------------------------------------

class HazardCurveCalibrator:
    """Calibrate a credit hazard-rate curve from CDS quotes.

    Parameters
    ----------
    entity : str
        Entity identifier (issuer name or ticker).
    seniority : str
        Debt seniority (e.g., ``"senior"``, ``"subordinated"``).
    recovery_rate : float
        Assumed recovery rate (0.0 -- 1.0).
    base_date : datetime.date
        Curve base date.
    currency : str or Currency
        Curve currency.
    discount_curve : str or None, optional
        Discount curve identifier. If ``None``, a default is used.

    Examples
    --------
    >>> from finstack.valuations.calibration.methods import HazardCurveCalibrator
    >>> cal = HazardCurveCalibrator("ACME", "senior", 0.40, date(2024, 1, 1), "USD")
    >>> curve, report = cal.calibrate(cds_quotes, market)
    """

    def __init__(
        self,
        entity: str,
        seniority: str,
        recovery_rate: float,
        base_date: datetime.date,
        currency: str | Any,
        discount_curve: str | None = None,
    ) -> None:
        """Create a hazard curve calibrator.

        Parameters
        ----------
        entity : str
            Entity identifier.
        seniority : str
            Debt seniority level.
        recovery_rate : float
            Assumed recovery rate.
        base_date : datetime.date
            Base date.
        currency : str or Currency
            Curve currency.
        discount_curve : str or None, optional
            Discount curve identifier.
        """
        ...

    def with_finstack_config(self, config: FinstackConfig) -> HazardCurveCalibrator:
        """Apply configuration from a :class:`FinstackConfig`.

        Parameters
        ----------
        config : FinstackConfig
            Configuration with calibration extensions.

        Returns
        -------
        HazardCurveCalibrator
            New calibrator with updated configuration.
        """
        ...

    def with_par_interp(self, interpolation: str) -> HazardCurveCalibrator:
        """Set par-rate interpolation method.

        Parameters
        ----------
        interpolation : str
            ``"linear"`` or ``"log_linear"``.

        Returns
        -------
        HazardCurveCalibrator
            New calibrator with updated interpolation.

        Raises
        ------
        ValueError
            If *interpolation* is not recognised.
        """
        ...

    def calibrate(
        self,
        quotes: list[CreditQuote],
        market: MarketContext,
    ) -> tuple[HazardCurve, CalibrationReport]:
        """Calibrate a hazard curve from CDS quotes.

        Parameters
        ----------
        quotes : list[CreditQuote]
            CDS market quotes.
        market : MarketContext
            Market context containing the discount curve.

        Returns
        -------
        tuple[HazardCurve, CalibrationReport]
            Calibrated hazard curve and diagnostic report.

        Raises
        ------
        RuntimeError
            If calibration fails.
        """
        ...


# ---------------------------------------------------------------------------
# InflationCurveCalibrator
# ---------------------------------------------------------------------------

class InflationCurveCalibrator:
    """Calibrate an inflation (CPI) curve from inflation swap / zero-coupon quotes.

    Parameters
    ----------
    curve_id : str
        Identifier for the resulting curve.
    base_date : datetime.date
        Curve base date.
    currency : str or Currency
        Curve currency.
    base_cpi : float
        CPI index value at the base date.
    discount_curve_id : str
        Discount curve identifier for present-value discounting.

    Examples
    --------
    >>> from finstack.valuations.calibration.methods import InflationCurveCalibrator
    >>> cal = InflationCurveCalibrator("USD-CPI", date(2024, 1, 1), "USD", 308.417, "USD-OIS")
    >>> curve, report = cal.calibrate(quotes)
    """

    def __init__(
        self,
        curve_id: str,
        base_date: datetime.date,
        currency: str | Any,
        base_cpi: float,
        discount_curve_id: str,
    ) -> None:
        """Create an inflation curve calibrator.

        Parameters
        ----------
        curve_id : str
            Curve identifier.
        base_date : datetime.date
            Base date.
        currency : str or Currency
            Curve currency.
        base_cpi : float
            Base CPI index level.
        discount_curve_id : str
            Discount curve identifier.
        """
        ...

    def with_finstack_config(self, config: FinstackConfig) -> InflationCurveCalibrator:
        """Apply configuration from a :class:`FinstackConfig`.

        Parameters
        ----------
        config : FinstackConfig
            Configuration with calibration extensions.

        Returns
        -------
        InflationCurveCalibrator
            New calibrator with updated configuration.
        """
        ...

    def with_solve_interp(self, interp: str | Any) -> InflationCurveCalibrator:
        """Set the interpolation style for solving.

        Parameters
        ----------
        interp : str or InterpStyle
            Interpolation style.

        Returns
        -------
        InflationCurveCalibrator
            New calibrator with updated interpolation.
        """
        ...

    def with_time_dc(self, day_count: str | Any) -> InflationCurveCalibrator:
        """Set the time day-count convention.

        Parameters
        ----------
        day_count : str or DayCount
            Day-count convention.

        Returns
        -------
        InflationCurveCalibrator
            New calibrator with updated day count.
        """
        ...

    def with_accrual_dc(self, day_count: str | Any) -> InflationCurveCalibrator:
        """Set the accrual day-count convention.

        Parameters
        ----------
        day_count : str or DayCount
            Day-count convention.

        Returns
        -------
        InflationCurveCalibrator
            New calibrator with updated accrual day count.
        """
        ...

    def with_inflation_lag_months(self, months: int) -> InflationCurveCalibrator:
        """Set inflation index lag in months.

        Parameters
        ----------
        months : int
            Number of months of lag.

        Returns
        -------
        InflationCurveCalibrator
            New calibrator with updated lag.
        """
        ...

    def with_inflation_lag_days(self, days: int) -> InflationCurveCalibrator:
        """Set inflation index lag in days.

        Parameters
        ----------
        days : int
            Number of days of lag.

        Returns
        -------
        InflationCurveCalibrator
            New calibrator with updated lag.
        """
        ...

    def with_no_inflation_lag(self) -> InflationCurveCalibrator:
        """Remove inflation index lag.

        Returns
        -------
        InflationCurveCalibrator
            New calibrator with no lag.
        """
        ...

    def with_seasonality_adjustments(self, adjustments: list[float]) -> InflationCurveCalibrator:
        """Set monthly seasonality adjustment factors.

        Parameters
        ----------
        adjustments : list[float]
            Exactly 12 monthly adjustment factors.

        Returns
        -------
        InflationCurveCalibrator
            New calibrator with seasonality adjustments.

        Raises
        ------
        ValueError
            If *adjustments* does not contain exactly 12 elements.
        """
        ...

    def with_inflation_interpolation(self, interpolation: str | None = None) -> InflationCurveCalibrator:
        """Set the inflation index interpolation method.

        Parameters
        ----------
        interpolation : str or None, optional
            Interpolation method (e.g., ``"linear"``). ``None`` defaults to linear.

        Returns
        -------
        InflationCurveCalibrator
            New calibrator with updated interpolation.
        """
        ...

    def calibrate(
        self,
        quotes: list[InflationQuote],
        market: MarketContext | None = None,
    ) -> tuple[InflationCurve, CalibrationReport]:
        """Calibrate an inflation curve from inflation swap quotes.

        Parameters
        ----------
        quotes : list[InflationQuote]
            Inflation market quotes.
        market : MarketContext or None, optional
            Market context containing the discount curve.

        Returns
        -------
        tuple[InflationCurve, CalibrationReport]
            Calibrated inflation curve and diagnostic report.

        Raises
        ------
        RuntimeError
            If calibration fails.
        """
        ...


# ---------------------------------------------------------------------------
# VolSurfaceCalibrator
# ---------------------------------------------------------------------------

class VolSurfaceCalibrator:
    """Calibrate an implied-volatility surface (e.g., SABR) from option quotes.

    Parameters
    ----------
    surface_id : str
        Identifier for the resulting surface.
    beta : float
        SABR beta parameter (typically 0.0 -- 1.0).
    target_expiries : list[float]
        Target expiry times in years for the output grid.
    target_strikes : list[float]
        Target strikes for the output grid.

    Examples
    --------
    >>> from finstack.valuations.calibration.methods import VolSurfaceCalibrator
    >>> cal = VolSurfaceCalibrator("SPX-VOL", 0.5, [0.25, 0.5, 1.0], [90, 100, 110])
    >>> cal = cal.with_base_date(date(2024, 1, 1))
    >>> surface, report = cal.calibrate(vol_quotes, market)
    """

    def __init__(
        self,
        surface_id: str,
        beta: float,
        target_expiries: list[float],
        target_strikes: list[float],
    ) -> None:
        """Create a vol surface calibrator.

        Parameters
        ----------
        surface_id : str
            Surface identifier.
        beta : float
            SABR beta parameter.
        target_expiries : list[float]
            Target expiry grid.
        target_strikes : list[float]
            Target strike grid.
        """
        ...

    def with_base_date(self, base_date: datetime.date) -> VolSurfaceCalibrator:
        """Set the base date for the surface.

        Parameters
        ----------
        base_date : datetime.date
            Base date.

        Returns
        -------
        VolSurfaceCalibrator
            New calibrator with updated base date.
        """
        ...

    def with_finstack_config(self, config: FinstackConfig) -> VolSurfaceCalibrator:
        """Apply configuration from a :class:`FinstackConfig`.

        Parameters
        ----------
        config : FinstackConfig
            Configuration with calibration extensions.

        Returns
        -------
        VolSurfaceCalibrator
            New calibrator with updated configuration.
        """
        ...

    def with_base_currency(self, currency: str | Any) -> VolSurfaceCalibrator:
        """Set the base currency for the surface.

        Parameters
        ----------
        currency : str or Currency
            Currency code or object.

        Returns
        -------
        VolSurfaceCalibrator
            New calibrator with updated currency.
        """
        ...

    def with_time_dc(self, day_count: str | Any) -> VolSurfaceCalibrator:
        """Set the time day-count convention.

        Parameters
        ----------
        day_count : str or DayCount
            Day-count convention.

        Returns
        -------
        VolSurfaceCalibrator
            New calibrator with updated day count.
        """
        ...

    def with_surface_interp(self, interpolation: str | None = None) -> VolSurfaceCalibrator:
        """Set surface interpolation method.

        Parameters
        ----------
        interpolation : str or None, optional
            Interpolation method (e.g., ``"bilinear"``). ``None`` defaults to bilinear.

        Returns
        -------
        VolSurfaceCalibrator
            New calibrator with updated interpolation.
        """
        ...

    def with_discount_id(self, discount_curve_id: str) -> VolSurfaceCalibrator:
        """Set the discount curve identifier for forward construction.

        Parameters
        ----------
        discount_curve_id : str
            Discount curve identifier.

        Returns
        -------
        VolSurfaceCalibrator
            New calibrator with updated discount id.
        """
        ...

    def with_spot_override(self, spot: float) -> VolSurfaceCalibrator:
        """Override the spot price used for forward construction.

        Parameters
        ----------
        spot : float
            Spot price override.

        Returns
        -------
        VolSurfaceCalibrator
            New calibrator with spot override.
        """
        ...

    def with_dividend_yield_override(self, dividend_yield: float) -> VolSurfaceCalibrator:
        """Override the dividend yield used for forward construction.

        Parameters
        ----------
        dividend_yield : float
            Dividend yield in decimal terms (e.g., ``0.02`` for 2 %).

        Returns
        -------
        VolSurfaceCalibrator
            New calibrator with dividend yield override.
        """
        ...

    def calibrate(
        self,
        quotes: list[VolQuote],
        market: MarketContext,
    ) -> tuple[VolSurface, CalibrationReport]:
        """Calibrate a volatility surface from option quotes.

        Parameters
        ----------
        quotes : list[VolQuote]
            Vol market quotes.
        market : MarketContext
            Market context containing discount curves and spot data.

        Returns
        -------
        tuple[VolSurface, CalibrationReport]
            Calibrated surface and diagnostic report.

        Raises
        ------
        RuntimeError
            If calibration fails.
        """
        ...


# ---------------------------------------------------------------------------
# BaseCorrelationCalibrator
# ---------------------------------------------------------------------------

class BaseCorrelationCalibrator:
    """Calibrate a base correlation curve for CDS tranches.

    Uses market-standard bootstrapping with one-factor Gaussian Copula.
    Tranches are sorted by detachment point and solved sequentially.

    Parameters
    ----------
    index_id : str
        Index identifier (e.g., ``"CDX.NA.IG.42"``).
    series : int
        Index series number.
    maturity_years : float
        Maturity for the correlation curve in years.
    base_date : datetime.date
        Base date for calibration.

    Examples
    --------
    >>> from finstack.valuations.calibration.methods import BaseCorrelationCalibrator
    >>> cal = BaseCorrelationCalibrator("CDX.NA.IG.42", 42, 5.0, date(2024, 1, 1))
    >>> cal = cal.with_discount_curve_id("USD-OIS")
    >>> cal = cal.with_detachment_points([3.0, 7.0, 10.0, 15.0, 30.0])
    >>> curve, report = cal.calibrate(tranche_quotes, market)
    """

    def __init__(
        self,
        index_id: str,
        series: int,
        maturity_years: float,
        base_date: datetime.date,
    ) -> None:
        """Create a base correlation calibrator.

        Parameters
        ----------
        index_id : str
            Index identifier.
        series : int
            Index series number.
        maturity_years : float
            Maturity in years.
        base_date : datetime.date
            Base date for calibration.
        """
        ...

    def with_finstack_config(self, config: FinstackConfig) -> BaseCorrelationCalibrator:
        """Apply configuration from a :class:`FinstackConfig`.

        Parameters
        ----------
        config : FinstackConfig
            Configuration with calibration extensions.

        Returns
        -------
        BaseCorrelationCalibrator
            New calibrator with updated configuration.
        """
        ...

    def with_detachment_points(self, points: list[float]) -> BaseCorrelationCalibrator:
        """Set custom detachment points for calibration.

        Parameters
        ----------
        points : list[float]
            Detachment points in percent (e.g., ``[3.0, 7.0, 10.0, 15.0, 30.0]``).

        Returns
        -------
        BaseCorrelationCalibrator
            New calibrator with updated detachment points.
        """
        ...

    def with_discount_curve_id(self, discount_curve_id: str | Any) -> BaseCorrelationCalibrator:
        """Set the discount curve identifier for tranche pricing.

        Parameters
        ----------
        discount_curve_id : str
            Discount curve identifier (e.g., ``"USD-OIS"``).

        Returns
        -------
        BaseCorrelationCalibrator
            New calibrator with updated discount curve.
        """
        ...

    def calibrate(
        self,
        quotes: list[CreditQuote],
        market: MarketContext,
    ) -> tuple[BaseCorrelationCurve, CalibrationReport]:
        """Calibrate a base correlation curve from CDS tranche quotes.

        Only CDSTranche quotes matching the calibrator's ``index_id`` will be
        used.  Quotes are automatically sorted by detachment point for
        bootstrapping.

        Parameters
        ----------
        quotes : list[CreditQuote]
            Credit market quotes (must contain CDSTranche quotes).
        market : MarketContext
            Market context with discount and credit curves.

        Returns
        -------
        tuple[BaseCorrelationCurve, CalibrationReport]
            Calibrated curve and diagnostic report.

        Raises
        ------
        RuntimeError
            If calibration fails or insufficient quotes provided.
        """
        ...

    def __repr__(self) -> str: ...


__all__ = [
    "DiscountCurveCalibrator",
    "ForwardCurveCalibrator",
    "HazardCurveCalibrator",
    "InflationCurveCalibrator",
    "VolSurfaceCalibrator",
    "BaseCorrelationCalibrator",
]
