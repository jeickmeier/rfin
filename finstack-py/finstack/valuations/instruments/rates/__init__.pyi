"""Interest rate instrument wrappers."""

from __future__ import annotations
from .deposit import Deposit as Deposit
from .basis_swap import BasisSwap as BasisSwap, BasisSwapLeg as BasisSwapLeg
from .fra import ForwardRateAgreement as ForwardRateAgreement
from .cap_floor import InterestRateOption as InterestRateOption
from .ir_future import InterestRateFuture as InterestRateFuture
from .irs import InterestRateSwap as InterestRateSwap
from .swaption import Swaption as Swaption
from .inflation_swap import (
    InflationSwap as InflationSwap,
    InflationSwapBuilder as InflationSwapBuilder,
)
from .inflation_cap_floor import (
    InflationCapFloor as InflationCapFloor,
    InflationCapFloorBuilder as InflationCapFloorBuilder,
)
from .repo import (
    Repo as Repo,
    RepoBuilder as RepoBuilder,
    RepoCollateral as RepoCollateral,
)
from .xccy_swap import (
    CrossCurrencySwap as CrossCurrencySwap,
    CrossCurrencySwapBuilder as CrossCurrencySwapBuilder,
)
from .cms_option import CmsOption as CmsOption
from .range_accrual import RangeAccrual as RangeAccrual

__all__ = [
    "Deposit",
    "BasisSwap",
    "BasisSwapLeg",
    "ForwardRateAgreement",
    "InterestRateOption",
    "InterestRateFuture",
    "InterestRateSwap",
    "Swaption",
    "InflationSwap",
    "InflationSwapBuilder",
    "InflationCapFloor",
    "InflationCapFloorBuilder",
    "Repo",
    "RepoBuilder",
    "RepoCollateral",
    "CrossCurrencySwap",
    "CrossCurrencySwapBuilder",
    "CmsOption",
    "RangeAccrual",
]
