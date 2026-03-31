"""Instrument wrappers for finstack-valuations (rates, FX, credit, equity)."""

from __future__ import annotations
from typing import Any

# Re-export category subpackages
from . import commodity as commodity
from . import credit_derivatives as credit_derivatives
from . import equity as equity
from . import exotics as exotics
from . import fixed_income as fixed_income
from . import fx as fx
from . import rates as rates

def instrument_from_json(data: str) -> Any:
    """Construct any supported instrument from a JSON string."""
    ...

def instrument_from_dict(data: dict[str, Any]) -> Any:
    """Construct any supported instrument from a Python dictionary."""
    ...

def instrument_to_dict(instrument: Any) -> dict[str, Any]:
    """Serialize an instrument to a versioned Python dictionary."""
    ...

def instrument_to_json(instrument: Any) -> str:
    """Serialize an instrument to a versioned JSON string."""
    ...

# Fixed Income
from .fixed_income.mbs_passthrough import (
    AgencyProgram as AgencyProgram,
    PoolType as PoolType,
    AgencyMbsPassthrough as AgencyMbsPassthrough,
    AgencyMbsPassthroughBuilder as AgencyMbsPassthroughBuilder,
)
from .fixed_income.tba import (
    TbaTerm as TbaTerm,
    TbaSettlement as TbaSettlement,
    AgencyTba as AgencyTba,
    AgencyTbaBuilder as AgencyTbaBuilder,
)
from .fixed_income.dollar_roll import (
    DollarRoll as DollarRoll,
    DollarRollBuilder as DollarRollBuilder,
)
from .fixed_income.cmo import (
    CmoTranche as CmoTranche,
    CmoTrancheType as CmoTrancheType,
    CmoWaterfall as CmoWaterfall,
    PacCollar as PacCollar,
    AgencyCmo as AgencyCmo,
    AgencyCmoBuilder as AgencyCmoBuilder,
)
from .fixed_income.bond import (
    AccrualMethod as AccrualMethod,
    Bond as Bond,
    BondBuilder as BondBuilder,
    BondSettlementConvention as BondSettlementConvention,
    CallPut as CallPut,
    CallPutSchedule as CallPutSchedule,
    CashflowSpec as CashflowSpec,
    MakeWholeSpec as MakeWholeSpec,
    MertonMcConfig as MertonMcConfig,
    MertonMcResult as MertonMcResult,
    MertonModel as MertonModel,
    MertonAssetDynamics as MertonAssetDynamics,
    MertonBarrierType as MertonBarrierType,
    EndogenousHazardSpec as EndogenousHazardSpec,
    DynamicRecoverySpec as DynamicRecoverySpec,
    ToggleExerciseModel as ToggleExerciseModel,
)
from .fixed_income.bond_future import (
    BondFuture as BondFuture,
    BondFutureBuilder as BondFutureBuilder,
    BondFutureSpecs as BondFutureSpecs,
    DeliverableBond as DeliverableBond,
)
from .fixed_income.convertible import (
    AntiDilutionPolicy as AntiDilutionPolicy,
    ConversionEvent as ConversionEvent,
    ConversionPolicy as ConversionPolicy,
    ConversionSpec as ConversionSpec,
    ConvertibleBond as ConvertibleBond,
    ConvertibleBondBuilder as ConvertibleBondBuilder,
    ConvertibleGreeks as ConvertibleGreeks,
    ConvertibleTreeType as ConvertibleTreeType,
    DilutionEvent as DilutionEvent,
    DividendAdjustment as DividendAdjustment,
    SoftCallTrigger as SoftCallTrigger,
)
from .fixed_income.inflation_linked_bond import (
    InflationLinkedBond as InflationLinkedBond,
    InflationLinkedBondBuilder as InflationLinkedBondBuilder,
)
from .fixed_income.revolving_credit import (
    BaseRateSpec as BaseRateSpec,
    DrawRepayEvent as DrawRepayEvent,
    DrawRepaySpec as DrawRepaySpec,
    EnhancedMonteCarloResult as EnhancedMonteCarloResult,
    FeeTier as FeeTier,
    PathResult as PathResult,
    RevolvingCredit as RevolvingCredit,
    RevolvingCreditBuilder as RevolvingCreditBuilder,
    RevolvingCreditFees as RevolvingCreditFees,
    StochasticUtilizationSpec as StochasticUtilizationSpec,
    ThreeFactorPathData as ThreeFactorPathData,
    UtilizationProcess as UtilizationProcess,
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
from .rates.deposit import Deposit as Deposit, DepositBuilder as DepositBuilder
from .rates.basis_swap import (
    BasisSwap as BasisSwap,
    BasisSwapBuilder as BasisSwapBuilder,
    BasisSwapLeg as BasisSwapLeg,
)
from .rates.fra import (
    ForwardRateAgreement as ForwardRateAgreement,
    ForwardRateAgreementBuilder as ForwardRateAgreementBuilder,
)
from .rates.cap_floor import (
    InterestRateOption as InterestRateOption,
    InterestRateOptionBuilder as InterestRateOptionBuilder,
    RateOptionType as RateOptionType,
)
from .rates.ir_future import (
    InterestRateFuture as InterestRateFuture,
    InterestRateFutureBuilder as InterestRateFutureBuilder,
)
from .rates.ir_future_option import (
    IrFutureOption as IrFutureOption,
    IrFutureOptionBuilder as IrFutureOptionBuilder,
)
from .rates.irs import (
    FloatingLegCompounding as FloatingLegCompounding,
    InterestRateSwap as InterestRateSwap,
    InterestRateSwapBuilder as InterestRateSwapBuilder,
    ParRateMethod as ParRateMethod,
    PayReceive as PayReceive,
)
from .rates.swaption import (
    BermudanSchedule as BermudanSchedule,
    BermudanSwaption as BermudanSwaption,
    BermudanType as BermudanType,
    GreekInputs as GreekInputs,
    SABRParameters as SABRParameters,
    Swaption as Swaption,
    SwaptionExercise as SwaptionExercise,
    SwaptionSettlement as SwaptionSettlement,
)
from .rates.inflation_swap import (
    InflationSwap as InflationSwap,
    InflationSwapBuilder as InflationSwapBuilder,
    YoYInflationSwap as YoYInflationSwap,
    YoYInflationSwapBuilder as YoYInflationSwapBuilder,
)
from .rates.inflation_cap_floor import (
    InflationCapFloor as InflationCapFloor,
    InflationCapFloorBuilder as InflationCapFloorBuilder,
    InflationCapFloorType as InflationCapFloorType,
)
from .rates.repo import (
    CollateralType as CollateralType,
    Repo as Repo,
    RepoBuilder as RepoBuilder,
    RepoCollateral as RepoCollateral,
    RepoType as RepoType,
)
from .rates.xccy_swap import (
    CrossCurrencySwap as CrossCurrencySwap,
    CrossCurrencySwapBuilder as CrossCurrencySwapBuilder,
    LegSide as LegSide,
    NotionalExchange as NotionalExchange,
)
from .rates.cms_option import CmsOption as CmsOption
from .rates.cms_swap import CmsSwap as CmsSwap
from .rates.range_accrual import (
    BoundsType as BoundsType,
    RangeAccrual as RangeAccrual,
    RangeAccrualBuilder as RangeAccrualBuilder,
)

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
from .fx.fx import (
    FxOption as FxOption,
    FxOptionBuilder as FxOptionBuilder,
    FxSpot as FxSpot,
    FxSpotBuilder as FxSpotBuilder,
    FxSwap as FxSwap,
    FxSwapBuilder as FxSwapBuilder,
)
from .fx.fx_barrier_option import (
    FxBarrierOption as FxBarrierOption,
    FxBarrierOptionBuilder as FxBarrierOptionBuilder,
)
from .fx.fx_digital_option import (
    DigitalPayoutType as DigitalPayoutType,
    FxDigitalOption as FxDigitalOption,
    FxDigitalOptionBuilder as FxDigitalOptionBuilder,
)
from .fx.fx_forward import (
    FxForward as FxForward,
    FxForwardBuilder as FxForwardBuilder,
)
from .fx.fx_touch_option import (
    BarrierDirection as BarrierDirection,
    FxTouchOption as FxTouchOption,
    FxTouchOptionBuilder as FxTouchOptionBuilder,
    PayoutTiming as PayoutTiming,
    TouchType as TouchType,
)
from .fx.fx_variance_swap import (
    FxVarianceSwap as FxVarianceSwap,
    FxVarianceSwapBuilder as FxVarianceSwapBuilder,
    FxVarianceDirection as FxVarianceDirection,
    FxRealizedVarianceMethod as FxRealizedVarianceMethod,
)
from .fx.ndf import Ndf as Ndf, NdfBuilder as NdfBuilder
from .fx.quanto_option import (
    QuantoOption as QuantoOption,
    QuantoOptionBuilder as QuantoOptionBuilder,
)

# Equity
from .equity.equity import Equity as Equity, EquityBuilder as EquityBuilder
from .equity.equity_option import (
    EquityOption as EquityOption,
    EquityOptionGreeks as EquityOptionGreeks,
)
from .equity.equity_index_future import (
    EquityIndexFuture as EquityIndexFuture,
    EquityIndexFutureBuilder as EquityIndexFutureBuilder,
    EquityFutureSpecs as EquityFutureSpecs,
    FuturePosition as FuturePosition,
)
from .equity.vol_index_future import (
    VolatilityIndexFuture as VolatilityIndexFuture,
    VolatilityIndexFutureBuilder as VolatilityIndexFutureBuilder,
    VolIndexContractSpecs as VolIndexContractSpecs,
)
from .equity.vol_index_option import (
    VolatilityIndexOption as VolatilityIndexOption,
    VolatilityIndexOptionBuilder as VolatilityIndexOptionBuilder,
    VolIndexOptionSpecs as VolIndexOptionSpecs,
)
from .equity.cliquet_option import CliquetOption as CliquetOption
from .equity.autocallable import Autocallable as Autocallable, FinalPayoffType as FinalPayoffType
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
from .equity.real_estate import (
    RealEstateAsset as RealEstateAsset,
    RealEstateValuationMethod as RealEstateValuationMethod,
)
from .equity.levered_real_estate_equity import LeveredRealEstateEquity as LeveredRealEstateEquity
from .equity.dcf import (
    DilutionSecurity as DilutionSecurity,
    DiscountedCashFlow as DiscountedCashFlow,
    DiscountedCashFlowBuilder as DiscountedCashFlowBuilder,
    EquityBridge as EquityBridge,
    TerminalValueSpec as TerminalValueSpec,
    ValuationDiscounts as ValuationDiscounts,
)

# Commodity
from .commodity.commodity_asian_option import (
    CommodityAsianOption as CommodityAsianOption,
    CommodityAsianOptionBuilder as CommodityAsianOptionBuilder,
)
from .commodity.commodity_forward import (
    CommodityForward as CommodityForward,
    CommodityForwardBuilder as CommodityForwardBuilder,
)
from .commodity.commodity_option import (
    CommodityOption as CommodityOption,
    CommodityOptionBuilder as CommodityOptionBuilder,
)
from .commodity.commodity_spread_option import (
    CommoditySpreadOption as CommoditySpreadOption,
    CommoditySpreadOptionBuilder as CommoditySpreadOptionBuilder,
)
from .commodity.commodity_swap import (
    CommoditySwap as CommoditySwap,
    CommoditySwapBuilder as CommoditySwapBuilder,
)
from .commodity.commodity_swaption import (
    CommoditySwaption as CommoditySwaption,
    CommoditySwaptionBuilder as CommoditySwaptionBuilder,
)

# Exotics
from .exotics.asian_option import AsianOption as AsianOption, AveragingMethod as AveragingMethod
from .exotics.barrier_option import BarrierOption as BarrierOption, BarrierType as BarrierType
from .exotics.basket import (
    BasketAssetType as BasketAssetType,
    Basket as Basket,
    BasketCalculator as BasketCalculator,
    BasketConstituent as BasketConstituent,
    BasketPricingConfig as BasketPricingConfig,
)
from .exotics.lookback_option import LookbackOption as LookbackOption, LookbackType as LookbackType

__all__ = [
    "instrument_from_json",
    "instrument_from_dict",
    "instrument_to_dict",
    "instrument_to_json",
    # Agency MBS
    "AgencyCmo",
    "AgencyCmoBuilder",
    "AgencyMbsPassthrough",
    "AgencyMbsPassthroughBuilder",
    "AgencyProgram",
    "AgencyTba",
    "AgencyTbaBuilder",
    "CmoTranche",
    "CmoTrancheType",
    "CmoWaterfall",
    "DollarRoll",
    "DollarRollBuilder",
    "PacCollar",
    "PoolType",
    "TbaSettlement",
    "TbaTerm",
    # Fixed Income - Bond
    "AccrualMethod",
    "Bond",
    "BondBuilder",
    "BondFuture",
    "BondFutureBuilder",
    "BondFutureSpecs",
    "BondSettlementConvention",
    "CallPut",
    "CallPutSchedule",
    "CashflowSpec",
    "DeliverableBond",
    "MakeWholeSpec",
    # Bond structural credit pricing models
    "MertonMcConfig",
    "MertonMcResult",
    "MertonModel",
    "MertonAssetDynamics",
    "MertonBarrierType",
    "EndogenousHazardSpec",
    "DynamicRecoverySpec",
    "ToggleExerciseModel",
    # Fixed Income - Convertible
    "AntiDilutionPolicy",
    "ConversionEvent",
    "ConversionPolicy",
    "ConversionSpec",
    "ConvertibleBond",
    "ConvertibleBondBuilder",
    "ConvertibleGreeks",
    "ConvertibleTreeType",
    "DilutionEvent",
    "DividendAdjustment",
    "SoftCallTrigger",
    # Fixed Income - Inflation Linked
    "InflationLinkedBond",
    "InflationLinkedBondBuilder",
    # Fixed Income - Revolving Credit
    "BaseRateSpec",
    "DrawRepayEvent",
    "DrawRepaySpec",
    "EnhancedMonteCarloResult",
    "FeeTier",
    "PathResult",
    "RevolvingCredit",
    "RevolvingCreditBuilder",
    "RevolvingCreditFees",
    "StochasticUtilizationSpec",
    "ThreeFactorPathData",
    "UtilizationProcess",
    # Fixed Income - Structured Credit
    "DealType",
    "StructuredCredit",
    "StructuredCreditBuilder",
    "TrancheSeniority",
    # Rates
    "BasisSwap",
    "BasisSwapBuilder",
    "BasisSwapLeg",
    "BermudanSchedule",
    "BermudanSwaption",
    "BermudanType",
    "BoundsType",
    "CmsOption",
    "CmsSwap",
    "CrossCurrencySwap",
    "CrossCurrencySwapBuilder",
    "Deposit",
    "DepositBuilder",
    "FloatingLegCompounding",
    "ForwardRateAgreement",
    "ForwardRateAgreementBuilder",
    "GreekInputs",
    "InflationCapFloor",
    "InflationCapFloorBuilder",
    "InflationCapFloorType",
    "InflationSwap",
    "InflationSwapBuilder",
    "InterestRateFuture",
    "InterestRateFutureBuilder",
    "IrFutureOption",
    "IrFutureOptionBuilder",
    "InterestRateOption",
    "InterestRateOptionBuilder",
    "InterestRateSwap",
    "InterestRateSwapBuilder",
    "LegSide",
    "NotionalExchange",
    "ParRateMethod",
    "PayReceive",
    "RangeAccrual",
    "RangeAccrualBuilder",
    "RateOptionType",
    "CollateralType",
    "Repo",
    "RepoBuilder",
    "RepoCollateral",
    "RepoType",
    "SABRParameters",
    "Swaption",
    "SwaptionExercise",
    "SwaptionSettlement",
    "YoYInflationSwap",
    "YoYInflationSwapBuilder",
    # FX
    "FxBarrierOption",
    "FxBarrierOptionBuilder",
    "BarrierDirection",
    "DigitalPayoutType",
    "FxDigitalOption",
    "FxDigitalOptionBuilder",
    "FxForward",
    "FxForwardBuilder",
    "FxOption",
    "FxOptionBuilder",
    "FxRealizedVarianceMethod",
    "FxSpot",
    "FxSpotBuilder",
    "FxSwap",
    "FxSwapBuilder",
    "FxTouchOption",
    "FxTouchOptionBuilder",
    "PayoutTiming",
    "TouchType",
    "FxVarianceDirection",
    "FxVarianceSwap",
    "FxVarianceSwapBuilder",
    "Ndf",
    "NdfBuilder",
    "QuantoOption",
    "QuantoOptionBuilder",
    # Equity
    "Autocallable",
    "FinalPayoffType",
    "CliquetOption",
    "Equity",
    "EquityBuilder",
    "EquityFutureSpecs",
    "EquityIndexFuture",
    "EquityIndexFutureBuilder",
    "EquityOption",
    "EquityOptionGreeks",
    "FuturePosition",
    "DilutionSecurity",
    "DiscountedCashFlow",
    "DiscountedCashFlowBuilder",
    "EquityBridge",
    "LeveredRealEstateEquity",
    "PrivateMarketsFund",
    "RealEstateAsset",
    "RealEstateValuationMethod",
    "RealizedVarianceMethod",
    "VarianceDirection",
    "VarianceSwap",
    "VarianceSwapBuilder",
    "VolatilityIndexFuture",
    "VolatilityIndexFutureBuilder",
    "VolIndexContractSpecs",
    "VolatilityIndexOption",
    "VolatilityIndexOptionBuilder",
    "VolIndexOptionSpecs",
    # Commodity
    "CommodityAsianOption",
    "CommodityAsianOptionBuilder",
    "CommodityForward",
    "CommodityForwardBuilder",
    "CommodityOption",
    "CommodityOptionBuilder",
    "CommoditySpreadOption",
    "CommoditySpreadOptionBuilder",
    "CommoditySwap",
    "CommoditySwapBuilder",
    "CommoditySwaption",
    "CommoditySwaptionBuilder",
    # Credit Derivatives
    "CDSConvention",
    "CDSIndex",
    "CDSIndexBuilder",
    "CDSIndexConstituent",
    "CDSOption",
    "CDSOptionBuilder",
    "CDSPayReceive",
    "CDSTranche",
    "CDSTrancheBuilder",
    "CreditDefaultSwap",
    "TrancheSide",
    # Exotics
    "AsianOption",
    "AveragingMethod",
    "BarrierOption",
    "BarrierType",
    "Basket",
    "BasketAssetType",
    "BasketCalculator",
    "BasketConstituent",
    "BasketPricingConfig",
    "LookbackOption",
    "LookbackType",
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
    # Total Return Swaps
    "EquityTotalReturnSwap",
    "EquityTotalReturnSwapBuilder",
    "EquityUnderlying",
    "FiIndexTotalReturnSwap",
    "FiIndexTotalReturnSwapBuilder",
    "IndexUnderlying",
    "TrsFinancingLegSpec",
    "TrsScheduleSpec",
    "TrsSide",
    # DCF Valuation
    "TerminalValueSpec",
    "ValuationDiscounts",
]
