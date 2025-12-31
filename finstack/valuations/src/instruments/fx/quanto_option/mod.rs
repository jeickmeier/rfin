//! Quanto option instruments with cross-currency adjustments.
//!
//! Quanto options provide exposure to a foreign asset with payoff settled
//! in domestic currency at a fixed exchange rate. The quanto adjustment
//! accounts for the correlation between asset and FX rate.
//!
//! # Quanto Feature
//!
//! For a quanto call on foreign asset with strike K:
//! - Underlying: Foreign asset with price S (in foreign currency)
//! - Payoff: Notional_DOM × max(S_T - K, 0) settled in domestic currency
//! - Fixed FX: No FX risk - always converts at predetermined rate
//!
//! # Quanto Adjustment
//!
//! The quanto drift adjustment modifies the asset drift by:
//!
//! ```text
//! Adjusted drift = r_domestic - q_foreign + ρ · σ_asset · σ_FX
//! ```
//!
//! where ρ is correlation between asset and FX rate.
//!
//! Positive correlation (asset and FX move together) increases value.
//! Negative correlation decreases value.
//!
//! # Pricing Model
//!
//! Modified Black-Scholes with quanto drift:
//! - Use domestic rate for discounting
//! - Apply quanto drift adjustment
//! - Use asset volatility σ_asset
//!
//! # References
//!
//! - Garman, M. B., & Kohlhagen, S. W. (1983). "Foreign Currency Option Values."
//!   *Journal of International Money and Finance*, 2(3), 231-237.
//!
//! - Derman, E., Karasinski, P., & Wecker, J. (1990). "Understanding Guaranteed
//!   Exchange-Rate Contracts in Foreign Stock Investments." Goldman Sachs
//!   Quantitative Strategies Research Notes.
//!
//! # See Also
//!
//! - [`QuantoOption`] for instrument struct
//! - [`models::closed_form::quanto`](crate::instruments::common::models::closed_form::quanto) for formulas

pub(crate) mod metrics;
pub(crate) mod pricer;
pub(crate) mod types;

pub use types::QuantoOption;
