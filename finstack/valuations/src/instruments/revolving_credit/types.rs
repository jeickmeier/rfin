use crate::cashflow::builder::CashFlowSchedule;
use crate::cashflow::primitives::{CFKind, CashFlow, Notional};
use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::pricing::HasDiscountCurve;
use crate::instruments::common::traits::{Attributes, Instrument};
use crate::metrics::MetricId;
use crate::pricer::InstrumentType;
use crate::results::ValuationResult;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, DayCountCtx, Frequency, StubKind};
use crate::cashflow::builder::schedule_utils::{build_dates_with_eom};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;
use std::fmt;

/// Interest rate specification for the revolving credit facility.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InterestRateSpec {
    /// Fixed interest rate (per annum).
    Fixed { rate: f64 },
    /// Floating rate referencing a forward curve plus spread.
    Floating {
        fwd_id: CurveId,
        spread_bp: f64,
        reset_lag_days: i32,
        #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
        reset_frequency: Option<Frequency>,
        #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
        reset_calendar_id: Option<String>,
        #[cfg_attr(feature = "serde", serde(default))]
        reset_convention: ResetConvention,
    },
}
/// Floating reset convention for index fixing timing.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ResetConvention {
    /// Fix in advance at (adjusted) period start.
    InAdvance,
    /// Fix in arrears over [start, end].
    InArrears,
    /// Fix with lag relative to payment date (negative lag means before payment).
    LagDays,
}

impl Default for ResetConvention {
    fn default() -> Self { Self::InAdvance }
}


/// Fee specification for an RCF (commitment/utilization/upfront).
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RcfFeeSpec {
    /// Commitment fee (annualised, in basis points) applied to undrawn amount.
    pub commitment_fee_bp: f64,
    /// Utilization fee (annualised, in basis points) applied to drawn amount.
    pub utilization_fee_bp: f64,
    /// Optional upfront fee charged at facility inception.
    pub upfront_fee: Option<Money>,
}

/// Transaction type for explicit drawdowns/repayments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TransactionType {
    Drawdown,
    Repayment,
}

/// Explicit drawdown/repayment transaction on the facility.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RcfTransaction {
    pub date: Date,
    pub amount: Money,
    pub transaction_type: TransactionType,
}

/// Revolving Credit Facility instrument.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RevolvingCreditFacility {
    pub id: InstrumentId,
    pub credit_limit: Money,
    pub initial_drawn: Money,
    pub start_date: Date,
    pub maturity_date: Date,
    pub interest: InterestRateSpec,
    pub fees: RcfFeeSpec,
    #[builder(default = Vec::new())]
    pub transactions: Vec<RcfTransaction>,
    pub disc_id: CurveId,
    pub attributes: Attributes,
    #[builder(default = DayCount::Act360)]
    pub day_count: DayCount,
    #[builder(default = Frequency::Months(3))]
    pub payment_frequency: Frequency,
    #[builder(default = StubKind::ShortFront)]
    pub payment_stub: StubKind,
    #[builder(default = BusinessDayConvention::ModifiedFollowing)]
    pub payment_bdc: BusinessDayConvention,
    #[builder(default, setter(strip_option))]
    pub payment_calendar_id: Option<String>,
    #[builder(default)]
    pub align_end_of_month: bool,
}

