//! xVA (Valuation Adjustments) framework for counterparty credit risk.
//!
//! Implements CVA (Credit Valuation Adjustment), DVA (Debit Valuation Adjustment),
//! and scaffolds for FVA (Funding Valuation Adjustment).
//!
//! # xVA Overview
//!
//! Post-crisis, derivative valuation includes multiple adjustments:
//!
//! - **CVA**: Cost of counterparty default risk
//! - **DVA**: Benefit of own default risk  
//! - **FVA**: Funding costs for uncollateralized exposure
//! - **KVA**: Capital costs (Basel III)
//! - **MVA**: Margin valuation adjustment (SIMM/SA-CCR)
//!
//! This module focuses on CVA/DVA with exposure calculation via Monte Carlo.
//!
//! # Key Concepts
//!
//! ## Exposure Profile
//!
//! - **EE(t)**: Expected Exposure at time t = E[max(V(t), 0)]
//! - **EPE**: Expected Positive Exposure = average of EE(t)
//! - **PFE(α, t)**: Potential Future Exposure at confidence α
//!
//! ## CVA Formula
//!
//! ```text
//! CVA = LGD * Σ EE(t_i) * PD(t_{i-1}, t_i) * DF(t_i)
//! ```
//!
//! where:
//! - LGD = Loss Given Default (1 - recovery rate)
//! - PD = Probability of Default
//! - DF = Discount Factor
//!
//! # Architecture
//!
//! ```text
//! MC Paths → Exposure Profile → CVA/DVA/FVA
//!    │            │                 │
//!    │            ├─ EE(t)          ├─ CVA
//!    │            ├─ ENE(t)         ├─ DVA
//!    │            └─ PFE(t, α)      └─ FVA
//! ```

pub mod collateral;
pub mod cva;
pub mod exposure;

pub use collateral::{apply_collateral, CollateralAgreement};
pub use cva::{calculate_cva, calculate_dva, CvaResult};
pub use exposure::{calculate_exposure_profile, ExposureProfile};
