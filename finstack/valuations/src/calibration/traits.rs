//! Core calibration traits and abstractions.
//!
//! Defines the fundamental [`Calibrator`] trait that all calibration processes
//! implement, enabling consistent interfaces across discount curves, forward curves,
//! hazard curves, volatility surfaces, and correlation structures.
//!
//! # Key Types
//!
//! - [`Calibrator`]: Generic trait for calibrating market structures from quotes
//!
//! # Examples
//!
//! ```rust
//! use finstack_valuations::calibration::{Calibrator, MarketQuote, CalibrationReport};
//! use finstack_core::market_data::MarketContext;
//! use finstack_core::market_data::term_structures::DiscountCurve;
//! use finstack_core::Result;
//!
//! // Custom calibrator implementing the Calibrator trait
//! struct MyCalibrator;
//!
//! impl Calibrator<MarketQuote, DiscountCurve> for MyCalibrator {
//!     fn calibrate(
//!         &self,
//!         quotes: &[MarketQuote],
//!         base_context: &MarketContext,
//!     ) -> Result<(DiscountCurve, CalibrationReport)> {
//!         // Implementation would build curve from quotes
//!         # unimplemented!()
//!     }
//! }
//! ```

use super::report::CalibrationReport;
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;

/// Core trait for calibrating market data structures from instrument quotes.
///
/// This trait provides a unified interface for all calibration processes,
/// whether they involve sequential bootstrapping (e.g., discount curves) or
/// global optimization (e.g., volatility surfaces). Implementors take market
/// quotes and return calibrated structures with diagnostic reports.
///
/// # Type Parameters
///
/// * `Input` - Quote type (e.g., `MarketQuote`, `RatesQuote`, `VolQuote`)
/// * `Output` - Calibrated structure (e.g., `DiscountCurve`, `SABRSurface`)
///
/// # Calibration Approaches
///
/// ## Sequential Bootstrapping
///
/// Builds structures iteratively by solving one point at a time:
/// - **Discount curves**: Bootstrap from deposits and swaps
/// - **Hazard curves**: Bootstrap from CDS spreads
/// - **Forward curves**: Bootstrap from FRAs and futures
///
/// Each point is solved to match the corresponding market quote, with
/// later points depending on earlier ones.
///
/// ## Global Optimization
///
/// Fits all parameters simultaneously to minimize pricing errors:
/// - **SABR surfaces**: Calibrate α, β, ρ, ν per expiry
/// - **Base correlation**: Fit correlation skew to tranche quotes
/// - **Stochastic models**: Calibrate model parameters to option prices
///
/// # Implementation Guidelines
///
/// Calibrators should:
/// - Validate input quotes (ordering, no gaps, reasonable values)
/// - Use `base_context` for any prerequisite market data
/// - Return both the calibrated structure and a diagnostic report
/// - Populate report with convergence metrics and residuals
/// - Handle edge cases (insufficient quotes, solver failures)
///
/// # Examples
///
/// ## Implementing a Custom Calibrator
///
/// ```rust
/// use finstack_valuations::calibration::{Calibrator, CalibrationReport, MarketQuote};
/// use finstack_core::market_data::MarketContext;
/// use finstack_core::market_data::term_structures::DiscountCurve;
/// use finstack_core::Result;
///
/// struct SimpleDiscountCalibrator {
///     curve_id: String,
/// }
///
/// impl Calibrator<MarketQuote, DiscountCurve> for SimpleDiscountCalibrator {
///     fn calibrate(
///         &self,
///         quotes: &[MarketQuote],
///         _base_context: &MarketContext,
///     ) -> Result<(DiscountCurve, CalibrationReport)> {
///         // 1. Validate quotes
///         // 2. Bootstrap discount factors
///         // 3. Build curve
///         // 4. Generate report
///         # unimplemented!("Example only")
///     }
/// }
/// ```
///
/// ## Using a Calibrator
///
/// ```rust
/// use finstack_valuations::calibration::{
///     Calibrator, MarketQuote, RatesQuote, CalibrationConfig
/// };
/// # use finstack_core::market_data::MarketContext;
/// # use finstack_valuations::calibration::methods::DiscountCurveCalibrator;
/// # use finstack_core::currency::Currency;
/// # use finstack_core::dates::create_date;
/// # use time::Month;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let base_date = create_date(2025, Month::January, 15)?;
/// let calibrator = DiscountCurveCalibrator::new(
///     "USD-OIS",
///     base_date,
///     Currency::USD,
/// );
///
/// // Example quotes (fields depend on actual RatesQuote structure)
/// // let quotes = vec![...];
///
/// // let market_context = MarketContext::new();
/// // let (curve, report) = calibrator.calibrate(&quotes, &market_context)?;
///
/// // Check calibration quality
/// // assert!(report.success);
/// // assert!(report.max_error < 1e-6);
/// # Ok(())
/// # }
/// ```
///
/// # See Also
///
/// - [`CalibrationReport`] for diagnostic output structure
/// - [`CalibrationConfig`](super::CalibrationConfig) for solver settings
/// - [`SimpleCalibration`](super::SimpleCalibration) for end-to-end calibration
pub trait Calibrator<Input, Output> {
    /// Calibrate the target structure to match market quotes.
    ///
    /// Takes a collection of market quotes and produces a calibrated structure
    /// (curve, surface, etc.) that prices those instruments at the quoted levels.
    /// Returns both the calibrated structure and a diagnostic report.
    ///
    /// # Arguments
    ///
    /// * `quotes` - Market quotes providing calibration constraints. Quotes should
    ///   be ordered by maturity/tenor and cover the desired calibration range.
    /// * `base_context` - Base market data required for calibration. For example:
    ///   - Discount curves when calibrating forward curves
    ///   - Discount curves when calibrating hazard curves
    ///   - Empty context when calibrating base discount curves
    ///
    /// # Returns
    ///
    /// Tuple of:
    /// - Calibrated output structure matching the market quotes
    /// - `CalibrationReport` with convergence metrics and residuals
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Insufficient quotes for calibration
    /// - Quotes are invalid or inconsistent
    /// - Required market data missing from `base_context`
    /// - Solver fails to converge
    /// - Numerical instability encountered
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::calibration::{
    ///     Calibrator, MarketQuote, RatesQuote
    /// };
    /// # use finstack_core::market_data::MarketContext;
    /// # use finstack_valuations::calibration::methods::DiscountCurveCalibrator;
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::dates::create_date;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let base_date = create_date(2025, Month::January, 15)?;
    /// let calibrator = DiscountCurveCalibrator::new(
    ///     "USD-OIS",
    ///     base_date,
    ///     Currency::USD,
    /// );
    ///
    /// // Example quotes (fields depend on actual RatesQuote structure)
    /// // let quotes = vec![...];
    ///
    /// // let market_context = MarketContext::new();
    /// // let (discount_curve, report) = calibrator.calibrate(&quotes, &market_context)?;
    ///
    /// // Inspect calibration quality
    /// // println!("Calibration success: {}", report.success);
    /// // println!("Max pricing error: {:.2e}", report.max_error);
    /// // println!("Iterations: {}", report.iterations);
    /// # Ok(())
    /// # }
    /// ```
    fn calibrate(
        &self,
        quotes: &[Input],
        base_context: &MarketContext,
    ) -> Result<(Output, CalibrationReport)>;
}
