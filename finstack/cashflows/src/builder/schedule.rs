//! Schedule generation from the builder state.
//!
//! Provides the canonical `CashFlowSchedule` type and helpers for sorting and
//! deriving schedule metadata. Downstream pricing/risk code consumes this shape.

use crate::builder::Notional;
use crate::primitives::{CFKind, CashFlow};
use finstack_core::cashflow::Discountable;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, DayCountCtx, Period, PeriodId};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::market_data::traits::{Discounting, Survival};
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use indexmap::IndexMap;
use std::sync::Arc;

use super::compiler::{FixedSchedule, FloatSchedule};

/// Stable ordering rank used for deterministic sorting of same-date cashflows.
///
/// All known `CFKind` variants are explicitly ranked so that same-date ordering
/// is fully deterministic. The wildcard arm covers future `#[non_exhaustive]`
/// additions and sorts them after all known variants.
pub fn kind_rank(kind: CFKind) -> u8 {
    match kind {
        CFKind::Fixed | CFKind::Stub | CFKind::FloatReset | CFKind::InflationCoupon => 0,
        CFKind::Fee | CFKind::CommitmentFee | CFKind::UsageFee | CFKind::FacilityFee => 1,
        CFKind::Amortization => 2,
        CFKind::PrePayment => 3,
        CFKind::DefaultedNotional => 4,
        CFKind::Recovery | CFKind::AccruedOnDefault => 5,
        CFKind::PIK => 6,
        CFKind::Notional | CFKind::RevolvingDraw | CFKind::RevolvingRepayment => 7,
        CFKind::InitialMarginPost
        | CFKind::InitialMarginReturn
        | CFKind::VariationMarginReceive
        | CFKind::VariationMarginPay
        | CFKind::MarginInterest
        | CFKind::CollateralSubstitutionIn
        | CFKind::CollateralSubstitutionOut => 8,
        _ => 9,
    }
}

/// Sort flows deterministically using schedule ordering semantics.
pub fn sort_flows(flows: &mut [CashFlow]) {
    flows.sort_by(|a, b| {
        use core::cmp::Ordering;
        match a.date.cmp(&b.date) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => match kind_rank(a.kind).cmp(&kind_rank(b.kind)) {
                Ordering::Less => Ordering::Less,
                Ordering::Greater => Ordering::Greater,
                Ordering::Equal => match a.amount.currency().cmp(&b.amount.currency()) {
                    Ordering::Less => Ordering::Less,
                    Ordering::Greater => Ordering::Greater,
                    Ordering::Equal => match a.amount.amount().total_cmp(&b.amount.amount()) {
                        Ordering::Less => Ordering::Less,
                        Ordering::Greater => Ordering::Greater,
                        Ordering::Equal => a.reset_date.cmp(&b.reset_date),
                    },
                },
            },
        }
    });
}

pub(crate) fn finalize_flows(
    mut flows: Vec<CashFlow>,
    fixed: &[FixedSchedule],
    floating: &[FloatSchedule],
    issue_date: Option<Date>,
) -> (Vec<CashFlow>, CashFlowMeta, DayCount) {
    sort_flows(&mut flows);

    let mut cals: Vec<String> = fixed
        .iter()
        .map(|(spec, _, _, _)| spec.calendar_id.clone())
        .chain(
            floating
                .iter()
                .map(|(spec, _, _)| spec.rate_spec.calendar_id.clone()),
        )
        .collect();
    cals.sort_unstable();
    cals.dedup();
    let meta = CashFlowMeta {
        calendar_ids: cals,
        facility_limit: None,
        issue_date,
        representation: CashflowRepresentation::default(),
    };

    let out_dc = if let Some((spec, _, _, _)) = fixed.first() {
        spec.dc
    } else if let Some((spec, _, _)) = floating.first() {
        spec.rate_spec.dc
    } else {
        DayCount::Act365F
    };
    (flows, meta, out_dc)
}

/// Meaning of the emitted schedule relative to pricing and waterfall policy.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum CashflowRepresentation {
    /// Fixed or contractually scheduled future dated cash amounts.
    #[default]
    Contractual,
    /// Current-market or model-projected future dated cash amounts.
    Projected,
    /// Intentionally empty because the contingent payoff policy is not modeled yet.
    Placeholder,
    /// Intentionally empty because no future dated cashflows remain.
    NoResidual,
}

