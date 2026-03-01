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

from .generator import MonteCarloPathGenerator
from .params import ProcessParams
from .paths import CashflowType, PathPoint, SimulatedPath, PathDataset, PathDatasetIterator
from .result import MonteCarloResult
from .time_grid import TimeGrid
from .estimate import Estimate, ConvergenceDiagnostics
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
    "ConvergenceDiagnostics",
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
]