impl RevolvingCreditFacility {
    fn accrue_period(
        &self,
        curves: &MarketContext,
        balance: Money,
        year_fraction: f64,
        pay_date: Date,
        out: &mut Vec<(Date, Money, CFKind)>,
    ) -> Result<()> {
        // Interest
        match &self.interest {
            InterestRateSpec::Fixed { rate } => {
                let interest = balance * (*rate * year_fraction);
                out.push((pay_date, interest, CFKind::Fixed));
            }
            InterestRateSpec::Floating {
                fwd_id,
                spread_bp,
                reset_convention,
                reset_lag_days,
                reset_calendar_id,
                ..
            } => {
                if let Ok(fwd_curve) = curves.get_forward_ref(fwd_id) {
                    let fwd_dc = fwd_curve.day_count();
                    let base = fwd_curve.base_date();
                    let ctx = DayCountCtx::default();
                    // Determine fixing period bounds according to convention
                    let (t_fix, t_pay) = {
                        match reset_convention {
                            ResetConvention::InAdvance => {
                                // Fix at (possibly lagged) start; use pay date for end time mapping
                                // Note: for simplicity, map to curve base using pay_date and pay_date
                                let t_pay = fwd_dc.year_fraction(base, pay_date, ctx).unwrap_or(0.0);
                                // Start time proxied by t_pay - tenor if positive
                                let t_fix = (t_pay - fwd_curve.tenor()).max(0.0);
                                (t_fix, t_pay)
                            }
                            ResetConvention::InArrears => {
                                let t_end = fwd_dc.year_fraction(base, pay_date, ctx).unwrap_or(0.0);
                                let t_start = (t_end - fwd_curve.tenor()).max(0.0);
                                (t_start, t_end)
                            }
                            ResetConvention::LagDays => {
                                // Apply day lag using calendar when available; fall back to calendar days
                                let fix_date = if let Some(cal_id) = reset_calendar_id.as_ref() {
                                    if let Some(cal) = finstack_core::dates::calendar::calendar_by_id(cal_id) {
                                        // Negative lag means before payment
                                        let offset = *reset_lag_days as i64;
                                        let mut d = pay_date;
                                        if offset != 0 {
                                            // Business-day aware shift
                                            let mut remaining = offset.abs();
                                            while remaining > 0 {
                                                d += time::Duration::days(if offset > 0 { 1 } else { -1 });
                                                if cal.is_business_day(d) { remaining -= 1; }
                                            }
                                        }
                                        d
                                    } else {
                                        pay_date - time::Duration::days((*reset_lag_days).max(0) as i64)
                                    }
                                } else {
                                    pay_date - time::Duration::days((*reset_lag_days).max(0) as i64)
                                };
                                let t_end = fwd_dc.year_fraction(base, pay_date, ctx).unwrap_or(0.0);
                                let t_fix = fwd_dc.year_fraction(base, fix_date, ctx).unwrap_or(0.0);
                                (t_fix.min(t_end), t_end)
                            }
                        }
                    };
                    let fwd_rate = if t_pay > t_fix { fwd_curve.rate_period(t_fix, t_pay) } else { 0.0 };
                    let total_rate = fwd_rate + (*spread_bp * 1e-4);
                    let interest = balance * (total_rate * year_fraction);
                    out.push((pay_date, interest, CFKind::FloatReset));
                }
            }
        }

        // Commitment fee on undrawn (clamped at zero)
        if self.fees.commitment_fee_bp != 0.0 {
            let mut undrawn = (self.credit_limit - balance).unwrap_or(self.credit_limit);
            if undrawn.amount() < 0.0 {
                undrawn = Money::new(0.0, undrawn.currency());
            }
            let fee_rate = self.fees.commitment_fee_bp * 1e-4;
            let fee = undrawn * (fee_rate * year_fraction);
            out.push((pay_date, fee, CFKind::Fee));
        }

        // Utilization fee on drawn
        if self.fees.utilization_fee_bp != 0.0 {
            let fee_rate = self.fees.utilization_fee_bp * 1e-4;
            let fee = balance * (fee_rate * year_fraction);
            out.push((pay_date, fee, CFKind::Fee));
        }

        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: InstrumentId,
        credit_limit: Money,
        initial_drawn: Money,
        start_date: Date,
        maturity_date: Date,
        interest: InterestRateSpec,
        fees: RcfFeeSpec,
        disc_id: CurveId,
    ) -> Self {
        Self::builder()
            .id(id)
            .credit_limit(credit_limit)
            .initial_drawn(initial_drawn)
            .start_date(start_date)
            .maturity_date(maturity_date)
            .interest(interest)
            .fees(fees)
            .disc_id(disc_id)
            .attributes(Attributes::new())
            .transactions(Vec::new())
            .day_count(finstack_core::dates::DayCount::Act360)
            .align_end_of_month(true)
            .build()
            .expect("RevolvingCreditFacility::new should not fail")
    }

    pub fn current_drawn_amount(&self, as_of: Date) -> Money {
        let mut drawn = self.initial_drawn;
        for tx in &self.transactions {
            if tx.date <= as_of {
                match tx.transaction_type {
                    TransactionType::Drawdown => drawn = (drawn + tx.amount).unwrap_or(drawn),
                    TransactionType::Repayment => drawn = (drawn - tx.amount).unwrap_or(drawn),
                }
            }
        }
        drawn
    }

    pub(crate) fn build_outstanding_schedule(&self) -> Vec<(Date, Money)> {
        let mut events = self.transactions.clone();
        events.sort_by(|a, b| a.date.cmp(&b.date));
        let mut schedule = Vec::with_capacity(events.len() + 2);
        let mut balance = self.initial_drawn;
        schedule.push((self.start_date, balance));
        for tx in events {
            balance = match tx.transaction_type {
                TransactionType::Drawdown => (balance + tx.amount).unwrap_or(balance),
                TransactionType::Repayment => (balance - tx.amount).unwrap_or(balance),
            };
            schedule.push((tx.date, balance));
        }
        schedule.push((self.maturity_date, balance));
        schedule
    }
}

