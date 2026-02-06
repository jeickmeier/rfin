//! XVA (Valuation Adjustments) framework.
//!
//! Implements credit valuation adjustment (CVA) and related metrics
//! for OTC derivative portfolios. XVA adjustments capture the cost
//! of counterparty credit risk, funding, and capital for uncollateralized
//! or partially collateralized derivative positions.
//!
//! # Overview
//!
//! The XVA framework provides:
//!
//! - **CVA** (Credit Valuation Adjustment): Expected loss from counterparty default
//! - **Exposure simulation**: Deterministic exposure profiles (EPE, ENE, PFE)
//! - **Netting**: Close-out netting under ISDA master agreements
//! - **Collateral**: CSA collateral reduction of credit exposure
//!
//! # Architecture
//!
//! ```text
//! Instruments ──> Exposure Engine ──> Exposure Profile
//!                      │                     │
//!                      ├─ Market Roll        │
//!                      ├─ Netting            │
//!                      └─ CSA Collateral     │
//!                                            ▼
//!                               CVA Calculator
//!                                    │
//!                                    ├─ Hazard Curve (PD)
//!                                    ├─ Discount Curve (DF)
//!                                    └─ Recovery Rate (LGD)
//!                                    │
//!                                    ▼
//!                               XvaResult
//! ```
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use finstack_valuations::xva::{
//!     types::{XvaConfig, NettingSet},
//!     exposure::compute_exposure_profile,
//!     cva::compute_cva,
//! };
//! use std::sync::Arc;
//! # use finstack_core::market_data::context::MarketContext;
//!
//! # fn example() -> finstack_core::Result<()> {
//! // 1. Configure XVA parameters
//! let config = XvaConfig::default();
//!
//! // 2. Define the netting set
//! let netting_set = NettingSet {
//!     id: "NS-001".into(),
//!     counterparty_id: "COUNTERPARTY-CREDIT".into(),
//!     csa: None, // uncollateralized
//! };
//!
//! // 3. Compute exposure profile
//! // let profile = compute_exposure_profile(&instruments, &market, as_of, &config, &netting_set)?;
//!
//! // 4. Compute CVA
//! // let result = compute_cva(&profile, &hazard_curve, &discount_curve, config.recovery_rate)?;
//! // println!("CVA = {:.2}", result.cva);
//! # Ok(())
//! # }
//! ```
//!
//! # Regulatory Context
//!
//! | Standard | Metric | Implementation Status |
//! |----------|--------|----------------------|
//! | Basel III SA-CCR | Effective EPE | ✅ Computed |
//! | IFRS 13 / ASC 820 | Fair value CVA | ✅ Unilateral CVA |
//! | Basel III CVA risk | CVA capital | ❌ Future work |
//! | SA-CVA / BA-CVA | Standardized CVA | ❌ Future work |
//!
//! # Future Extensions
//!
//! - **DVA** (Debit Valuation Adjustment): Own-default benefit
//! - **FVA** (Funding Valuation Adjustment): Funding cost/benefit
//! - **KVA** (Capital Valuation Adjustment): Cost of regulatory capital
//! - **MVA** (Margin Valuation Adjustment): Cost of initial margin
//! - **Monte Carlo exposure**: Stochastic risk factor simulation
//! - **Wrong-way risk**: Exposure–default correlation modeling
//!
//! # References
//!
//! - Gregory, J. (2020). *The xVA Challenge*, 4th ed. Wiley.
//! - Green, A. (2015). *XVA: Credit, Funding and Capital Valuation Adjustments*. Wiley.
//! - Brigo, D., Morini, M. & Pallavicini, A. (2013). *Counterparty Credit Risk,
//!   Collateral and Funding*. Wiley.
//! - Pykhtin, M. & Zhu, S. (2007). "A Guide to Modelling Counterparty Credit Risk."
//!   GARP Risk Review.

pub mod cva;
pub mod exposure;
pub mod netting;
pub mod types;
