//! Swaption instruments with Black (1976) and SABR volatility models.
//!
//! Swaptions are options on interest rate swaps, giving the holder the right
//! (but not obligation) to enter into a swap at a predetermined fixed rate.
//! They are key instruments for managing long-term interest rate exposure.
//!
//! # Swaption Types
//!
//! - **Payer swaption**: Right to enter payer swap (pay fixed, receive floating)
//!   - Benefits when rates rise (swap value becomes positive)
//!
//! - **Receiver swaption**: Right to enter receiver swap (receive fixed, pay floating)
//!   - Benefits when rates fall (swap value becomes positive)
//!
//! # Exercise Styles
//!
//! - **European**: Single exercise date
//! - **Bermudan**: Exercise on any coupon date in a window
//! - **American**: Exercise any time (rare in practice)
//!
//! # Settlement Types
//!
//! - **Physical**: Deliver the underlying swap upon exercise
//! - **Cash**: Cash settlement based on swap present value
//!
//! # Pricing Model: Black (1976)
//!
//! European swaptions are priced using Black (1976) model for options on
//! forward swap rates:
//!
//! **Payer Swaption:**
//! ```text
//! V_payer = A(0,T) · [S · N(d₁) - K · N(d₂)]
//! ```
//!
//! **Receiver Swaption:**
//! ```text
//! V_receiver = A(0,T) · [K · N(-d₂) - S · N(-d₁)]
//! ```
//!
//! where:
//! ```text
//! d₁ = [ln(S/K) + 0.5σ²T] / (σ√T)
//! d₂ = d₁ - σ√T
//! A(0,T) = Σ τᵢ · DF(t_i)  (swap annuity)
//! S = forward swap rate
//! K = strike rate
//! σ = implied volatility
//! T = time to expiration
//! ```
//!
//! # SABR Volatility Interpolation
//!
//! Market swaption volatilities are typically quoted on a strike grid and
//! interpolated using the SABR stochastic volatility model (Hagan et al. 2002).
//!
//! # Market Conventions
//!
//! Standard swaption quoting conventions:
//!
//! - **USD**: 3M or 6M into 2Y, 5Y, 10Y, 30Y swaps
//! - **EUR**: 1Y, 2Y, 5Y, 10Y expiries into various tenors
//! - **Volatility**: Quoted as lognormal (Black) or normal (Bachelier)
//! - **Daycount**: Follow underlying swap conventions
//!
//! # References
//!
//! - Black, F. (1976). "The Pricing of Commodity Contracts." *Journal of
//!   Financial Economics*, 3(1-2), 167-179.
//!   (Black model extended to swaptions)
//!
//! - Hagan, P. S., Kumar, D., Lesniewski, A. S., & Woodward, D. E. (2002).
//!   "Managing Smile Risk." *Wilmott Magazine*, September, 84-108.
//!   (SABR model for volatility interpolation)
//!
//! - Rebonato, R. (2004). *Volatility and Correlation: The Perfect Hedger and
//!   the Fox* (2nd ed.). Wiley. Part II: Swaptions.
//!
//! - Brigo, D., & Mercurio, F. (2006). *Interest Rate Models - Theory and Practice*
//!   (2nd ed.). Springer. Chapter 13: Swaption Pricing.
//!
//! # Implementation Notes
//!
//! - European swaptions use Black (1976) with implied volatility lookup
//! - Bermudan swaptions require tree-based or LSM pricing
//! - Volatility interpolation via SABR model when enabled
//! - Settlement conventions affect discount factor adjustments
//!
//! # Examples
//!
//! See [`Swaption`] for construction and usage examples.
//!
//! # See Also
//!
//! - [`Swaption`] for swaption instrument struct
//! - [`SwaptionExercise`] for exercise style specification
//! - [`SwaptionSettlement`] for settlement type
//! - [`metrics`] for swaption risk metrics
//! - [`SimpleSwaptionBlackPricer`] for Black model pricer

pub mod metrics;
pub mod parameters;
pub mod pricer;
mod types;

pub use pricer::SimpleSwaptionBlackPricer;
pub use types::{Swaption, SwaptionExercise, SwaptionSettlement};
