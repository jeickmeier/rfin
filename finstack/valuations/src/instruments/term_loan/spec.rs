//! Serde-stable specification types for Term Loan and DDTL features.

use crate::instruments::pricing_overrides::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use super::types::RateSpec;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum OidPolicy {
    WithheldPct(i32),
    WithheldAmount(Money),
    SeparatePct(i32),
    SeparateAmount(Money),
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DrawEvent {
    pub date: Date,
    pub amount: Money,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CommitmentStepDown {
    pub date: Date,
    pub new_limit: Money,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum CommitmentFeeBase {
    Undrawn,
    CommitmentMinusOutstanding,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DdtlSpec {
    pub commitment_limit: Money,
    pub availability_start: Date,
    pub availability_end: Date,
    pub draws: Vec<DrawEvent>,
    pub commitment_step_downs: Vec<CommitmentStepDown>,
    pub usage_fee_bp: i32,
    pub commitment_fee_bp: i32,
    pub fee_base: CommitmentFeeBase,
    pub oid_policy: Option<OidPolicy>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct MarginStepUp {
    pub date: Date,
    pub delta_bp: i32,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct PikToggle {
    pub date: Date,
    pub enable_pik: bool,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CashSweepEvent {
    pub date: Date,
    pub amount: Money,
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CovenantSpec {
    pub margin_stepups: Vec<MarginStepUp>,
    pub pik_toggles: Vec<PikToggle>,
    pub cash_sweeps: Vec<CashSweepEvent>,
    pub draw_stop_dates: Vec<Date>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum AmortizationSpec {
    None,
    Linear { start: Date, end: Date },
    PercentPerPeriod { bp: i32 },
    Custom(Vec<(Date, Money)>),
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct PikSpec {
    pub fraction_of_interest: rust_decimal::Decimal,
    pub toggle_schedule: Vec<(Date, bool)>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum OidEirMethod {
    SolveEIR,
    ExplicitRate(rust_decimal::Decimal),
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct OidEirSpec {
    pub amount: Money,
    pub accrual_frequency: Frequency,
    pub eir_method: OidEirMethod,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct TermLoanSpec {
    pub id: InstrumentId,
    pub discount_curve_id: CurveId,
    pub currency: Currency,
    pub issue: Date,
    pub maturity: Date,
    pub rate: RateSpec,
    pub pay_freq: Frequency,
    pub day_count: DayCount,
    pub bdc: BusinessDayConvention,
    pub calendar_id: Option<String>,
    pub stub: StubKind,
    pub amortization: AmortizationSpec,
    pub coupon_type: crate::cashflow::builder::types::CouponType,
    pub upfront_fee: Option<Money>,
    pub ddtl: Option<DdtlSpec>,
    pub covenants: Option<CovenantSpec>,
    pub oid_eir: Option<OidEirSpec>,
    pub pricing_overrides: PricingOverrides,
    /// Optional call schedule (borrower callability)
    pub call_schedule: Option<LoanCallSchedule>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct LoanCall {
    pub date: Date,
    /// Redemption price as % of par (outstanding principal) at call date
    pub price_pct_of_par: f64,
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct LoanCallSchedule {
    pub calls: Vec<LoanCall>,
}


