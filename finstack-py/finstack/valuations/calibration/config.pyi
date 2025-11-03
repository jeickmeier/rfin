"""Calibration configuration classes."""

from typing import Optional, Dict, Any

class SolverKind:
    """Solver kind enumeration for calibration."""

    # Class attributes
    NEWTON: SolverKind
    BRENT: SolverKind
    HYBRID: SolverKind
    LEVENBERG_MARQUARDT: SolverKind
    DIFFERENTIAL_EVOLUTION: SolverKind

    @classmethod
    def from_name(cls, name: str) -> SolverKind:
        """Create solver kind from name.

        Args:
            name: Solver name (e.g., "newton", "brent")

        Returns:
            SolverKind: Corresponding solver kind

        Raises:
            KeyError: If name is unknown
        """
        ...

    @property
    def name(self) -> str:
        """Solver name.

        Returns:
            str: Solver name
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __richcmp__(self, other: object, op: int) -> object: ...

class MultiCurveConfig:
    """Multi-curve calibration configuration."""

    def __init__(self, calibrate_basis: bool = True, enforce_separation: bool = True) -> None:
        """Create multi-curve configuration.

        Args:
            calibrate_basis: Whether to calibrate basis
            enforce_separation: Whether to enforce separation
        """
        ...

    @classmethod
    def new_standard(cls) -> MultiCurveConfig:
        """Create standard multi-curve configuration.

        Returns:
            MultiCurveConfig: Standard configuration
        """
        ...

    @property
    def calibrate_basis(self) -> bool:
        """Whether to calibrate basis."""
        ...

    @property
    def enforce_separation(self) -> bool:
        """Whether to enforce separation."""
        ...

    def with_calibrate_basis(self, value: bool) -> MultiCurveConfig:
        """Create new config with updated calibrate_basis."""
        ...

    def with_enforce_separation(self, value: bool) -> MultiCurveConfig:
        """Create new config with updated enforce_separation."""
        ...

    def __repr__(self) -> str: ...

class CalibrationConfig:
    """Calibration configuration."""

    def __init__(
        self,
        *,
        tolerance: float = 1e-10,
        max_iterations: int = 100,
        use_parallel: bool = False,
        random_seed: Optional[int] = 42,
        verbose: bool = False,
        solver_kind: Optional[SolverKind] = None,
        multi_curve: Optional[MultiCurveConfig] = None,
        entity_seniority: Optional[Dict[str, str]] = None,
    ) -> None:
        """Create calibration configuration.

        Args:
            tolerance: Convergence tolerance
            max_iterations: Maximum iterations
            use_parallel: Whether to use parallel processing
            random_seed: Random seed for reproducibility
            verbose: Whether to enable verbose output
            solver_kind: Solver kind to use
            multi_curve: Multi-curve configuration
            entity_seniority: Entity seniority mapping
        """
        ...

    @classmethod
    def multi_curve(cls) -> CalibrationConfig:
        """Create multi-curve calibration configuration.

        Returns:
            CalibrationConfig: Multi-curve configuration
        """
        ...

    @property
    def tolerance(self) -> float:
        """Convergence tolerance."""
        ...

    @property
    def max_iterations(self) -> int:
        """Maximum iterations."""
        ...

    @property
    def use_parallel(self) -> bool:
        """Whether to use parallel processing."""
        ...

    @property
    def random_seed(self) -> Optional[int]:
        """Random seed for reproducibility."""
        ...

    @property
    def verbose(self) -> bool:
        """Whether to enable verbose output."""
        ...

    @property
    def solver_kind(self) -> SolverKind:
        """Solver kind to use."""
        ...

    @property
    def multi_curve_config(self) -> MultiCurveConfig:
        """Multi-curve configuration."""
        ...

    @property
    def entity_seniority(self) -> Dict[str, str]:
        """Entity seniority mapping."""
        ...

    def with_tolerance(self, tolerance: float) -> CalibrationConfig:
        """Create new config with updated tolerance."""
        ...

    def with_max_iterations(self, max_iterations: int) -> CalibrationConfig:
        """Create new config with updated max_iterations."""
        ...

    def with_parallel(self, flag: bool) -> CalibrationConfig:
        """Create new config with updated use_parallel."""
        ...

    def with_random_seed(self, seed: Optional[int]) -> CalibrationConfig:
        """Create new config with updated random_seed."""
        ...

    def with_verbose(self, verbose: bool) -> CalibrationConfig:
        """Create new config with updated verbose."""
        ...

    def with_solver_kind(self, kind: SolverKind) -> CalibrationConfig:
        """Create new config with updated solver_kind."""
        ...

    def with_multi_curve_config(self, config: MultiCurveConfig) -> CalibrationConfig:
        """Create new config with updated multi_curve."""
        ...

    def with_entity_seniority(self, mapping: Dict[str, str]) -> CalibrationConfig:
        """Create new config with updated entity_seniority."""
        ...

    def __repr__(self) -> str: ...
