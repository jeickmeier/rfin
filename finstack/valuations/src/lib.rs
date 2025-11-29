#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]

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
//! use finstack_core::market_data::MarketContext;
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
//!     0.05,           // 5% coupon
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
//! // let result = registry.price_with_registry(&bond, ModelKey::Discounting, &market, as_of)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Risk Metrics
//!
//! ```rust
//! use finstack_valuations::instruments::Bond;
//! use finstack_valuations::metrics::{MetricId, standard_registry};
//! use finstack_core::market_data::MarketContext;
//! # use finstack_core::currency::Currency;
//! # use finstack_core::money::Money;
//! # use finstack_core::dates::create_date;
//! # use time::Month;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let issue = create_date(2025, Month::January, 15)?;
//! # let maturity = create_date(2030, Month::January, 15)?;
//! # let bond = Bond::fixed("US-BOND-001", Money::new(1_000_000.0, Currency::USD),
//! #     0.05, issue, maturity, "USD-OIS");
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
//! ```rust,ignore
//! // Note: SimpleCalibration API may have changed - see calibration module docs
//! // use finstack_valuations::calibration::{
//! //     SimpleCalibration, MarketQuote, RatesQuote, CalibrationConfig
//! // };
//! use finstack_core::currency::Currency;
//! use finstack_core::dates::create_date;
//! use time::Month;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let base_date = create_date(2025, Month::January, 15)?;
//!
//! // Create calibration builder
//! let mut calibration = SimpleCalibration::new(base_date, Currency::USD);
//!
//! // Calibration would use market quotes with correct RatesQuote structure
//! // (See calibration module documentation for full examples)
//!
//! // Example: let (market_context, report) = calibration.calibrate(...)?;
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
//! # Supported Instruments
//!
//! ## Fixed Income
//! - [`Bond`](instruments::Bond): Fixed and floating-rate bonds, callable/putable, amortizing
//! - [`InterestRateSwap`](instruments::InterestRateSwap): Plain vanilla and basis swaps
//! - [`Swaption`](instruments::Swaption): European and Bermudan swaptions
//! - [`CapFloor`](instruments::cap_floor): Interest rate caps and floors
//! - [`Deposit`](instruments::Deposit): Money market deposits
//! - [`ForwardRateAgreement`](instruments::ForwardRateAgreement): FRAs
//! - [`InterestRateFuture`](instruments::InterestRateFuture): Futures contracts
//!
//! ## Credit
//! - [`CreditDefaultSwap`](instruments::CreditDefaultSwap): Single-name CDS
//! - [`CDSIndex`](instruments::CDSIndex): Credit indices (CDX, iTraxx)
//! - [`CdsTranche`](instruments::CdsTranche): Synthetic CDO tranches
//! - [`CdsOption`](instruments::CdsOption): Options on CDS
//! - [`StructuredCredit`](instruments::StructuredCredit): ABS, RMBS, CMBS, CLO
//!
//! ## Equity & FX
//! - [`Equity`](instruments::Equity): Equity spot positions
//! - [`EquityOption`](instruments::EquityOption): Vanilla equity options
//! - [`FxSpot`](instruments::FxSpot): FX spot positions
//! - [`FxOption`](instruments::FxOption): Vanilla FX options (Garman-Kohlhagen)
//! - [`FxSwap`](instruments::FxSwap): FX forwards and swaps
//! - [`Basket`](instruments::Basket): Multi-asset baskets
//!
//! ## Exotic Options (requires `mc` feature)
//! - [`AsianOption`](instruments::AsianOption): Asian (average price/strike) options
//! - [`BarrierOption`](instruments::BarrierOption): Barrier options (knock-in/out)
//! - [`LookbackOption`](instruments::LookbackOption): Lookback options
//! - [`Autocallable`](instruments::Autocallable): Autocallable notes
//! - [`CliquetOption`](instruments::CliquetOption): Cliquet/ratchet options
//! - [`QuantoOption`](instruments::QuantoOption): Quanto options
//!
//! ## Structured Products
//! - [`ConvertibleBond`](instruments::ConvertibleBond): Convertible bonds
//! - [`Repo`](instruments::Repo): Repurchase agreements
//! - [`VarianceSwap`](instruments::VarianceSwap): Variance and volatility swaps
//! - [`PrivateMarketsFund`](instruments::PrivateMarketsFund): Private equity/credit funds
//! - [`RevolvingCredit`](instruments::RevolvingCredit): Revolving credit facilities
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
//! - [`finstack_core`]: Core primitives (Money, dates, curves, expressions)
//! - [`finstack_statements`]: Financial statement modeling
//! - [`finstack_portfolio`]: Multi-instrument portfolio aggregation
//! - [`finstack_scenarios`]: Scenario analysis and stress testing

pub mod calibration;
pub mod cashflow;
pub mod constants;
pub mod margin;
pub mod pricer;
pub mod results;
pub mod schema;

// Export macros before instruments module
#[macro_use]
pub mod instruments;
pub mod attribution;
pub mod covenants;
pub mod metrics;
