//! Prepared quote envelopes for calibration pipelines.

use crate::instruments::Instrument;
use finstack_core::dates::Date;
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
/// ```rust
/// # use finstack_valuations::market::build::prepared::PreparedQuote;
/// # use finstack_valuations::market::quotes::rates::RateQuote;
/// # use finstack_valuations::market::quotes::ids::QuoteId;
/// # use finstack_core::dates::Date;
/// # use std::sync::Arc;
/// # use crate::instruments::Instrument;
/// #
/// # fn example() -> finstack_core::Result<()> {
/// // In practice, this would be created by a builder function
/// // let prepared = prepare_quote(quote, ctx)?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct PreparedQuote<Q> {
    /// The original market quote.
    ///
    /// Stored as `Arc` to allow sharing across multiple solver iterations without cloning.
    pub quote: Arc<Q>,
    /// The constructed instrument, fully configured for pricing.
    ///
    /// The instrument is ready to be priced and includes all necessary curve references,
    /// dates, and market conventions resolved from the quote.
    pub instrument: Arc<dyn Instrument>,
    /// The maturity date of the pillar (used for sorting / time axis).
    ///
    /// This is the resolved maturity date from the quote's pillar (either from a tenor
    /// calculation or a direct date specification).
    pub pillar_date: Date,
    /// The time-to-maturity of the pillar (in years), precomputed for the solver.
    ///
    /// This value is calculated once during construction and reused by calibration solvers
    /// for sorting, grouping, and time-axis calculations.
    pub pillar_time: f64,
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
    /// ```rust
    /// # use finstack_valuations::market::build::prepared::PreparedQuote;
    /// # use finstack_core::dates::Date;
    /// # use std::sync::Arc;
    /// # use crate::instruments::Instrument;
    /// #
    /// # fn example(quote: Arc<String>, instrument: Arc<dyn Instrument>) -> finstack_core::Result<()> {
    /// let pillar_date = Date::from_calendar_date(2025, time::Month::January, 2).unwrap();
    /// let as_of = Date::from_calendar_date(2024, time::Month::January, 2).unwrap();
    /// let pillar_time = (pillar_date - as_of).whole_days() as f64 / 365.25;
    ///
    /// let prepared = PreparedQuote::new(quote, instrument, pillar_date, pillar_time);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        quote: Arc<Q>,
        instrument: Arc<dyn Instrument>,
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
