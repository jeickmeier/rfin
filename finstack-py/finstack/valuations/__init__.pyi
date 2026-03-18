"""Valuations bindings mirroring finstack-valuations: instruments, pricers, metrics, and result envelopes."""

from __future__ import annotations
from . import common
from . import cashflow
from . import constants
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
from . import schema
from . import xva

# Import common types that are re-exported at the valuations level
from .common import InstrumentType, ModelKey, PricerKey
from .common.monte_carlo import (
    PathPoint,
    SimulatedPath,
    PathDataset,
    ProcessParams,
    MonteCarloResult,
    MonteCarloPathGenerator,
)
from .common import parse
from .pricer import PricerRegistry, create_standard_registry
from .results import ValuationResult, ResultsMeta, CovenantReport
from .metrics import MetricId, MetricRegistry
from .performance import xirr, npv, irr_periodic
from .risk import (
    VarMethod,
    VarConfig,
    VarResult,
    RiskFactorType,
    RiskFactorShift,
    MarketScenario,
    MarketHistory,
    calculate_var,
    krd_dv01_ladder,
    cs01_ladder,
)
from .attribution import (
    AttributionMethod,
    AttributionMeta,
    CarryDetail,
    CorrelationsAttribution,
    CrossFactorDetail,
    RatesCurvesAttribution,
    CreditCurvesAttribution,
    CurveRestoreFlags,
    FxAttribution,
    InflationCurvesAttribution,
    MarketSnapshot,
    ModelParamsAttribution,
    PnlAttribution,
    PortfolioAttribution,
    ScalarsAttribution,
    ScalarsSnapshot,
    TaylorAttributionConfig,
    TaylorAttributionResult,
    TaylorFactorResult,
    VolatilitySnapshot,
    VolAttribution,
    attribute_pnl,
    attribute_pnl_taylor,
    attribute_portfolio_pnl,
    attribute_pnl_from_json,
    attribution_result_to_json,
    compute_pnl,
    compute_pnl_with_fx,
    convert_currency,
    default_waterfall_order,
    reprice_instrument,
)
from .calibration import (
    CalibrationDiagnostics,
    CalibrationReport,
    QuoteQuality,
    RateBounds,
    RateBoundsPolicy,
    SolverKind,
    ValidationConfig,
    ValidationMode,
    CALIBRATION_SCHEMA,
    execute_calibration,
)
from .instruments.fixed_income.structured_credit.waterfall import (
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
    CDSTrancheBuildOverrides,
    build_rate_instrument,
    build_cds_instrument,
    build_cds_tranche_instrument,
)

# Bumps exports
from .bumps import (
    BumpRequest,
    bump_discount_curve,
    bump_discount_curve_synthetic,
    bump_hazard_spreads,
    bump_hazard_shift,
    bump_inflation_rates,
)

# Schema exports
from .schema import bond_schema, valuation_result_schema

# XVA exports
from .xva import (
    FundingConfig,
    XvaConfig,
    CsaTerms,
    NettingSet,
    ExposureDiagnostics,
    ExposureProfile,
    StochasticExposureConfig,
    StochasticExposureProfile,
    XvaResult,
    apply_netting,
    apply_collateral,
    compute_exposure_profile,
    compute_cva,
    compute_dva,
    compute_fva,
    compute_bilateral_xva,
)

__all__ = [
    # Submodules
    "instruments",
    "common",
    "constants",
    "calibration",
    "pricer",
    "results",
    "metrics",
    "lsmc",
    "conventions",
    "margin",
    "market",
    "bumps",
    "schema",
    "xva",
    "performance",
    "risk",
    "attribution",
    "cashflow",
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
    # Risk
    "VarMethod",
    "VarConfig",
    "VarResult",
    "RiskFactorType",
    "RiskFactorShift",
    "MarketScenario",
    "MarketHistory",
    "calculate_var",
    "krd_dv01_ladder",
    "cs01_ladder",
    # Attribution
    "AttributionMethod",
    "AttributionMeta",
    "CarryDetail",
    "CorrelationsAttribution",
    "CrossFactorDetail",
    "RatesCurvesAttribution",
    "CreditCurvesAttribution",
    "CurveRestoreFlags",
    "FxAttribution",
    "InflationCurvesAttribution",
    "MarketSnapshot",
    "ModelParamsAttribution",
    "PnlAttribution",
    "PortfolioAttribution",
    "ScalarsAttribution",
    "ScalarsSnapshot",
    "TaylorAttributionConfig",
    "TaylorAttributionResult",
    "TaylorFactorResult",
    "VolatilitySnapshot",
    "VolAttribution",
    "attribute_pnl",
    "attribute_pnl_taylor",
    "attribute_portfolio_pnl",
    "attribute_pnl_from_json",
    "attribution_result_to_json",
    "compute_pnl",
    "compute_pnl_with_fx",
    "convert_currency",
    "default_waterfall_order",
    "reprice_instrument",
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
    "CDSTrancheBuildOverrides",
    "build_rate_instrument",
    "build_cds_instrument",
    "build_cds_tranche_instrument",
    # Bumps
    "BumpRequest",
    "bump_discount_curve",
    "bump_discount_curve_synthetic",
    "bump_hazard_spreads",
    "bump_hazard_shift",
    "bump_inflation_rates",
    # Schema
    "bond_schema",
    "valuation_result_schema",
    # XVA
    "FundingConfig",
    "XvaConfig",
    "CsaTerms",
    "NettingSet",
    "ExposureDiagnostics",
    "ExposureProfile",
    "StochasticExposureConfig",
    "StochasticExposureProfile",
    "XvaResult",
    "apply_netting",
    "apply_collateral",
    "compute_exposure_profile",
    "compute_cva",
    "compute_dva",
    "compute_fva",
    "compute_bilateral_xva",
    # Calibration
    "CalibrationDiagnostics",
    "CalibrationReport",
    "QuoteQuality",
    "RateBounds",
    "RateBoundsPolicy",
    "SolverKind",
    "ValidationConfig",
    "ValidationMode",
    "CALIBRATION_SCHEMA",
    "execute_calibration",
]
