"""Toggle exercise model bindings for PIK/cash coupon decisions."""

from __future__ import annotations

class ToggleExerciseModel:
    """Toggle exercise model for PIK/cash coupon decisions.

    Models the borrower's decision to pay-in-kind (PIK) or pay cash at each
    coupon date. The toggle decision depends on observable credit state and can
    follow a hard threshold rule, a stochastic (sigmoid) model, or an optimal
    exercise strategy.

    Examples
    --------
        >>> model = ToggleExerciseModel.threshold("hazard_rate", 0.15)
        >>> model.name
        'Threshold'
    """

    @classmethod
    def threshold(
        cls,
        variable: str,
        threshold: float,
        direction: str = "above",
    ) -> ToggleExerciseModel:
        """Create a threshold toggle model.

        PIK is elected when the credit metric crosses the boundary in the
        specified direction.

        Parameters
        ----------
        variable : str
            Credit state variable: ``"hazard_rate"``, ``"distance_to_default"``,
            or ``"leverage"``.
        threshold : float
            Threshold value for the comparison.
        direction : str, optional
            Direction for comparison: ``"above"`` (default) or ``"below"``.

        Returns
        -------
        ToggleExerciseModel

        Raises
        ------
        ValueError
            If ``variable`` or ``direction`` is not recognised.
        """
        ...

    @classmethod
    def stochastic(
        cls,
        variable: str,
        intercept: float,
        sensitivity: float,
    ) -> ToggleExerciseModel:
        """Create a stochastic (sigmoid) toggle model.

        PIK probability follows a logistic function:
        ``P(PIK) = 1 / (1 + exp(-(intercept + sensitivity * state)))``

        Parameters
        ----------
        variable : str
            Credit state variable: ``"hazard_rate"``, ``"distance_to_default"``,
            or ``"leverage"``.
        intercept : float
            Intercept of the logistic function.
        sensitivity : float
            Sensitivity (slope) of the logistic function.

        Returns
        -------
        ToggleExerciseModel

        Raises
        ------
        ValueError
            If ``variable`` is not recognised.
        """
        ...

    @classmethod
    def optimal_exercise(
        cls,
        nested_paths: int = 200,
        equity_discount_rate: float = 0.10,
        asset_vol: float = 0.30,
        risk_free_rate: float = 0.03,
        horizon: float = 1.0,
    ) -> ToggleExerciseModel:
        """Create an optimal exercise toggle model using nested Monte Carlo.

        At each coupon date, a small nested MC simulation estimates equity
        value under cash vs PIK scenarios to make the optimal toggle
        decision.

        Parameters
        ----------
        nested_paths : int, optional
            Number of nested Monte Carlo paths (default: 200).
        equity_discount_rate : float, optional
            Equity holder discount rate for NPV (default: 0.10).
        asset_vol : float, optional
            Annualised asset volatility for the nested GBM simulation
            (default: 0.30).
        risk_free_rate : float, optional
            Risk-free rate (continuous) used as drift in the nested
            simulation (default: 0.03).
        horizon : float, optional
            Forward-looking horizon in years (default: 1.0).

        Returns
        -------
        ToggleExerciseModel
        """
        ...

    @property
    def name(self) -> str:
        """Canonical name of the toggle model variant (``'Threshold'``, ``'Stochastic'``, or ``'OptimalExercise'``)."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
