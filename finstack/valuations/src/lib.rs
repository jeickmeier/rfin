#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::new_without_default)]
#![warn(clippy::float_cmp)]
#![cfg_attr(test, allow(clippy::float_cmp))]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
// Allow expect() in doc tests (they are test code)
#![doc(test(attr(allow(clippy::expect_used))))]

//! Comprehensive financial instrument pricing, risk, and cashflow analysis.
//!
//! This crate provides a deterministic, production-ready valuation engine for fixed income,
//! equity, credit, and derivative instruments. Built on accounting-grade numerics
//! (Decimal by default), currency safety, and stable wire formats.
//!
//! # Features
//!
//! - **Instrument pricing**: NPV, yields, spreads across 40+ instrument types
//! - **Risk metrics**: DV01, CS01, Greeks, bucketed sensitivities, time decay
//! - **Cashflow generation**: Schedule building with amortization, floating rates, and caps
//! - **Calibration**: Bootstrap and optimize curves (discount, forward, hazard, volatility)
//! - **Monte Carlo**: Path generation, variance reduction, LSM for early exercise
//! - **Analytical formulas**: Black-Scholes, SABR, barrier options, Asian options
//! - **Registry-based pricing**: Type-safe dispatch without macro complexity
//! - **Metrics framework**: Composable calculators with dependency resolution
//!
//! # Architecture
//!
//! ```text
//! Instruments ──> Pricer Registry ──> Pricing Models
//!      │               │                     │
//!      │               ├─ Discounting        ├─ Analytical (Black-Scholes, SABR)
//!      │               ├─ Tree-based         ├─ Monte Carlo (GBM, Heston)
//!      │               ├─ Monte Carlo        └─ Specialized (CDS, Convertibles)
//!      │               └─ Custom
//!      │
//!      └──> Metrics Registry ──> Risk Calculators
//!                  │                   │
//!                  │                   ├─ Greeks (delta, gamma, vega, theta, rho)
//!                  │                   ├─ DV01/CS01 (bucketed and total)
//!                  │                   ├─ Spreads (Z-spread, OAS, ASW)
//!                  │                   └─ Custom metrics
//!                  │
//!                  └──> ValuationResult (PV + Metrics + Metadata)
//! ```
//!
//! # Quick Start
//!
//! ## Basic Bond Pricing
//!
//! ```rust
//! use finstack_valuations::instruments::Bond;
//! use finstack_valuations::pricer::{create_standard_registry, ModelKey};
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! use finstack_core::dates::create_date;
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::types::Rate;
//! use time::Month;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create pricing registry
//! let registry = create_standard_registry();
//!
//! // Build a fixed-rate bond
//! let issue = create_date(2025, Month::January, 15)?;
//! let maturity = create_date(2030, Month::January, 15)?;
//! let bond = Bond::fixed(
//!     "US-BOND-001",
//!     Money::new(1_000_000.0, Currency::USD),
//!     Rate::from_percent(5.0),
//!     issue,
//!     maturity,
//!     "USD-OIS"       // Discount curve ID
//! );
//!
//! // Create market context with curves
//! // Note: Market context requires calibrated discount curves.
//! // In practice, populate with discount curves via calibration module.
//! // let market = MarketContext::new();
//! // let as_of = create_date(2025, Month::January, 1)?;
//!
//! // Price the bond (requires populated market context)
//! // let result = registry.price_with_registry(&bond, ModelKey::Discounting, &market, as_of, None)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Risk Metrics
//!
//! ```rust
//! use finstack_valuations::instruments::Bond;
//! use finstack_valuations::metrics::{MetricId, standard_registry};
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::types::Rate;
//! # use finstack_core::currency::Currency;
//! # use finstack_core::money::Money;
//! # use finstack_core::dates::create_date;
//! # use time::Month;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let issue = create_date(2025, Month::January, 15)?;
//! # let maturity = create_date(2030, Month::January, 15)?;
//! # let bond = Bond::fixed("US-BOND-001", Money::new(1_000_000.0, Currency::USD),
//! #     Rate::from_percent(5.0), issue, maturity, "USD-OIS");
//! # let market = MarketContext::new();
//! # let as_of = create_date(2025, Month::January, 1)?;
//!
//! // Compute risk metrics
//! use finstack_valuations::instruments::Instrument;
//! let metrics_to_compute = vec![
//!     MetricId::Ytm,
//!     MetricId::DurationMod,  // Modified duration
//!     MetricId::Convexity,
//!     MetricId::Dv01,
//! ];
//!
//! // Note: Requires populated market context with "USD-OIS" discount curve
//! // let result = bond.price_with_metrics(&market, as_of, &metrics_to_compute)?;
//! // println!("YTM: {:.2}%", result.measures.get("ytm").expect("should succeed") * 100.0);
//! // println!("DV01: ${:.2}", result.measures.get("dv01").expect("should succeed"));
//! # Ok(())
//! # }
//! ```
//!
//! ## Calibration
//!
//! ```rust,no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use finstack_valuations::calibration::api::engine;
//! use finstack_valuations::calibration::api::schema::{
//!     CalibrationEnvelope, CalibrationPlan, CALIBRATION_SCHEMA,
//! };
//!
//! // Build a plan-driven v2 envelope and execute it.
//! // (See `calibration::api::schema` for the full contract.)
//! let envelope = CalibrationEnvelope {
//!     schema: CALIBRATION_SCHEMA.to_string(),
//!     plan: CalibrationPlan {
//!         id: "plan".to_string(),
//!         description: None,
//!         quote_sets: Default::default(),
//!         steps: vec![],
//!         settings: Default::default(),
//!     },
//!     initial_market: None,
//! };
//! let _result = engine::execute(&envelope)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Module Organization
//!
//! - [`calibration`]: Curve and surface calibration from market quotes
//! - [`cashflow`]: Cashflow schedule generation and aggregation
//! - [`instruments`]: Financial instrument definitions (bonds, swaps, options, etc.)
//! - [`metrics`]: Risk metric calculators and registry
//! - [`pricer`]: Pricing dispatch and registry infrastructure
//! - [`results`]: Valuation result envelopes and metadata
//! - [`constants`]: Common numerical constants (basis points, etc.)
//! - [`covenants`]: Covenant checking for structured products
//! - [`schema`]: JSON Schema generation for API contracts
//!
//! # API Layers
//!
//! The public API is organized into three layers:
//!
//! ## Layer 1: Core API (Most Common)
//! - [`instruments`]: Financial instrument types (bonds, swaps, options, etc.)
//! - [`pricer`]: Pricing registry and dispatch ([`pricer::PricerRegistry`], [`pricer::create_standard_registry`])
//! - [`metrics`]: Risk metric calculation ([`metrics::MetricId`], [`metrics::standard_registry`])
//! - [`results`]: Valuation result envelopes ([`results::ValuationResult`])
//! - [`calibration::api`]: Calibration schema and execution engine
//! - [`prelude`]: Convenient re-exports of commonly used types
//!
//! ## Layer 2: Extended API (Less Common)
//! - [`margin`]: Margin calculations (VM/IM/CSA) for collateralized derivatives
//! - [`attribution`]: P&L attribution analysis
//! - [`covenants`]: Covenant checking for structured products
//! - [`cashflow`]: Advanced cashflow schedule builders
//! - [`instruments::common`]: Shared traits, parameters, schedules, models, and MC primitives
//! - [`market`]: Market quote schemas and conventions
//! - [`calibration::bumps`]: Shared re-calibration helpers for scenarios
//!
//! ## Layer 3: Internal API (Use with Caution)
//! - Individual pricer implementations (use via [`pricer::PricerRegistry`] instead)
//! - Calibration solvers (use via [`calibration::api`] instead)
//! - Low-level market data helpers
//!
//! For most users, Layer 1 + `prelude` imports are sufficient.
//! Import with `use finstack_valuations::prelude::*;` to get started quickly.
//!
//! # Supported Instruments
//!
//! ## Fixed Income
//! - `Bond`: Fixed and floating-rate bonds, callable/putable, amortizing
//! - `InterestRateSwap`: Plain vanilla and basis swaps
//! - `Swaption`: European and Bermudan swaptions
//! - `InterestRateOption`: Interest rate caps and floors
//! - `Deposit`: Money market deposits
//! - `ForwardRateAgreement`: FRAs
//! - `InterestRateFuture`: Futures contracts
//!
//! ## Credit
//! - `CreditDefaultSwap`: Single-name CDS
//! - `CDSIndex`: Credit indices (CDX, iTraxx)
//! - `CDSTranche`: Synthetic CDO tranches
//! - `CDSOption`: Options on CDS
//! - `StructuredCredit`: ABS, RMBS, CMBS, CLO
//!
//! ## Equity & FX
//! - `Equity`: Equity spot positions
//! - `EquityOption`: Vanilla equity options
//! - `FxSpot`: FX spot positions
//! - `FxOption`: Vanilla FX options (Garman-Kohlhagen)
//! - `FxSwap`: FX forwards and swaps
//! - `Basket`: Multi-asset baskets
//!
//! ## Exotic Options (requires `mc` feature)
//! - `AsianOption`: Asian (average price/strike) options
//! - `BarrierOption`: Barrier options (knock-in/out)
//! - `LookbackOption`: Lookback options
//! - `Autocallable`: Autocallable notes
//! - `CliquetOption`: Cliquet/ratchet options
//! - `QuantoOption`: Quanto options
//!
//! ## Structured Products
//! - `ConvertibleBond`: Convertible bonds
//! - `Repo`: Repurchase agreements
//! - `VarianceSwap`: Variance and volatility swaps
//! - `PrivateMarketsFund`: Private equity/credit funds
//! - `RevolvingCredit`: Revolving credit facilities
//!
//! # Pricing Models
//!
//! ## Analytical
//! - **Black-Scholes-Merton**: European options on equity and FX
//! - **Black (1976)**: Caps, floors, swaptions
//! - **Garman-Kohlhagen**: FX options
//! - **SABR**: Stochastic volatility surface interpolation
//! - **Barrier formulas**: Rubinstein-Reiner barrier options
//! - **Asian approx**: Turnbull-Wakeman and geometric averaging
//!
//! ## Tree Methods
//! - **Binomial trees**: Cox-Ross-Rubinstein, Jarrow-Rudd
//! - **Trinomial trees**: Short rate models, convertibles
//! - **Hull-White**: Interest rate trees for callable bonds
//!
//! ## Monte Carlo (requires `mc` feature)
//! - **Geometric Brownian Motion**: Standard equity/FX simulation
//! - **Heston**: Stochastic volatility with Andersen QE discretization
//! - **Longstaff-Schwartz**: American and Bermudan options via LSM
//! - **Variance reduction**: Antithetic variates, control variates
//!
//! # Determinism and Reproducibility
//!
//! All pricing and calibration is deterministic by default:
//! - Decimal arithmetic via [`rust_decimal`] ensures consistent results
//! - Monte Carlo uses seedable RNGs with stable algorithms
//! - Parallel execution produces identical results to serial
//! - Calibration solvers use deterministic iteration orders
//!
//! # Performance
//!
//! - **Vectorized execution**: Polars-based expression engine for time-series
//! - **Caching**: Intermediate results (curves, cashflows) cached per valuation
//! - **Parallelism**: Optional Rayon parallelism without changing results
//! - **Lazy evaluation**: Metrics computed only when requested
//!
//! # Error Handling
//!
//! All public APIs return `Result<T, finstack_core::Error>` with structured error types:
//! - `CurveNotFound`: Missing discount or forward curve
//! - `InvalidInstrument`: Inconsistent instrument parameters
//! - `CalibrationFailed`: Calibration did not converge
//! - `PricingError`: Pricing calculation failed
//!
//! # Feature Flags
//!
//! - `mc`: Enable Monte Carlo pricing (adds ~200KB to binary)
//! - `serde`: Enable serialization/deserialization
//! - `parallel`: Enable Rayon parallelism (deterministic results maintained)
//!
//! # See Also
//!
//! - `finstack_core`: Core primitives (Money, dates, curves, expressions)
//! - `finstack_statements`: Financial statement modeling
//! - `finstack_portfolio`: Multi-instrument portfolio aggregation
//! - `finstack_scenarios`: Scenario analysis and stress testing

