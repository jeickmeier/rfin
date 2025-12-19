use crate::instruments::Instrument;
use finstack_core::dates::Date;
use std::fmt;
use std::sync::Arc;

/// A quote accompanied by its constructed instrument and precomputed time pillar.
///
/// This structure is the primary input for calibration solvers. It decouples the
/// solver from the details of quote parsing, convention resolution, and instrument construction.
///
/// Note: This is ephemeral and valid only for the `as_of` date used during construction.
#[derive(Clone)]
pub struct PreparedQuote<Q> {
    /// The original market quote.
    pub quote: Arc<Q>,
    /// The constructed instrument, fully configured for pricing.
    pub instrument: Arc<dyn Instrument>,
    /// The maturity date of the pillar (used for sorting / time axis).
    pub pillar_date: Date,
    /// The time-to-maturity of the pillar (in years), precomputed for the solver.
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
