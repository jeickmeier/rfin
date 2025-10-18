"""Instrument wrappers for finstack-valuations (rates, FX, credit, equity)."""

from .bond import Bond
from .deposit import Deposit
from .basis_swap import BasisSwap
from .fra import ForwardRateAgreement
from .cap_floor import InterestRateOption
from .ir_future import InterestRateFuture
from .irs import InterestRateSwap
from .fx import FxSpot, FxOption, FxSwap
from .equity import Equity
from .equity_option import EquityOption
from .convertible import ConvertibleBond
from .cds import CreditDefaultSwap
from .cds_index import CdsIndex
from .cds_option import CdsOption
from .cds_tranche import CdsTranche
from .repo import Repo
from .trs import EquityTotalReturnSwap, FiIndexTotalReturnSwap
from .variance_swap import VarianceSwap
from .inflation_linked_bond import InflationLinkedBond
from .inflation_swap import InflationSwap
from .basket import Basket
from .structured_credit import StructuredCredit
from .private_markets_fund import PrivateMarketsFund

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
    # Equity
    "Equity",
    "EquityOption",
    "ConvertibleBond",
    # Credit
    "CreditDefaultSwap",
    "CdsIndex",
    "CdsOption",
    "CdsTranche",
    "StructuredCredit",
    # Other
    "Repo",
    "EquityTotalReturnSwap",
    "FiIndexTotalReturnSwap",
    "VarianceSwap",
    "Basket",
    "PrivateMarketsFund",
]
