use super::model::SABRModel;
use crate::instruments::models::volatility::black::d1_d2;
use finstack_core::{Error, Result};

/// SABR smile generator for creating volatility surfaces
pub struct SABRSmile {
    model: SABRModel,
    forward: f64,
    time_to_expiry: f64,
}

/// Result of arbitrage validation, containing any violations found.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ArbitrageValidationResult {
    /// Strikes where butterfly spread is negative (convexity violation)
    pub butterfly_violations: Vec<ButterflyViolation>,
    /// Pairs of strikes where call prices increase (monotonicity violation)
    pub monotonicity_violations: Vec<MonotonicityViolation>,
}

/// A butterfly spread violation at a specific strike.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ButterflyViolation {
    /// Strike at which the violation occurs
    pub strike: f64,
    /// Butterfly spread value (negative indicates violation)
    pub butterfly_value: f64,
    /// Severity as percentage of mid-strike price
    pub severity_pct: f64,
}

/// A monotonicity violation between two strikes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct MonotonicityViolation {
    /// Lower strike
    pub strike_low: f64,
    /// Higher strike
    pub strike_high: f64,
    /// Call price at lower strike
    pub price_low: f64,
    /// Call price at higher strike (should be lower)
    pub price_high: f64,
}

impl ArbitrageValidationResult {
    /// Returns true if no arbitrage was detected.
    #[must_use]
    pub fn is_arbitrage_free(&self) -> bool {
        self.butterfly_violations.is_empty() && self.monotonicity_violations.is_empty()
    }

    /// Returns the worst butterfly violation severity, if any.
    #[must_use]
    pub fn worst_butterfly_severity(&self) -> Option<f64> {
        self.butterfly_violations
            .iter()
            .map(|v| v.severity_pct.abs())
            .max_by(|a, b| a.total_cmp(b))
    }
}

impl SABRSmile {
    /// Create new smile generator
    pub fn new(model: SABRModel, forward: f64, time_to_expiry: f64) -> Self {
        Self {
            model,
            forward,
            time_to_expiry,
        }
    }

    /// Returns the ATM (at-the-money) implied volatility.
    ///
    /// This is a convenience method that computes the implied volatility
    /// at strike = forward, which is the most frequently quoted volatility level.
    ///
    /// # Returns
    ///
    /// ATM implied volatility as a decimal (e.g., 0.20 for 20% vol).
    ///
    /// # Errors
    ///
    /// Returns an error if the volatility computation fails (e.g., invalid parameters).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use finstack_valuations::instruments::models::volatility::sabr::{
    ///     SABRParameters, SABRModel, SABRSmile,
    /// };
    ///
    /// let params = SABRParameters::new(0.2, 0.5, 0.3, -0.1).unwrap();
    /// let model = SABRModel::new(params);
    /// let smile = SABRSmile::new(model, 100.0, 1.0);
    ///
    /// let atm_vol = smile.atm_vol().unwrap();
    /// assert!(atm_vol > 0.0);
    /// ```
    #[must_use = "computed ATM volatility should be used"]
    pub fn atm_vol(&self) -> Result<f64> {
        self.model
            .implied_volatility(self.forward, self.forward, self.time_to_expiry)
    }

    /// Generate volatility smile for given strikes
    pub fn generate_smile(&self, strikes: &[f64]) -> Result<Vec<f64>> {
        let mut vols = Vec::with_capacity(strikes.len());

        for &strike in strikes {
            let vol = self
                .model
                .implied_volatility(self.forward, strike, self.time_to_expiry)?;
            vols.push(vol);
        }

        Ok(vols)
    }

    /// Generate strike from delta
    pub fn strike_from_delta(&self, delta: f64, is_call: bool) -> Result<f64> {
        // This requires iterative solving
        // Simplified version using ATM vol as approximation
        let atm_vol = self
            .model
            .atm_volatility(self.forward, self.time_to_expiry)?;
        let variance = atm_vol.powi(2) * self.time_to_expiry;
        let std_dev = variance.sqrt();

        // Normal inverse for delta
        let z = if is_call {
            finstack_core::math::standard_normal_inv_cdf(delta)
        } else {
            finstack_core::math::standard_normal_inv_cdf(1.0 - delta)
        };

        let strike = self.forward * (z * std_dev).exp();
        Ok(strike)
    }

