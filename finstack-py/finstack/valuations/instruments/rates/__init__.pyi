"""Interest rate instrument wrappers."""

from __future__ import annotations
from .deposit import Deposit as Deposit, DepositBuilder as DepositBuilder
from .basis_swap import BasisSwap as BasisSwap, BasisSwapLeg as BasisSwapLeg
from .fra import (
    ForwardRateAgreement as ForwardRateAgreement,
    ForwardRateAgreementBuilder as ForwardRateAgreementBuilder,
)
from .cap_floor import InterestRateOption as InterestRateOption
from .ir_future import (
    InterestRateFuture as InterestRateFuture,
    InterestRateFutureBuilder as InterestRateFutureBuilder,
)
from .irs import (
    InterestRateSwap as InterestRateSwap,
    FloatingLegCompounding as FloatingLegCompounding,
    ParRateMethod as ParRateMethod,
)
from .swaption import (
    Swaption as Swaption,
    BermudanSwaption as BermudanSwaption,
    BermudanSchedule as BermudanSchedule,
    BermudanType as BermudanType,
    SABRParameters as SABRParameters,
)
from .inflation_swap import (
    InflationSwap as InflationSwap,
    InflationSwapBuilder as InflationSwapBuilder,
    YoYInflationSwap as YoYInflationSwap,
    YoYInflationSwapBuilder as YoYInflationSwapBuilder,
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
from .range_accrual import (
    RangeAccrual as RangeAccrual,
    RangeAccrualBuilder as RangeAccrualBuilder,
    BoundsType as BoundsType,
)

__all__ = [
    "Deposit",
    "DepositBuilder",
    "BasisSwap",
    "BasisSwapLeg",
    "ForwardRateAgreement",
    "ForwardRateAgreementBuilder",
    "InterestRateOption",
    "InterestRateFuture",
    "InterestRateFutureBuilder",
    "InterestRateSwap",
    "FloatingLegCompounding",
    "ParRateMethod",
    "Swaption",
    "BermudanSwaption",
    "BermudanSchedule",
    "BermudanType",
    "SABRParameters",
    "InflationSwap",
    "InflationSwapBuilder",
    "YoYInflationSwap",
    "YoYInflationSwapBuilder",
    "InflationCapFloor",
    "InflationCapFloorBuilder",
    "Repo",
    "RepoBuilder",
    "RepoCollateral",
    "CrossCurrencySwap",
    "CrossCurrencySwapBuilder",
    "CmsOption",
    "RangeAccrual",
    "RangeAccrualBuilder",
    "BoundsType",
]
