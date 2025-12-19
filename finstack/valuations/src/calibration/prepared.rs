//! Prepared quotes for calibration.
//!
//! Bridges the raw market data schema with the instrument-based calibration solvers.

use crate::instruments::common::traits::Instrument;
use crate::market::build::prepared::PreparedQuote;
use crate::market::quotes::cds::CdsQuote;
use crate::market::quotes::cds_tranche::CdsTrancheQuote;
use crate::market::quotes::inflation::InflationQuote;
use crate::market::quotes::rates::RateQuote;
use finstack_core::money::Money;

/// A prepared quote ready for use in calibration.
///
/// This wraps a Quote (data), a prepared Instrument (constructed via builder),
/// and the pre-calculated pillar time.
#[derive(Debug, Clone)]
pub enum CalibrationQuote {
    /// Rates quote (Deposit, FRA, Swap, Future)
    Rates(PreparedQuote<RateQuote>),
    /// Credit quote (CDS Par Spread, Upfront). Includes optional upfront cashflow.
    Cds(PreparedQuote<CdsQuote>, Option<Money>),
    /// CDS Tranche quote.
    CdsTranche(PreparedQuote<CdsTrancheQuote>, Option<Money>),
    /// Inflation quote (ZCIS)
    Inflation(PreparedQuote<InflationQuote>),
    // Add Vol later
}

impl CalibrationQuote {
    /// Get reference to the underlying instrument
    pub fn instrument(&self) -> &dyn Instrument {
        match self {
            CalibrationQuote::Rates(q) => q.instrument.as_ref(),
            CalibrationQuote::Cds(q, _) => q.instrument.as_ref(),
            CalibrationQuote::CdsTranche(q, _) => q.instrument.as_ref(),
            CalibrationQuote::Inflation(q) => q.instrument.as_ref(),
        }
    }

    /// Get the pillar time (year fraction from as_of)
    pub fn pillar_time(&self) -> f64 {
        match self {
            CalibrationQuote::Rates(q) => q.pillar_time,
            CalibrationQuote::Cds(q, _) => q.pillar_time,
            CalibrationQuote::CdsTranche(q, _) => q.pillar_time,
            CalibrationQuote::Inflation(q) => q.pillar_time,
        }
    }
}
