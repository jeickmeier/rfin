"""Monte Carlo simulation infrastructure for path generation and pricing."""

from __future__ import annotations
from . import generator
from . import params
from . import paths
from . import result
from . import time_grid
from . import estimate
from . import processes
from . import discretization
from . import payoffs
from . import rng
from . import engine
from . import variance_reduction

from .generator import MonteCarloPathGenerator
from .params import ProcessParams
from .paths import CashflowType, PathPoint, SimulatedPath, PathDataset, PathDatasetIterator
from .result import MonteCarloResult
from .time_grid import TimeGrid
from .estimate import Estimate
from .processes import (
    GbmParams,
    HestonParams,
    CirParams,
    HullWhite1FParams,
    MertonJumpParams,
    SchwartzSmithParams,
    BrownianParams,
    MultiOuParams,
)
from .discretization import (
    ExactGbmScheme,
    EulerMaruyamaScheme,
    LogEulerScheme,
    MilsteinScheme,
    LogMilsteinScheme,
    QeHestonScheme,
    QeCirScheme,
    ExactHullWhite1FScheme,
    JumpEulerScheme,
    ExactSchwartzSmithScheme,
)
from .payoffs import EuropeanCall, EuropeanPut, Digital, Forward
from .rng import PhiloxRng
from .engine import EuropeanPricerConfig, EuropeanMcPricer, price_european
from .variance_reduction import AntitheticConfig, black_scholes_call, black_scholes_put

__all__ = [
    # Submodules
    "generator",
    "params",
    "paths",
    "result",
    "time_grid",
    "estimate",
    "processes",
    "discretization",
    "payoffs",
    "rng",
    "engine",
    "variance_reduction",
    # Existing classes
    "MonteCarloPathGenerator",
    "ProcessParams",
    "CashflowType",
    "PathPoint",
    "SimulatedPath",
    "PathDataset",
    "PathDatasetIterator",
    "MonteCarloResult",
    # New building blocks
    "TimeGrid",
    "Estimate",
    # Process parameters
    "GbmParams",
    "HestonParams",
    "CirParams",
    "HullWhite1FParams",
    "MertonJumpParams",
    "SchwartzSmithParams",
    "BrownianParams",
    "MultiOuParams",
    # Discretization schemes
    "ExactGbmScheme",
    "EulerMaruyamaScheme",
    "LogEulerScheme",
    "MilsteinScheme",
    "LogMilsteinScheme",
    "QeHestonScheme",
    "QeCirScheme",
    "ExactHullWhite1FScheme",
    "JumpEulerScheme",
    "ExactSchwartzSmithScheme",
    # Payoff types
    "EuropeanCall",
    "EuropeanPut",
    "Digital",
    "Forward",
    # RNG
    "PhiloxRng",
    # Engine / pricer
    "EuropeanPricerConfig",
    "EuropeanMcPricer",
    # Variance reduction
    "AntitheticConfig",
    # Free functions
    "price_european",
    "black_scholes_call",
    "black_scholes_put",
]
