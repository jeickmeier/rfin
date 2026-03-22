//! Credit analysis tools.
//!
//! - [`covenants`] — covenant forecasting bridge between statements and the covenant engine
//! - [`credit_context`] — coverage ratios (DSCR, interest coverage, LTV) from statement data

pub mod covenants;
pub mod credit_context;

pub use covenants::forecast_breaches;
pub use credit_context::{compute_credit_context, CreditContextMetrics};
