//! Expected Credit Loss (ECL) framework for IFRS 9 and CECL.
//!
//! This module implements the complete ECL computation pipeline:
//!
//! - **Stage classification** ([`staging`]) -- IFRS 9 three-stage impairment model
//!   with quantitative (PD delta, DPD backstops) and qualitative triggers
//! - **ECL calculation** ([`engine`]) -- time-bucketed PD x LGD x EAD x DF
//!   integration with probability-weighted macro scenarios
//! - **CECL variant** ([`cecl`]) -- US GAAP ASC 326 lifetime ECL with
//!   reasonable-and-supportable forecast and historical reversion
//! - **Portfolio aggregation** ([`portfolio`]) -- stage breakdown, segment
//!   analysis, migration matrices, and provision waterfall
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use finstack_statements_analytics::analysis::ecl::*;
//!
//! // 1. Define a PD curve
//! let pd_curve = RawPdCurve::new("BBB", vec![
//!     (0.0, 0.0), (1.0, 0.02), (5.0, 0.10),
//! ])?;
//!
//! // 2. Build ECL configuration
//! let config = EclConfigBuilder::new()
//!     .bucket_width(0.25)
//!     .build()?;
//!
//! // 3. Create engine and process exposures
//! let scenario = &config.scenarios[0];
//! let engine = EclEngine::new(config, vec![(scenario, &pd_curve)]);
//! let result = engine.process_exposure(&exposure)?;
//! ```
//!
//! # References
//!
//! - IFRS 9 Financial Instruments, Section 5.5 -- Impairment
//! - ASC 326-20 -- Financial Instruments: Credit Losses
//! - Basel Committee on Banking Supervision (2015), "Guidance on credit risk
//!   and accounting for expected credit losses"

pub mod cecl;
pub mod engine;
pub mod portfolio;
pub mod staging;
pub mod types;

// Re-export core types for convenience.
pub use types::{Exposure, PdTermStructure, QualitativeFlags, RawPdCurve, Stage};

// Re-export staging API.
pub use staging::{classify_stage, StageResult, StagingConfig, StagingTrigger};

// Re-export ECL engine API.
pub use engine::{
    compute_ecl_single, compute_ecl_weighted, EclBucket, EclConfig, EclConfigBuilder, EclEngine,
    EclResult, ExposureEclResult, LgdType, MacroScenario, WeightedEclResult,
};

// Re-export CECL API.
pub use cecl::{CeclConfig, CeclEngine, CeclMethodology, CeclResult, ReversionMethod};

// Re-export portfolio aggregation.
pub use portfolio::{compute_waterfall, PortfolioEclResult, ProvisionWaterfall};
