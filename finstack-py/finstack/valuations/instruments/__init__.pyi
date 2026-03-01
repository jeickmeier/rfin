"""Instrument wrappers for finstack-valuations (rates, FX, credit, equity)."""

from __future__ import annotations
from .agency_mbs import (
    AgencyCmo as AgencyCmo,
    AgencyMbsPassthrough as AgencyMbsPassthrough,
    AgencyProgram as AgencyProgram,
    AgencyTba as AgencyTba,
    DollarRoll as DollarRoll,
    PoolType as PoolType,
    TbaTerm as TbaTerm,
)
from .bond import Bond as Bond, BondBuilder as BondBuilder
from .bond_future import (
    BondFuture as BondFuture,
    BondFutureBuilder as BondFutureBuilder,
    BondFutureSpecs as BondFutureSpecs,
)
from .deposit import Deposit as Deposit
from .basis_swap import BasisSwap as BasisSwap, BasisSwapLeg as BasisSwapLeg
from .fra import ForwardRateAgreement as ForwardRateAgreement
from .cap_floor import InterestRateOption as InterestRateOption
from .ir_future import InterestRateFuture as InterestRateFuture
from .irs import InterestRateSwap as InterestRateSwap
from .swaption import Swaption as Swaption
from .inflation_linked_bond import (
    InflationLinkedBond as InflationLinkedBond,
    InflationLinkedBondBuilder as InflationLinkedBondBuilder,
)
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
from .fx import FxSpot as FxSpot, FxOption as FxOption, FxSwap as FxSwap
from .fx_barrier_option import FxBarrierOption as FxBarrierOption
from .fx_digital_option import FxDigitalOption as FxDigitalOption
from .fx_forward import (
    FxForward as FxForward,
    FxForwardBuilder as FxForwardBuilder,
)
from .fx_touch_option import FxTouchOption as FxTouchOption
from .fx_variance_swap import (
    FxVarianceSwap as FxVarianceSwap,
    FxVarianceSwapBuilder as FxVarianceSwapBuilder,
    FxVarianceDirection as FxVarianceDirection,
    FxRealizedVarianceMethod as FxRealizedVarianceMethod,
)
from .ndf import Ndf as Ndf, NdfBuilder as NdfBuilder
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
from .lookback_option import LookbackOption as LookbackOption, LookbackType as LookbackType
from .cliquet_option import CliquetOption as CliquetOption
from .cms_option import CmsOption as CmsOption
from .convertible import ConvertibleBond as ConvertibleBond
from .quanto_option import QuantoOption as QuantoOption
from .range_accrual import RangeAccrual as RangeAccrual
from .asian_option import AsianOption as AsianOption, AveragingMethod as AveragingMethod
from .autocallable import Autocallable as Autocallable
from .basket import Basket as Basket
from .variance_swap import (
    VarianceSwap as VarianceSwap,
    VarianceSwapBuilder as VarianceSwapBuilder,
    VarianceDirection as VarianceDirection,
    RealizedVarianceMethod as RealizedVarianceMethod,
)
from .commodity_asian_option import (
    CommodityAsianOption as CommodityAsianOption,
    CommodityAsianOptionBuilder as CommodityAsianOptionBuilder,
)
from .commodity_forward import CommodityForward as CommodityForward
from .commodity_option import CommodityOption as CommodityOption
from .commodity_swap import CommoditySwap as CommoditySwap
from .cds import CreditDefaultSwap as CreditDefaultSwap, CDSPayReceive as CDSPayReceive
from .cds_index import CDSIndex as CdsIndex
from .cds_option import CdsOption as CdsOption
from .cds_tranche import CdsTranche as CdsTranche
from .barrier_option import BarrierOption as BarrierOption, BarrierType as BarrierType
from .structured_credit import StructuredCredit as StructuredCredit
from .private_markets_fund import PrivateMarketsFund as PrivateMarketsFund
from .term_loan import TermLoan as TermLoan
from .revolving_credit import (
    RevolvingCredit as RevolvingCredit,
    EnhancedMonteCarloResult as EnhancedMonteCarloResult,
    PathResult as PathResult,
    ThreeFactorPathData as ThreeFactorPathData,
)
from .real_estate import RealEstateAsset as RealEstateAsset
from .levered_real_estate_equity import LeveredRealEstateEquity as LeveredRealEstateEquity
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
from .dcf import evaluate_dcf as evaluate_dcf

__all__ = [
    # Agency MBS
    "AgencyProgram",
    "PoolType",
    "TbaTerm",
    "AgencyMbsPassthrough",
    "AgencyTba",
    "DollarRoll",
    "AgencyCmo",
    # Fixed Income
    "Bond",
    "BondBuilder",
    "BondFuture",
    "BondFutureBuilder",
    "BondFutureSpecs",
    "Deposit",
    "InterestRateSwap",
    "ForwardRateAgreement",
    "InterestRateOption",
    "InterestRateFuture",
    "BasisSwap",
    "BasisSwapLeg",
    "Swaption",
    "InflationLinkedBond",
    "InflationLinkedBondBuilder",
    "InflationSwap",
    "InflationSwapBuilder",
    "InflationCapFloor",
    "InflationCapFloorBuilder",
    "Repo",
    "RepoBuilder",
    "RepoCollateral",
    "CrossCurrencySwap",
    "CrossCurrencySwapBuilder",
    # FX
    "FxSpot",
    "FxOption",
    "FxSwap",
    "FxForward",
    "FxForwardBuilder",
    "FxBarrierOption",
    "FxDigitalOption",
    "FxTouchOption",
    "FxVarianceSwap",
    "FxVarianceSwapBuilder",
    "FxVarianceDirection",
    "FxRealizedVarianceMethod",
    "Ndf",
    "NdfBuilder",
    # Equity
    "Equity",
    "EquityOption",
    "EquityIndexFuture",
    "EquityIndexFutureBuilder",
    "EquityFutureSpecs",
    "FuturePosition",
    "VolatilityIndexFuture",
    "VolatilityIndexOption",
    "LookbackOption",
    "LookbackType",
    "CliquetOption",
    "ConvertibleBond",
    "QuantoOption",
    "RangeAccrual",
    "AsianOption",
    "AveragingMethod",
    "Autocallable",
    "Basket",
    "VarianceSwap",
    "VarianceSwapBuilder",
    "VarianceDirection",
    "RealizedVarianceMethod",
    "CommodityForward",
    "CommodityOption",
    "CommoditySwap",
    "CommodityAsianOption",
    "CommodityAsianOptionBuilder",
    # Credit
    "CreditDefaultSwap",
    "CDSPayReceive",
    "CdsIndex",
    "CdsOption",
    "CdsTranche",
    "CmsOption",
    "StructuredCredit",
    # Other
    "BarrierOption",
    "BarrierType",
    "PrivateMarketsFund",
    "TermLoan",
    "RevolvingCredit",
    "EnhancedMonteCarloResult",
    "PathResult",
    "ThreeFactorPathData",
    "RealEstateAsset",
    "LeveredRealEstateEquity",
    # Total Return Swaps
    "TrsSide",
    "TrsFinancingLegSpec",
    "TrsScheduleSpec",
    "EquityUnderlying",
    "IndexUnderlying",
    "EquityTotalReturnSwapBuilder",
    "EquityTotalReturnSwap",
    "FiIndexTotalReturnSwapBuilder",
    "FiIndexTotalReturnSwap",
    # DCF Valuation
    "evaluate_dcf",
]
