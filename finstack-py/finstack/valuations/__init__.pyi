"""Valuations bindings mirroring finstack-valuations: instruments, pricers, metrics, and result envelopes."""

from . import common
from . import cashflow
from . import results
from . import pricer
from . import metrics
from . import instruments
from . import calibration
from . import performance

# Import common types that are re-exported at the valuations level
from .common import InstrumentType, ModelKey, PricerKey
from .common.mc import PathPoint, SimulatedPath, PathDataset, ProcessParams, MonteCarloResult, MonteCarloPathGenerator
from .common import parse
from .pricer import PricerRegistry, create_standard_registry
from .results import ValuationResult, ResultsMeta, CovenantReport
from .metrics import MetricId, MetricRegistry
from .performance import xirr, npv, irr_periodic
from .instruments.structured_credit import (
    AllocationMode,
    PaymentType,
    WaterfallTier,
)

__all__ = [
    # Common types
    "InstrumentType",
    "ModelKey",
    "PricerKey",
    "parse",
    # Pricer
    "PricerRegistry",
    "create_standard_registry",
    # Results
    "ValuationResult",
    "ResultsMeta",
    "CovenantReport",
    # Metrics
    "MetricId",
    "MetricRegistry",
    # Monte Carlo Path Visualization
    "PathPoint",
    "SimulatedPath",
    "PathDataset",
    "ProcessParams",
    "MonteCarloResult",
    "MonteCarloPathGenerator",
    # Performance
    "xirr",
    "npv",
    "irr_periodic",
    # Waterfall Engine
    "AllocationMode",
    "PaymentType",
    "WaterfallTier",
    # Instruments (imported from submodule)
    # Calibration (imported from submodule)
    # Cashflow (imported from submodule)
]
