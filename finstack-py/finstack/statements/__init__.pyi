"""Financial statement modeling engine.

This module provides tools for building, evaluating, and analyzing
financial statement models with deterministic evaluation, currency-safe
arithmetic, and support for forecasting methods, extensions, and dynamic
metric registries.
"""

from .types import (
    NodeType,
    NodeSpec,
    ForecastMethod,
    ForecastSpec,
    SeasonalMode,
    AmountOrScalar,
    FinancialModelSpec,
    CapitalStructureSpec,
    DebtInstrumentSpec,
)
from .builder import ModelBuilder
from .evaluator import ResultsMeta, Results, Evaluator
from .extensions import (
    ExtensionMetadata,
    ExtensionStatus,
    ExtensionResult,
    ExtensionContext,
    ExtensionRegistry,
    CorkscrewExtension,
    CreditScorecardExtension,
)
from .registry import (
    Registry,
    MetricDefinition,
    MetricRegistry,
    UnitType,
)
from .analysis import (
    ParameterSpec,
    SensitivityMode,
    SensitivityConfig,
    SensitivityScenario,
    SensitivityResult,
    SensitivityAnalyzer,
    TornadoEntry,
    generate_tornado_chart,
)
from .explain import (
    ExplanationStep,
    Explanation,
    FormulaExplainer,
    DependencyTree,
    DependencyTracer,
    render_tree_ascii,
    render_tree_detailed,
)
from .evaluator import (
    EvaluatorWithContext,
    DependencyGraph,
)
from .reports import (
    Alignment,
    TableBuilder,
    PLSummaryReport,
    CreditAssessmentReport,
    DebtSummaryReport,
    print_debt_summary,
)

__all__ = [
    # Types
    "NodeType",
    "NodeSpec",
    "ForecastMethod",
    "ForecastSpec",
    "SeasonalMode",
    "AmountOrScalar",
    "FinancialModelSpec",
    "CapitalStructureSpec",
    "DebtInstrumentSpec",
    # Builder
    "ModelBuilder",
    # Evaluator
    "ResultsMeta",
    "Results",
    "Evaluator",
    "EvaluatorWithContext",
    "DependencyGraph",
    # Extensions
    "ExtensionMetadata",
    "ExtensionStatus",
    "ExtensionResult",
    "ExtensionContext",
    "ExtensionRegistry",
    "CorkscrewExtension",
    "CreditScorecardExtension",
    # Registry
    "Registry",
    "MetricDefinition",
    "MetricRegistry",
    "UnitType",
    # Analysis
    "ParameterSpec",
    "SensitivityMode",
    "SensitivityConfig",
    "SensitivityScenario",
    "SensitivityResult",
    "SensitivityAnalyzer",
    "TornadoEntry",
    "generate_tornado_chart",
    # Explain
    "ExplanationStep",
    "Explanation",
    "FormulaExplainer",
    "DependencyTree",
    "DependencyTracer",
    "render_tree_ascii",
    "render_tree_detailed",
    # Reports
    "Alignment",
    "TableBuilder",
    "PLSummaryReport",
    "CreditAssessmentReport",
    "DebtSummaryReport",
    "print_debt_summary",
]
