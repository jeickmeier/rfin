//! Core calibration traits.

use super::report::CalibrationReport;
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;

/// Core trait for calibrating market data structures from instrument quotes.
///
/// This trait provides a unified interface for all calibration processes,
/// whether they involve sequential bootstrapping or global optimization.
pub trait Calibrator<Input, Output> {
    /// Calibrate the target structure to match market quotes.
    ///
    /// # Arguments
    /// * `instruments` - Market instruments providing calibration constraints
    /// * `quotes` - Market quotes for the instruments  
    /// * `base_context` - Base market data (e.g., discount curves for credit calibration)
    ///
    /// # Returns
    /// Calibrated output structure and diagnostic report
    fn calibrate(
        &self,
        quotes: &[Input],
        base_context: &MarketContext,
    ) -> Result<(Output, CalibrationReport)>;
}
