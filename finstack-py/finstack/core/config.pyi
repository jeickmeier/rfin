"""Configuration types from ``finstack-core``: rounding, tolerances, and global config.

Provides :class:`RoundingMode`, :class:`ToleranceConfig`, and
:class:`FinstackConfig` for controlling rounding behaviour and
numerical tolerance thresholds across the library.
"""

from __future__ import annotations

from typing import Optional

__all__ = [
    "RoundingMode",
    "ToleranceConfig",
    "FinstackConfig",
]

class RoundingMode:
    """Rounding mode for monetary and rate calculations.

    Enum-style class with class-level constants for each supported mode.
    """

    BANKERS: RoundingMode
    """Banker's rounding (ties to even)."""
    AWAY_FROM_ZERO: RoundingMode
    """Round halves away from zero."""
    TOWARD_ZERO: RoundingMode
    """Round toward zero (truncate)."""
    FLOOR: RoundingMode
    """Round toward negative infinity."""
    CEIL: RoundingMode
    """Round toward positive infinity."""

    @classmethod
    def from_name(cls, name: str) -> RoundingMode:
        """Parse a rounding mode from a human-readable label (case-insensitive).

        Parameters
        ----------
        name : str
            Label such as ``"bankers"``, ``"away_from_zero"``, ``"floor"``.

        Returns
        -------
        RoundingMode

        Raises
        ------
        ValueError
            If *name* is not recognised.
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...

class ToleranceConfig:
    """Numerical tolerance settings for rate and generic comparisons.

    Parameters
    ----------
    rate_epsilon : float | None
        Epsilon for rate-style comparisons. If ``None``, the library
        default is used.
    generic_epsilon : float | None
        Epsilon for generic floating-point comparisons. If ``None``,
        the library default is used.
    """

    def __init__(
        self,
        rate_epsilon: Optional[float] = None,
        generic_epsilon: Optional[float] = None,
    ) -> None:
        """Create tolerance settings, optionally overriding default epsilons.

        Parameters
        ----------
        rate_epsilon : float | None
            Epsilon for rate-style comparisons.
        generic_epsilon : float | None
            Epsilon for generic floating-point comparisons.
        """
        ...

    def get_rate_epsilon(self) -> float:
        """Epsilon used for rate-style comparisons.

        Returns
        -------
        float
        """
        ...

    def get_generic_epsilon(self) -> float:
        """Epsilon used for generic floating-point comparisons.

        Returns
        -------
        float
        """
        ...

    def __repr__(self) -> str: ...

class FinstackConfig:
    """Top-level library configuration combining rounding and tolerances.

    Parameters
    ----------
    rounding_mode : RoundingMode | None
        Rounding mode override. If ``None``, the library default is used.
    tolerances : ToleranceConfig | None
        Tolerance configuration override. If ``None``, the library default
        is used.
    """

    def __init__(
        self,
        rounding_mode: Optional[RoundingMode] = None,
        tolerances: Optional[ToleranceConfig] = None,
    ) -> None:
        """Create a configuration, optionally overriding rounding mode and tolerances.

        Parameters
        ----------
        rounding_mode : RoundingMode | None
            Rounding mode.
        tolerances : ToleranceConfig | None
            Tolerance configuration.
        """
        ...

    def get_output_scale(self, currency: str) -> int:
        """Effective output decimal scale for a currency.

        Parameters
        ----------
        currency : str
            ISO-4217 alphabetic currency code.

        Returns
        -------
        int
            Number of decimal places for output formatting.

        Raises
        ------
        ValueError
            If *currency* is not recognised.
        """
        ...

    def get_ingest_scale(self, currency: str) -> int:
        """Effective ingest decimal scale for a currency.

        Parameters
        ----------
        currency : str
            ISO-4217 alphabetic currency code.

        Returns
        -------
        int
            Number of decimal places for input parsing.

        Raises
        ------
        ValueError
            If *currency* is not recognised.
        """
        ...

    def __repr__(self) -> str: ...
