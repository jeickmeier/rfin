"""Calibration Bump Helpers type stubs."""

from __future__ import annotations
from typing import Any, List, Tuple
from datetime import date
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, HazardCurve, InflationCurve
from finstack.valuations.calibration import RatesQuote

class BumpRequest:
    """Request for a curve bump operation.

    Create using static methods `parallel()` or `tenors()`.

    Examples:
        >>> # Parallel +10bp bump
        >>> bump = BumpRequest.parallel(10.0)

        >>> # Key-rate bumps at specific tenors
        >>> bump = BumpRequest.tenors([(2.0, 5.0), (5.0, 10.0), (10.0, 15.0)])
    """

    @staticmethod
    def parallel(bp: float) -> BumpRequest:
        """Create a parallel bump (shift all rates by the same amount).

        Args:
            bp: Bump size in basis points. Positive = rates up, negative = rates down.

        Returns:
            BumpRequest: A parallel bump request.

        Examples:
            >>> bump = BumpRequest.parallel(10.0)  # +10bp parallel shift
        """
        ...

    @staticmethod
    def tenors(tenors: List[Tuple[float, float]]) -> BumpRequest:
        """Create a tenor-specific (key-rate) bump.

        Args:
            tenors: List of (tenor_years, bump_bp) tuples. Each tuple specifies
                a maturity in years and the bump size in basis points.

        Returns:
            BumpRequest: A tenor-specific bump request.

        Raises:
            ValueError: If tenors list is empty.

        Examples:
            >>> # Bump 2Y by +5bp, 5Y by +10bp, 10Y by +15bp
            >>> bump = BumpRequest.tenors([(2.0, 5.0), (5.0, 10.0), (10.0, 15.0)])
        """
        ...

    def is_parallel(self) -> bool:
        """Check if this is a parallel bump."""
        ...

    def is_tenors(self) -> bool:
        """Check if this is a tenor-specific bump."""
        ...

    def parallel_bp(self) -> float | None:
        """Get the parallel bump size (if parallel), or None."""
        ...

    def tenor_bumps(self) -> List[Tuple[float, float]] | None:
        """Get the tenor bumps (if tenors), or None."""
        ...

def bump_discount_curve(
    quotes: List[RatesQuote],
    params: dict[str, Any],
    market: MarketContext,
    bump: BumpRequest,
) -> DiscountCurve:
    """Bump a discount curve by shocking rate quotes and re-calibrating.

    Args:
        quotes: List of RatesQuote objects used in the original calibration.
        params: Dict matching the DiscountCurveParams calibration schema.
        market: Market context providing any required dependencies.
        bump: The bump request (parallel or tenor-specific).

    Returns:
        DiscountCurve: A new bumped discount curve.
    """
    ...

def bump_discount_curve_synthetic(
    curve: DiscountCurve,
    market: MarketContext,
    bump: BumpRequest,
    as_of: date | Tuple[int, int, int],
) -> DiscountCurve:
    """Bump a discount curve by synthesizing quotes and re-calibrating.

    This function extracts par rates from the current curve, applies the bump,
    and re-calibrates. Use when original quotes are unavailable.

    Args:
        curve: The discount curve to bump.
        market: Market context containing the curve.
        bump: The bump request (parallel or tenor-specific).
        as_of: Valuation date.

    Returns:
        DiscountCurve: A new bumped discount curve.

    Examples:
        >>> bumped = bump_discount_curve_synthetic(
        ...     curve=discount_curve, market=market_context, bump=BumpRequest.parallel(10.0), as_of=(2025, 1, 1)
        ... )
    """
    ...

def bump_hazard_spreads(
    hazard_curve: HazardCurve,
    market: MarketContext,
    bump: BumpRequest,
    discount_id: str,
) -> HazardCurve:
    """Bump a hazard curve by re-calibrating from par spreads.

    This function extracts par spreads, applies the bump, and re-calibrates
    the hazard curve. Requires a discount curve for discounting.

    Args:
        hazard_curve: The hazard curve to bump.
        market: Market context containing necessary curves.
        bump: The bump request (parallel or tenor-specific).
        discount_id: Identifier for the discount curve to use.

    Returns:
        HazardCurve: A new bumped hazard curve.

    Examples:
        >>> bumped = bump_hazard_spreads(
        ...     hazard_curve=hazard,
        ...     market=market_context,
        ...     bump=BumpRequest.parallel(50.0),  # +50bp spread
        ...     discount_id="USD-OIS",
        ... )
    """
    ...

def bump_hazard_shift(
    hazard_curve: HazardCurve,
    bump: BumpRequest,
) -> HazardCurve:
    """Bump a hazard curve directly without re-calibration.

    This function applies a direct shift to hazard rates without re-calibrating
    from par spreads. Faster but less accurate for large bumps.

    Args:
        hazard_curve: The hazard curve to bump.
        bump: The bump request (parallel or tenor-specific).

    Returns:
        HazardCurve: A new bumped hazard curve.

    Examples:
        >>> bumped = bump_hazard_shift(
        ...     hazard_curve=hazard,
        ...     bump=BumpRequest.parallel(10.0),  # +10bp hazard rate
        ... )
    """
    ...

def bump_inflation_rates(
    curve: InflationCurve,
    market: MarketContext,
    bump: BumpRequest,
    discount_id: str,
    as_of: date | Tuple[int, int, int],
) -> InflationCurve:
    """Bump an inflation curve by re-calibrating from implied rates.

    This function extracts implied zero-coupon swap rates, applies the bump,
    and re-calibrates the inflation curve.

    Args:
        curve: The inflation curve to bump.
        market: Market context containing necessary curves.
        bump: The bump request (parallel or tenor-specific).
        discount_id: Identifier for the discount curve to use.
        as_of: Valuation date.

    Returns:
        InflationCurve: A new bumped inflation curve.

    Examples:
        >>> bumped = bump_inflation_rates(
        ...     curve=inflation_curve,
        ...     market=market_context,
        ...     bump=BumpRequest.parallel(25.0),  # +25bp inflation
        ...     discount_id="USD-OIS",
        ...     as_of=(2025, 1, 1),
        ... )
    """
    ...

__all__ = [
    "BumpRequest",
    "bump_discount_curve",
    "bump_discount_curve_synthetic",
    "bump_hazard_spreads",
    "bump_hazard_shift",
    "bump_inflation_rates",
]
