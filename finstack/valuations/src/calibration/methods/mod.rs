//! Curve and surface calibration methods with market-standard methodologies.
//!
//! Provides calibrators for all major market data structures using industry-standard
//! algorithms. Each calibrator implements the [`Calibrator`](super::Calibrator) trait
//! with specialized logic for its target structure.
//!
//! # Calibration Methods
//!
//! ## Term Structure Bootstrapping
//!
//! Sequential bootstrapping solves for curve points iteratively, with each
//! point depending on previously solved points:
//!
//! - [`discount`]: OIS discount curves from deposits and OIS swaps
//! - [`forward_curve`]: Forward rate curves from FRAs, futures, and basis swaps
//! - [`hazard_curve`]: Credit hazard curves from CDS spreads (ISDA standard)
//! - [`inflation_curve`]: Real CPI curves from zero-coupon inflation swaps
//!
//! ## Surface Calibration
//!
//! Global optimization fitting all parameters simultaneously:
//!
//! - [`sabr_surface`]: SABR volatility surfaces from swaption quotes
//! - [`swaption_vol`]: Volatility interpolation helpers
//! - [`base_correlation`]: Credit correlation from CDO tranche quotes
//!
//! ## Convexity Adjustments
//!
//! - [`convexity`]: Convexity corrections for futures and CMS
//!
//! # Bootstrap Algorithm
//!
//! General bootstrap pattern for term structures:
//!
//! 1. **Sort quotes** by maturity
//! 2. **For each quote** (starting from shortest maturity):
//!    - Set up pricing function using current partial curve
//!    - Solve for discount factor that matches market quote
//!    - Add point to curve
//! 3. **Interpolate** between solved points
//! 4. **Validate** no arbitrage conditions
//!
//! # Multi-Curve Framework
//!
//! Post-2008 market convention separates:
//! - **Discount curves**: OIS curves for discounting (risk-free)
//! - **Forward curves**: IBOR/RFR curves for projecting floating rates
//!
//! Calibration order:
//! 1. Calibrate OIS discount curve first (from deposits + OIS swaps)
//! 2. Calibrate forward curves using discount curve (from FRAs + tenor swaps)
//!
//! This captures the **basis spread** between different rate indices.
//!
//! # References
//!
//! ## Interest Rate Curves
//!
//! - Hagan, P. S., & West, G. (2006). "Interpolation Methods for Curve
//!   Construction." *Applied Mathematical Finance*, 13(2), 89-129.
//!
//! - Ametrano, F. M., & Bianchetti, M. (2013). "Everything You Always Wanted
//!   to Know About Multiple Interest Rate Curve Bootstrapping but Were Afraid
//!   to Ask." *SSRN Working Paper*.
//!
//! ## Credit Curves
//!
//! - O'Kane, D., & Turnbull, S. (2003). "Valuation of Credit Default Swaps."
//!   Lehman Brothers Fixed Income Quantitative Credit Research.
//!
//! - ISDA (2009). "ISDA CDS Standard Model." Version 1.8.2.
//!
//! ## Volatility Surfaces
//!
//! - Hagan, P. S., Kumar, D., Lesniewski, A. S., & Woodward, D. E. (2002).
//!   "Managing Smile Risk." *Wilmott Magazine*, September, 84-108.
//!   (SABR model)
//!
//! # See Also
//!
//! - [`discount`] for OIS curve calibration
//! - [`forward_curve`] for forward rate curve calibration
//! - [`hazard_curve`] for CDS curve calibration
//! - [`sabr_surface`] for volatility surface calibration

pub mod base_correlation;
pub mod convexity;
pub mod discount;
pub mod forward_curve;
pub mod hazard_curve;
pub mod hull_white;
pub mod inflation_curve;
pub mod pricing;
pub mod sabr_surface;
pub mod swaption_market_conventions;
pub mod swaption_vol;
pub mod xccy;

pub use base_correlation::BaseCorrelationCalibrator;
pub use discount::DiscountCurveCalibrator;
pub use forward_curve::ForwardCurveCalibrator;
pub use hazard_curve::HazardCurveCalibrator;
pub use hull_white::{
    Bounds as HullWhiteBounds, HullWhiteCalibrationConfig, HullWhiteCalibrationConfigV1,
    HullWhiteCalibrationResult, HullWhiteCalibrationTargets, HullWhiteCalibrator, WeightFunction,
    HULL_WHITE_CALIBRATION_CONFIG_KEY_V1,
};
pub use inflation_curve::InflationCurveCalibrator;
pub use pricing::create_ois_swap_from_quote;
pub use pricing::CalibrationPricer;
pub use sabr_surface::SurfaceInterp;
pub use sabr_surface::VolSurfaceCalibrator;
pub use swaption_vol::SwaptionVolCalibrator;
pub use xccy::{SpreadOn as XccySpreadOn, XccyBasisCalibrator, XccyBasisQuote};
