//! Revolving credit facility instruments with stochastic utilization.
//!
//! Models corporate revolving credit facilities (revolvers) with:
//! - Draw and repayment schedules (deterministic or stochastic)
//! - Complex fee structures (commitment, usage, facility, upfront)
//! - Floating rate bases with spreads
//! - Utilization limits and covenants
//!
//! # Facility Structure
//!
//! - **Commitment**: Maximum drawable amount
//! - **Utilization**: Amount currently drawn
//! - **Availability**: Commitment - Utilization
//! - **Pricing**: Interest on drawn amounts + fees on commitment
//!
//! # Fee Structure
//!
//! Typical fees:
//! - **Upfront fee**: Paid at facility origination (% of commitment)
//! - **Commitment fee**: Paid on undrawn amount (bps per annum)
//! - **Usage fee**: Additional spread when utilization > threshold
//! - **Facility fee**: Paid on total commitment regardless of usage
//!
//! # Pricing
//!
//! Present value combines interest and fees:
//!
//! ```text
//! PV = PV(interest on drawn) + PV(commitment fees) + PV(usage fees) - Upfront
//! ```
//!
//! For stochastic utilization, Monte Carlo simulates draw/repayment paths.
//!
//! # Utilization Modeling
//!
//! - **Deterministic**: Fixed draw/repayment schedule
//! - **Stochastic**: Monte Carlo with mean reversion to target utilization
//! - **Seasonal**: Cyclical patterns (e.g., retail seasonal borrowing)
//!
//! # Key Metrics
//!
//! - **Facility value**: PV of all cashflows
//! - **Utilization rate**: Drawn / Commitment
//! - **DV01**: Interest rate sensitivity
//! - **CS01**: Credit spread sensitivity
//!
//! # See Also
//!
//! - [`RevolvingCredit`] for instrument struct
//! - [`DrawRepayEvent`] for utilization events
//! - [`RevolvingCreditFees`] for fee specifications
//! - [`UtilizationProcess`] for stochastic modeling

pub mod cashflows;
pub mod metrics;
pub mod pricer;
pub mod types;

mod utils;

// Re-export main types
pub use types::{
    BaseRateSpec, DrawRepayEvent, DrawRepaySpec, RevolvingCredit, RevolvingCreditFees,
    StochasticUtilizationSpec, UtilizationProcess,
};
