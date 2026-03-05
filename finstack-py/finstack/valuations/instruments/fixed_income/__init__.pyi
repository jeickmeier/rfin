"""Fixed income instrument wrappers."""

from __future__ import annotations
from .mbs_passthrough import (
    AgencyProgram as AgencyProgram,
    PoolType as PoolType,
    AgencyMbsPassthrough as AgencyMbsPassthrough,
    AgencyMbsPassthroughBuilder as AgencyMbsPassthroughBuilder,
)
from .tba import (
    TbaTerm as TbaTerm,
    TbaSettlement as TbaSettlement,
    AgencyTba as AgencyTba,
    AgencyTbaBuilder as AgencyTbaBuilder,
)
from .dollar_roll import (
    DollarRoll as DollarRoll,
    DollarRollBuilder as DollarRollBuilder,
)
from .cmo import (
    CmoTrancheType as CmoTrancheType,
    PacCollar as PacCollar,
    CmoTranche as CmoTranche,
    CmoWaterfall as CmoWaterfall,
    AgencyCmo as AgencyCmo,
    AgencyCmoBuilder as AgencyCmoBuilder,
)
from .bond import (
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
from .bond_future import (
    BondFuture as BondFuture,
    BondFutureBuilder as BondFutureBuilder,
    BondFutureSpecs as BondFutureSpecs,
    DeliverableBond as DeliverableBond,
)
from .convertible import (
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
from .inflation_linked_bond import (
    InflationLinkedBond as InflationLinkedBond,
    InflationLinkedBondBuilder as InflationLinkedBondBuilder,
)
from .revolving_credit import (
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
from .fi_trs import (
    FiIndexTotalReturnSwap as FiIndexTotalReturnSwap,
    FiIndexTotalReturnSwapBuilder as FiIndexTotalReturnSwapBuilder,
)
from .structured_credit import (
    DealType as DealType,
    StructuredCredit as StructuredCredit,
    StructuredCreditBuilder as StructuredCreditBuilder,
    TrancheSeniority as TrancheSeniority,
)
from .term_loan import (
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

__all__ = [
    # Agency MBS Passthrough
    "AgencyProgram",
    "PoolType",
    "AgencyMbsPassthrough",
    "AgencyMbsPassthroughBuilder",
    # TBA
    "TbaTerm",
    "TbaSettlement",
    "AgencyTba",
    "AgencyTbaBuilder",
    # Dollar Roll
    "DollarRoll",
    "DollarRollBuilder",
    # CMO
    "CmoTrancheType",
    "PacCollar",
    "CmoTranche",
    "CmoWaterfall",
    "AgencyCmo",
    "AgencyCmoBuilder",
    # Bond
    "AccrualMethod",
    "Bond",
    "BondBuilder",
    "BondSettlementConvention",
    "CallPut",
    "CallPutSchedule",
    "CashflowSpec",
    "MakeWholeSpec",
    # Bond credit pricing models
    "MertonMcConfig",
    "MertonMcResult",
    "MertonModel",
    "MertonAssetDynamics",
    "MertonBarrierType",
    "EndogenousHazardSpec",
    "DynamicRecoverySpec",
    "ToggleExerciseModel",
    # Bond Future
    "BondFuture",
    "BondFutureBuilder",
    "BondFutureSpecs",
    "DeliverableBond",
    # Convertible
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
    # Inflation-Linked
    "InflationLinkedBond",
    "InflationLinkedBondBuilder",
    # Revolving Credit
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
    # FI Index TRS
    "FiIndexTotalReturnSwap",
    "FiIndexTotalReturnSwapBuilder",
    # Structured Credit
    "DealType",
    "TrancheSeniority",
    "StructuredCreditBuilder",
    "StructuredCredit",
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
]
