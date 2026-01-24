"""Instrument wrappers for finstack-valuations (rates, FX, credit, equity)."""

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
from .deposit import Deposit as Deposit
from .basis_swap import BasisSwap as BasisSwap, BasisSwapLeg as BasisSwapLeg
from .fra import ForwardRateAgreement as ForwardRateAgreement
from .cap_floor import InterestRateOption as InterestRateOption
from .ir_future import InterestRateFuture as InterestRateFuture
from .irs import InterestRateSwap as InterestRateSwap
from .fx import FxSpot as FxSpot, FxOption as FxOption, FxSwap as FxSwap
from .fx_barrier_option import FxBarrierOption as FxBarrierOption
from .equity import Equity as Equity
from .equity_option import EquityOption as EquityOption
from .vol_index_future import VolatilityIndexFuture as VolatilityIndexFuture
from .vol_index_option import VolatilityIndexOption as VolatilityIndexOption
from .lookback_option import LookbackOption as LookbackOption, LookbackType as LookbackType
from .cliquet_option import CliquetOption as CliquetOption
from .cms_option import CmsOption as CmsOption
from .convertible import ConvertibleBond as ConvertibleBond
from .quanto_option import QuantoOption as QuantoOption
from .range_accrual import RangeAccrual as RangeAccrual
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
    "Deposit",
    "InterestRateSwap",
    "ForwardRateAgreement",
    "InterestRateOption",
    "InterestRateFuture",
    "BasisSwap",
    "BasisSwapLeg",
    # FX
    "FxSpot",
    "FxOption",
    "FxSwap",
    "FxBarrierOption",
    # Equity
    "Equity",
    "EquityOption",
    "VolatilityIndexFuture",
    "VolatilityIndexOption",
    "LookbackOption",
    "LookbackType",
    "CliquetOption",
    "ConvertibleBond",
    "QuantoOption",
    "RangeAccrual",
    "CommodityForward",
    "CommodityOption",
    "CommoditySwap",
    # Credit
    "CreditDefaultSwap",
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
]
