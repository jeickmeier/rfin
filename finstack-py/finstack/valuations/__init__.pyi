"""Valuations bindings mirroring finstack-valuations: instruments, pricers, metrics, and result envelopes."""

from __future__ import annotations
from . import common
from . import cashflow
from . import results
from . import pricer
from . import metrics
from . import instruments
from . import calibration
from . import risk
from . import attribution
from . import performance
from . import lsmc
from . import conventions
from . import margin
from . import market
from . import bumps

# Import common types that are re-exported at the valuations level
from .common import InstrumentType, ModelKey, PricerKey
from .common.mc import PathPoint, SimulatedPath, PathDataset, ProcessParams, MonteCarloResult, MonteCarloPathGenerator
from .common import parse
from .pricer import PricerRegistry, create_standard_registry
from .results import ValuationResult, ResultsMeta, CovenantReport
from .metrics import MetricId, MetricRegistry
from .performance import xirr, npv, irr_periodic
from .instruments.structured_credit.waterfall import (
    AllocationMode,
    PaymentType,
    WaterfallTier,
)

# LSMC exports
from .lsmc import (
    AmericanPut,
    AmericanCall,
    PolynomialBasis,
    LaguerreBasis,
    LsmcConfig,
    LsmcResult,
    LsmcPricer,
)

# Conventions exports
from .conventions import (
    CdsDocClause,
    RateIndexKind,
    CdsConventionKey,
    RateIndexConventions,
    CdsConventions,
    SwaptionConventions,
    InflationSwapConventions,
    OptionConventions,
    IrFutureConventions,
    ConventionRegistry,
)

# Margin exports
from .margin import (
    MarginTenor,
    ImMethodology,
    MarginCallTiming,
    VmParameters,
    ImParameters,
    EligibleCollateralSchedule,
    CsaSpec,
)

# Market builder exports
from .market import (
    QuoteId,
    Pillar,
    BuildCtx,
    BuiltInstrument,
    RateQuote,
    CdsQuote,
    CdsTrancheQuote,
    CdsTrancheBuildOverrides,
    build_rate_instrument,
    build_cds_instrument,
    build_cds_tranche_instrument,
)

# Bumps exports
from .bumps import (
    BumpRequest,
    bump_discount_curve_synthetic,
    bump_hazard_spreads,
    bump_hazard_shift,
    bump_inflation_rates,
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
    # LSMC
    "AmericanPut",
    "AmericanCall",
    "PolynomialBasis",
    "LaguerreBasis",
    "LsmcConfig",
    "LsmcResult",
    "LsmcPricer",
    # Conventions
    "CdsDocClause",
    "RateIndexKind",
    "CdsConventionKey",
    "RateIndexConventions",
    "CdsConventions",
    "SwaptionConventions",
    "InflationSwapConventions",
    "OptionConventions",
    "IrFutureConventions",
    "ConventionRegistry",
    # Margin
    "MarginTenor",
    "ImMethodology",
    "MarginCallTiming",
    "VmParameters",
    "ImParameters",
    "EligibleCollateralSchedule",
    "CsaSpec",
    # Market builders
    "QuoteId",
    "Pillar",
    "BuildCtx",
    "BuiltInstrument",
    "RateQuote",
    "CdsQuote",
    "CdsTrancheQuote",
    "CdsTrancheBuildOverrides",
    "build_rate_instrument",
    "build_cds_instrument",
    "build_cds_tranche_instrument",
    # Bumps
    "BumpRequest",
    "bump_discount_curve_synthetic",
    "bump_hazard_spreads",
    "bump_hazard_shift",
    "bump_inflation_rates",
    # Risk
    "risk",
    # Attribution
    "attribution",
    # Cashflow
    "cashflow",
    # Instruments (imported from submodule)
    # Calibration (imported from submodule)
    # Cashflow (imported from submodule)
]
