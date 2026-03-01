"""Equity instrument wrappers."""

from __future__ import annotations
from .equity import Equity as Equity
from .equity_option import EquityOption as EquityOption
from .equity_index_future import (
    EquityIndexFuture as EquityIndexFuture,
    EquityIndexFutureBuilder as EquityIndexFutureBuilder,
    EquityFutureSpecs as EquityFutureSpecs,
    FuturePosition as FuturePosition,
)
from .vol_index_future import VolatilityIndexFuture as VolatilityIndexFuture
from .vol_index_option import VolatilityIndexOption as VolatilityIndexOption
from .cliquet_option import CliquetOption as CliquetOption
from .autocallable import Autocallable as Autocallable
from .variance_swap import (
    VarianceSwap as VarianceSwap,
    VarianceSwapBuilder as VarianceSwapBuilder,
    VarianceDirection as VarianceDirection,
    RealizedVarianceMethod as RealizedVarianceMethod,
)
from .trs import (
    TrsSide as TrsSide,
    TrsFinancingLegSpec as TrsFinancingLegSpec,
    TrsScheduleSpec as TrsScheduleSpec,
    EquityUnderlying as EquityUnderlying,
    IndexUnderlying as IndexUnderlying,
    EquityTotalReturnSwapBuilder as EquityTotalReturnSwapBuilder,
    EquityTotalReturnSwap as EquityTotalReturnSwap,
    FiIndexTotalReturnSwapBuilder as FiIndexTotalReturnSwapBuilder,
    FiIndexTotalReturnSwap as FiIndexTotalReturnSwap,
)
from .private_markets_fund import PrivateMarketsFund as PrivateMarketsFund
from .real_estate import RealEstateAsset as RealEstateAsset
from .levered_real_estate_equity import LeveredRealEstateEquity as LeveredRealEstateEquity
from .dcf import evaluate_dcf as evaluate_dcf

__all__ = [
    "Equity",
    "EquityOption",
    "EquityIndexFuture",
    "EquityIndexFutureBuilder",
    "EquityFutureSpecs",
    "FuturePosition",
    "VolatilityIndexFuture",
    "VolatilityIndexOption",
    "CliquetOption",
    "Autocallable",
    "VarianceSwap",
    "VarianceSwapBuilder",
    "VarianceDirection",
    "RealizedVarianceMethod",
    "TrsSide",
    "TrsFinancingLegSpec",
    "TrsScheduleSpec",
    "EquityUnderlying",
    "IndexUnderlying",
    "EquityTotalReturnSwapBuilder",
    "EquityTotalReturnSwap",
    "FiIndexTotalReturnSwapBuilder",
    "FiIndexTotalReturnSwap",
    "PrivateMarketsFund",
    "RealEstateAsset",
    "LeveredRealEstateEquity",
    "evaluate_dcf",
]
