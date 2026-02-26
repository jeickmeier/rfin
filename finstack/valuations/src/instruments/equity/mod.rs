//! Equity instruments and equity derivatives.
//!
//! This module provides equity-linked instruments spanning vanilla options,
//! structured products, volatility derivatives, and private markets. Pricing
//! models include Black-Scholes, Monte Carlo, and binomial trees.
//!
//! # Features
//!
//! - **Vanilla Options**: European calls/puts with analytical Greeks
//! - **Volatility Products**: Variance swaps, VIX futures/options
//! - **Structured Products**: Autocallables, cliquets, equity-linked notes
//! - **Total Return Swaps**: Equity TRS with financing legs
//! - **Private Markets**: PE funds, real estate with waterfall distributions
//! - **Corporate Valuation**: DCF models with terminal value
//!
//! # Pricing Models
//!
//! | Instrument | Models Available |
//! |------------|------------------|
//! | Vanilla Options | Black-Scholes, Heston Fourier, Monte Carlo |
//! | Autocallables | Monte Carlo with early exercise |
//! | Variance Swaps | Replication via log contract |
//! | Equity TRS | Discounted cashflows |
//! | Private Markets | DCF with distribution waterfall |
//!
//! # Quick Example
//!
//! ```rust
//! use finstack_valuations::instruments::{Attributes, EquityOption, ExerciseStyle, OptionType, PricingOverrides, SettlementType};
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! use finstack_core::types::{CurveId, InstrumentId};
//! use time::macros::date;
//!
//! // Create a 6-month ATM call option
//! let option = EquityOption::builder()
//!     .id(InstrumentId::new("SPX-CALL-4500"))
//!     .underlying_ticker("SPX".to_string())
//!     .strike(4500.0)
//!     .option_type(OptionType::Call)
//!     .exercise_style(ExerciseStyle::European)
//!     .expiry(date!(2025 - 07 - 15))
//!     .notional(Money::new(100.0, Currency::USD))
//!     .day_count(finstack_core::dates::DayCount::Act365F)
//!     .settlement(SettlementType::Cash)
//!     .discount_curve_id(CurveId::new("USD-OIS"))
//!     .spot_id("EQUITY-SPOT".into())
//!     .vol_surface_id(CurveId::new("EQUITY-VOL"))
//!     .div_yield_id_opt(Some(CurveId::new("EQUITY-DIVYIELD")))
//!     .pricing_overrides(PricingOverrides::default())
//!     .attributes(Attributes::new())
//!     .build()
//!     .expect("valid option");
//! ```
//!
//! # Greeks
//!
//! Equity options support full analytical Greeks:
//! - **Delta**: ∂V/∂S (spot sensitivity)
//! - **Gamma**: ∂²V/∂S² (delta convexity)
//! - **Vega**: ∂V/∂σ (volatility sensitivity)
//! - **Theta**: ∂V/∂t (time decay)
//! - **Rho**: ∂V/∂r (rate sensitivity)
//!
//! # References
//!
//! - Black, F., & Scholes, M. (1973). "The Pricing of Options and Corporate Liabilities."
//! - Heston, S. L. (1993). "A Closed-Form Solution for Options with Stochastic Volatility."
//!
//! # See Also
//!
//! - [`EquityOption`] for vanilla options
//! - [`VarianceSwap`] for variance/volatility swaps
//! - [`Autocallable`] for structured notes
//! - [`PrivateMarketsFund`] for PE/credit fund valuation

/// Autocallable module - Autocallable structured notes.
pub mod autocallable;
/// Cliquet option module - Cliquet/ratchet options.
pub mod cliquet_option;
/// DCF equity module - Discounted cash flow for equity (renamed from dcf).
pub mod dcf_equity;
/// Equity index future module.
pub mod equity_index_future;
/// Equity option module - Vanilla equity options.
pub mod equity_option;
/// Equity TRS module - Equity total return swaps.
pub mod equity_trs;
/// PE fund module - Private equity/markets funds (renamed from private_markets_fund).
pub mod pe_fund;
/// Real estate module - Real estate asset valuation.
pub mod real_estate;
/// Equity spot module - Equity spot positions.
pub mod spot;
/// Variance swap module - Variance and volatility swaps.
pub mod variance_swap;
/// Volatility index future module.
pub mod vol_index_future;
/// Volatility index option module.
pub mod vol_index_option;

// Re-export primary types
pub use autocallable::{Autocallable, FinalPayoffType};
pub use cliquet_option::CliquetOption;
pub use dcf_equity::{DiscountedCashFlow, TerminalValueSpec};
pub use equity_index_future::{EquityFutureSpecs, EquityIndexFuture};
pub use equity_option::EquityOption;
pub use equity_trs::EquityTotalReturnSwap;
pub use pe_fund::PrivateMarketsFund;
pub use real_estate::{LeveredRealEstateEquity, RealEstateAsset, RealEstateValuationMethod};
pub use spot::Equity;
pub use variance_swap::VarianceSwap;
pub use vol_index_future::{VolIndexContractSpecs, VolatilityIndexFuture};
pub use vol_index_option::{VolIndexOptionSpecs, VolatilityIndexOption};
