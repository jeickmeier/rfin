//! Schedule generation from the builder state.
//!
//! Provides the canonical `CashFlowSchedule` type and helpers for sorting and
//! deriving schedule metadata. Downstream pricing/risk code consumes this shape.

use crate::cashflow::builder::Notional;
use crate::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::dates::{Date, DayCount, DayCountCtx, Period, PeriodId};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::market_data::traits::{Discounting, Survival};
use finstack_core::money::Money;
use finstack_core::types::{Currency, CurveId};
use indexmap::IndexMap;
use std::sync::Arc;

use super::specs::{FixedCouponSpec, FloatingCouponSpec};

pub(crate) fn kind_rank(kind: CFKind) -> u8 {
    match kind {
        CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => 0,
        CFKind::Fee => 1,
        CFKind::Amortization => 2,
        CFKind::PIK => 3,
        CFKind::Notional => 4,
        _ => 5,
    }
}

pub(crate) fn finalize_flows(
    mut flows: Vec<CashFlow>,
    fixed: &[FixedCouponSpec],
    floating: &[FloatingCouponSpec],
) -> (Vec<CashFlow>, CashFlowMeta, DayCount) {
    flows.sort_by(|a, b| {
        use core::cmp::Ordering;
        match a.date.cmp(&b.date) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => kind_rank(a.kind).cmp(&kind_rank(b.kind)),
        }
    });

    let mut cals: Vec<String> = fixed
        .iter()
        .filter_map(|s| s.calendar_id.clone())
        .chain(
            floating
                .iter()
                .filter_map(|s| s.rate_spec.calendar_id.clone()),
        )
        .collect();
    cals.sort_unstable();
    cals.dedup();
    let meta = CashFlowMeta {
        calendar_ids: cals,
        facility_limit: None,
    };

    let out_dc = if let Some(s) = fixed.first() {
        s.dc
    } else if let Some(s) = floating.first() {
        s.rate_spec.dc
    } else {
        DayCount::Act365F
    };
    (flows, meta, out_dc)
}

/// Minimal schedule metadata for a built schedule.
///
/// Tracks referenced calendar IDs so callers can understand adjustment context.
/// Metadata for cashflow schedules (calendar IDs, facility limits).
#[derive(Debug, Clone, Default)]
pub struct CashFlowMeta {
    /// Holiday calendar IDs used for schedule adjustments
    pub calendar_ids: Vec<String>,
    /// Optional facility limit/commitment for instruments like RCFs
    pub facility_limit: Option<Money>,
}

/// Cashflow schedule output from the composable builder.
///
/// Contains ordered cashflows plus notional and a representative `DayCount`.
/// Methods provide convenient accessors commonly used by pricing and analysis.
#[derive(Debug, Clone)]
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

impl CashFlowSchedule {
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
    /// use finstack_valuations::cashflow::builder::{CashFlowSchedule, CouponType, FixedCouponSpec};
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
    ///     calendar_id: None,
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
    /// - **`outstanding_path()`**: Use for coupon accrual calculations on fixed
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
    /// use finstack_valuations::cashflow::builder::schedule::{CashFlowMeta, CashFlowSchedule};
    /// use finstack_core::cashflow::{CashFlow, CFKind};
    /// use finstack_valuations::cashflow::builder::Notional;
    /// use time::Month;
    ///
    /// let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    /// let notional = Notional { initial: Money::new(100.0, Currency::USD), amort: Default::default() };
    /// let flows = vec![
    ///   CashFlow { date: base, reset_date: None, amount: Money::new(10.0, Currency::USD), kind: CFKind::Amortization, accrual_factor: 0.0, rate: None },
    ///   CashFlow { date: base, reset_date: None, amount: Money::new(5.0, Currency::USD), kind: CFKind::PIK, accrual_factor: 0.0, rate: None },
    /// ];
    /// let s = CashFlowSchedule { flows, notional, day_count: finstack_core::dates::DayCount::Act365F, meta: CashFlowMeta::default() };
    /// let path = s.outstanding_path().expect("valid schedule");
    /// assert_eq!(path.len(), 2);
    /// assert_eq!(path[0].1.amount(), 90.0);  // 100 - 10 = 90
    /// assert_eq!(path[1].1.amount(), 95.0);  // 90 + 5 = 95
    /// ```
    pub fn outstanding_path(&self) -> finstack_core::Result<Vec<(Date, Money)>> {
        let mut out = Vec::new();
        let mut outstanding = self.notional.initial;
        for cf in &self.flows {
            // `outstanding_path` historically ignored notional draws/repays and
            // only tracked Amortization and PIK. Preserve that behavior by
            // passing `include_notional = false`.
            apply_flow_to_outstanding(&mut outstanding, cf, false, false)?;
            out.push((cf.date, outstanding));
        }
        Ok(out)
    }