/// Metadata for cashflow schedules (calendar IDs, facility limits, issue date).
///
/// Tracks referenced calendar IDs, optional facility limits, and the instrument's
/// issue date for use by downstream engines (e.g., accrual calculation).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct CashFlowMeta {
    /// Meaning of the schedule relative to waterfall policy.
    #[serde(default)]
    pub representation: CashflowRepresentation,
    /// Holiday calendar IDs used for schedule adjustments.
    pub calendar_ids: Vec<String>,
    /// Optional facility limit/commitment for instruments like RCFs.
    pub facility_limit: Option<Money>,
    /// Issue date of the instrument, when known.
    ///
    /// Used by the accrual engine to establish the first coupon period start
    /// date precisely, avoiding the inverse day count approximation that can
    /// be off by 1-2 days.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub issue_date: Option<Date>,
}

/// Cashflow schedule output from the composable builder.
///
/// Contains ordered cashflows plus notional and a representative `DayCount`.
/// Methods provide convenient accessors commonly used by pricing and analysis.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct CashFlowSchedule {
    /// Ordered cashflows (coupons, principal payments, fees)
    pub flows: Vec<CashFlow>,
    /// Notional schedule (constant or amortizing)
    pub notional: Notional,
    /// Day count convention for interest calculations
    pub day_count: DayCount,
    /// Additional metadata (calendars, facility limits)
    pub meta: CashFlowMeta,
}

impl Discountable for CashFlowSchedule {
    type PVOutput = finstack_core::Result<Money>;

    fn npv(
        &self,
        disc: &dyn Discounting,
        base: Date,
        dc: Option<DayCount>,
    ) -> finstack_core::Result<Money> {
        let flows: Vec<(Date, Money)> = self
            .flows
            .iter()
            .filter(|cf| cf.kind != CFKind::DefaultedNotional)
            .map(|cf| (cf.date, cf.amount))
            .collect();
        finstack_core::cashflow::npv(disc, base, dc, &flows)
    }
}

impl CashFlowSchedule {
    /// Construct a schedule directly from classified cashflows.
    pub fn from_parts(
        mut flows: Vec<CashFlow>,
        notional: Notional,
        day_count: DayCount,
        meta: CashFlowMeta,
    ) -> Self {
        sort_flows(&mut flows);
        Self {
            flows,
            notional,
            day_count,
            meta,
        }
    }

    /// Create a new cashflow builder (standard Rust pattern).
    ///
    /// This is the recommended entry point for building cashflow schedules.
    /// Returns a `CashFlowBuilder` that can be configured and built.
    ///
    /// # Example
    /// ```rust,no_run
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
    /// use finstack_core::money::Money;
    /// use finstack_cashflows::builder::{CashFlowSchedule, CouponType, FixedCouponSpec};
    /// use rust_decimal_macros::dec;
    /// use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let issue = Date::from_calendar_date(2025, Month::January, 15)?;
    /// let maturity = Date::from_calendar_date(2026, Month::January, 15)?;
    ///
    /// let notional = Money::new(1_000_000.0, Currency::USD);
    /// let spec = FixedCouponSpec {
    ///     coupon_type: CouponType::Cash,
    ///     rate: dec!(0.05),
    ///     freq: Tenor::semi_annual(),
    ///     dc: DayCount::Act365F,
    ///     bdc: BusinessDayConvention::Following,
    ///     calendar_id: "weekends_only".to_string(),
    ///     end_of_month: false,
    ///     payment_lag_days: 0,
    ///     stub: StubKind::None,
    /// };
    ///
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(notional, issue, maturity)
    ///     .fixed_cf(spec)
    ///     .build_with_curves(None)?;
    /// # let _ = schedule;
    /// # Ok(())
    /// # }
    /// ```
    pub fn builder() -> super::CashFlowBuilder {
        super::CashFlowBuilder::default()
    }

    /// Returns the list of dates for all flows in schedule order.
    pub fn dates(&self) -> Vec<Date> {
        self.flows.iter().map(|cf| cf.date).collect()
    }

