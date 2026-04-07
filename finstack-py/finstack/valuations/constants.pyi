"""Shared numerical constants for pricing and risk calculations.

Examples
--------
    >>> from finstack.valuations.constants import ONE_BASIS_POINT
    >>> ONE_BASIS_POINT
    0.0001
"""

from __future__ import annotations

ONE_BASIS_POINT: float
"""One basis point (0.0001)."""

BASIS_POINTS_PER_UNIT: float
"""Number of basis points in one unit (10,000)."""

PERCENT_TO_DECIMAL: float
"""Multiply a percentage by this to get a decimal (0.01)."""

DECIMAL_TO_PERCENT: float
"""Multiply a decimal by this to get a percentage (100.0)."""

class numerical:
    """Numerical tolerances used by solvers and comparisons."""

    ZERO_TOLERANCE: float
    """Tolerance for checking if a value is effectively zero (1e-10)."""

    INTEGRATION_STEP_FACTOR: float
    """Step size factor for numerical differentiation and integration (1e-4)."""

    SOLVER_TOLERANCE: float
    """Tolerance for iterative solver convergence (1e-8)."""

    RATE_COMPARISON_TOLERANCE: float
    """Tolerance for comparing floating-point rates and spreads (1e-12)."""

    DIVISION_EPSILON: float
    """Small epsilon to prevent division by zero (1e-15)."""

    DF_EPSILON: float
    """Minimum threshold for discount factor values (1e-10)."""

class isda:
    """ISDA standard conventions for credit derivatives."""

    STANDARD_RECOVERY_SENIOR: float
    """Standard recovery rate for senior unsecured (0.40)."""

    STANDARD_RECOVERY_SUB: float
    """Standard recovery rate for subordinated (0.20)."""

    STANDARD_INTEGRATION_POINTS: int
    """Standard integration points per year for protection leg (40)."""

class credit:
    """Credit-derivatives specific constants."""

    SURVIVAL_PROBABILITY_FLOOR: float
    """Survival probability floor for numerical stability (1e-15)."""

    MIN_TIME_TO_EXPIRY_GREEKS: float
    """Minimum time-to-expiry in years for Greeks calculations (~1/365)."""

    MIN_VOLATILITY_GREEKS: float
    """Minimum volatility for option Greeks calculations (0.001)."""

    MIN_FORWARD_SPREAD: float
    """Minimum forward spread in decimal for CDS option Black formula (1e-8)."""

    MIN_HAZARD_RATE: float
    """Minimum hazard rate for bootstrapping (1e-5)."""

    DEFAULT_MAX_HAZARD_RATE: float
    """Default maximum hazard rate for bootstrapping (1.0)."""

    HAZARD_RATE_BRACKET_MULTIPLIER: float
    """Hazard rate multiplier for adaptive upper bound in bootstrapping (2.0)."""

    PAR_SPREAD_DENOM_TOLERANCE: float
    """Par spread denominator tolerance (1e-12)."""

    SMALL_POOL_THRESHOLD: int
    """Small pool threshold for exact convolution vs SPA in tranche pricing (16)."""

    CALENDAR_DAYS_PER_YEAR: float
    """Calendar days per year for settlement delay calculations (365.0)."""

class time:
    """Business day counts per year by market region."""

    BUSINESS_DAYS_PER_YEAR_US: float
    """Business days per year for North America / US markets (252.0)."""

    BUSINESS_DAYS_PER_YEAR_UK: float
    """Business days per year for Europe / UK markets (250.0)."""

    BUSINESS_DAYS_PER_YEAR_JP: float
    """Business days per year for Asia / Japan markets (255.0)."""

__all__ = [
    "ONE_BASIS_POINT",
    "BASIS_POINTS_PER_UNIT",
    "PERCENT_TO_DECIMAL",
    "DECIMAL_TO_PERCENT",
    "numerical",
    "isda",
    "credit",
    "time",
]