    /// Get an iterator over coupon cashflows (Fixed and Stub types).
    pub fn coupons(&self) -> impl Iterator<Item = &CashFlow> {
        self.flows
            .iter()
            .filter(|cf| cf.kind == CFKind::Fixed || cf.kind == CFKind::Stub)
    }

    /// Full outstanding path including Amortization, PIK, and Notional draws/repays.
    ///
    /// Returns one entry per unique date after applying all balance-affecting flows
    /// on that date. This is the **canonical method** for tracking outstanding balance
    /// in instruments with dynamic draws/repays (RCFs, delayed-draw term loans).
    ///
    /// # When to Use Each Method
    ///
    /// - **[`Self::outstanding_path()`]**: Simplified view for scheduled amortization
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
        let mut result: Vec<(Date, Money)> = Vec::new();
        if self.flows.is_empty() {
            return Ok(result);
        }

        let mut outstanding = self.notional.initial;

        // Identify the first date in the schedule (issue date)
        let first_date = self.flows.first().map(|cf| cf.date);

        let mut i = 0usize;
        while i < self.flows.len() {
            let d = self.flows[i].date;
            // Process all flows on this date in their deterministic order
            let mut j = i;
            while j < self.flows.len() && self.flows[j].date == d {
                // Skip the initial funding notional flow (negative, equal to -notional.initial)
                // This is already accounted for in notional.initial
                let is_initial_funding = first_date == Some(d)
                    && self.flows[j].amount.amount() < 0.0
                    && amounts_approx_equal(
                        self.flows[j].amount.amount().abs(),
                        self.notional.initial.amount(),
                    );

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

/// Compare two amounts using relative epsilon for floating-point tolerance.
///
/// Uses a relative tolerance scaled by the magnitude of the values, with a
/// minimum absolute tolerance of 1e-12 for values near zero.
fn amounts_approx_equal(a: f64, b: f64) -> bool {
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
        CFKind::Amortization => {
            // Amortization amounts are stored as positive in the builder
            // but economically represent principal reductions
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

impl CashFlowSchedule {
    /// Compute pre-period present values with explicit day-count context.
    ///
    /// Like [`Self::pre_period_pv`], but accepts a `DayCountCtx` for conventions
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
    pub fn pre_period_pv_with_ctx(
        &self,
        periods: &[Period],
        disc: &dyn Discounting,
        base: Date,
        dc: DayCount,
        dc_ctx: DayCountCtx,
    ) -> finstack_core::Result<IndexMap<PeriodId, IndexMap<Currency, Money>>> {
        let flows: Vec<(Date, Money)> = self.flows.iter().map(|cf| (cf.date, cf.amount)).collect();
        crate::cashflow::aggregation::pv_by_period_with_ctx(&flows, periods, disc, base, dc, dc_ctx)
    }

    /// Compute pre-period present values with market context and explicit day-count context.
    ///
    /// Like [`Self::pre_period_pv_with_market`], but accepts `DayCountCtx` for full control.
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
    pub fn pre_period_pv_with_market_and_ctx(
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

        if hazard.is_some() {
            // Use the detailed function that preserves CFKind and applies recovery
            // to principal flows (Amortization, Notional) but not interest/fees.
            let date_ctx = crate::cashflow::aggregation::DateContext::new(base, dc, dc_ctx);
            crate::cashflow::aggregation::pv_by_period_credit_adjusted_detailed(
                &self.flows,
                periods,
                disc,
                hazard,
                curves.recovery_rate(),
                date_ctx,
            )
        } else {
            let flows: Vec<(Date, Money)> =
                self.flows.iter().map(|cf| (cf.date, cf.amount)).collect();
            crate::cashflow::aggregation::pv_by_period_with_ctx(
                &flows, periods, disc, base, dc, dc_ctx,
            )
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
