"""Interpolation and extrapolation methods.

This is the canonical location for interpolation types. They are also
re-exported from finstack.core.market_data.interp for backward compatibility.
"""

from __future__ import annotations

class InterpStyle:
    """Interpolation styles for term structures and curves.

    InterpStyle defines how values are interpolated between known points
    on a curve. Different styles have different properties (smoothness,
    monotonicity, convexity) suitable for different financial applications.

    Available styles:
    - LINEAR: Linear interpolation (simple, fast)
    - LOG_LINEAR: Logarithmic linear interpolation (for discount factors)
    - MONOTONE_CONVEX: Monotone convex interpolation (preserves monotonicity)
    - CUBIC_HERMITE: Cubic Hermite spline (smooth, may overshoot)
    - PIECEWISE_QUADRATIC_FORWARD: Smooth forward curve (C² forwards)
    - FLAT_FWD: Flat forward interpolation (for forward rates)

    Examples
    --------
    Use in curve construction:

        >>> from finstack.core.market_data import DiscountCurve
        >>> from finstack.core.math.interp import InterpStyle
        >>> curve = DiscountCurve(
        ...     id="USD",
        ...     base_date=date(2025, 1, 1),
        ...     knots=[(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)],
        ...     interp=InterpStyle.LOG_LINEAR,  # Log-linear for discount factors
        ... )

    Notes
    -----
    - LOG_LINEAR is standard for discount curves
    - MONOTONE_CONVEX preserves monotonicity (important for yield curves)
    - CUBIC_HERMITE provides smoothness but may overshoot
    - PIECEWISE_QUADRATIC_FORWARD produces smooth (C²) forwards in log-DF space
    - FLAT_FWD is used for forward rate curves

    See Also
    --------
    :class:`ExtrapolationPolicy`: Extrapolation behavior beyond curve bounds
    """

    LINEAR: InterpStyle
    LOG_LINEAR: InterpStyle
    MONOTONE_CONVEX: InterpStyle
    CUBIC_HERMITE: InterpStyle
    PIECEWISE_QUADRATIC_FORWARD: InterpStyle
    FLAT_FWD: InterpStyle

    @classmethod
    def from_name(cls, name: str) -> InterpStyle:
        """Parse an interpolation style from a snake-/kebab-case label.

        Parameters
        ----------
        name : str
            One of "linear", "log_linear", "monotone_convex",
            "cubic_hermite", "piecewise_quadratic_forward", or "flat_fwd"
            (kebab-case forms also accepted).

        Returns
        -------
        InterpStyle
            Enum value corresponding to name.
        """
        ...

    @property
    def name(self) -> str:
        """Snake-case label for this interpolation style."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class ExtrapolationPolicy:
    """Extrapolation policies for curves beyond their bounds.

    ExtrapolationPolicy defines how curves behave when queried at points
    beyond the last knot (or before the first knot). This is critical for
    long-dated instruments and forward-looking queries.

    Available policies:
    - FLAT_ZERO: Extrapolate as zero (for discount factors, hazard rates)
    - FLAT_FORWARD: Extrapolate using the last forward rate (for forward curves)

    Examples
    --------
    Use in curve construction:

        >>> from finstack.core.market_data import DiscountCurve
        >>> from finstack.core.math.interp import ExtrapolationPolicy
        >>> curve = DiscountCurve(
        ...     id="USD",
        ...     base_date=date(2025, 1, 1),
        ...     knots=[(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)],
        ...     extrapolation=ExtrapolationPolicy.FLAT_ZERO,  # Zero beyond 5 years
        ... )

    Notes
    -----
    - FLAT_ZERO is standard for discount factors (DF → 0 as t → ∞)
    - FLAT_FORWARD is used for forward rate curves
    - Extrapolation behavior affects long-dated instrument pricing

    See Also
    --------
    :class:`InterpStyle`: Interpolation styles
    """

    FLAT_ZERO: ExtrapolationPolicy
    FLAT_FORWARD: ExtrapolationPolicy

    @classmethod
    def from_name(cls, name: str) -> ExtrapolationPolicy:
        """Parse an extrapolation policy from a snake-/kebab-case label.

        Parameters
        ----------
        name : str
            One of "flat_zero" or "flat_forward" (kebab-case forms also accepted).

        Returns
        -------
        ExtrapolationPolicy
            Enum value corresponding to name.
        """
        ...

    @property
    def name(self) -> str:
        """Snake-case label for this extrapolation policy."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

__all__ = [
    "InterpStyle",
    "ExtrapolationPolicy",
]