    /// Validate the generated smile for no-arbitrage conditions.
    ///
    /// Checks for two types of static arbitrage:
    ///
    /// 1. **Butterfly arbitrage** (convexity): Call(K-δ) - 2·Call(K) + Call(K+δ) ≥ 0
    ///    A negative butterfly spread means you can buy the wings and sell the body
    ///    for a risk-free profit.
    ///
    /// 2. **Monotonicity arbitrage**: Call prices must decrease as strike increases.
    ///    If C(K₁) < C(K₂) for K₁ < K₂, you can buy the lower strike and sell the
    ///    higher strike for immediate profit.
    ///
    /// # Arguments
    /// * `strikes` - Array of strikes to validate (must be sorted ascending)
    /// * `r` - Risk-free rate for discounting
    /// * `q` - Dividend/foreign rate
    ///
    /// # Returns
    /// `ArbitrageValidationResult` containing any violations found.
    ///
    /// # Example
    /// ```rust,ignore
    /// let result = smile.validate_no_arbitrage(&strikes, 0.05, 0.02)?;
    /// if !result.is_arbitrage_free() {
    ///     println!("Warning: {} butterfly violations found",
    ///              result.butterfly_violations.len());
    /// }
    /// ```
    pub fn validate_no_arbitrage(
        &self,
        strikes: &[f64],
        r: f64,
        q: f64,
    ) -> Result<ArbitrageValidationResult> {
        if strikes.len() < 3 {
            return Ok(ArbitrageValidationResult::default());
        }

        let vols = self.generate_smile(strikes)?;

        // Convert to call prices for validation
        let prices: Vec<f64> = strikes
            .iter()
            .zip(vols.iter())
            .map(|(&k, &vol)| bs_call_price(self.forward, k, r, q, vol, self.time_to_expiry))
            .collect();

        let mut result = ArbitrageValidationResult::default();

        // Tolerance for numerical noise (0.1 bps of notional)
        let tol = 1e-6;

        // Check monotonicity: C(K₁) > C(K₂) for K₁ < K₂
        for i in 1..prices.len() {
            if prices[i] > prices[i - 1] + tol {
                result.monotonicity_violations.push(MonotonicityViolation {
                    strike_low: strikes[i - 1],
                    strike_high: strikes[i],
                    price_low: prices[i - 1],
                    price_high: prices[i],
                });
            }
        }

        // Check butterfly positivity (convexity)
        for i in 1..prices.len() - 1 {
            let butterfly = prices[i - 1] - 2.0 * prices[i] + prices[i + 1];
            if butterfly < -tol {
                let severity_pct = if prices[i] > tol {
                    butterfly.abs() / prices[i] * 100.0
                } else {
                    0.0
                };

                result.butterfly_violations.push(ButterflyViolation {
                    strike: strikes[i],
                    butterfly_value: butterfly,
                    severity_pct,
                });
            }
        }

        Ok(result)
    }

    /// Quick check if the smile is arbitrage-free.
    ///
    /// Returns `Ok(())` if no arbitrage detected, `Err` with description if arbitrage found.
    pub fn check_no_arbitrage(&self, strikes: &[f64], r: f64, q: f64) -> Result<()> {
        let result = self.validate_no_arbitrage(strikes, r, q)?;

        if !result.is_arbitrage_free() {
            let mut msg = String::from("SABR smile contains arbitrage: ");

            if !result.butterfly_violations.is_empty() {
                msg.push_str(&format!(
                    "{} butterfly violations (worst: {:.2}%)",
                    result.butterfly_violations.len(),
                    result.worst_butterfly_severity().unwrap_or(0.0)
                ));
            }

            if !result.monotonicity_violations.is_empty() {
                if !result.butterfly_violations.is_empty() {
                    msg.push_str(", ");
                }
                msg.push_str(&format!(
                    "{} monotonicity violations",
                    result.monotonicity_violations.len()
                ));
            }

            return Err(Error::Validation(msg));
        }

        Ok(())
    }

