"""Sensitivity analysis for financial statement models."""

from typing import Any, Dict, List
from ..evaluator import Results
from ..types import FinancialModelSpec
from ...core.dates.periods import PeriodId

class SensitivityMode:
    """Sensitivity analysis mode."""

    DIAGONAL: SensitivityMode
    FULL_GRID: SensitivityMode
    TORNADO: SensitivityMode

    def __repr__(self) -> str: ...

class ParameterSpec:
    """Parameter specification for sensitivity analysis."""

    def __init__(self, node_id: str, period_id: PeriodId, base_value: float, perturbations: List[float]) -> None:
        """Create a new parameter specification.

        Args:
            node_id: Node identifier
            period_id: Period to vary
            base_value: Base value
            perturbations: Perturbations to apply
        """
        ...

    @staticmethod
    def with_percentages(node_id: str, period_id: PeriodId, base_value: float, pct_range: List[float]) -> ParameterSpec:
        """Create a parameter spec with percentage perturbations.

        Args:
            node_id: Node identifier
            period_id: Period to vary
            base_value: Base value
            pct_range: Percentage range (e.g., [-10.0, 0.0, 10.0] for ±10%)

        Returns:
            ParameterSpec: Parameter specification
        """
        ...

    @property
    def node_id(self) -> str: ...
    @property
    def period_id(self) -> PeriodId: ...
    @property
    def base_value(self) -> float: ...
    @property
    def perturbations(self) -> List[float]: ...
    def __repr__(self) -> str: ...

class SensitivityConfig:
    """Sensitivity analysis configuration."""

    def __init__(self, mode: SensitivityMode) -> None:
        """Create a new sensitivity configuration.

        Args:
            mode: Analysis mode
        """
        ...

    def add_parameter(self, param: ParameterSpec) -> None:
        """Add a parameter to vary.

        Args:
            param: Parameter specification
        """
        ...

    def add_target_metric(self, metric: str) -> None:
        """Add a target metric to track.

        Args:
            metric: Metric identifier
        """
        ...

    @property
    def mode(self) -> SensitivityMode: ...
    @property
    def parameters(self) -> List[ParameterSpec]: ...
    @property
    def target_metrics(self) -> List[str]: ...
    def __repr__(self) -> str: ...

class SensitivityScenario:
    """Result of a single sensitivity scenario."""

    @property
    def parameter_values(self) -> Dict[str, float]: ...
    @property
    def results(self) -> Results: ...
    def __repr__(self) -> str: ...

class SensitivityResult:
    """Results of sensitivity analysis."""

    @property
    def config(self) -> SensitivityConfig: ...
    @property
    def scenarios(self) -> List[SensitivityScenario]: ...
    def __len__(self) -> int: ...
    def __repr__(self) -> str: ...

class SensitivityAnalyzer:
    """Sensitivity analyzer for financial models."""

    def __init__(self, model: FinancialModelSpec) -> None:
        """Create a new sensitivity analyzer.

        Args:
            model: Financial model to analyze
        """
        ...

    def run(self, config: SensitivityConfig) -> SensitivityResult:
        """Run sensitivity analysis.

        Args:
            config: Analysis configuration

        Returns:
            SensitivityResult: Analysis results
        """
        ...

    def __repr__(self) -> str: ...

class TornadoEntry:
    """Entry in a tornado chart."""

    def __init__(self, parameter_id: str, downside_impact: float, upside_impact: float) -> None:
        """Create a tornado entry.

        Args:
            parameter_id: Parameter identifier
            downside_impact: Impact of low value
            upside_impact: Impact of high value
        """
        ...

    @property
    def parameter_id(self) -> str: ...
    @property
    def downside_impact(self) -> float: ...
    @property
    def upside_impact(self) -> float: ...
    @property
    def swing(self) -> float: ...
    def __repr__(self) -> str: ...

def generate_tornado_chart(result: SensitivityResult, metric: str) -> List[TornadoEntry]:
    """Generate tornado chart data from sensitivity results.

    Args:
        result: Sensitivity analysis results
        metric: Target metric identifier

    Returns:
        List[TornadoEntry]: Tornado entries sorted by swing magnitude
    """
    ...

__all__ = [
    "ParameterSpec",
    "SensitivityMode",
    "SensitivityConfig",
    "SensitivityScenario",
    "SensitivityResult",
    "SensitivityAnalyzer",
    "TornadoEntry",
    "generate_tornado_chart",
]