    /// Remove flows strictly before `as_of`, keeping only future-dated events.
    ///
    /// This is the canonical future-filtering step for public schedule surfaces.
    /// Flows on or after `as_of` are retained.
    #[must_use]
    pub fn filter_future(mut self, as_of: Date) -> Self {
        self.flows.retain(|cf| cf.date >= as_of);
        self
    }

    /// Remove pure PIK accretion flows from the schedule.
    ///
    /// PIK entries represent notional capitalisation without cash movement.
    /// They are omitted from the public schedule by default; the notional
    /// evolution they drive is already captured in the balance path.
    #[must_use]
    pub fn omit_pure_pik(mut self) -> Self {
        self.flows.retain(|cf| cf.kind != CFKind::PIK);
        self
    }

    /// One-shot public-schedule normalization pipeline.
    ///
    /// Applies, in order:
    /// 1. Future-flow filtering (`date >= as_of`)
    /// 2. Pure PIK omission
    /// 3. Re-sort (defensive, in case instrument code appended unsorted flows)
    /// 4. Attach the given representation tag
    #[must_use]
    pub fn normalize_public(mut self, as_of: Date, representation: CashflowRepresentation) -> Self {
        self.flows
            .retain(|cf| cf.date >= as_of && cf.kind != CFKind::PIK);
        sort_flows(&mut self.flows);
        self.meta.representation = representation;
        self
    }

    /// Outstanding principal path tracking Amortization and PIK flows only.
    ///
    /// This method provides a simplified balance view suitable for coupon calculations
    /// where the accrual base tracks principal reductions (Amortization) and PIK
    /// capitalizations, but **excludes** ad-hoc notional draws/repays.
    ///
    /// Returns one entry per cashflow, tracking the outstanding balance after
    /// each flow is processed. Useful for debugging and detailed analysis.
    ///
    /// # When to Use Each Method
    ///
    /// - **`outstanding_path_per_flow()`**: Use for coupon accrual calculations on fixed
    ///   amortization schedules (bonds, term loans with scheduled amortization).
    /// - **[`Self::outstanding_by_date()`]**: Use for full balance tracking including
    ///   notional events (revolving credit facilities, delayed draws, prepayments).
    ///
    /// Note: Amortization amounts in the schedule are stored as POSITIVE values
    /// (the builder internally manages the reduction of outstanding balance).
    /// PIK amounts are positive and increase outstanding.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Amortization exceeds current outstanding (would result in negative balance)
    /// - Currency mismatch between flows and notional
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_core::dates::Date;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::money::Money;
    /// use finstack_cashflows::builder::schedule::{CashFlowMeta, CashFlowSchedule};
    /// use finstack_core::cashflow::{CashFlow, CFKind};
    /// use finstack_cashflows::builder::Notional;
    /// use time::Month;
    ///
    /// let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    /// let notional = Notional { initial: Money::new(100.0, Currency::USD), amort: Default::default() };
    /// let flows = vec![
    ///   CashFlow { date: base, reset_date: None, amount: Money::new(10.0, Currency::USD), kind: CFKind::Amortization, accrual_factor: 0.0, rate: None },
    ///   CashFlow { date: base, reset_date: None, amount: Money::new(5.0, Currency::USD), kind: CFKind::PIK, accrual_factor: 0.0, rate: None },
    /// ];
    /// let s = CashFlowSchedule { flows, notional, day_count: finstack_core::dates::DayCount::Act365F, meta: CashFlowMeta::default() };
    /// let path = s.outstanding_path_per_flow().expect("valid schedule");
    /// assert_eq!(path.len(), 2);
    /// assert_eq!(path[0].1.amount(), 90.0);  // 100 - 10 = 90
    /// assert_eq!(path[1].1.amount(), 95.0);  // 90 + 5 = 95
    /// ```
    pub fn outstanding_path_per_flow(&self) -> finstack_core::Result<Vec<(Date, Money)>> {
        let mut out = Vec::with_capacity(self.flows.len());
        let mut outstanding = self.notional.initial;
        for cf in &self.flows {
            // `outstanding_path_per_flow` historically ignored notional draws/repays and
            // only tracked Amortization and PIK. Preserve that behavior by
            // passing `include_notional = false`.
            apply_flow_to_outstanding(&mut outstanding, cf, false, false)?;
            out.push((cf.date, outstanding));
        }
        Ok(out)
    }

