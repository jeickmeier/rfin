//! Bachelier (Normal) model helpers.
//!
//! The Bachelier model assumes the underlying asset follows a normal distribution
//! (arithmetic Brownian motion), allowing for negative rates.
//!
//! Pricing formulas:
//!
//! Call = D(0,T) * [ (F - K) * N(d) + sigma * sqrt(T) * n(d) ]
//! Put  = D(0,T) * [ (K - F) * N(-d) + sigma * sqrt(T) * n(d) ]
//!
//! where d = (F - K) / (sigma * sqrt(T))

use finstack_core::math::{norm_cdf, norm_pdf};

/// Calculate d parameter for Bachelier model
///
/// d = (F - K) / (sigma * sqrt(T))
#[inline]
pub fn d_bachelier(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    if t <= 0.0 || sigma <= 0.0 {
        return 0.0;
    }
    (forward - strike) / (sigma * t.sqrt())
}

/// Bachelier (Normal) model price for a call/payer option
#[inline]
pub fn bachelier_price(
    option_type: crate::instruments::common::parameters::OptionType,
    forward: f64,
    strike: f64,
    sigma: f64,
    t: f64,
    annuity: f64,
) -> f64 {
    if t <= 0.0 {
        return match option_type {
            crate::instruments::common::parameters::OptionType::Call => {
                (forward - strike).max(0.0) * annuity
            }
            crate::instruments::common::parameters::OptionType::Put => {
                (strike - forward).max(0.0) * annuity
            }
        };
    }

    let d = d_bachelier(forward, strike, sigma, t);
    let disc_vol = sigma * t.sqrt();

    let term1 = (forward - strike) * norm_cdf(d);
    let term2 = disc_vol * norm_pdf(d);

    match option_type {
        crate::instruments::common::parameters::OptionType::Call => annuity * (term1 + term2),
        crate::instruments::common::parameters::OptionType::Put => {
            // Put-Call Parity or direct formula:
            // Put = Call - (F - K) * A
            //     = A * [(F-K)N(d) + v*n(d) - (F-K)]
            //     = A * [(F-K)(N(d)-1) + v*n(d)]
            //     = A * [(K-F)N(-d) + v*n(d)]
            // Since n(d) == n(-d)
            let term1_put = (strike - forward) * norm_cdf(-d);
            annuity * (term1_put + term2)
        }
    }
}
