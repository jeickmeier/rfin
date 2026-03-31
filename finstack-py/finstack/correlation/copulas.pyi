"""Type stubs for copula model bindings."""

from __future__ import annotations

class GaussianCopula:
    """One-factor Gaussian copula (market standard).

    The industry-standard model for credit index tranche pricing.
    Zero tail dependence; use with base correlation to capture the smile.

    Examples:
        >>> from finstack.correlation import GaussianCopula
        >>> copula = GaussianCopula()
        >>> copula.num_factors()
        1
    """

    def __init__(self, quadrature_order: int | None = None) -> None: ...
    def conditional_default_prob(
        self,
        default_threshold: float,
        factor_realization: list[float],
        correlation: float,
    ) -> float:
        """Conditional default probability P(default | Z)."""
        ...
    def num_factors(self) -> int:
        """Number of systematic factors (always 1)."""
        ...
    def model_name(self) -> str:
        """Model name for diagnostics."""
        ...
    def tail_dependence(self, correlation: float) -> float:
        """Lower-tail dependence (always 0 for Gaussian)."""
        ...
    def __repr__(self) -> str: ...

class StudentTCopula:
    """Student-t copula with configurable degrees of freedom.

    Captures tail dependence — joint extreme defaults cluster more than
    Gaussian predicts. Lower df = more tail dependence.

    Examples:
        >>> from finstack.correlation import StudentTCopula
        >>> copula = StudentTCopula(degrees_of_freedom=5.0)
        >>> copula.tail_dependence(0.5) > 0
        True
    """

    def __init__(
        self, degrees_of_freedom: float, quadrature_order: int | None = None
    ) -> None: ...
    @property
    def degrees_of_freedom(self) -> float:
        """Degrees of freedom."""
        ...
    def conditional_default_prob(
        self,
        default_threshold: float,
        factor_realization: list[float],
        correlation: float,
    ) -> float:
        """Conditional default probability P(default | M)."""
        ...
    def num_factors(self) -> int:
        """Number of systematic factors (always 1)."""
        ...
    def model_name(self) -> str:
        """Model name for diagnostics."""
        ...
    def tail_dependence(self, correlation: float) -> float:
        """Lower-tail dependence coefficient λ_L."""
        ...
    def __repr__(self) -> str: ...

class MultiFactorCopula:
    """Multi-factor Gaussian copula with sector structure.

    Uses a global factor plus sector-specific factors to model
    intra-sector vs. inter-sector correlation differences.

    Examples:
        >>> from finstack.correlation import MultiFactorCopula
        >>> copula = MultiFactorCopula(num_factors=2)
        >>> copula.intra_sector_correlation >= copula.inter_sector_correlation
        True
    """

    def __init__(
        self,
        num_factors: int,
        global_loading: float | None = None,
        sector_loading: float | None = None,
        sector_fraction: float | None = None,
    ) -> None: ...
    @property
    def inter_sector_correlation(self) -> float:
        """Inter-sector correlation (β_G²)."""
        ...
    @property
    def intra_sector_correlation(self) -> float:
        """Intra-sector correlation (β_G² + β_S²)."""
        ...
    def decompose_correlation(
        self, total_correlation: float, sector_fraction: float
    ) -> tuple[float, float]:
        """Decompose total correlation into (global_loading, sector_loading)."""
        ...
    def conditional_default_prob(
        self,
        default_threshold: float,
        factor_realization: list[float],
        correlation: float,
    ) -> float:
        """Conditional default probability P(default | Z_G, Z_S)."""
        ...
    def num_factors(self) -> int:
        """Number of systematic factors."""
        ...
    def model_name(self) -> str:
        """Model name for diagnostics."""
        ...
    def tail_dependence(self, correlation: float) -> float:
        """Lower-tail dependence (always 0 for multi-factor Gaussian)."""
        ...
    def __repr__(self) -> str: ...

class RandomFactorLoadingCopula:
    """Random Factor Loading copula with stochastic correlation.

    Models correlation itself as random, capturing increased correlation
    during market stress. Important for senior tranche pricing.

    Examples:
        >>> from finstack.correlation import RandomFactorLoadingCopula
        >>> copula = RandomFactorLoadingCopula(loading_volatility=0.15)
        >>> copula.num_factors()
        2
    """

    def __init__(
        self, loading_volatility: float, quadrature_order: int | None = None
    ) -> None: ...
    @property
    def loading_volatility(self) -> float:
        """Loading volatility."""
        ...
    def conditional_default_prob(
        self,
        default_threshold: float,
        factor_realization: list[float],
        correlation: float,
    ) -> float:
        """Conditional default probability P(default | Z, η)."""
        ...
    def num_factors(self) -> int:
        """Number of systematic factors (2: market + loading shock)."""
        ...
    def model_name(self) -> str:
        """Model name for diagnostics."""
        ...
    def tail_dependence(self, correlation: float) -> float:
        """Stress-dependence gauge (monotone proxy, not strict λ_L)."""
        ...
    def __repr__(self) -> str: ...

class CopulaSpec:
    """Copula model specification for configuration and serialization.

    Use static constructors: ``CopulaSpec.gaussian()``, ``CopulaSpec.student_t(5.0)``, etc.

    Examples:
        >>> from finstack.correlation import CopulaSpec
        >>> spec = CopulaSpec.gaussian()
        >>> spec.is_gaussian()
        True
    """

    @staticmethod
    def gaussian() -> CopulaSpec:
        """Create a Gaussian copula specification."""
        ...
    @staticmethod
    def student_t(degrees_of_freedom: float) -> CopulaSpec:
        """Create a Student-t copula specification (df must be > 2)."""
        ...
    @staticmethod
    def random_factor_loading(loading_volatility: float) -> CopulaSpec:
        """Create a Random Factor Loading specification."""
        ...
    @staticmethod
    def multi_factor(num_factors: int) -> CopulaSpec:
        """Create a multi-factor copula specification."""
        ...
    def is_gaussian(self) -> bool: ...
    def is_student_t(self) -> bool: ...
    def is_rfl(self) -> bool: ...
    def is_multi_factor(self) -> bool: ...
    def to_json(self) -> str:
        """Serialize to JSON string."""
        ...
    @staticmethod
    def from_json(json: str) -> CopulaSpec:
        """Deserialize from JSON string."""
        ...
    def __repr__(self) -> str: ...
