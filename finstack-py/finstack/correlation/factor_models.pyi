"""Type stubs for factor model bindings."""

from __future__ import annotations

class SingleFactorModel:
    """Single-factor model (common market factor).

    Models all correlation through a single systematic factor.

    Examples:
        >>> from finstack.correlation import SingleFactorModel
        >>> model = SingleFactorModel(volatility=0.25, mean_reversion=0.10)
        >>> model.num_factors()
        1
    """

    def __init__(self, volatility: float, mean_reversion: float) -> None: ...
    @property
    def volatility(self) -> float:
        """Factor volatility."""
        ...
    @property
    def mean_reversion(self) -> float:
        """Mean reversion speed."""
        ...
    def num_factors(self) -> int:
        """Number of factors (always 1)."""
        ...
    def correlation_matrix(self) -> list[float]:
        """Factor correlation matrix (flattened row-major)."""
        ...
    def volatilities(self) -> list[float]:
        """Factor volatilities."""
        ...
    def factor_names(self) -> list[str]:
        """Factor names for reporting."""
        ...
    def model_name(self) -> str:
        """Model name for diagnostics."""
        ...
    def diagonal_factor_contribution(self, factor_index: int, z: float) -> float:
        """Diagonal factor contribution for a single z draw."""
        ...
    def __repr__(self) -> str: ...

class TwoFactorModel:
    """Two-factor model for prepayment and credit.

    Captures the empirical negative correlation between prepayment and default.

    Examples:
        >>> from finstack.correlation import TwoFactorModel
        >>> model = TwoFactorModel.rmbs_standard()
        >>> model.correlation < 0
        True
    """

    def __init__(
        self, prepay_vol: float, credit_vol: float, correlation: float
    ) -> None: ...
    @staticmethod
    def rmbs_standard() -> TwoFactorModel:
        """Standard RMBS calibration (prepay=0.20, credit=0.25, corr=-0.30)."""
        ...
    @staticmethod
    def clo_standard() -> TwoFactorModel:
        """Standard CLO calibration (prepay=0.15, credit=0.30, corr=-0.20)."""
        ...
    @property
    def prepay_vol(self) -> float:
        """Prepayment factor volatility."""
        ...
    @property
    def credit_vol(self) -> float:
        """Credit factor volatility."""
        ...
    @property
    def correlation(self) -> float:
        """Factor correlation."""
        ...
    @property
    def cholesky_l10(self) -> float:
        """Cholesky L[1][0] coefficient."""
        ...
    @property
    def cholesky_l11(self) -> float:
        """Cholesky L[1][1] coefficient."""
        ...
    def num_factors(self) -> int:
        """Number of factors (always 2)."""
        ...
    def correlation_matrix(self) -> list[float]:
        """Factor correlation matrix (flattened row-major)."""
        ...
    def volatilities(self) -> list[float]:
        """Factor volatilities."""
        ...
    def factor_names(self) -> list[str]:
        """Factor names for reporting."""
        ...
    def model_name(self) -> str:
        """Model name for diagnostics."""
        ...
    def diagonal_factor_contribution(self, factor_index: int, z: float) -> float:
        """Diagonal factor contribution for a single z draw."""
        ...
    def __repr__(self) -> str: ...

class MultiFactorModel:
    """Multi-factor model with custom correlation structure.

    Supports arbitrary number of factors with a validated correlation matrix.
    Uses pivoted Cholesky decomposition for correlated factor generation.

    Examples:
        >>> from finstack.correlation import MultiFactorModel
        >>> model = MultiFactorModel.uncorrelated(num_factors=2, volatilities=[0.2, 0.3])
        >>> model.generate_correlated_factors([1.0, -1.0])
        [0.2, -0.3]
    """

    def __init__(
        self,
        num_factors: int,
        volatilities: list[float],
        correlations: list[float],
    ) -> None: ...
    @staticmethod
    def uncorrelated(num_factors: int, volatilities: list[float]) -> MultiFactorModel:
        """Create an uncorrelated (identity) multi-factor model."""
        ...
    def generate_correlated_factors(self, independent_z: list[float]) -> list[float]:
        """Generate correlated factor values from independent standard normal draws."""
        ...
    def num_factors(self) -> int:
        """Number of factors."""
        ...
    def correlation_matrix(self) -> list[float]:
        """Factor correlation matrix (flattened row-major)."""
        ...
    def volatilities(self) -> list[float]:
        """Factor volatilities."""
        ...
    def factor_names(self) -> list[str]:
        """Factor names for reporting."""
        ...
    def model_name(self) -> str:
        """Model name for diagnostics."""
        ...
    def diagonal_factor_contribution(self, factor_index: int, z: float) -> float:
        """Diagonal factor contribution for a single z draw."""
        ...
    def cholesky_factor_matrix(self) -> list[float]:
        """Cholesky factor matrix (flattened row-major)."""
        ...
    def __repr__(self) -> str: ...

class FactorSpec:
    """Factor model specification for configuration and serialization.

    Use static constructors: ``FactorSpec.single_factor(...)``, ``FactorSpec.two_factor(...)``.

    Examples:
        >>> from finstack.correlation import FactorSpec
        >>> spec = FactorSpec.single_factor(volatility=0.25, mean_reversion=0.10)
        >>> spec.num_factors()
        1
    """

    @staticmethod
    def single_factor(volatility: float, mean_reversion: float) -> FactorSpec:
        """Create a single factor specification."""
        ...
    @staticmethod
    def two_factor(
        prepay_vol: float, credit_vol: float, correlation: float
    ) -> FactorSpec:
        """Create a two-factor specification."""
        ...
    def num_factors(self) -> int:
        """Number of factors implied by this specification."""
        ...
    def to_json(self) -> str:
        """Serialize to JSON string."""
        ...
    @staticmethod
    def from_json(json: str) -> FactorSpec:
        """Deserialize from JSON string."""
        ...
    def __repr__(self) -> str: ...
