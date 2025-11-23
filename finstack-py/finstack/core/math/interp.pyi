"""Interpolation and extrapolation methods.

This is the canonical location for interpolation types. They are also
re-exported from finstack.core.market_data.interp for backward compatibility.
"""

class InterpStyle:
    """Enumerate interpolation styles available to term structures."""

    LINEAR: InterpStyle
    LOG_LINEAR: InterpStyle
    MONOTONE_CONVEX: InterpStyle
    CUBIC_HERMITE: InterpStyle
    FLAT_FWD: InterpStyle

    @classmethod
    def from_name(cls, name: str) -> InterpStyle:
        """Parse an interpolation style from a snake-/kebab-case label.

        Parameters
        ----------
        name : str
            One of "linear", "log_linear", "monotone_convex",
            "cubic_hermite", or "flat_fwd" (kebab-case forms also accepted).

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
    """Enumerate extrapolation policies used when evaluating beyond curve bounds."""

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
