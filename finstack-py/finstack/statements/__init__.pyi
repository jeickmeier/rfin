"""Financial statement modeling engine.

This module provides tools for building, evaluating, and analyzing
financial statement models with deterministic evaluation, currency-safe
arithmetic, and support for forecasting methods, extensions, and dynamic
metric registries.

Module Structure (mirrors Rust):
- types - Wire types for serialization
- builder - Type-safe builder API
- evaluator - Evaluation engine
- analysis - Analysis tools (sensitivity, dependency tracing, reports)
- extensions - Plugin framework
- registry - Dynamic metric registry
"""

from __future__ import annotations
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
from .evaluator import ResultsMeta, StatementResult, Evaluator, EvaluatorWithContext, DependencyGraph
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
    # Sensitivity Analysis
    ParameterSpec,
    SensitivityMode,
    SensitivityConfig,
    SensitivityScenario,
    SensitivityResult,
    SensitivityAnalyzer,
    TornadoEntry,
    generate_tornado_chart,
    # Dependency Tracing & Formula Explanation
    ExplanationStep,
    Explanation,
    FormulaExplainer,
    DependencyTree,
    DependencyTracer,
    render_tree_ascii,
    render_tree_detailed,
    # Reports
    Alignment,
    TableBuilder,
    PLSummaryReport,
    CreditAssessmentReport,
    DebtSummaryReport,
    print_debt_summary,
    # Variance Analysis
    VarianceConfig,
    VarianceRow,
    VarianceReport,
    BridgeStep,
    BridgeChart,
    VarianceAnalyzer,
    # Scenario Management
    ScenarioDefinition,
    ScenarioSet,
    ScenarioResults,
    ScenarioDiff,
)
from .forecast import apply_forecast
from .dsl import StmtExpr, parse_formula, compile_formula, parse_and_compile
from . import templates
from . import capital_structure
from .templates import (
    LeaseSpec,
    RentStepSpec,
    FreeRentWindowSpec,
    RenewalSpec,
    LeaseGrowthConvention,
    ManagementFeeBase,
    ManagementFeeSpec,
    LeaseSpecV2,
    RentRollOutputNodes,
    PropertyTemplateNodes,
)
from .capital_structure import (
    CashflowBreakdown,
    CapitalStructureCashflows,
    aggregate_instrument_cashflows,
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
    "StatementResult",
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
    # Analysis (Sensitivity)
    "ParameterSpec",
    "SensitivityMode",
    "SensitivityConfig",
    "SensitivityScenario",
    "SensitivityResult",
    "SensitivityAnalyzer",
    "TornadoEntry",
    "generate_tornado_chart",
    # Analysis (Dependency Tracing & Explanation)
    "ExplanationStep",
    "Explanation",
    "FormulaExplainer",
    "DependencyTree",
    "DependencyTracer",
    "render_tree_ascii",
    "render_tree_detailed",
    # Analysis (Reports)
    "Alignment",
    "TableBuilder",
    "PLSummaryReport",
    "CreditAssessmentReport",
    "DebtSummaryReport",
    "print_debt_summary",
    # Analysis (Variance)
    "VarianceConfig",
    "VarianceRow",
    "VarianceReport",
    "BridgeStep",
    "BridgeChart",
    "VarianceAnalyzer",
    # Analysis (Scenario Management)
    "ScenarioDefinition",
    "ScenarioSet",
    "ScenarioResults",
    "ScenarioDiff",
    # Forecast helpers
    "apply_forecast",
    # DSL helpers
    "StmtExpr",
    "parse_formula",
    "compile_formula",
    "parse_and_compile",
    # Templates (Real Estate)
    "LeaseSpec",
    "RentStepSpec",
    "FreeRentWindowSpec",
    "RenewalSpec",
    "LeaseGrowthConvention",
    "ManagementFeeBase",
    "ManagementFeeSpec",
    "LeaseSpecV2",
    "RentRollOutputNodes",
    "PropertyTemplateNodes",
    # Capital Structure
    "CashflowBreakdown",
    "CapitalStructureCashflows",
    "aggregate_instrument_cashflows",
]
