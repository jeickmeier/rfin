//! Credit analysis tools.
//!
//! - [`covenants`] — covenant forecasting bridge between statements and the covenant engine
//! - [`credit_context`] — coverage ratios (DSCR, interest coverage, LTV) from statement data
//! - [`adjusted_net_debt`] — rating-agency Adjusted Net Debt bridge

pub mod adjusted_net_debt;
pub mod covenants;
pub mod credit_context;

pub use adjusted_net_debt::{AdjustedNetDebtSpec, AdjustedNetDebtSpecBuilder};
pub use covenants::forecast_breaches;
pub use credit_context::{compute_credit_context, CreditContextMetrics};
