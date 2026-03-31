"""Type stubs for recovery model bindings."""

from __future__ import annotations

class ConstantRecovery:
    """Constant recovery rate model.

    Recovery is fixed regardless of market conditions. ISDA standard is 40%.

    Examples:
        >>> from finstack.correlation import ConstantRecovery
        >>> model = ConstantRecovery(rate=0.40)
        >>> model.expected_recovery()
        0.4
    """

    def __init__(self, rate: float) -> None: ...
    @staticmethod
    def isda_standard() -> ConstantRecovery:
        """ISDA standard recovery rate (40%)."""
        ...
    @staticmethod
    def senior_secured() -> ConstantRecovery:
        """Senior secured recovery rate (55%)."""
        ...
    @staticmethod
    def subordinated() -> ConstantRecovery:
        """Subordinated debt recovery rate (25%)."""
        ...
    @property
    def rate(self) -> float:
        """Recovery rate."""
        ...
    def expected_recovery(self) -> float:
        """Expected (unconditional) recovery rate."""
        ...
    def conditional_recovery(self, market_factor: float) -> float:
        """Recovery rate conditional on market factor (constant for this model)."""
        ...
    def lgd(self) -> float:
        """Loss given default = 1 - recovery."""
        ...
    def conditional_lgd(self, market_factor: float) -> float:
        """Conditional LGD given market factor."""
        ...
    def recovery_volatility(self) -> float:
        """Recovery-rate volatility (0 for constant models)."""
        ...
    def is_stochastic(self) -> bool:
        """Whether this model is stochastic (always False for constant)."""
        ...
    def model_name(self) -> str:
        """Model name for diagnostics."""
        ...
    def __repr__(self) -> str: ...

class CorrelatedRecovery:
    """Market-correlated stochastic recovery model (Andersen-Sidenius).

    Recovery varies with the systematic market factor, capturing the
    empirical negative correlation between defaults and recovery.

    Examples:
        >>> from finstack.correlation import CorrelatedRecovery
        >>> model = CorrelatedRecovery.market_standard()
        >>> model.is_stochastic()
        True
    """

    def __init__(
        self,
        mean_recovery: float,
        recovery_volatility: float,
        factor_correlation: float,
    ) -> None: ...
    @staticmethod
    def with_bounds(
        mean_recovery: float,
        recovery_volatility: float,
        factor_correlation: float,
        min_recovery: float,
        max_recovery: float,
    ) -> CorrelatedRecovery:
        """Create with custom recovery bounds."""
        ...
    @staticmethod
    def market_standard() -> CorrelatedRecovery:
        """Market-standard calibration (mean=40%, vol=25%, corr=-40%)."""
        ...
    @staticmethod
    def conservative() -> CorrelatedRecovery:
        """Conservative calibration (mean=40%, vol=30%, corr=-50%)."""
        ...
    @property
    def mean(self) -> float:
        """Mean recovery rate."""
        ...
    @property
    def volatility(self) -> float:
        """Recovery volatility."""
        ...
    @property
    def correlation(self) -> float:
        """Factor correlation."""
        ...
    def expected_recovery(self) -> float:
        """Expected (unconditional) recovery rate."""
        ...
    def conditional_recovery(self, market_factor: float) -> float:
        """Recovery rate conditional on market factor."""
        ...
    def lgd(self) -> float:
        """Loss given default = 1 - recovery."""
        ...
    def conditional_lgd(self, market_factor: float) -> float:
        """Conditional LGD given market factor."""
        ...
    def recovery_volatility(self) -> float:
        """Recovery-rate volatility scale."""
        ...
    def is_stochastic(self) -> bool:
        """Whether this model is stochastic."""
        ...
    def model_name(self) -> str:
        """Model name for diagnostics."""
        ...
    def __repr__(self) -> str: ...

class RecoverySpec:
    """Recovery model specification for configuration and serialization.

    Use static constructors: ``RecoverySpec.constant(0.40)``,
    ``RecoverySpec.market_correlated(...)``.

    Examples:
        >>> from finstack.correlation import RecoverySpec
        >>> spec = RecoverySpec.constant(0.40)
        >>> spec.expected_recovery()
        0.4
    """

    @staticmethod
    def constant(rate: float) -> RecoverySpec:
        """Create a constant recovery specification."""
        ...
    @staticmethod
    def market_correlated(
        mean_recovery: float, recovery_volatility: float, factor_correlation: float
    ) -> RecoverySpec:
        """Create a market-correlated recovery specification."""
        ...
    @staticmethod
    def market_standard_stochastic() -> RecoverySpec:
        """Market-standard stochastic recovery (mean=40%, vol=25%, corr=-40%)."""
        ...
    def expected_recovery(self) -> float:
        """Expected recovery rate from specification."""
        ...
    def to_json(self) -> str:
        """Serialize to JSON string."""
        ...
    @staticmethod
    def from_json(json: str) -> RecoverySpec:
        """Deserialize from JSON string."""
        ...
    def __repr__(self) -> str: ...
