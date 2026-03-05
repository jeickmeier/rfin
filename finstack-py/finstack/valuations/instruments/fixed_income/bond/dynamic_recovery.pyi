"""Dynamic (notional-dependent) recovery rate model bindings."""

from __future__ import annotations

class DynamicRecoverySpec:
    """Dynamic (notional-dependent) recovery rate specification.

    Recovery rates decline as PIK accrual increases the notional relative to
    the asset base, capturing the intuition that higher leverage dilutes
    recovery in default.

    Supported models:

    - **Constant**: ``R(t) = R_0`` (ignores notional).
    - **InverseLinear**: ``R(t) = R_0 * (N_0 / N(t))``.
    - **InversePower**: ``R(t) = R_0 * (N_0 / N(t))^alpha``.
    - **FlooredInverse**: ``R(t) = max(floor, R_0 * (N_0 / N(t)))``.
    - **LinearDecline**: ``R(t) = clamp(R_0 * (1 - beta * (N(t)/N_0 - 1)), floor, R_0)``.

    Examples
    --------
        >>> spec = DynamicRecoverySpec.constant(0.40)
        >>> spec.recovery_at_notional(150.0)
        0.4
    """

    @classmethod
    def constant(cls, recovery: float) -> DynamicRecoverySpec:
        """Create a constant recovery spec (ignores notional changes).

        Parameters
        ----------
        recovery : float
            Fixed recovery rate.

        Returns
        -------
        DynamicRecoverySpec
        """
        ...

    @classmethod
    def inverse_linear(
        cls,
        base_recovery: float,
        base_notional: float,
    ) -> DynamicRecoverySpec:
        """Create an inverse-linear recovery spec.

        ``R(N) = R_0 * (N_0 / N)``, clamped to ``[0, R_0]``.

        Parameters
        ----------
        base_recovery : float
            Base recovery rate at the base notional.
        base_notional : float
            Reference notional.

        Returns
        -------
        DynamicRecoverySpec
        """
        ...

    @classmethod
    def inverse_power(
        cls,
        base_recovery: float,
        base_notional: float,
        exponent: float,
    ) -> DynamicRecoverySpec:
        """Create an inverse-power recovery spec.

        ``R(N) = R_0 * (N_0 / N)^exponent``, clamped to ``[0, R_0]``.

        Parameters
        ----------
        base_recovery : float
            Base recovery rate at the base notional.
        base_notional : float
            Reference notional.
        exponent : float
            Power exponent controlling the rate of decline.

        Returns
        -------
        DynamicRecoverySpec
        """
        ...

    @classmethod
    def floored_inverse(
        cls,
        base_recovery: float,
        base_notional: float,
        floor: float,
    ) -> DynamicRecoverySpec:
        """Create a floored inverse recovery spec.

        ``R(N) = max(floor, R_0 * (N_0 / N))``, clamped to ``[0, R_0]``.

        Parameters
        ----------
        base_recovery : float
            Base recovery rate at the base notional.
        base_notional : float
            Reference notional.
        floor : float
            Minimum recovery rate floor.

        Returns
        -------
        DynamicRecoverySpec
        """
        ...

    @classmethod
    def linear_decline(
        cls,
        base_recovery: float,
        base_notional: float,
        sensitivity: float,
        floor: float,
    ) -> DynamicRecoverySpec:
        """Create a linear-decline recovery spec.

        ``R(N) = clamp(R_0 * (1 - sensitivity * (N/N_0 - 1)), floor, R_0)``

        Parameters
        ----------
        base_recovery : float
            Base recovery rate at the base notional.
        base_notional : float
            Reference notional.
        sensitivity : float
            Sensitivity of recovery to notional increase.
        floor : float
            Minimum recovery rate floor.

        Returns
        -------
        DynamicRecoverySpec
        """
        ...

    def recovery_at_notional(self, current_notional: float) -> float:
        """Compute recovery rate given current accreted notional.

        Parameters
        ----------
        current_notional : float
            Current (PIK-augmented) notional outstanding.

        Returns
        -------
        float
            Recovery rate clamped to ``[0, base_recovery]``.
        """
        ...

    @property
    def base_recovery(self) -> float:
        """Base (reference) recovery rate."""
        ...

    @property
    def base_notional(self) -> float:
        """Base (reference) notional."""
        ...

    @property
    def name(self) -> str:
        """Canonical name of the recovery model variant."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
