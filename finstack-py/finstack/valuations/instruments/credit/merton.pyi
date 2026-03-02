"""Merton structural credit model bindings."""

from __future__ import annotations
from datetime import date
from ....core.market_data.term_structures import HazardCurve

class MertonAssetDynamics:
    """Asset dynamics specification for the Merton structural credit model.

    Controls the stochastic process assumed for the firm's asset value.

    Examples
    --------
        >>> MertonAssetDynamics.GEOMETRIC_BROWNIAN
        MertonAssetDynamics('GeometricBrownian')
        >>> MertonAssetDynamics.jump_diffusion(0.5, -0.05, 0.10)
        MertonAssetDynamics('JumpDiffusion')
    """

    GEOMETRIC_BROWNIAN: MertonAssetDynamics
    """Standard geometric Brownian motion (lognormal diffusion)."""

    @classmethod
    def jump_diffusion(
        cls,
        jump_intensity: float,
        jump_mean: float,
        jump_vol: float,
    ) -> MertonAssetDynamics:
        """Jump-diffusion process (Merton 1976) with Poisson jumps.

        Parameters
        ----------
        jump_intensity : float
            Poisson jump arrival intensity (jumps per year).
        jump_mean : float
            Mean log-jump size.
        jump_vol : float
            Volatility of log-jump size.

        Returns
        -------
        MertonAssetDynamics
        """
        ...

    @classmethod
    def credit_grades(
        cls,
        barrier_uncertainty: float,
        mean_recovery: float,
    ) -> MertonAssetDynamics:
        """CreditGrades model extension with uncertain recovery barrier.

        Parameters
        ----------
        barrier_uncertainty : float
            Uncertainty in the default barrier level.
        mean_recovery : float
            Mean recovery rate at default.

        Returns
        -------
        MertonAssetDynamics
        """
        ...

    @property
    def name(self) -> str:
        """Canonical name of the dynamics variant."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class MertonBarrierType:
    """Barrier monitoring type for default determination.

    Examples
    --------
        >>> MertonBarrierType.TERMINAL
        MertonBarrierType('Terminal')
        >>> MertonBarrierType.first_passage(0.05)
        MertonBarrierType('FirstPassage')
    """

    TERMINAL: MertonBarrierType
    """Terminal barrier (classic Merton): default only assessed at maturity."""

    @classmethod
    def first_passage(cls, barrier_growth_rate: float) -> MertonBarrierType:
        """First-passage barrier (Black-Cox extension): continuous monitoring.

        Parameters
        ----------
        barrier_growth_rate : float
            Growth rate of the default barrier over time.

        Returns
        -------
        MertonBarrierType
        """
        ...

    @property
    def name(self) -> str:
        """Canonical name of the barrier type variant."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class MertonModel:
    """Merton structural credit model for estimating firm default probability.

    Models a firm's equity as a call option on its assets, where default
    occurs when asset value falls below the debt barrier.

    Examples
    --------
        >>> m = MertonModel(asset_value=100, asset_vol=0.20, debt_barrier=80, risk_free_rate=0.05)
        >>> m.distance_to_default(1.0)
        1.2657...
        >>> m.default_probability(1.0)
        0.1028...

    Sources
    -------
    - Merton, R. C. (1974). On the Pricing of Corporate Debt.
    - Black, F. & Cox, J. C. (1976). Valuing Corporate Securities.
    """

    def __init__(
        self,
        asset_value: float,
        asset_vol: float,
        debt_barrier: float,
        risk_free_rate: float,
        *,
        payout_rate: float = 0.0,
        barrier_type: MertonBarrierType | None = None,
        dynamics: MertonAssetDynamics | None = None,
    ) -> None:
        """Construct a Merton structural credit model.

        Parameters
        ----------
        asset_value : float
            Current market value of the firm's assets (must be > 0).
        asset_vol : float
            Annualized volatility of asset returns (must be >= 0).
        debt_barrier : float
            Face value of debt / default point (must be > 0).
        risk_free_rate : float
            Continuous risk-free rate.
        payout_rate : float, optional
            Continuous dividend / payout yield on assets (default: 0.0).
        barrier_type : MertonBarrierType, optional
            Barrier monitoring type (default: Terminal).
        dynamics : MertonAssetDynamics, optional
            Asset return dynamics specification (default: GeometricBrownian).

        Raises
        ------
        ValueError
            If inputs are invalid.
        """
        ...

    @classmethod
    def from_equity(
        cls,
        equity_value: float,
        equity_vol: float,
        total_debt: float,
        risk_free_rate: float,
        maturity: float = 1.0,
    ) -> MertonModel:
        """KMV calibration from observed equity value and equity volatility.

        Parameters
        ----------
        equity_value : float
            Observed market equity value.
        equity_vol : float
            Observed equity volatility.
        total_debt : float
            Face value of debt.
        risk_free_rate : float
            Risk-free rate.
        maturity : float, optional
            Time to maturity in years (default: 1.0).

        Returns
        -------
        MertonModel

        Raises
        ------
        ValueError
            If inputs are invalid or calibration fails.
        """
        ...

    @classmethod
    def from_cds_spread(
        cls,
        cds_spread_bp: float,
        recovery: float,
        total_debt: float,
        risk_free_rate: float,
        maturity: float,
        asset_value: float,
    ) -> MertonModel:
        """Calibrate asset volatility from a target CDS spread.

        Parameters
        ----------
        cds_spread_bp : float
            Target CDS spread in basis points.
        recovery : float
            Recovery rate (fraction).
        total_debt : float
            Face value of debt.
        risk_free_rate : float
            Risk-free rate.
        maturity : float
            Time to maturity in years.
        asset_value : float
            Assumed initial asset value.

        Returns
        -------
        MertonModel

        Raises
        ------
        ValueError
            If inputs are invalid or solver fails.
        """
        ...

    @classmethod
    def credit_grades(
        cls,
        equity_value: float,
        equity_vol: float,
        total_debt: float,
        risk_free_rate: float,
        barrier_uncertainty: float,
        mean_recovery: float,
    ) -> MertonModel:
        """CreditGrades model construction from equity observables.

        Parameters
        ----------
        equity_value : float
            Observed market equity value.
        equity_vol : float
            Observed equity volatility.
        total_debt : float
            Face value of debt.
        risk_free_rate : float
            Risk-free rate.
        barrier_uncertainty : float
            Uncertainty in the default barrier level.
        mean_recovery : float
            Mean recovery rate at default.

        Returns
        -------
        MertonModel

        Raises
        ------
        ValueError
            If inputs are invalid.
        """
        ...

    def distance_to_default(self, horizon: float = 1.0) -> float:
        """Distance-to-default over the given horizon.

        Parameters
        ----------
        horizon : float, optional
            Time horizon in years (default: 1.0).

        Returns
        -------
        float
        """
        ...

    def default_probability(self, horizon: float = 1.0) -> float:
        """Default probability over the given horizon.

        Parameters
        ----------
        horizon : float, optional
            Time horizon in years (default: 1.0).

        Returns
        -------
        float
        """
        ...

    def implied_spread(self, horizon: float, recovery: float) -> float:
        """Implied credit spread from default probability and recovery rate.

        Parameters
        ----------
        horizon : float
            Time horizon in years.
        recovery : float
            Assumed recovery rate.

        Returns
        -------
        float
        """
        ...

    def implied_equity(self, horizon: float = 1.0) -> tuple[float, float]:
        """Compute implied equity value and equity volatility.

        Parameters
        ----------
        horizon : float, optional
            Time horizon in years (default: 1.0).

        Returns
        -------
        tuple[float, float]
            ``(equity_value, equity_vol)``
        """
        ...

    def to_hazard_curve(
        self,
        curve_id: str,
        base_date: date,
        tenors: list[float] | None = None,
        recovery: float = 0.40,
    ) -> HazardCurve:
        """Generate a HazardCurve from structural model default probabilities.

        Parameters
        ----------
        curve_id : str
            Curve identifier.
        base_date : datetime.date
            Valuation date for the curve.
        tenors : list[float], optional
            Tenor grid in years (default: ``[0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0]``).
        recovery : float, optional
            Recovery rate assumption (default: 0.40).

        Returns
        -------
        HazardCurve

        Raises
        ------
        ValueError
            If the curve cannot be built.
        """
        ...

    @property
    def asset_value(self) -> float:
        """Current market value of the firm's assets."""
        ...

    @property
    def asset_vol(self) -> float:
        """Annualized volatility of asset returns."""
        ...

    @property
    def debt_barrier(self) -> float:
        """Face value of debt / default point."""
        ...

    @property
    def risk_free_rate(self) -> float:
        """Continuous risk-free rate."""
        ...

    @property
    def payout_rate(self) -> float:
        """Continuous dividend / payout yield on assets."""
        ...

    @property
    def barrier_type(self) -> MertonBarrierType:
        """Barrier monitoring type."""
        ...

    @property
    def dynamics(self) -> MertonAssetDynamics:
        """Asset return dynamics specification."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