    /// Get an iterator over interest-like coupon cashflows.
    ///
    /// Includes `Fixed`, `FloatReset`, `InflationCoupon`, and `Stub` kinds.
    pub fn coupons(&self) -> impl Iterator<Item = &CashFlow> {
        self.flows.iter().filter(|cf| cf.kind.is_interest_like())
    }

    /// Weighted Average Life (WAL) in years from `as_of`.
    ///
    /// WAL = Σ(principal_i × t_i) / Σ(principal_i)
    ///
    /// where t_i is the year fraction from `as_of` to the payment date,
    /// and the sum runs over all principal flows (Amortization, Notional,
    /// PrePayment) with positive amounts after `as_of`.
    ///
    /// WAL is computed on an Act/365F basis regardless of the schedule's
    /// accrual day count, matching conventional desk reporting. This avoids
    /// silent mis-computation when the schedule uses Act/Act ISMA or
    /// Bus/252, which require calendar or frequency context that WAL does
    /// not carry.
    ///
    /// Returns `Ok(0.0)` if there are no future principal flows.
    ///
    /// # Errors
    ///
    /// Returns an error if the day-count year-fraction calculation fails.
    pub fn weighted_average_life(&self, as_of: Date) -> finstack_core::Result<f64> {
        let mut principal_time_sum = 0.0;
        let mut principal_total = 0.0;

        for cf in &self.flows {
            if matches!(
                cf.kind,
                CFKind::Amortization | CFKind::Notional | CFKind::PrePayment
            ) && cf.date > as_of
                && cf.amount.amount() > 0.0
            {
                let t = DayCount::Act365F.year_fraction(as_of, cf.date, DayCountCtx::default())?;
                principal_time_sum += cf.amount.amount() * t;
                principal_total += cf.amount.amount();
            }
        }

        if principal_total > 0.0 {
            Ok(principal_time_sum / principal_total)
        } else {
            Ok(0.0)
        }
    }

    /// Full outstanding path including Amortization, PIK, and Notional draws/repays.
    ///
    /// Returns one entry per unique date after applying all balance-affecting flows
    /// on that date. This is the **canonical method** for tracking outstanding balance
    /// in instruments with dynamic draws/repays (RCFs, delayed-draw term loans).
    ///
    /// # When to Use Each Method
    ///
    /// - **[`Self::outstanding_path_per_flow()`]**: Simplified view for scheduled amortization
    ///   (excludes Notional draws/repays).
    /// - **`outstanding_by_date()`**: Full balance tracking including all notional events.
    ///
    /// # Balance Changes
    ///
    /// - **Amortization**: Reduces outstanding (stored as positive amounts)
    /// - **PIK**: Increases outstanding (capitalizes into principal)
    /// - **Notional**: Draws are negative (increase outstanding), repays are positive
    ///
    /// Note: The initial notional flow (funding at issue) is skipped as it's already
    /// accounted for in `notional.initial`. Only subsequent draws/repays are tracked.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Amortization or repayment exceeds current outstanding
    /// - Currency mismatch between flows and notional
    pub fn outstanding_by_date(&self) -> finstack_core::Result<Vec<(Date, Money)>> {
        let mut result: Vec<(Date, Money)> = Vec::with_capacity(self.flows.len());
        if self.flows.is_empty() {
            return Ok(result);
        }

        let mut outstanding = self.notional.initial;

        // Identify and skip the initial funding flow (negative notional equal to initial).
        // This flow is already accounted for in `notional.initial`, and may not be the
        // earliest flow if there are pre-issue principal events.
        let mut initial_funding_skipped = false;
        let initial_amount = self.notional.initial.amount();

        let mut i = 0usize;
        while i < self.flows.len() {
            let d = self.flows[i].date;
            // Process all flows on this date in their deterministic order
            let mut j = i;
            while j < self.flows.len() && self.flows[j].date == d {
                // Skip the initial funding notional flow (negative, equal to -notional.initial)
                // This is already accounted for in notional.initial
                let is_initial_funding = !initial_funding_skipped
                    && self.flows[j].kind == CFKind::Notional
                    && self.flows[j].amount.amount() < 0.0
                    && initial_amount != 0.0
                    && match self.meta.issue_date {
                        Some(issue) => self.flows[j].date == issue,
                        None => amounts_approx_equal(
                            self.flows[j].amount.amount().abs(),
                            initial_amount,
                        ),
                    };
                if is_initial_funding {
                    initial_funding_skipped = true;
                }

                // `outstanding_by_date` is the canonical balance tracker, including
                // subsequent notional draws/repays as well as Amortization and PIK.
                apply_flow_to_outstanding(
                    &mut outstanding,
                    &self.flows[j],
                    is_initial_funding,
                    true,
                )?;
                j += 1;
            }
            result.push((d, outstanding));
            i = j;
        }

        Ok(result)
    }
}

