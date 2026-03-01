"""Instrument wrappers for finstack-valuations (rates, FX, credit, equity)."""

from __future__ import annotations

# Re-export category subpackages
from . import commodity as commodity
from . import credit_derivatives as credit_derivatives
from . import equity as equity
from . import exotics as exotics
from . import fixed_income as fixed_income
from . import fx as fx
from . import rates as rates

# Fixed Income
from .fixed_income.agency_mbs import (
    AgencyCmo as AgencyCmo,
    AgencyMbsPassthrough as AgencyMbsPassthrough,
    AgencyProgram as AgencyProgram,
    AgencyTba as AgencyTba,
    DollarRoll as DollarRoll,
    PoolType as PoolType,
    TbaTerm as TbaTerm,
)
from .fixed_income.bond import Bond as Bond, BondBuilder as BondBuilder
from .fixed_income.bond_future import (
    BondFuture as BondFuture,
    BondFutureBuilder as BondFutureBuilder,
    BondFutureSpecs as BondFutureSpecs,
)
from .fixed_income.convertible import ConvertibleBond as ConvertibleBond
from .fixed_income.inflation_linked_bond import (
    InflationLinkedBond as InflationLinkedBond,
    InflationLinkedBondBuilder as InflationLinkedBondBuilder,
)
from .fixed_income.revolving_credit import (
    RevolvingCredit as RevolvingCredit,
    EnhancedMonteCarloResult as EnhancedMonteCarloResult,
    PathResult as PathResult,
    ThreeFactorPathData as ThreeFactorPathData,
)
from .fixed_income.structured_credit import (
    DealType as DealType,
    StructuredCredit as StructuredCredit,
    StructuredCreditBuilder as StructuredCreditBuilder,
    TrancheSeniority as TrancheSeniority,
)
from .fixed_income.term_loan import (
    CashSweepEvent as CashSweepEvent,
    CommitmentFeeBase as CommitmentFeeBase,
    CommitmentStepDown as CommitmentStepDown,
    CouponType as CouponType,
    CovenantSpec as CovenantSpec,
    DdtlSpec as DdtlSpec,
    DrawEvent as DrawEvent,
    LoanCall as LoanCall,
    LoanCallSchedule as LoanCallSchedule,
    LoanCallType as LoanCallType,
    MarginStepUp as MarginStepUp,
    OidEirSpec as OidEirSpec,
    OidPolicy as OidPolicy,
    PikToggle as PikToggle,
    RateSpec as RateSpec,
    TermLoan as TermLoan,
    TermLoanAmortizationSpec as TermLoanAmortizationSpec,
    TermLoanBuilder as TermLoanBuilder,
)

# Rates
from .rates.deposit import Deposit as Deposit
from .rates.basis_swap import BasisSwap as BasisSwap, BasisSwapLeg as BasisSwapLeg
from .rates.fra import ForwardRateAgreement as ForwardRateAgreement
from .rates.cap_floor import InterestRateOption as InterestRateOption
from .rates.ir_future import InterestRateFuture as InterestRateFuture
from .rates.irs import InterestRateSwap as InterestRateSwap
from .rates.swaption import Swaption as Swaption
from .rates.inflation_swap import (
    InflationSwap as InflationSwap,
    InflationSwapBuilder as InflationSwapBuilder,
)
from .rates.inflation_cap_floor import (
    InflationCapFloor as InflationCapFloor,
    InflationCapFloorBuilder as InflationCapFloorBuilder,
)
from .rates.repo import (
    Repo as Repo,
    RepoBuilder as RepoBuilder,
    RepoCollateral as RepoCollateral,
)
from .rates.xccy_swap import (
    CrossCurrencySwap as CrossCurrencySwap,
    CrossCurrencySwapBuilder as CrossCurrencySwapBuilder,
)
from .rates.cms_option import CmsOption as CmsOption
from .rates.range_accrual import RangeAccrual as RangeAccrual

# Credit Derivatives
from .credit_derivatives.cds import (
    CreditDefaultSwap as CreditDefaultSwap,
    CDSPayReceive as CDSPayReceive,
    CDSConvention as CDSConvention,
)
from .credit_derivatives.cds_index import (
    CDSIndex as CDSIndex,
    CDSIndexBuilder as CDSIndexBuilder,
    CDSIndexConstituent as CDSIndexConstituent,
)
from .credit_derivatives.cds_option import CDSOption as CDSOption, CDSOptionBuilder as CDSOptionBuilder
from .credit_derivatives.cds_tranche import (
    CDSTranche as CDSTranche,
    CDSTrancheBuilder as CDSTrancheBuilder,
    TrancheSide as TrancheSide,
)

