"""Instrument wrappers for finstack-valuations (rates, FX, credit, equity)."""

from .bond import Bond
from .deposit import Deposit
from .basis_swap import BasisSwap
from .fra import ForwardRateAgreement
from .cap_floor import InterestRateOption
from .ir_future import InterestRateFuture
from .irs import InterestRateSwap
from .fx import FxSpot, FxOption, FxSwap
from .fx_barrier_option import FxBarrierOption
from .equity import Equity
from .equity_option import EquityOption
from .lookback_option import LookbackOption, LookbackType
from .cliquet_option import CliquetOption
from .cms_option import CmsOption
from .convertible import ConvertibleBond
from .quanto_option import QuantoOption
from .range_accrual import RangeAccrual
from .cds import CreditDefaultSwap, CDSPayReceive
from .cds_index import CdsIndex
from .cds_option import CdsOption
from .cds_tranche import CdsTranche
from .barrier_option import BarrierOption, BarrierType
from .structured_credit import StructuredCredit
from .private_markets_fund import PrivateMarketsFund
from .term_loan import TermLoan
from .revolving_credit import (
    RevolvingCredit,
    EnhancedMonteCarloResult,
    PathResult,
    ThreeFactorPathData,
)

__all__ = [
    # Fixed Income
    "Bond",
    "Deposit",
    "InterestRateSwap",
    "ForwardRateAgreement",
    "InterestRateOption",
    "InterestRateFuture",
    "BasisSwap",
    "InflationLinkedBond",
    "InflationSwap",
    # FX
    "FxSpot",
    "FxOption",
    "FxSwap",
    "FxBarrierOption",
    # Equity
    "Equity",
    "EquityOption",
    "LookbackOption",
    "LookbackType",
    "CliquetOption",
    "ConvertibleBond",
    "QuantoOption",
    "RangeAccrual",
    # Credit
    "CreditDefaultSwap",
    "CdsIndex",
    "CdsOption",
    "CdsTranche",
    "CmsOption",
    "StructuredCredit",
    "evaluate_dcf",
    # Other
    "Repo",
    "RevolvingCredit",
    "TermLoan",
    "EquityTotalReturnSwap",
    "FiIndexTotalReturnSwap",
    "VarianceSwap",
    "AsianOption",
    "AveragingMethod",
    "Autocallable",
    "Basket",
    "BarrierOption",
    "BarrierType",
    "PrivateMarketsFund",
    "TermLoan",
    "RevolvingCredit",
    "EnhancedMonteCarloResult",
    "PathResult",
    "ThreeFactorPathData",
]
