"""Endogenous (leverage-dependent) hazard rate model bindings."""

from __future__ import annotations

class EndogenousHazardSpec:
    """Endogenous (leverage-dependent) hazard rate specification.

    Provides a feedback loop where PIK accrual increases leverage, which in turn
    increases the hazard rate and expected loss.

    Three mapping functions are supported:

    - **Power law**: ``lambda(L) = lambda_0 * (L / L_0)^beta``
    - **Exponential**: ``lambda(L) = lambda_0 * exp(beta * (L - L_0))``
    - **Tabular**: Linear interpolation from empirical calibration with flat
      extrapolation at the edges.

    Examples
    --------
        >>> spec = EndogenousHazardSpec.power_law(0.10, 1.5, 2.5)
        >>> spec.hazard_at_leverage(1.5)
        0.1
    """

    @classmethod
    def power_law(
        cls,
        base_hazard: float,
        base_leverage: float,
        exponent: float,
    ) -> EndogenousHazardSpec:
        """Create a power-law endogenous hazard spec.

        ``lambda(L) = base_hazard * (L / base_leverage)^exponent``

        Parameters
        ----------
        base_hazard : float
            Base (reference) hazard rate at the base leverage.
        base_leverage : float
            Base (reference) leverage level.
        exponent : float
            Power-law exponent controlling sensitivity to leverage changes.

        Returns
        -------
        EndogenousHazardSpec
        """
        ...

    @classmethod
    def exponential(
        cls,
        base_hazard: float,
        base_leverage: float,
        sensitivity: float,
    ) -> EndogenousHazardSpec:
        """Create an exponential endogenous hazard spec.

        ``lambda(L) = base_hazard * exp(sensitivity * (L - base_leverage))``

        Parameters
        ----------
        base_hazard : float
            Base (reference) hazard rate at the base leverage.
        base_leverage : float
            Base (reference) leverage level.
        sensitivity : float
            Exponential sensitivity to leverage changes.

        Returns
        -------
        EndogenousHazardSpec
        """
        ...

    @classmethod
    def tabular(
        cls,
        leverage_points: list[float],
        hazard_points: list[float],
    ) -> EndogenousHazardSpec:
        """Create a tabular endogenous hazard spec from empirical calibration.

        Uses linear interpolation between the given points and flat
        extrapolation beyond the edges.

        Parameters
        ----------
        leverage_points : list[float]
            Leverage breakpoints (must be sorted ascending).
        hazard_points : list[float]
            Corresponding hazard rates at each breakpoint.

        Returns
        -------
        EndogenousHazardSpec

        Raises
        ------
        ValueError
            If ``leverage_points`` and ``hazard_points`` have different lengths
            or are empty.
        """
        ...

    def hazard_at_leverage(self, leverage: float) -> float:
        """Compute the hazard rate at a given leverage level.

        Parameters
        ----------
        leverage : float
            Current leverage ratio.

        Returns
        -------
        float
            Hazard rate (floored at 0.0).
        """
        ...

    def hazard_after_pik_accrual(
        self,
        original_notional: float,
        accreted_notional: float,
        asset_value: float,
    ) -> float:
        """Compute the hazard rate after PIK accrual changes the notional.

        Leverage is computed as ``accreted_notional / asset_value``.

        Parameters
        ----------
        original_notional : float
            Original face notional.
        accreted_notional : float
            Current (PIK-augmented) notional.
        asset_value : float
            Current asset value.

        Returns
        -------
        float
            Hazard rate at the implied leverage.
        """
        ...

    @property
    def base_hazard_rate(self) -> float:
        """Base (reference) hazard rate."""
        ...

    @property
    def base_leverage(self) -> float:
        """Base (reference) leverage level."""
        ...

    @property
    def name(self) -> str:
        """Canonical name of the hazard map variant (``'PowerLaw'``, ``'Exponential'``, or ``'Tabular'``)."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