extern crate self as finstack_valuations;

/// Curve and surface calibration tooling.
pub mod calibration;
/// Cashflow schedule generation and builders.
pub mod cashflow;
/// Shared numerical constants and basis point helpers.
pub mod constants;
/// Error types for pricing and valuation workflows.
pub mod error;
/// Margin calculation for collateralized derivatives.
///
/// Provides VM (Variation Margin) and IM (Initial Margin) calculations,
/// CSA (Credit Support Annex) modeling, and netting set aggregation.
pub mod margin;
/// Market quotes and conventions
pub mod market;
/// Convenient re-exports for pricing and risk calculations
/// Convenient re-exports for pricing and risk calculations.
pub mod prelude;
/// Pricing dispatch and registry infrastructure.
pub mod pricer;
/// Valuation result envelopes and metadata.
pub mod results;
/// JSON Schema generation for API contracts.
pub mod schema;

// Export macros before instruments module
#[macro_use]
/// Financial instrument definitions and builders.
pub mod instruments;
/// P&L attribution analysis utilities.
pub mod attribution;
/// Covenant checking for structured products.
pub mod covenants;
/// Risk metric calculators and registries.
pub mod metrics;
/// XVA (Valuation Adjustments) framework.
///
/// Provides credit valuation adjustment (CVA) and exposure simulation
/// for OTC derivative portfolios under ISDA netting and CSA collateral.
pub mod xva;

// Re-export pricing error for convenience.
pub use pricer::PricingError;

// Re-export unified valuations error type.
pub use error::{Error, Result};
