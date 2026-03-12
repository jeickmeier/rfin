"""Equity instrument wrappers."""

from __future__ import annotations
from .equity import Equity as Equity, EquityBuilder as EquityBuilder
from .equity_option import EquityOption as EquityOption, EquityOptionGreeks as EquityOptionGreeks
from .equity_index_future import (
    EquityIndexFuture as EquityIndexFuture,
    EquityIndexFutureBuilder as EquityIndexFutureBuilder,
    EquityFutureSpecs as EquityFutureSpecs,
    FuturePosition as FuturePosition,
)
from .vol_index_future import (
    VolatilityIndexFuture as VolatilityIndexFuture,
    VolatilityIndexFutureBuilder as VolatilityIndexFutureBuilder,
    VolIndexContractSpecs as VolIndexContractSpecs,
)
from .vol_index_option import (
    VolatilityIndexOption as VolatilityIndexOption,
    VolatilityIndexOptionBuilder as VolatilityIndexOptionBuilder,
    VolIndexOptionSpecs as VolIndexOptionSpecs,
)
from .cliquet_option import CliquetOption as CliquetOption
from .autocallable import Autocallable as Autocallable, FinalPayoffType as FinalPayoffType
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
)
from .private_markets_fund import PrivateMarketsFund as PrivateMarketsFund
from .real_estate import RealEstateAsset as RealEstateAsset, RealEstateValuationMethod as RealEstateValuationMethod
from .levered_real_estate_equity import LeveredRealEstateEquity as LeveredRealEstateEquity
from .dcf import (
    DilutionSecurity as DilutionSecurity,
    DiscountedCashFlow as DiscountedCashFlow,
    DiscountedCashFlowBuilder as DiscountedCashFlowBuilder,
    EquityBridge as EquityBridge,
    TerminalValueSpec as TerminalValueSpec,
    ValuationDiscounts as ValuationDiscounts,
)

__all__ = [
    "Equity",
    "EquityBuilder",
    "EquityOption",
    "EquityOptionGreeks",
    "EquityIndexFuture",
    "EquityIndexFutureBuilder",
    "EquityFutureSpecs",
    "FuturePosition",
    "VolatilityIndexFuture",
    "VolatilityIndexFutureBuilder",
    "VolatilityIndexOption",
    "VolatilityIndexOptionBuilder",
    "CliquetOption",
    "Autocallable",
    "VarianceSwap",
    "VarianceSwapBuilder",
    "VolIndexContractSpecs",
    "VolIndexOptionSpecs",
    "VarianceDirection",
    "RealizedVarianceMethod",
    "DilutionSecurity",
    "DiscountedCashFlow",
    "DiscountedCashFlowBuilder",
    "EquityBridge",
    "TerminalValueSpec",
    "ValuationDiscounts",
    "TrsSide",
    "TrsFinancingLegSpec",
    "TrsScheduleSpec",
    "EquityUnderlying",
    "IndexUnderlying",
    "EquityTotalReturnSwapBuilder",
    "EquityTotalReturnSwap",
    "FinalPayoffType",
    "PrivateMarketsFund",
    "RealEstateAsset",
    "RealEstateValuationMethod",
    "LeveredRealEstateEquity",
]
