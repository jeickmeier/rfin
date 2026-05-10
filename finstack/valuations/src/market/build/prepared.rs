//! Prepared quote envelopes for calibration pipelines.

use crate::instruments::DynInstrument;
use crate::market::build::cds::{build_cds_instrument, resolve_cds_quote_dates};
use crate::market::build::helpers::{resolve_calendar, resolve_spot_date};
use crate::market::build::rates::build_rate_instrument;
use crate::market::conventions::registry::ConventionRegistry;
use crate::market::quotes::cds::CdsQuote;
use crate::market::quotes::ids::Pillar;
use crate::market::quotes::rates::RateQuote;
use crate::market::BuildCtx;
use finstack_core::dates::{adjust, Date, DateExt};
use finstack_core::dates::{DayCount, DayCountContext, TenorUnit};
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

/// Prepare a rate quote into an instrument + pillar time.
pub(crate) fn prepare_rate_quote(
    quote: RateQuote,
    build_ctx: &BuildCtx,
    curve_day_count: DayCount,
    base_date: Date,
    swap_use_payment_delay: bool,
) -> Result<PreparedQuote<RateQuote>> {
    let maturity_date = rate_quote_pillar_date(&quote, build_ctx, swap_use_payment_delay)?;
    let instrument = build_rate_instrument(&quote, build_ctx)?;
    let instrument: Arc<DynInstrument> = instrument.into();

    let pillar_time =
        curve_day_count.year_fraction(base_date, maturity_date, DayCountContext::default())?;

    Ok(PreparedQuote::new(
        Arc::new(quote),
        instrument,
        maturity_date,
        pillar_time,
    ))
}

fn rate_quote_pillar_date(
    quote: &RateQuote,
    build_ctx: &BuildCtx,
    swap_use_payment_delay: bool,
) -> Result<Date> {
    let registry = ConventionRegistry::try_global()?;
    match quote {
        RateQuote::Deposit { index, pillar, .. } => {
            let conv = registry.require_rate_index(index)?;
            let spot = resolve_spot_date(
                build_ctx.as_of(),
                &conv.market_calendar_id,
                conv.market_settlement_days,
                conv.market_business_day_convention,
            )?;
            let cal = resolve_calendar(&conv.market_calendar_id)?;
            match pillar {
                Pillar::Tenor(t) => {
                    t.add_to_date(spot, Some(cal), conv.market_business_day_convention)
                }
                Pillar::Date(d) => adjust(*d, conv.market_business_day_convention, cal),
            }
        }
        RateQuote::Fra {
            index, end: pillar, ..
        } => {
            let conv = registry.require_rate_index(index)?;
            let spot = resolve_spot_date(
                build_ctx.as_of(),
                &conv.market_calendar_id,
                conv.market_settlement_days,
                conv.market_business_day_convention,
            )?;
            let cal = resolve_calendar(&conv.market_calendar_id)?;
            match pillar {
                Pillar::Tenor(t) => {
                    t.add_to_date(spot, Some(cal), conv.market_business_day_convention)
                }
                Pillar::Date(d) => adjust(*d, conv.market_business_day_convention, cal),
            }
        }
        RateQuote::Futures {
            contract, expiry, ..
        } => {
            let fut_conv = registry.require_ir_future(contract)?;
            let idx_conv = registry.require_rate_index(&fut_conv.index_id)?;
            let cal = resolve_calendar(&fut_conv.calendar_id)?;
            let bdc = idx_conv.market_business_day_convention;
            let expiry_date = adjust(*expiry, bdc, cal)?;
            let period_start_unadj =
                expiry_date.add_business_days(fut_conv.settlement_days, cal)?;
            let period_start = adjust(period_start_unadj, bdc, cal)?;
            let delivery_tenor = finstack_core::dates::Tenor::new(
                fut_conv.delivery_months as u32,
                TenorUnit::Months,
            );
            delivery_tenor.add_to_date(period_start, Some(cal), bdc)
        }
        RateQuote::Swap { index, pillar, .. } => {
            let conv = registry.require_rate_index(index)?;
            let spot = resolve_spot_date(
                build_ctx.as_of(),
                &conv.market_calendar_id,
                conv.market_settlement_days,
                conv.market_business_day_convention,
            )?;
            let cal = resolve_calendar(&conv.market_calendar_id)?;
            let maturity = match pillar {
                Pillar::Tenor(t) => {
                    t.add_to_date(spot, Some(cal), conv.market_business_day_convention)?
                }
                Pillar::Date(d) => adjust(*d, conv.market_business_day_convention, cal)?,
            };
            if swap_use_payment_delay {
                crate::instruments::common_impl::pricing::swap_legs::add_payment_delay(
                    maturity,
                    conv.default_payment_lag_days,
                    Some(&conv.market_calendar_id),
                )
            } else {
                Ok(maturity)
            }
        }
    }
}

/// Prepare a CDS quote into an instrument + pillar time.
pub(crate) fn prepare_cds_quote(
    quote: CdsQuote,
    build_ctx: &BuildCtx,
    day_count: DayCount,
    base_date: Date,
) -> Result<PreparedQuote<CdsQuote>> {
    let maturity_date = resolve_cds_quote_dates(&quote, build_ctx)?.maturity;
    let instrument = build_cds_instrument(&quote, build_ctx)?;
    let instrument: Arc<DynInstrument> = instrument.into();

    let pillar_time =
        day_count.year_fraction(base_date, maturity_date, DayCountContext::default())?;

    Ok(PreparedQuote::new(
        Arc::new(quote),
        instrument,
        maturity_date,
        pillar_time,
    ))
}

#[cfg(test)]
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

        let prepared = prepare_rate_quote(quote, &ctx, DayCount::Act365F, as_of, true)
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

        let prepared = prepare_rate_quote(quote, &ctx, DayCount::Act365F, as_of, true)
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

    #[test]
    fn prepare_cds_quote_uses_resolved_imm_maturity_as_pillar() {
        let as_of = Date::from_calendar_date(2024, Month::January, 2).expect("valid date");
        let mut curve_ids = HashMap::default();
        curve_ids.insert("discount".to_string(), "USD-OIS".to_string());
        curve_ids.insert("credit".to_string(), "ABC-CORP".to_string());
        let ctx = BuildCtx::new(as_of, 10_000_000.0, curve_ids);

        let explicit_maturity =
            Date::from_calendar_date(2026, Month::June, 20).expect("valid maturity");
        let quote = CdsQuote::CdsParSpread {
            id: QuoteId::new("CDS-TEST-IMM"),
            entity: "Test Corp".to_string(),
            convention: crate::market::conventions::ids::CdsConventionKey {
                currency: finstack_core::currency::Currency::USD,
                doc_clause: crate::market::conventions::ids::CdsDocClause::IsdaNa,
            },
            pillar: Pillar::Date(explicit_maturity),
            spread_bp: 100.0,
            recovery_rate: 0.40,
        };

        let prepared =
            prepare_cds_quote(quote, &ctx, DayCount::Act365F, as_of).expect("prepared CDS quote");

        assert_eq!(prepared.pillar_date, explicit_maturity);
        assert!(prepared.pillar_time > 2.0);
    }
}
