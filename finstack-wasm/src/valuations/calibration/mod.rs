//! Calibration bindings for WASM.

pub mod config;
pub mod engine;
pub mod quote;
pub mod report;
pub mod validation;

pub use config::{
    JsCalibrationConfig, JsCalibrationMethod, JsRateBounds, JsRateBoundsPolicy,
    JsResidualWeightingScheme, JsSolverKind, JsValidationMode,
};
pub use quote::{
    JsCDSTrancheQuote, JsCreditQuote, JsInflationQuote, JsMarketQuote, JsRatesQuote, JsVolQuote,
};
pub use report::JsCalibrationReport;
pub use validation::{
    validate_discount_curve, validate_forward_curve, validate_hazard_curve,
    validate_inflation_curve, validate_market_context, validate_vol_surface, JsValidationConfig,
};