impl Instrument for RevolvingCreditFacility {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> InstrumentType {
        InstrumentType::RevolvingCredit
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn Instrument> {
        Box::new(self.clone())
    }

    fn value(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        super::pricer::npv(self, curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> Result<ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            self, curves, as_of, base_value, metrics,
        )
    }
}

impl HasDiscountCurve for RevolvingCreditFacility {
    fn discount_curve_id(&self) -> &CurveId {
        &self.disc_id
    }
}

impl CashflowProvider for RevolvingCreditFacility {
    fn build_schedule(&self, curves: &MarketContext, as_of: Date) -> Result<DatedFlows> {
        let events = self.generate_cashflow_events(curves, as_of)?;
        Ok(events
            .into_iter()
            .map(|(date, amount, _)| (date, amount))
            .collect())
    }

    fn build_full_schedule(&self, curves: &MarketContext, as_of: Date) -> Result<CashFlowSchedule> {
        let events = self.generate_cashflow_events(curves, as_of)?;
        let mut flows = Vec::with_capacity(events.len());
        for (date, amount, kind) in events {
            flows.push(CashFlow {
                date,
                reset_date: None,
                amount,
                kind,
                accrual_factor: 0.0,
            });
        }
        let notional = Notional::par(self.initial_drawn.amount(), self.initial_drawn.currency());
        Ok(CashFlowSchedule {
            flows,
            notional,
            day_count: self.day_count,
            meta: Default::default(),
        })
    }
}

impl RevolvingCreditFacility {
    fn generate_cashflow_events(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<Vec<(Date, Money, CFKind)>> {
        // Build canonical payment schedule using shared builder utils
        let sched = build_dates_with_eom(
            self.start_date,
            self.maturity_date,
            self.payment_frequency,
            self.payment_stub,
            self.payment_bdc,
            self.payment_calendar_id.as_deref(),
            self.align_end_of_month,
        );
        let payment_dates = sched.dates;
        let mut flows: Vec<(Date, Money, CFKind)> = Vec::new();
        let mut balance_events = self.build_outstanding_schedule();
        balance_events.sort_by_key(|(d, _)| *d);

        let mut balance_idx = 0usize;
        let mut current_balance = self.initial_drawn;

        for window in payment_dates.windows(2) {
            let start = window[0];
            let end = window[1];
            if end <= as_of {
                continue;
            }

            // Update current balance up to start
            while balance_idx + 1 < balance_events.len()
                && balance_events[balance_idx + 1].0 <= start
            {
                balance_idx += 1;
                current_balance = balance_events[balance_idx].1;
            }

            // Build segmentation points within (start, end)
            let mut seg_points: Vec<(Date, Money)> = Vec::new();
            seg_points.push((start, current_balance));
            let mut scan_idx = balance_idx + 1;
            while scan_idx < balance_events.len() {
                let (event_date, new_balance) = balance_events[scan_idx];
                if event_date >= end {
                    break;
                }
                seg_points.push((event_date, new_balance));
                scan_idx += 1;
            }
            seg_points.push((end, seg_points.last().map(|(_, b)| *b).unwrap_or(current_balance)));

            // Accrue per segment using effective window vs as_of
            for w in seg_points.windows(2) {
                let seg_start = w[0].0.max(as_of);
                let seg_end = w[1].0.max(as_of);
                if seg_end <= seg_start {
                    continue;
                }
                let seg_balance = w[0].1;
                let accrual = self
                    .day_count
                    .year_fraction(seg_start, seg_end, DayCountCtx::default())?;
                // Accrue into the period end date to avoid mid-period payments
                self.accrue_period(curves, seg_balance, accrual, end, &mut flows)?;
            }
        }

        if let Some(upfront) = &self.fees.upfront_fee {
            if self.start_date > as_of {
                flows.push((self.start_date, *upfront, CFKind::Fee));
            }
        }

        // Initial draw as principal outflow at start (if future)
        if self.start_date > as_of && self.initial_drawn.amount() != 0.0 {
            flows.push((
                self.start_date,
                Money::new(-self.initial_drawn.amount(), self.initial_drawn.currency()),
                CFKind::Notional,
            ));
        }

        // Principal transactions as cashflows (lender perspective)
        for tx in &self.transactions {
            if tx.date > as_of {
                match tx.transaction_type {
                    TransactionType::Drawdown => flows.push((tx.date, Money::new(-tx.amount.amount(), tx.amount.currency()), CFKind::Notional)),
                    TransactionType::Repayment => flows.push((tx.date, tx.amount, CFKind::Notional)),
                }
            }
        }

        Ok(flows)
    }
}

impl fmt::Display for RevolvingCreditFacility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RCF:{}", self.id)
    }
}

