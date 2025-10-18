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
]
