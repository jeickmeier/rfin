//! XVA (Valuation Adjustments) framework.
//!
//! Implements credit, debit, and funding valuation adjustments (CVA, DVA, FVA)
//! and related metrics for OTC derivative portfolios. XVA adjustments capture
//! the cost of counterparty credit risk, own-default benefit, funding, and
//! capital for uncollateralized or partially collateralized derivative positions.
//!
//! # Overview
//!
//! The XVA framework provides:
//!
//! - **CVA** (Credit Valuation Adjustment): Expected loss from counterparty default
//! - **DVA** (Debit Valuation Adjustment): Expected gain from own default
//! - **FVA** (Funding Valuation Adjustment): Cost/benefit of funding uncollateralized exposure
//! - **Bilateral XVA**: Combined CVA - DVA + FVA adjustment
//! - **Exposure simulation**: Deterministic exposure profiles (EPE, ENE, PFE)
//! - **Stochastic exposure simulation**: Monte Carlo pathwise exposure with quantile-based PFE
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
//!                               XVA Calculators
//!                                    │
//!                                    ├─ CVA (counterparty hazard + EPE)
//!                                    ├─ DVA (own hazard + ENE)
//!                                    ├─ FVA (funding spread + EPE/ENE)
//!                                    ├─ Discount Curve (DF)
//!                                    └─ Recovery Rates (LGD)
//!                                    │
//!                                    ▼
//!                               XvaResult
//! ```
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use finstack_valuations::xva::{
//!     types::{XvaConfig, NettingSet, FundingConfig},
//!     exposure::compute_exposure_profile,
//!     cva::{compute_cva, compute_dva, compute_fva, compute_bilateral_xva},
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
//! // Optional: under the `mc` feature, compute a stochastic exposure distribution
//! // and use `stochastic.profile` for CVA/DVA integration while preserving
//! // `stochastic.pfe_profile` for limit and tail-risk reporting.
//!
//! // 4. Compute unilateral CVA
//! // let result = compute_cva(&profile, &hazard_curve, &discount_curve, config.recovery_rate)?;
//! // println!("CVA = {:.2}", result.cva);
//!
//! // 5. Compute bilateral XVA (CVA - DVA + FVA)
//! // let funding = FundingConfig { funding_spread_bps: 50.0, funding_benefit_bps: None };
//! // let bilateral = compute_bilateral_xva(
//! //     &profile, &cpty_hazard, &own_hazard, &discount,
//! //     0.40, 0.40, Some(&funding),
//! // )?;
//! // println!("Bilateral XVA = {:.2}", bilateral.bilateral_cva.unwrap());
//! # Ok(())
//! # }
//! ```
//!
//! # Regulatory Context
//!
//! | Standard | Metric | Implementation Status |
//! |----------|--------|----------------------|
//! | Basel III SA-CCR | Effective EPE | Computed |
//! | IFRS 13 / ASC 820 | Fair value CVA | Unilateral CVA |
//! | IFRS 13 / ASC 820 | Fair value DVA | DVA (own-default) |
//! | IFRS 13 / ASC 820 | Funding adjustment | FVA |
//! | Basel III CVA risk | CVA capital | Future work |
//! | SA-CVA / BA-CVA | Standardized CVA | Future work |
//!
//! # Future Extensions
//!
//! - **KVA** (Capital Valuation Adjustment): Cost of regulatory capital
//! - **MVA** (Margin Valuation Adjustment): Cost of initial margin
//! - **Wrong-way risk**: Exposure-default correlation modeling
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