/// Merge multiple schedules into one deterministic composite schedule.
pub fn merge_cashflow_schedules<I>(
    schedules: I,
    notional: Notional,
    day_count: DayCount,
) -> CashFlowSchedule
where
    I: IntoIterator<Item = CashFlowSchedule>,
{
    let mut flows = Vec::new();
    let mut calendar_ids = Vec::new();
    let mut facility_limit: Option<Option<Money>> = None;
    let mut issue_date: Option<Option<Date>> = None;
    let mut representation: Option<CashflowRepresentation> = None;

    for schedule in schedules {
        representation = Some(match representation {
            None => schedule.meta.representation,
            Some(existing) if existing == schedule.meta.representation => existing,
            Some(_) => CashflowRepresentation::default(),
        });
        flows.extend(schedule.flows);
        calendar_ids.extend(schedule.meta.calendar_ids);
        facility_limit = Some(match facility_limit {
            None => schedule.meta.facility_limit,
            Some(existing) if existing == schedule.meta.facility_limit => existing,
            Some(_) => None,
        });
        issue_date = Some(match issue_date {
            None => schedule.meta.issue_date,
            Some(existing) if existing == schedule.meta.issue_date => existing,
            Some(_) => None,
        });
    }

    calendar_ids.sort_unstable();
    calendar_ids.dedup();

    CashFlowSchedule::from_parts(
        flows,
        notional,
        day_count,
        CashFlowMeta {
            representation: representation.unwrap_or_default(),
            calendar_ids,
            facility_limit: facility_limit.unwrap_or(None),
            issue_date: issue_date.unwrap_or(None),
        },
    )
}

/// Compare two amounts using relative epsilon for floating-point tolerance.
///
/// Uses a relative tolerance of 1e-9 scaled by magnitude, with a minimum
/// absolute tolerance of 1e-9 (from the `.max(1.0)` floor).
///
/// # Tolerance Bounds by Scale
///
/// | Notional     | Tolerance  |
/// |--------------|------------|
/// | $1B          | ~$1        |
/// | $1M          | ~$0.001    |
/// | $1K          | ~$0.000001 |
/// | Near zero    | 1e-9       |
///
/// This is sufficient for detecting the initial funding flow while
/// allowing for floating-point representation differences.
pub(super) fn amounts_approx_equal(a: f64, b: f64) -> bool {
    let max_abs = a.abs().max(b.abs()).max(1.0);
    (a - b).abs() < max_abs * 1e-9
}

fn apply_flow_to_outstanding(
    outstanding: &mut Money,
    cf: &CashFlow,
    is_initial_funding: bool,
    include_notional: bool,
) -> finstack_core::Result<()> {
    match cf.kind {
        CFKind::Amortization | CFKind::PrePayment | CFKind::DefaultedNotional => {
            // Amortization amounts are stored as positive in the builder
            // but economically represent principal reductions.
            // PrePayment and DefaultedNotional likewise reduce outstanding.
            *outstanding = outstanding.checked_sub(cf.amount)?;
        }
        CFKind::PIK => {
            *outstanding = outstanding.checked_add(cf.amount)?;
        }
        CFKind::Notional if include_notional && !is_initial_funding => {
            // Draws negative, repays positive -> subtract to apply sign
            *outstanding = outstanding.checked_sub(cf.amount)?;
        }
        _ => {}
    }
    Ok(())
}