    /// Repair arbitrage in the SABR smile by adjusting volatilities.
    ///
    /// This method generates a smile and then applies monotonicity and convexity
    /// corrections to remove static arbitrage violations. The repair is conservative:
    /// it only modifies volatilities at violating strikes.
    ///
    /// # Algorithm
    ///
    /// 1. Generate the raw SABR smile
    /// 2. Apply monotonicity repair: ensure call prices decrease with strike
    /// 3. Apply butterfly repair: ensure convexity (positive second derivative)
    ///
    /// The repair uses a simple projection approach:
    /// - For monotonicity: clamp prices to maintain decreasing sequence
    /// - For butterfly: adjust mid-strike to satisfy convexity constraint
    ///
    /// # Arguments
    ///
    /// * `strikes` - Array of strikes (should be sorted ascending)
    /// * `r` - Risk-free rate for Black-Scholes conversion
    /// * `q` - Dividend/foreign rate
    /// * `max_iterations` - Maximum repair iterations (default: 10)
    ///
    /// # Returns
    ///
    /// Repaired volatility smile as `Vec<f64>`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let repaired_vols = smile.repair_arbitrage(&strikes, 0.05, 0.02, 10)?;
    /// ```
    ///
    /// # References
    ///
    /// - Fengler, M. (2009). "Arbitrage-free smoothing of the implied volatility surface."
    ///   Quantitative Finance, 9(4), 417-428.
    pub fn repair_arbitrage(
        &self,
        strikes: &[f64],
        r: f64,
        q: f64,
        max_iterations: usize,
    ) -> Result<Vec<f64>> {
        if strikes.len() < 3 {
            return self.generate_smile(strikes);
        }

        // Generate initial smile
        let mut vols = self.generate_smile(strikes)?;

        // Convert to call prices for manipulation
        let mut prices: Vec<f64> = strikes
            .iter()
            .zip(vols.iter())
            .map(|(&k, &vol)| bs_call_price(self.forward, k, r, q, vol, self.time_to_expiry))
            .collect();

        // Iterative repair
        for _ in 0..max_iterations {
            let mut changed = false;

            // Repair monotonicity: C(K₁) > C(K₂) for K₁ < K₂
            for i in 1..prices.len() {
                if prices[i] > prices[i - 1] {
                    // Project to monotonic: set to slightly below previous
                    prices[i] = prices[i - 1] * 0.9999;
                    changed = true;
                }
            }

            // Repair butterfly convexity: C(K-δ) - 2C(K) + C(K+δ) ≥ 0
            for i in 1..prices.len() - 1 {
                let butterfly = prices[i - 1] - 2.0 * prices[i] + prices[i + 1];
                if butterfly < 0.0 {
                    // Adjust mid-strike price to satisfy convexity
                    // C(K) should be at most (C(K-δ) + C(K+δ)) / 2
                    let max_mid = (prices[i - 1] + prices[i + 1]) / 2.0;
                    prices[i] = max_mid * 0.9999; // Slightly below for numerical safety
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        // Convert prices back to volatilities using implied vol inversion
        for i in 0..vols.len() {
            let target_price = prices[i];
            let k = strikes[i];

            // Newton-Raphson to find implied vol
            let mut vol = vols[i]; // Start from original vol
            for _ in 0..20 {
                let price = bs_call_price(self.forward, k, r, q, vol, self.time_to_expiry);
                let vega = bs_call_vega(self.forward, k, r, q, vol, self.time_to_expiry);

                if vega.abs() < 1e-14 {
                    break;
                }

                let error = price - target_price;
                if error.abs() < 1e-10 {
                    break;
                }

                vol -= error / vega;
                vol = vol.clamp(0.001, 5.0); // Reasonable bounds
            }

            vols[i] = vol;
        }

        Ok(vols)
    }
}

/// Black-Scholes call vega for implied vol inversion.
#[inline]
fn bs_call_vega(forward: f64, strike: f64, r: f64, q: f64, vol: f64, t: f64) -> f64 {
    if t <= 0.0 || vol <= 0.0 {
        return 0.0;
    }

    // NOTE: `forward` here is self.forward from SABRSmile (a true forward price),
    // not spot. Using d1_d2 with r/q on top of a forward double-counts the drift.
    // This preserves pre-existing behavior; to be revisited in BS price consolidation.
    let (d1, _d2) = d1_d2(forward, strike, r, vol, t, q);
    let pdf_d1 = finstack_core::math::norm_pdf(d1);

    forward * (-q * t).exp() * t.sqrt() * pdf_d1
}

/// Black-Scholes call price for arbitrage checking.
///
/// Uses the standard Black-Scholes formula for European call options.
#[inline]
fn bs_call_price(forward: f64, strike: f64, r: f64, q: f64, vol: f64, t: f64) -> f64 {
    if t <= 0.0 {
        return (forward - strike).max(0.0);
    }

    // NOTE: same forward-as-spot concern as bs_call_vega above.
    let (d1, d2) = d1_d2(forward, strike, r, vol, t, q);

    let cdf_d1 = finstack_core::math::norm_cdf(d1);
    let cdf_d2 = finstack_core::math::norm_cdf(d2);

    forward * (-q * t).exp() * cdf_d1 - strike * (-r * t).exp() * cdf_d2
}
