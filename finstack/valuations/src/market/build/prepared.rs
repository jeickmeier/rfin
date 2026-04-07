//! Prepared quote envelopes for calibration pipelines.

use crate::instruments::DynInstrument;
use crate::market::build::cds::build_cds_instrument;
use crate::market::build::rates::build_rate_instrument;
use crate::market::quotes::cds::CdsQuote;
use crate::market::quotes::rates::RateQuote;
use crate::market::BuildCtx;
use finstack_core::dates::Date;
use finstack_core::dates::{DayCount, DayCountCtx};
use finstack_core::Result;
use std::fmt;
use std::sync::Arc;

/// A quote accompanied by its constructed instrument and precomputed time pillar.
///
/// This structure is the primary input for calibration solvers. It decouples the solver
/// from the details of quote parsing, convention resolution, and instrument construction.
/// The precomputed `pillar_time` allows solvers to efficiently sort and group quotes by
/// maturity without recalculating time-to-maturity for each iteration.
///
/// # Invariants
///
/// - `pillar_time` is calculated using the day-count convention chosen by the calibration target
/// - `pillar_date` corresponds to the maturity date of the instrument's pillar
/// - `instrument` is fully configured and ready for pricing
///
/// # Note
///
/// This is ephemeral and valid only for the `as_of` date used during construction.
/// If the valuation date changes, a new `PreparedQuote` must be created.
///
/// # Examples
///
/// ```text
/// # use finstack_valuations::market::build::prepared::PreparedQuote;
/// # use finstack_valuations::market::quotes::rates::RateQuote;
/// # use finstack_valuations::market::quotes::ids::QuoteId;
/// # use finstack_core::dates::Date;
/// # use std::sync::Arc;
/// # use finstack_valuations::instruments::DynInstrument;
/// #
/// # fn example() -> finstack_core::Result<()> {
/// // In practice, this would be created by a builder function
/// // let prepared = prepare_quote(quote, ctx)?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub(crate) struct PreparedQuote<Q> {
    /// The original market quote.
    ///
    /// Stored as `Arc` to allow sharing across multiple solver iterations without cloning.
    pub(crate) quote: Arc<Q>,
    /// The constructed instrument, fully configured for pricing.
    ///
    /// The instrument is ready to be priced and includes all necessary curve references,
    /// dates, and market conventions resolved from the quote.
    pub(crate) instrument: Arc<DynInstrument>,
    /// The maturity date of the pillar (used for sorting / time axis).
    ///
    /// This is the resolved maturity date from the quote's pillar (either from a tenor
    /// calculation or a direct date specification).
    pub(crate) pillar_date: Date,
    /// The time-to-maturity of the pillar (in years), precomputed for the solver.
    ///
    /// This value is calculated once during construction and reused by calibration solvers
    /// for sorting, grouping, and time-axis calculations.
    pub(crate) pillar_time: f64,
}

impl<Q: fmt::Debug> fmt::Debug for PreparedQuote<Q> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedQuote")
            .field("quote", &self.quote)
            .field("pillar_date", &self.pillar_date)
            .field("pillar_time", &self.pillar_time)
            .field("instrument", &"<Instrument>")
            .finish()
    }
}

impl<Q> PreparedQuote<Q> {
    /// Create a new prepared quote.
    ///
    /// # Arguments
    ///
    /// * `quote` - The original market quote (wrapped in `Arc` for sharing)
    /// * `instrument` - The constructed instrument ready for pricing
    /// * `pillar_date` - The resolved maturity date of the pillar
    /// * `pillar_time` - The time-to-maturity in years, calculated from `as_of` to `pillar_date`
    ///
    /// # Returns
    ///
    /// A new `PreparedQuote` instance.
    ///
    /// # Examples
    ///
    /// ```text
    /// # use finstack_valuations::market::build::prepared::PreparedQuote;
    /// # use finstack_core::dates::Date;
    /// # use std::sync::Arc;
    /// # use finstack_valuations::instruments::DynInstrument;
    /// #
    /// # fn example(quote: Arc<String>, instrument: Arc<DynInstrument>) -> finstack_core::Result<()> {
    /// let pillar_date = Date::from_calendar_date(2025, time::Month::January, 2).unwrap();
    /// let as_of = Date::from_calendar_date(2024, time::Month::January, 2).unwrap();
    /// let pillar_time = (pillar_date - as_of).whole_days() as f64 / 365.25;
    ///
    /// let prepared = PreparedQuote::new(quote, instrument, pillar_date, pillar_time);
    /// # Ok(())
    /// # }
    /// ```
    pub(crate) fn new(
        quote: Arc<Q>,
        instrument: Arc<DynInstrument>,
        pillar_date: Date,
        pillar_time: f64,
    ) -> Self {
        Self {
            quote,
            instrument,
            pillar_date,
            pillar_time,
        }
    }
}

/// Policy for resolving swap pillars.
#[derive(Debug, Clone)]
pub(crate) struct PillarPolicy {
    /// When true, swap pillars use payment-delay-adjusted end dates (matches discount target).
    pub(crate) swap_use_payment_delay: bool,
}

impl Default for PillarPolicy {
    fn default() -> Self {
        Self {
            swap_use_payment_delay: true,
        }
    }
}

