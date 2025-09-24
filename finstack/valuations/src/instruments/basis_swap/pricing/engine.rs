//! Core basis swap pricing engine and shared helpers.

use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::Result;
use finstack_core::F;

/// Common basis swap pricing engine providing core calculation methods.
///
/// This engine contains the fundamental pricing logic for basis swap legs,
/// including present value calculations and annuity computations.
pub struct BasisEngine;

/// Parameters for floating leg present value calculation.
///
/// Contains all the necessary parameters to calculate the present value
/// of a floating rate leg in a basis swap.
///
/// # Examples
/// ```rust
/// use finstack_core::{dates::*, money::Money, currency::Currency};
/// use finstack_valuations::instruments::basis_swap::pricing::engine::FloatLegParams;
/// use finstack_valuations::cashflow::builder::schedule_utils::PeriodSchedule;
/// use time::Month;
///
/// let schedule = PeriodSchedule {
///     dates: vec![],
///     first_or_last: hashbrown::HashSet::new(),
///     prev: hashbrown::HashMap::new()
/// };
/// let params = FloatLegParams {
///     schedule: &schedule,
///     notional: Money::new(1_000_000.0, Currency::USD),
///     disc_id: "OIS".into(),
///     fwd_id: "3M-SOFR".into(),
///     accrual_dc: DayCount::Act360,
///     spread: 0.0005,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct FloatLegParams<'a> {
    /// Period schedule for this leg.
    pub schedule: &'a crate::cashflow::builder::schedule_utils::PeriodSchedule,
    /// Notional amount for the leg.
    pub notional: Money,
    /// Discount curve identifier.
    pub disc_id: CurveId,
    /// Forward curve identifier.
    pub fwd_id: CurveId,
    /// Day count for accrual calculations.
    pub accrual_dc: DayCount,
    /// Spread in decimal form (e.g., 0.0005 for 5 basis points).
    pub spread: F,
}

impl BasisEngine {
    /// Calculates the present value of a floating rate leg.
    ///
    /// # Arguments
    /// * `params` — Parameters defining the leg characteristics
    /// * `context` — Market context containing curves and rates
    /// * `valuation_date` — Date for present value calculation
    ///
    /// # Returns
    /// The present value of the floating leg as a `Money` amount.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::{dates::*, money::Money, currency::Currency};
    /// use finstack_valuations::instruments::basis_swap::pricing::engine::{BasisEngine, FloatLegParams};
    /// use finstack_valuations::cashflow::builder::schedule_utils::PeriodSchedule;
    /// use time::Month;
    ///
    /// let schedule = PeriodSchedule {
    ///     dates: vec![],
    ///     first_or_last: hashbrown::HashSet::new(),
    ///     prev: hashbrown::HashMap::new()
    /// };
    /// let params = FloatLegParams {
    ///     schedule: &schedule,
    ///     notional: Money::new(1_000_000.0, Currency::USD),
    ///     disc_id: "OIS".into(),
    ///     fwd_id: "3M-SOFR".into(),
    ///     accrual_dc: DayCount::Act360,
    ///     spread: 0.0005,
    /// };
    /// // Note: Requires proper MarketContext setup for actual usage
    /// ```
    pub fn pv_float_leg(
        params: FloatLegParams,
        context: &MarketContext,
        valuation_date: Date,
    ) -> Result<Money> {
        // Curves
        let disc = context
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            params.disc_id.clone(),
        )?;
        let fwd = context
            .get_ref::<finstack_core::market_data::term_structures::forward_curve::ForwardCurve>(
            params.fwd_id.clone(),
        )?;

        let mut pv = 0.0;
        let currency = params.notional.currency();
        let dc_ctx = DayCountCtx::default();

        // Iterate periods
        for i in 1..params.schedule.dates.len() {
            let period_start = params.schedule.dates[i - 1];
            let period_end = params.schedule.dates[i];

            // Skip past periods
            if period_end <= valuation_date {
                continue;
            }

            // Forward rate for the accrual period using the forward curve's own time basis
            let fwd_dc = fwd.day_count();
            let fwd_base = fwd.base_date();
            let t_start = fwd_dc.year_fraction(fwd_base, period_start, dc_ctx)?;
            let t_end = fwd_dc.year_fraction(fwd_base, period_end, dc_ctx)?;
            let forward_rate = fwd.rate_period(t_start, t_end);

            // Total rate (add spread)
            let total_rate = forward_rate + params.spread;

            // Accrual year fraction
            let year_frac = params
                .accrual_dc
                .year_fraction(period_start, period_end, dc_ctx)?;

            // Payment
            let payment = params.notional.amount() * total_rate * year_frac;

            // Discount factor to payment date using the curve's own day-count basis
            let df = disc.df_on_date_curve(period_end);
            pv += payment * df;
        }

        Ok(Money::new(pv, currency))
    }

    /// Calculates the discounted accrual sum (annuity) for a leg.
    ///
    /// This method computes the sum of discounted year fractions for a leg,
    /// which is useful for DV01 calculations and par spread computations.
    ///
    /// # Arguments
    /// * `schedule` — Period schedule for the leg
    /// * `accrual_dc` — Day count convention for accrual calculations
    /// * `disc_curve_id` — Discount curve identifier
    /// * `curves` — Market context containing the discount curve
    ///
    /// # Returns
    /// The discounted accrual sum as a floating point value.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::dates::DayCount;
    /// use finstack_valuations::instruments::basis_swap::pricing::engine::BasisEngine;
    /// use finstack_valuations::cashflow::builder::schedule_utils::PeriodSchedule;
    ///
    /// let schedule = PeriodSchedule {
    ///     dates: vec![],
    ///     first_or_last: hashbrown::HashSet::new(),
    ///     prev: hashbrown::HashMap::new()
    /// };
    /// // Note: Requires proper MarketContext setup for actual usage
    /// ```
    pub fn annuity_for_leg(
        schedule: &crate::cashflow::builder::schedule_utils::PeriodSchedule,
        accrual_dc: DayCount,
        disc_curve_id: &str,
        curves: &MarketContext,
    ) -> Result<F> {
        let disc = curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            CurveId::from(disc_curve_id),
        )?;
        let mut annuity = 0.0;
        let mut prev = schedule.dates[0];
        for &d in &schedule.dates[1..] {
            let yf = accrual_dc.year_fraction(prev, d, DayCountCtx::default())?;
            // Discount using the curve's own day-count basis
            let df = disc.df_on_date_curve(d);
            annuity += yf * df;
            prev = d;
        }
        Ok(annuity)
    }
}
