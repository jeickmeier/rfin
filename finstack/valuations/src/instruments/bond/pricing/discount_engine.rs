use finstack_core::dates::Date;
use finstack_core::dates::DateExt;
use finstack_core::explain::{ExplainOpts, ExplanationTrace, TraceEntry};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

use crate::cashflow::traits::CashflowProvider;
// Discountable trait not required after switching to curve day-count path

use super::super::types::Bond;
use super::super::CashflowSpec;

/// Bond pricing engine providing core valuation methods.
///
/// The engine expects **holder-view** cashflows from `Bond::build_schedule`,
/// i.e. all contractual amounts received by a long holder (coupons,
/// amortization, redemption) are positive, and any cash outflows are
/// represented separately at trade level (e.g. purchase price).
///
/// # Pricing Formula
///
/// The present value is computed by discounting all future holder-view cashflows:
/// ```text
/// PV = Σ CF_i · DF(settle_date → t_i)
/// ```
/// where:
/// - `CF_i` are holder-view cashflows (coupons, amortization, redemption)
/// - `DF(settle_date → t_i)` is the discount factor from settlement date to cashflow date
/// - Settlement date is computed from `as_of` using `bond.settlement_days` and calendar conventions
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::bond::Bond;
/// use finstack_valuations::instruments::bond::pricing::discount_engine::BondEngine;
/// use finstack_core::market_data::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example();
/// # let market = MarketContext::new();
/// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
/// let pv = BondEngine::price(&bond, &market, as_of)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct BondEngine;

impl BondEngine {
    /// Price a bond using discount curve present value calculation.
    ///
    /// Computes the present value by discounting all future holder-view cashflows
    /// from the settlement date using the bond's discount curve.
    ///
    /// # Arguments
    ///
    /// * `bond` - The bond to price
    /// * `context` - Market context containing the discount curve
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    ///
    /// Present value of the bond in the bond's currency, discounted from settlement date.
    ///
    /// # Errors
    ///
    /// Returns `Err` when:
    /// - Discount curve is not found in market context
    /// - Bond has no future cashflows
    /// - Cashflow schedule building fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::bond::Bond;
    /// use finstack_valuations::instruments::bond::pricing::discount_engine::BondEngine;
    /// use finstack_core::market_data::MarketContext;
    /// use finstack_core::dates::Date;
    ///
    /// # let bond = Bond::example();
    /// # let market = MarketContext::new();
    /// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
    /// let pv = BondEngine::price(&bond, &market, as_of)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn price(bond: &Bond, context: &MarketContext, as_of: Date) -> Result<Money> {
        Self::price_with_explanation(bond, context, as_of, ExplainOpts::disabled())
            .map(|(pv, _)| pv)
    }

    /// Price a bond with optional explanation trace.
    ///
    /// Returns the present value and an optional trace containing
    /// cashflow-level PV breakdown when explanation is enabled.
    ///
    /// # Arguments
    ///
    /// * `bond` - The bond to price
    /// * `context` - Market context containing the discount curve
    /// * `as_of` - Valuation date
    /// * `explain` - Explanation options controlling trace generation
    ///
    /// # Returns
    ///
    /// Tuple of `(Money, Option<ExplanationTrace>)`:
    /// - Present value of the bond
    /// - Optional explanation trace with cashflow-level breakdown (if enabled)
    ///
    /// # Errors
    ///
    /// Returns `Err` when:
    /// - Discount curve is not found in market context
    /// - Bond has no future cashflows
    /// - Cashflow schedule building fails
    /// - Calendar adjustment fails (if settlement days and calendar are specified)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::bond::Bond;
    /// use finstack_valuations::instruments::bond::pricing::discount_engine::BondEngine;
    /// use finstack_core::explain::ExplainOpts;
    /// use finstack_core::market_data::MarketContext;
    /// use finstack_core::dates::Date;
    ///
    /// # let bond = Bond::example();
    /// # let market = MarketContext::new();
    /// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
    /// let (pv, trace) = BondEngine::price_with_explanation(
    ///     &bond,
    ///     &market,
    ///     as_of,
    ///     ExplainOpts::enabled(),
    /// )?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn price_with_explanation(
        bond: &Bond,
        context: &MarketContext,
        as_of: Date,
        explain: ExplainOpts,
    ) -> Result<(Money, Option<ExplanationTrace>)> {
        let flows = bond.build_schedule(context, as_of)?;
        let disc = context.get_discount(bond.discount_curve_id.as_str())?;
        if flows.is_empty() {
            return Err(finstack_core::error::InputError::TooFewPoints.into());
        }
        let ccy = flows[0].1.currency();
        let mut total = Money::new(0.0, ccy);

        // Initialize explanation trace if requested
        let mut trace = if explain.enabled {
            Some(ExplanationTrace::new("pricing"))
        } else {
            None
        };

        // Settlement PV: start discounting from settlement date if provided
        let settle_date = if let Some(sd_u32) = bond.settlement_days {
            let sd: i32 = sd_u32 as i32;
            let (calendar_id, bdc) = match &bond.cashflow_spec {
                CashflowSpec::Fixed(spec) => (spec.calendar_id.as_deref(), spec.bdc),
                CashflowSpec::Floating(spec) => {
                    (spec.rate_spec.calendar_id.as_deref(), spec.rate_spec.bdc)
                }
                CashflowSpec::Amortizing { base, .. } => match &**base {
                    CashflowSpec::Fixed(spec) => (spec.calendar_id.as_deref(), spec.bdc),
                    CashflowSpec::Floating(spec) => {
                        (spec.rate_spec.calendar_id.as_deref(), spec.rate_spec.bdc)
                    }
                    _ => (None, finstack_core::dates::BusinessDayConvention::Following),
                },
            };
            if let Some(id) = calendar_id {
                if let Some(cal) = finstack_core::dates::calendar::calendar_by_id(id) {
                    // Walk business days using the provided calendar
                    let mut d = as_of;
                    let mut remaining = sd;
                    let step = if remaining >= 0 { 1 } else { -1 };
                    while remaining != 0 {
                        d = d.saturating_add(time::Duration::days(step as i64));
                        if cal.is_business_day(d) {
                            remaining -= step;
                        }
                    }
                    finstack_core::dates::adjust(d, bdc, cal)?
                } else {
                    as_of.add_weekdays(sd)
                }
            } else {
                as_of.add_weekdays(sd)
            }
        } else {
            as_of
        };
        // Pre-compute settle-date discount factor for correct theta using the
        // curve's own date mapping.
        let df_settle = disc.df_on_date_curve(settle_date);

        for (d, amt) in &flows {
            if *d <= settle_date {
                continue;
            }
            // Discount from settle_date (which is derived from as_of) using
            // curve-provided DF(date).
            let df_d_abs = disc.df_on_date_curve(*d);
            let df = if df_settle != 0.0 {
                df_d_abs / df_settle
            } else {
                1.0
            };
            let pv_cf = *amt * df;
            total = (total + pv_cf)?;

            // Add trace entry if explanation is enabled
            if let Some(ref mut t) = trace {
                t.push(
                    TraceEntry::CashflowPV {
                        date: d.to_string(),
                        cashflow_amount: amt.amount(),
                        cashflow_currency: amt.currency().to_string(),
                        discount_factor: df,
                        pv_amount: pv_cf.amount(),
                        pv_currency: pv_cf.currency().to_string(),
                        curve_id: bond.discount_curve_id.to_string(),
                    },
                    explain.max_entries,
                );
            }
        }
        Ok((total, trace))
    }
}