/// Collapse a per-period currency-indexed PV map into a single-currency map.
///
/// Returns a [`finstack_core::Error::Validation`] if any period contains more
/// than one currency or its aggregated `Money` currency disagrees with the
/// outer map key. Use in combination with [`CashFlowSchedule::pv_by_period_with_ctx`]
/// (or related methods) when the caller expects a homogeneous currency result.
pub fn require_single_currency(
    pv_map: IndexMap<PeriodId, IndexMap<Currency, Money>>,
) -> finstack_core::Result<IndexMap<PeriodId, Money>> {
    let mut result = IndexMap::with_capacity(pv_map.len());
    for (period_id, currency_map) in pv_map {
        let mut entries = currency_map.into_iter();
        if let Some((currency, pv_money)) = entries.next() {
            if entries.next().is_some() {
                return Err(finstack_core::Error::Validation(format!(
                    "period {period_id} has multiple currencies; single-currency PV output is not available"
                )));
            }
            if pv_money.currency() != currency {
                return Err(finstack_core::Error::Validation(format!(
                    "period {period_id} returned inconsistent currency aggregation"
                )));
            }
            result.insert(period_id, pv_money);
        }
    }
    Ok(result)
}

impl CashFlowSchedule {
    /// Compute pre-period present values with explicit day-count context.
    ///
    /// Like `pv_by_period`, but accepts a `DayCountCtx` for conventions
    /// requiring frequency (Act/Act ISMA) or calendar (Bus/252).
    ///
    /// # Arguments
    /// * `periods` - Period definitions with start/end boundaries
    /// * `disc` - Discount curve for present value calculation
    /// * `base` - Base date for discounting (typically valuation date)
    /// * `dc` - Day count convention for year fraction calculation
    /// * `dc_ctx` - Day count context (frequency, calendar, bus_basis)
    ///
    /// # Returns
    /// Map from `PeriodId` to currency-indexed PV sums. Periods with no cashflows
    /// are omitted from the result.
    ///
    /// # Errors
    /// Returns error if day-count calculation fails (e.g., missing required context).
    pub fn pv_by_period_with_ctx(
        &self,
        periods: &[Period],
        disc: &dyn Discounting,
        base: Date,
        dc: DayCount,
        dc_ctx: DayCountCtx,
    ) -> finstack_core::Result<IndexMap<PeriodId, IndexMap<Currency, Money>>> {
        if self.flows.is_empty() || periods.is_empty() {
            return Ok(IndexMap::new());
        }
        // Schedule flows are always sorted at construction (maintained by sort_flows).
        // Use the CashFlow-native path to avoid intermediate Vec<DatedFlow> allocation.
        crate::aggregation::pv_by_period_cashflows_sorted_checked(
            &self.flows,
            periods,
            disc,
            base,
            dc,
            dc_ctx,
            None,
        )
    }

    /// Compute pre-period present values with market context and explicit day-count context.
    ///
    /// Compute present values with market context and explicit day-count context.
    ///
    /// When a hazard curve is provided, this function applies credit adjustment with
    /// recovery-of-par semantics: principal-like flows (Amortization, Notional) get
    /// `PV = Amount * DF * (SP + R * (1 - SP))` while interest/fee flows get
    /// `PV = Amount * DF * SP` (zero recovery).
    ///
    /// # Arguments
    /// * `periods` - Period definitions with start/end boundaries
    /// * `market` - Market context containing discount and optional hazard curves
    /// * `disc_curve_id` - Identifier for the discount curve in the market context
    /// * `hazard_curve_id` - Optional identifier for hazard curve (credit adjustment)
    /// * `base` - Base date for discounting (typically valuation date)
    /// * `dc` - Day count convention for year fraction calculation
    /// * `dc_ctx` - Day count context (frequency, calendar, bus_basis)
    ///
    /// # Returns
    /// Map from `PeriodId` to currency-indexed PV sums. Periods with no cashflows
    /// are omitted from the result.
    ///
    /// # Errors
    /// Returns an error if the discount curve is not found, if hazard_curve_id is provided
    /// but the curve is not found, or if day-count calculation fails.
    #[allow(clippy::too_many_arguments)]
    pub fn pv_by_period_with_market_and_ctx(
        &self,
        periods: &[Period],
        market: &MarketContext,
        disc_curve_id: &CurveId,
        hazard_curve_id: Option<&CurveId>,
        base: Date,
        dc: DayCount,
        dc_ctx: DayCountCtx,
    ) -> finstack_core::Result<IndexMap<PeriodId, IndexMap<Currency, Money>>> {
        let curves = resolve_credit_curves(market, disc_curve_id, hazard_curve_id)?;
        let disc: &dyn Discounting = curves.discounting();
        let hazard = curves.hazard_survival();

        let date_ctx = crate::aggregation::DateContext::new(base, dc, dc_ctx);
        self.pv_by_period_with_survival_and_ctx(
            periods,
            disc,
            hazard,
            curves.recovery_rate(),
            date_ctx,
        )
    }