# FX
from .fx.fx import FxSpot as FxSpot, FxOption as FxOption, FxSwap as FxSwap
from .fx.fx_barrier_option import FxBarrierOption as FxBarrierOption
from .fx.fx_digital_option import FxDigitalOption as FxDigitalOption
from .fx.fx_forward import (
    FxForward as FxForward,
    FxForwardBuilder as FxForwardBuilder,
)
from .fx.fx_touch_option import FxTouchOption as FxTouchOption
from .fx.fx_variance_swap import (
    FxVarianceSwap as FxVarianceSwap,
    FxVarianceSwapBuilder as FxVarianceSwapBuilder,
    FxVarianceDirection as FxVarianceDirection,
    FxRealizedVarianceMethod as FxRealizedVarianceMethod,
)
from .fx.ndf import Ndf as Ndf, NdfBuilder as NdfBuilder
from .fx.quanto_option import QuantoOption as QuantoOption

# Equity
from .equity.equity import Equity as Equity
from .equity.equity_option import EquityOption as EquityOption
from .equity.equity_index_future import (
    EquityIndexFuture as EquityIndexFuture,
    EquityIndexFutureBuilder as EquityIndexFutureBuilder,
    EquityFutureSpecs as EquityFutureSpecs,
    FuturePosition as FuturePosition,
)
from .equity.vol_index_future import VolatilityIndexFuture as VolatilityIndexFuture
from .equity.vol_index_option import VolatilityIndexOption as VolatilityIndexOption
from .equity.cliquet_option import CliquetOption as CliquetOption
from .equity.autocallable import Autocallable as Autocallable
from .equity.variance_swap import (
    VarianceSwap as VarianceSwap,
    VarianceSwapBuilder as VarianceSwapBuilder,
    VarianceDirection as VarianceDirection,
    RealizedVarianceMethod as RealizedVarianceMethod,
)
from .equity.trs import (
    TrsSide as TrsSide,
    TrsFinancingLegSpec as TrsFinancingLegSpec,
    TrsScheduleSpec as TrsScheduleSpec,
    EquityUnderlying as EquityUnderlying,
    IndexUnderlying as IndexUnderlying,
    EquityTotalReturnSwapBuilder as EquityTotalReturnSwapBuilder,
    EquityTotalReturnSwap as EquityTotalReturnSwap,
)
from .fixed_income.fi_trs import (
    FiIndexTotalReturnSwapBuilder as FiIndexTotalReturnSwapBuilder,
    FiIndexTotalReturnSwap as FiIndexTotalReturnSwap,
)
from .equity.private_markets_fund import PrivateMarketsFund as PrivateMarketsFund
from .equity.real_estate import RealEstateAsset as RealEstateAsset
from .equity.levered_real_estate_equity import LeveredRealEstateEquity as LeveredRealEstateEquity
from .equity.dcf import evaluate_dcf as evaluate_dcf

# Commodity
from .commodity.commodity_asian_option import (
    CommodityAsianOption as CommodityAsianOption,
    CommodityAsianOptionBuilder as CommodityAsianOptionBuilder,
)
from .commodity.commodity_forward import CommodityForward as CommodityForward
from .commodity.commodity_option import CommodityOption as CommodityOption
from .commodity.commodity_swap import CommoditySwap as CommoditySwap

# Exotics
from .exotics.asian_option import AsianOption as AsianOption, AveragingMethod as AveragingMethod
from .exotics.barrier_option import BarrierOption as BarrierOption, BarrierType as BarrierType
from .exotics.basket import Basket as Basket
from .exotics.lookback_option import LookbackOption as LookbackOption, LookbackType as LookbackType

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
    "CDSConvention",
    "CDSIndex",
    "CDSIndexBuilder",
    "CDSIndexConstituent",
    "CDSOption",
    "CDSOptionBuilder",
    "CDSTranche",
    "CDSTrancheBuilder",
    "TrancheSide",
    "CmsOption",
    "DealType",
    "TrancheSeniority",
    "StructuredCreditBuilder",
    "StructuredCredit",
    # Other
    "BarrierOption",
    "BarrierType",
    "PrivateMarketsFund",
    # Term Loan
    "CashSweepEvent",
    "CommitmentFeeBase",
    "CommitmentStepDown",
    "CouponType",
    "CovenantSpec",
    "DdtlSpec",
    "DrawEvent",
    "LoanCall",
    "LoanCallSchedule",
    "LoanCallType",
    "MarginStepUp",
    "OidEirSpec",
    "OidPolicy",
    "PikToggle",
    "RateSpec",
    "TermLoan",
    "TermLoanAmortizationSpec",
    "TermLoanBuilder",
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