/// Prepare a rate quote into an instrument + pillar time.
pub(crate) fn prepare_rate_quote(
    quote: RateQuote,
    build_ctx: &BuildCtx,
    curve_day_count: DayCount,
    base_date: Date,
    policy: &PillarPolicy,
) -> Result<PreparedQuote<RateQuote>> {
    let instrument = build_rate_instrument(&quote, build_ctx)?;
    let instrument: Arc<DynInstrument> = instrument.into();

    let maturity_date = if let Some(dep) = instrument
        .as_any()
        .downcast_ref::<crate::instruments::rates::deposit::Deposit>(
    ) {
        dep.effective_end_date()?
    } else if let Some(fra) = instrument
        .as_any()
        .downcast_ref::<crate::instruments::rates::fra::ForwardRateAgreement>(
    ) {
        fra.maturity
    } else if let Some(swp) = instrument
        .as_any()
        .downcast_ref::<crate::instruments::rates::irs::InterestRateSwap>()
    {
        let end = std::cmp::max(swp.fixed.end, swp.float.end);
        if policy.swap_use_payment_delay {
            crate::instruments::rates::irs::dates::add_payment_delay(
                end,
                swp.fixed.payment_lag_days,
                swp.fixed.calendar_id.as_deref(),
            )?
        } else {
            end
        }
    } else if let Some(fut) = instrument
        .as_any()
        .downcast_ref::<crate::instruments::rates::ir_future::InterestRateFuture>(
    ) {
        fut.period_end.unwrap_or(fut.expiry)
    } else {
        return Err(finstack_core::Error::Validation(
            "prepare_rate_quote: unrecognized instrument type (not Deposit, FRA, IRS, or IRFuture)"
                .to_string(),
        ));
    };

    let pillar_time =
        curve_day_count.year_fraction(base_date, maturity_date, DayCountCtx::default())?;

    Ok(PreparedQuote::new(
        Arc::new(quote),
        instrument,
        maturity_date,
        pillar_time,
    ))
}

/// Prepare a CDS quote into an instrument + pillar time.
pub(crate) fn prepare_cds_quote(
    quote: CdsQuote,
    build_ctx: &BuildCtx,
    day_count: DayCount,
    base_date: Date,
) -> Result<PreparedQuote<CdsQuote>> {
    let instrument = build_cds_instrument(&quote, build_ctx)?;
    let instrument: Arc<DynInstrument> = instrument.into();

    let maturity_date = instrument
        .as_any()
        .downcast_ref::<crate::instruments::credit_derivatives::cds::CreditDefaultSwap>()
        .map(|cds| cds.premium.end)
        .ok_or_else(|| finstack_core::Error::Validation("Expected CDS instrument".to_string()))?;

    let pillar_time = day_count.year_fraction(base_date, maturity_date, DayCountCtx::default())?;

    Ok(PreparedQuote::new(
        Arc::new(quote),
        instrument,
        maturity_date,
        pillar_time,
    ))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::market::quotes::ids::QuoteId;
    use crate::market::quotes::rates::RateQuote;
    use finstack_core::HashMap;
    use time::Month;

    #[test]
    fn prepare_rate_quote_uses_future_period_end_as_pillar() {
        let as_of = Date::from_calendar_date(2025, Month::January, 10).expect("valid date");
        let mut curve_ids = HashMap::default();
        curve_ids.insert("discount".to_string(), "USD-OIS".to_string());
        curve_ids.insert("forward".to_string(), "USD-SOFR".to_string());
        let ctx = BuildCtx::new(as_of, 1_000_000.0, curve_ids);

        let quote = RateQuote::Futures {
            id: QuoteId::new("USD-FUT-SEP25"),
            contract: "SR3".into(),
            expiry: Date::from_calendar_date(2025, Month::September, 15).expect("valid expiry"),
            price: 96.50,
            convexity_adjustment: None,
            vol_surface_id: None,
        };

        let prepared = prepare_rate_quote(
            quote,
            &ctx,
            DayCount::Act365F,
            as_of,
            &PillarPolicy::default(),
        )
        .expect("prepared futures quote");

        let future = prepared
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::rates::ir_future::InterestRateFuture>()
            .expect("expected interest rate future");

        assert_eq!(
            prepared.pillar_date,
            future.period_end.expect("future period_end")
        );
    }

    #[test]
    fn prepare_rate_quote_uses_swap_end_as_pillar() {
        let as_of = Date::from_calendar_date(2025, Month::January, 10).expect("valid date");
        let mut curve_ids = HashMap::default();
        curve_ids.insert("discount".to_string(), "USD-OIS".to_string());
        curve_ids.insert("forward".to_string(), "USD-SOFR".to_string());
        let ctx = BuildCtx::new(as_of, 1_000_000.0, curve_ids);

        let quote = RateQuote::Swap {
            id: QuoteId::new("USD-SOFR-OIS-SWAP-5Y"),
            index: crate::market::conventions::ids::IndexId::new("USD-SOFR-OIS"),
            pillar: crate::market::quotes::ids::Pillar::Tenor(finstack_core::dates::Tenor::new(
                5,
                finstack_core::dates::TenorUnit::Years,
            )),
            rate: 0.0450,
            spread_decimal: None,
        };

        let prepared = prepare_rate_quote(
            quote,
            &ctx,
            DayCount::Act365F,
            as_of,
            &PillarPolicy::default(),
        )
        .expect("prepared swap quote");

        assert!(
            prepared.pillar_time > 4.0,
            "5Y swap pillar time should be > 4.0, got {}",
            prepared.pillar_time
        );
        assert!(
            prepared.pillar_date > as_of,
            "swap pillar date should be after as_of"
        );
    }
}