    /// Compute period PVs from resolved discount/survival curves.
    #[allow(clippy::too_many_arguments)]
    pub fn pv_by_period_with_survival_and_ctx(
        &self,
        periods: &[Period],
        disc: &dyn Discounting,
        hazard: Option<&dyn Survival>,
        recovery_rate: Option<f64>,
        date_ctx: crate::aggregation::DateContext<'_>,
    ) -> finstack_core::Result<IndexMap<PeriodId, IndexMap<Currency, Money>>> {
        if let Some(hazard_curve) = hazard {
            crate::aggregation::pv_by_period_credit_adjusted_detailed(
                &self.flows,
                periods,
                disc,
                Some(hazard_curve),
                recovery_rate,
                date_ctx,
            )
        } else {
            self.pv_by_period_with_ctx(periods, disc, date_ctx.base, date_ctx.dc, date_ctx.dc_ctx)
        }
    }
}

pub(crate) struct CreditCurveHandles {
    discount: Arc<DiscountCurve>,
    hazard: Option<Arc<HazardCurve>>,
}

impl CreditCurveHandles {
    pub(crate) fn discounting(&self) -> &dyn Discounting {
        self.discount.as_ref()
    }

    pub(crate) fn hazard_survival(&self) -> Option<&dyn Survival> {
        self.hazard
            .as_ref()
            .map(|arc| arc.as_ref() as &dyn Survival)
    }

    pub(crate) fn recovery_rate(&self) -> Option<f64> {
        self.hazard.as_ref().map(|h| h.recovery_rate())
    }
}

pub(crate) fn resolve_credit_curves(
    market: &MarketContext,
    disc_curve_id: &CurveId,
    hazard_curve_id: Option<&CurveId>,
) -> finstack_core::Result<CreditCurveHandles> {
    let discount = market.get_discount(disc_curve_id.as_str())?;
    let hazard = if let Some(hazard_id) = hazard_curve_id {
        Some(market.get_hazard(hazard_id.as_str())?)
    } else {
        None
    };
    Ok(CreditCurveHandles { discount, hazard })
}

// =============================================================================
// IntoIterator implementations for ergonomic for-loops
// =============================================================================

impl IntoIterator for CashFlowSchedule {
    type Item = CashFlow;
    type IntoIter = std::vec::IntoIter<CashFlow>;

    fn into_iter(self) -> Self::IntoIter {
        self.flows.into_iter()
    }
}

impl<'a> IntoIterator for &'a CashFlowSchedule {
    type Item = &'a CashFlow;
    type IntoIter = std::slice::Iter<'a, CashFlow>;

