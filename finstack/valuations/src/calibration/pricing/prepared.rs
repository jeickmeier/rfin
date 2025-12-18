//! Prepared (pre-built) pricing objects for calibration hot paths.
//!
//! Calibration solvers can call the residual function thousands of times per step.
//! Building pricing instruments inside that residual function is extremely expensive
//! (heap allocations, string formatting, dynamic dispatch setup).
//!
//! The types in this module pre-build instruments once per quote so the solver loop
//! only performs:
//! `build curve -> update MarketContext -> instrument.value()`.

use crate::calibration::api::schema::HazardCurveParams;
use crate::calibration::pricing::quote_factory;
use crate::calibration::quotes::{CreditQuote, RatesQuote};
use crate::instruments::cds::CdsConventionResolved;
use crate::instruments::common::traits::Instrument;
use finstack_core::money::Money;
use finstack_core::types::Currency;
use finstack_core::Result;
use std::fmt;
use std::sync::Arc;

/// A rates quote paired with its pre-built pricing instrument.
///
/// Both the quote and instrument are stored behind `Arc` so `Clone` is cheap and
/// global-solvers can freely `to_vec()` without duplicating payloads.
#[derive(Clone)]
pub struct PreparedRatesQuote {
    /// Original quote payload.
    pub quote: Arc<RatesQuote>,
    /// Pre-built instrument for pricing against candidate curves.
    pub instrument: Arc<dyn Instrument>,
}

impl PreparedRatesQuote {
    /// Build a prepared rates quote by constructing the pricing instrument once.
    pub fn new(
        pricer: &crate::calibration::pricing::CalibrationPricer,
        quote: RatesQuote,
        currency: Currency,
        strict: bool,
    ) -> Result<Self> {
        let quote = Arc::new(quote);
        let instrument =
            quote_factory::build_instrument_for_rates_quote(pricer, quote.as_ref(), currency, strict)?;
        Ok(Self { quote, instrument })
    }
}

impl fmt::Debug for PreparedRatesQuote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedRatesQuote")
            .field("quote", &self.quote)
            .finish_non_exhaustive()
    }
}

/// A credit quote paired with its pre-built CDS pricing instrument and optional upfront cash.
#[derive(Clone)]
pub struct PreparedCreditQuote {
    /// Original quote payload.
    pub quote: Arc<CreditQuote>,
    /// Pre-built CDS instrument (prices against a candidate hazard curve in context).
    pub instrument: Arc<dyn Instrument>,
    /// Optional upfront cash (for upfront quotes) to subtract from PV.
    pub upfront_opt: Option<Money>,
}

impl PreparedCreditQuote {
    /// Build a prepared credit quote (CDS/CDSUpfront) once.
    pub(crate) fn new(
        quote: CreditQuote,
        params: &HazardCurveParams,
        cds_conventions: &CdsConventionResolved,
    ) -> Result<Self> {
        let quote = Arc::new(quote);
        let (instrument, upfront_opt) =
            quote_factory::build_instrument_for_credit_quote(quote.as_ref(), params, cds_conventions)?;
        Ok(Self {
            quote,
            instrument,
            upfront_opt,
        })
    }
}

impl fmt::Debug for PreparedCreditQuote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedCreditQuote")
            .field("quote", &self.quote)
            .field("upfront_opt", &self.upfront_opt)
            .finish_non_exhaustive()
    }
}


