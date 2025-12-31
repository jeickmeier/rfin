use finstack_core::dates::Date;
use finstack_core::explain::{ExplainOpts, ExplanationTrace, TraceEntry};
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::summation::kahan_sum;
use finstack_core::money::Money;
use finstack_core::Result;

use crate::cashflow::traits::CashflowProvider;
// Discountable trait not required after switching to curve day-count path

use super::super::types::Bond;

/// Bond pricing engine providing core valuation methods.
///
/// The engine expects **holder-view** cashflows from `CashflowProvider::build_dated_flows` on `Bond`,
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
/// Bond pricing is performed via the [`Instrument`] trait or the pricer registry:
///
/// ```rust,ignore
/// use finstack_valuations::instruments::Bond;
/// use finstack_valuations::instruments::common::traits::Instrument;
/// use finstack_core::market_data::context::MarketContext;
/// use time::macros::date;
///
/// let bond = Bond::example();
/// let market = MarketContext::new();
/// let as_of = date!(2024-01-15);
///
/// // Use Instrument trait for public API
/// let pv = bond.value(&market, as_of)?;
/// ```
///
/// [`Instrument`]: crate::instruments::common::traits::Instrument
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
    /// ```ignore
    /// use finstack_valuations::instruments::bond::Bond;
    /// use finstack_valuations::instruments::bond::pricing::discount_engine::BondEngine;
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::dates::Date;
    ///
    /// # let bond = Bond::example();
    /// # let market = MarketContext::new();
    /// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
    /// let pv = BondEngine::price(&bond, &market, as_of)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub(crate) fn price(bond: &Bond, context: &MarketContext, as_of: Date) -> Result<Money> {
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
    /// use finstack_core::market_data::context::MarketContext;
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
        let flows = bond.build_dated_flows(context, as_of)?;
        let disc = context.get_discount(bond.discount_curve_id.as_str())?;
        if flows.is_empty() {
            return Err(finstack_core::InputError::TooFewPoints.into());
        }
        let ccy = flows[0].1.currency();

        // Initialize explanation trace if requested
        let mut trace = if explain.enabled {
            Some(ExplanationTrace::new("pricing"))
        } else {
            None
        };

        // Settlement PV: start discounting from settlement date if provided
        let settle_date = super::settlement::settlement_date(bond, as_of)?;
        // Collect PV values for Kahan summation (O(1) error growth vs O(n) for naive sum).
        // This is particularly important for long-dated bonds (50Y+ monthly-pay).
        let mut pv_values: Vec<f64> = Vec::with_capacity(flows.len());

        for (d, amt) in &flows {
            if *d <= settle_date {
                continue;
            }
            let df = disc.df_between_dates(settle_date, *d)?;
            let pv_cf = *amt * df;
            pv_values.push(pv_cf.amount());

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

        // Use Kahan compensated summation from finstack-core for numerical stability
        let total = Money::new(kahan_sum(pv_values), ccy);
        Ok((total, trace))
    }
}