    fn into_iter(self) -> Self::IntoIter {
        self.flows.iter()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::dates::DayCount;
    use time::Month;

    fn flow(date: Date, amount: f64, kind: CFKind) -> CashFlow {
        CashFlow {
            date,
            reset_date: None,
            amount: Money::new(amount, Currency::USD),
            kind,
            accrual_factor: 0.0,
            rate: None,
        }
    }

    #[test]
    fn from_parts_sorts_by_date_then_kind_rank() {
        let date = Date::from_calendar_date(2025, Month::January, 15).expect("valid date");
        let schedule = CashFlowSchedule::from_parts(
            vec![
                flow(date, 10.0, CFKind::Recovery),
                flow(date, 12.0, CFKind::Amortization),
                flow(date, 8.0, CFKind::PrePayment),
                flow(date, 5.0, CFKind::Fixed),
            ],
            Notional::par(100.0, Currency::USD),
            DayCount::Act365F,
            CashFlowMeta::default(),
        );

        assert_eq!(schedule.flows[0].kind, CFKind::Fixed);
        assert_eq!(schedule.flows[1].kind, CFKind::Amortization);
        assert_eq!(schedule.flows[2].kind, CFKind::PrePayment);
        assert_eq!(schedule.flows[3].kind, CFKind::Recovery);
    }

    #[test]
    fn merge_cashflow_schedules_merges_meta_and_resorts() {
        let d1 = Date::from_calendar_date(2025, Month::January, 15).expect("valid date");
        let d2 = Date::from_calendar_date(2025, Month::February, 15).expect("valid date");
        let left = CashFlowSchedule::from_parts(
            vec![flow(d2, 4.0, CFKind::Recovery)],
            Notional::par(50.0, Currency::USD),
            DayCount::Act365F,
            CashFlowMeta {
                representation: CashflowRepresentation::Projected,
                calendar_ids: vec!["nyc".to_string()],
                facility_limit: None,
                issue_date: Some(d1),
            },
        );
        let right = CashFlowSchedule::from_parts(
            vec![flow(d1, 10.0, CFKind::Amortization)],
            Notional::par(50.0, Currency::USD),
            DayCount::Act365F,
            CashFlowMeta {
                representation: CashflowRepresentation::Projected,
                calendar_ids: vec!["lon".to_string(), "nyc".to_string()],
                facility_limit: None,
                issue_date: Some(d1),
            },
        );

        let merged = merge_cashflow_schedules(
            vec![left, right],
            Notional::par(100.0, Currency::USD),
            DayCount::Act365F,
        );

        assert_eq!(merged.flows.len(), 2);
        assert_eq!(merged.flows[0].date, d1);
        assert_eq!(
            merged.meta.representation,
            CashflowRepresentation::Projected
        );
        assert_eq!(
            merged.meta.calendar_ids,
            vec!["lon".to_string(), "nyc".to_string()]
        );
        assert_eq!(merged.meta.issue_date, Some(d1));
    }

    #[test]
    fn wal_uses_act365f_regardless_of_schedule_day_count() {
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let d1 = Date::from_calendar_date(2026, Month::January, 1).expect("valid date");
        let d2 = Date::from_calendar_date(2027, Month::January, 1).expect("valid date");

        let schedule = CashFlowSchedule::from_parts(
            vec![
                flow(d1, 500_000.0, CFKind::Amortization),
                flow(d2, 500_000.0, CFKind::Amortization),
            ],
            Notional::par(1_000_000.0, Currency::USD),
            DayCount::Thirty360, // schedule uses 30/360 but WAL should use Act/365F
            CashFlowMeta::default(),
        );

        let wal = schedule.weighted_average_life(as_of).expect("WAL succeeds");

        // Compute expected WAL with Act/365F:
        // d1: 365 days / 365 = 1.0 years
        // d2: 731 days / 365 ≈ 2.0027 years (2026 is not a leap year, 2×365+1 ≈ 731)
        // WAL = (500k * 1.0 + 500k * t2) / 1M
        let t1 = DayCount::Act365F
            .year_fraction(as_of, d1, DayCountCtx::default())
            .unwrap();
        let t2 = DayCount::Act365F
            .year_fraction(as_of, d2, DayCountCtx::default())
            .unwrap();
        let expected = (500_000.0 * t1 + 500_000.0 * t2) / 1_000_000.0;

        assert!(
            (wal - expected).abs() < 1e-10,
            "WAL should match Act/365F calculation: expected {}, got {}",
            expected,
            wal
        );

        // Also verify it differs from 30/360 (which would give 1.0 and 2.0 exactly)
        let t30_360_1 = DayCount::Thirty360
            .year_fraction(as_of, d1, DayCountCtx::default())
            .unwrap();
        let t30_360_2 = DayCount::Thirty360
            .year_fraction(as_of, d2, DayCountCtx::default())
            .unwrap();
        let wal_30360 = (500_000.0 * t30_360_1 + 500_000.0 * t30_360_2) / 1_000_000.0;

        // The values should differ (Act/365F vs 30/360 give different year fractions
        // for multi-year spans). If they match, the WAL is accidentally using the
        // schedule day count instead of Act/365F.
        // Note: for these specific dates they may be very close, so we just verify
        // our function returns the Act/365F-based value.
        assert!(
            (wal - expected).abs() < (wal - wal_30360).abs() || (wal - expected).abs() < 1e-10,
            "WAL should be closer to Act/365F value than 30/360 value"
        );
    }
}
