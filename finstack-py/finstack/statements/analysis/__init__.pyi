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
    """Sensitivity analyzer for financial statement models.

    SensitivityAnalyzer performs sensitivity analysis on financial models
    by varying input parameters and observing the impact on target metrics.
    It supports diagonal (one-at-a-time), full grid, and tornado chart
    analysis modes.

    Sensitivity analysis is used to:
    - Identify key drivers of financial performance
    - Assess model robustness
    - Generate tornado charts for visualization
    - Perform what-if analysis

    Examples
    --------
    Run sensitivity analysis:

        >>> from finstack.core.dates.periods import PeriodId
        >>> from finstack.statements.analysis import (
        ...     ParameterSpec,
        ...     SensitivityAnalyzer,
        ...     SensitivityConfig,
        ...     SensitivityMode,
        ... )
        >>> from finstack.statements.builder import ModelBuilder
        >>> from finstack.statements.types import AmountOrScalar
        >>> builder = ModelBuilder.new("DocCo")
        >>> builder.periods("2025Q1..Q2", None)
        >>> builder.value(
        ...     "revenue",
        ...     [
        ...         (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100.0)),
        ...         (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(110.0)),
        ...     ],
        ... )
        >>> builder.compute("net_income", "revenue * 0.2")
        >>> model = builder.build()
        >>> analyzer = SensitivityAnalyzer(model)
        >>> config = SensitivityConfig(SensitivityMode.DIAGONAL)
        >>> config.add_parameter(
        ...     ParameterSpec.with_percentages("revenue", PeriodId.quarter(2025, 1), 100.0, [-10.0, 0.0, 10.0])
        ... )
        >>> config.add_target_metric("net_income")
        >>> result = analyzer.run(config)
        >>> values = [round(s.parameter_values["revenue"], 1) for s in result.scenarios]
        >>> print(len(result.scenarios), min(values), max(values))
        3 90.0 110.0

    Notes
    -----
    - Supports multiple analysis modes (diagonal, full grid, tornado)
    - Can vary multiple parameters simultaneously
    - Tracks impact on multiple target metrics
    - Results can be exported for visualization

    See Also
    --------
    :class:`SensitivityConfig`: Analysis configuration
    :class:`SensitivityResult`: Analysis results
    :func:`generate_tornado_chart`: Tornado chart generation
    """

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

def generate_tornado_chart(result: SensitivityResult, metric: str) -> List[TornadoEntry]: ...

"""Generate tornado chart data from sensitivity analysis results.

A tornado chart shows the sensitivity of a target metric to variations
in input parameters. Entries are sorted by swing magnitude (difference
between upside and downside impact), making it easy to identify the
most influential parameters.

Parameters
----------
result : SensitivityResult
    Results from sensitivity analysis. Must include scenarios with
    parameter variations and target metric values.
metric : str
    Target metric identifier to analyze (e.g., "net_income", "ebitda").

Returns
-------
List[TornadoEntry]
    List of tornado entries sorted by swing magnitude (largest impact first).
    Each entry contains parameter identifier, downside impact, and upside impact.

Raises
------
ValueError
    If metric is not found in sensitivity results.

Examples
--------
    >>> from finstack.statements.analysis import generate_tornado_chart
    >>> 
    >>> # Run sensitivity analysis first
    >>> result = analyzer.run(config)
    >>> 
    >>> # Generate tornado chart
    >>> tornado = generate_tornado_chart(result, "net_income")
    >>> 
    >>> # Display results
    >>> for entry in tornado:
    ...     print(f"{entry.parameter_id}: {entry.swing:.2f} swing")
    revenue: 50000.00 swing
    cogs: -30000.00 swing
    operating_expenses: -15000.00 swing

Notes
-----
- Entries are sorted by absolute swing magnitude
- Downside impact is the change when parameter is decreased
- Upside impact is the change when parameter is increased
- Swing = upside_impact - downside_impact

See Also
--------
:class:`SensitivityAnalyzer`: Sensitivity analysis engine
:class:`TornadoEntry`: Tornado chart entry structure
"""

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
