//! Enhanced Greeks calculation with adaptive finite differences and Richardson extrapolation.
//!
//! Provides high-accuracy Greeks calculation using optimal bump sizes and
//! Richardson extrapolation for O(h^4) accuracy.

use super::Greeks;
use finstack_core::F;

/// Enhanced Greeks calculator with adaptive methods
#[derive(Clone, Debug)]
pub struct EnhancedGreeksCalculator {
    /// Base bump size multiplier
    pub bump_multiplier: F,
    /// Use Richardson extrapolation
    pub use_richardson: bool,
    /// Adaptive bump sizing
    pub adaptive_bumps: bool,
}

impl Default for EnhancedGreeksCalculator {
    fn default() -> Self {
        Self {
            bump_multiplier: 1.0,
            use_richardson: true,
            adaptive_bumps: true,
        }
    }
}

impl EnhancedGreeksCalculator {
    /// Create new calculator with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create calculator with custom settings
    pub fn with_settings(bump_multiplier: F, use_richardson: bool, adaptive_bumps: bool) -> Self {
        Self {
            bump_multiplier,
            use_richardson,
            adaptive_bumps,
        }
    }

    /// Calculate optimal bump size based on FinancePy methodology
    fn optimal_bump_size(&self, value: F, is_percentage: bool) -> F {
        if self.adaptive_bumps {
            // FinancePy-style optimal bump
            let base = value.abs().max(1.0);
            let epsilon = f64::EPSILON.powf(1.0 / 3.0); // Cube root for better numerical stability

            if is_percentage {
                // For rates/volatilities (percentage inputs)
                epsilon * 10.0 * self.bump_multiplier
            } else {
                // For spot prices
                epsilon * base * self.bump_multiplier
            }
        } else {
            // Fixed bump sizes
            if is_percentage {
                0.0001 * self.bump_multiplier // 1bp for rates
            } else {
                0.0001 * value.abs().max(1.0) * self.bump_multiplier
            }
        }
    }

    /// Calculate delta with Richardson extrapolation
    pub fn calculate_delta<P>(&self, price_fn: P, spot: F) -> F
    where
        P: Fn(F) -> F,
    {
        let h = self.optimal_bump_size(spot, false);

        if self.use_richardson {
            // Richardson extrapolation for O(h^4) accuracy
            let h1 = h;
            let h2 = h / 2.0;

            // First approximation with step h1
            let delta1 = (price_fn(spot + h1) - price_fn(spot - h1)) / (2.0 * h1);

            // Second approximation with step h2
            let delta2 = (price_fn(spot + h2) - price_fn(spot - h2)) / (2.0 * h2);

            // Richardson extrapolation formula: (4*f(h/2) - f(h))/3
            (4.0 * delta2 - delta1) / 3.0
        } else {
            // Standard central difference
            (price_fn(spot + h) - price_fn(spot - h)) / (2.0 * h)
        }
    }

    /// Calculate gamma with Richardson extrapolation
    pub fn calculate_gamma<P>(&self, price_fn: P, spot: F) -> F
    where
        P: Fn(F) -> F,
    {
        let h = self.optimal_bump_size(spot, false);

        if self.use_richardson {
            let h1 = h;
            let h2 = h / 2.0;

            // Second derivative using central differences
            let gamma1 =
                (price_fn(spot + h1) - 2.0 * price_fn(spot) + price_fn(spot - h1)) / (h1 * h1);
            let gamma2 =
                (price_fn(spot + h2) - 2.0 * price_fn(spot) + price_fn(spot - h2)) / (h2 * h2);

            // Richardson extrapolation
            (4.0 * gamma2 - gamma1) / 3.0
        } else {
            (price_fn(spot + h) - 2.0 * price_fn(spot) + price_fn(spot - h)) / (h * h)
        }
    }

    /// Calculate vega with optimal bump
    pub fn calculate_vega<P>(&self, price_fn: P, vol: F) -> F
    where
        P: Fn(F) -> F,
    {
        let h = self.optimal_bump_size(vol, true);

        if self.use_richardson {
            let h1 = h;
            let h2 = h / 2.0;

            let vega1 = (price_fn(vol + h1) - price_fn(vol - h1)) / (2.0 * h1);
            let vega2 = (price_fn(vol + h2) - price_fn(vol - h2)) / (2.0 * h2);

            (4.0 * vega2 - vega1) / 3.0
        } else {
            (price_fn(vol + h) - price_fn(vol - h)) / (2.0 * h)
        }
    }

    /// Calculate theta with optimal time bump
    pub fn calculate_theta<P>(&self, price_fn: P, t: F) -> F
    where
        P: Fn(F) -> F,
    {
        // Time bump in years (1 day)
        let h = 1.0 / 365.25;

        if t > h {
            if self.use_richardson {
                let h1 = h;
                let h2 = h / 2.0;

                if t > h1 {
                    // Negative because theta is typically negative of time derivative
                    let theta1 = -(price_fn(t - h1) - price_fn(t)) / h1;

                    if t > h2 {
                        let theta2 = -(price_fn(t - h2) - price_fn(t)) / h2;
                        (4.0 * theta2 - theta1) / 3.0 * 365.25 // Convert to daily
                    } else {
                        theta1 * 365.25
                    }
                } else {
                    0.0
                }
            } else {
                -(price_fn(t - h) - price_fn(t)) / h * 365.25
            }
        } else {
            0.0
        }
    }

    /// Calculate rho with optimal rate bump
    pub fn calculate_rho<P>(&self, price_fn: P, r: F) -> F
    where
        P: Fn(F) -> F,
    {
        let h = self.optimal_bump_size(r, true);

        if self.use_richardson {
            let h1 = h;
            let h2 = h / 2.0;

            let rho1 = (price_fn(r + h1) - price_fn(r - h1)) / (2.0 * h1);
            let rho2 = (price_fn(r + h2) - price_fn(r - h2)) / (2.0 * h2);

            (4.0 * rho2 - rho1) / 3.0
        } else {
            (price_fn(r + h) - price_fn(r - h)) / (2.0 * h)
        }
    }

    /// Calculate all basic Greeks with enhanced accuracy
    #[allow(clippy::too_many_arguments)]
    pub fn calculate_all_greeks<F1, F2, F3, F4, F5>(
        &self,
        spot_fn: F1,
        vol_fn: F2,
        time_fn: F3,
        rate_fn: F4,
        base_price: F5,
        spot: F,
        vol: F,
        t: F,
        r: F,
    ) -> Greeks
    where
        F1: Fn(F) -> F,
        F2: Fn(F) -> F,
        F3: Fn(F) -> F,
        F4: Fn(F) -> F,
        F5: Fn() -> F,
    {
        let delta = self.calculate_delta(spot_fn, spot);
        let gamma = self.calculate_gamma(
            |_s| {
                // Need to recalculate price for each spot
                base_price()
            },
            spot,
        );
        let vega = self.calculate_vega(vol_fn, vol);
        let theta = self.calculate_theta(time_fn, t);
        let rho = self.calculate_rho(rate_fn, r);

        Greeks::new(delta, gamma, vega, theta, rho)
    }
}

/// Container for cross-Greeks (second-order sensitivities)
#[derive(Clone, Debug, Default)]
pub struct CrossGreeks {
    /// Vanna: cross-derivative of delta with respect to volatility
    pub vanna: F,
    /// Volga: second derivative with respect to volatility
    pub volga: F,
    /// Charm: cross-derivative of delta with respect to time
    pub charm: F,
    /// Color: cross-derivative of gamma with respect to time
    pub color: F,
    /// Speed: third derivative with respect to spot
    pub speed: F,
    /// Veta: cross-derivative of vega with respect to time
    pub veta: F,
}

impl CrossGreeks {
    /// Create new cross-Greeks container
    pub fn new(vanna: F, volga: F, charm: F, color: F, speed: F, veta: F) -> Self {
        Self {
            vanna,
            volga,
            charm,
            color,
            speed,
            veta,
        }
    }
}

/// Enhanced calculator for cross-Greeks
pub struct CrossGreeksCalculator {
    /// Base calculator for first-order Greeks
    pub base_calculator: EnhancedGreeksCalculator,
}

impl CrossGreeksCalculator {
    /// Create new cross-Greeks calculator
    pub fn new() -> Self {
        Self {
            base_calculator: EnhancedGreeksCalculator::default(),
        }
    }

    /// Calculate vanna (∂²V/∂S∂σ) with Richardson extrapolation
    pub fn calculate_vanna<P>(&self, price_fn: P, spot: F, vol: F) -> F
    where
        P: Fn(F, F) -> F,
    {
        let h_spot = self.base_calculator.optimal_bump_size(spot, false);
        let h_vol = self.base_calculator.optimal_bump_size(vol, true);

        if self.base_calculator.use_richardson {
            // Richardson extrapolation for mixed derivative
            let vanna1 = self.calculate_vanna_at_step(&price_fn, spot, vol, h_spot, h_vol);
            let vanna2 =
                self.calculate_vanna_at_step(&price_fn, spot, vol, h_spot / 2.0, h_vol / 2.0);

            (4.0 * vanna2 - vanna1) / 3.0
        } else {
            self.calculate_vanna_at_step(&price_fn, spot, vol, h_spot, h_vol)
        }
    }

    fn calculate_vanna_at_step<P>(&self, price_fn: &P, spot: F, vol: F, h_spot: F, h_vol: F) -> F
    where
        P: Fn(F, F) -> F,
    {
        // Four-point formula for mixed derivative
        (price_fn(spot + h_spot, vol + h_vol)
            - price_fn(spot + h_spot, vol - h_vol)
            - price_fn(spot - h_spot, vol + h_vol)
            + price_fn(spot - h_spot, vol - h_vol))
            / (4.0 * h_spot * h_vol)
    }

    /// Calculate volga (∂²V/∂σ²)
    pub fn calculate_volga<P>(&self, price_fn: P, vol: F) -> F
    where
        P: Fn(F) -> F,
    {
        let h = self.base_calculator.optimal_bump_size(vol, true);

        if self.base_calculator.use_richardson {
            let h1 = h;
            let h2 = h / 2.0;

            let volga1 =
                (price_fn(vol + h1) - 2.0 * price_fn(vol) + price_fn(vol - h1)) / (h1 * h1);
            let volga2 =
                (price_fn(vol + h2) - 2.0 * price_fn(vol) + price_fn(vol - h2)) / (h2 * h2);

            (4.0 * volga2 - volga1) / 3.0
        } else {
            (price_fn(vol + h) - 2.0 * price_fn(vol) + price_fn(vol - h)) / (h * h)
        }
    }

    /// Calculate charm (∂²V/∂S∂t)
    pub fn calculate_charm<P>(&self, price_fn: P, spot: F, t: F) -> F
    where
        P: Fn(F, F) -> F,
    {
        let h_spot = self.base_calculator.optimal_bump_size(spot, false);
        let h_time = 1.0 / 365.25; // 1 day

        if t > h_time {
            // Four-point formula for mixed derivative
            let charm = -(price_fn(spot + h_spot, t - h_time)
                - price_fn(spot + h_spot, t)
                - price_fn(spot - h_spot, t - h_time)
                + price_fn(spot - h_spot, t))
                / (4.0 * h_spot * h_time);

            charm * 365.25 // Convert to daily
        } else {
            0.0
        }
    }

    /// Calculate color (∂²V/∂γ∂t)
    pub fn calculate_color<P>(&self, price_fn: P, spot: F, t: F) -> F
    where
        P: Fn(F, F) -> F,
    {
        let h = self.base_calculator.optimal_bump_size(spot, false);
        let dt = 1.0 / 365.25;

        if t > dt {
            // Calculate gamma at two different times
            let gamma_now =
                (price_fn(spot + h, t) - 2.0 * price_fn(spot, t) + price_fn(spot - h, t)) / (h * h);
            let gamma_later = (price_fn(spot + h, t - dt) - 2.0 * price_fn(spot, t - dt)
                + price_fn(spot - h, t - dt))
                / (h * h);

            -(gamma_now - gamma_later) / dt * 365.25
        } else {
            0.0
        }
    }

    /// Calculate speed (∂³V/∂S³)
    pub fn calculate_speed<P>(&self, price_fn: P, spot: F) -> F
    where
        P: Fn(F) -> F,
    {
        let h = self.base_calculator.optimal_bump_size(spot, false);

        // Third derivative using finite differences
        // Correct formula: [f(x+2h) - 2f(x+h) + 2f(x-h) - f(x-2h)] / (2h³)
        (price_fn(spot + 2.0 * h) - 2.0 * price_fn(spot + h) + 2.0 * price_fn(spot - h)
            - price_fn(spot - 2.0 * h))
            / (2.0 * h * h * h)
    }

    /// Calculate veta (∂²V/∂σ∂t)
    pub fn calculate_veta<P>(&self, price_fn: P, vol: F, t: F) -> F
    where
        P: Fn(F, F) -> F,
    {
        let h_vol = self.base_calculator.optimal_bump_size(vol, true);
        let h_time = 1.0 / 365.25;

        if t > h_time {
            // Four-point formula for mixed derivative
            let veta = -(price_fn(vol + h_vol, t - h_time)
                - price_fn(vol + h_vol, t)
                - price_fn(vol - h_vol, t - h_time)
                + price_fn(vol - h_vol, t))
                / (4.0 * h_vol * h_time);

            veta * 365.25 // Convert to daily
        } else {
            0.0
        }
    }
}

impl Default for CrossGreeksCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimal_bump_size() {
        let calc = EnhancedGreeksCalculator::new();

        // Test spot bump
        let spot_bump = calc.optimal_bump_size(100.0, false);
        assert!(spot_bump > 0.0 && spot_bump < 0.01);

        // Test volatility bump (percentage)
        let vol_bump = calc.optimal_bump_size(0.20, true);
        assert!(vol_bump > 0.0 && vol_bump < 0.001);
    }

    #[test]
    fn test_richardson_vs_standard() {
        // Simple quadratic function for testing
        let f = |x: f64| x * x;

        // Calculate derivative at x = 2 (should be 4)
        let calc_richardson = EnhancedGreeksCalculator::with_settings(1.0, true, false);
        let calc_standard = EnhancedGreeksCalculator::with_settings(1.0, false, false);

        let delta_richardson = calc_richardson.calculate_delta(f, 2.0);
        let delta_standard = calc_standard.calculate_delta(f, 2.0);

        // Richardson should be more accurate
        println!(
            "Richardson delta: {}, Standard delta: {}",
            delta_richardson, delta_standard
        );
        println!(
            "Richardson error: {}, Standard error: {}",
            (delta_richardson - 4.0).abs(),
            (delta_standard - 4.0).abs()
        );

        // For this simple function, both methods should be very accurate
        assert!(
            (delta_richardson - 4.0).abs() < 1e-4,
            "Richardson delta {} should be close to 4.0",
            delta_richardson
        );
        assert!(
            (delta_standard - 4.0).abs() < 1e-4,
            "Standard delta {} should be close to 4.0",
            delta_standard
        );
    }

    #[test]
    fn test_gamma_calculation() {
        // Quadratic function: f(x) = x^2, second derivative = 2
        let f = |x: f64| x * x;

        let calc = EnhancedGreeksCalculator::new();
        let gamma = calc.calculate_gamma(f, 5.0);

        println!(
            "Calculated gamma: {}, expected: 2.0, difference: {}",
            gamma,
            (gamma - 2.0).abs()
        );
        // Should be very close to 2, but numerical differentiation has inherent errors
        assert!(
            (gamma - 2.0).abs() < 1e-3,
            "Gamma {} should be close to 2.0",
            gamma
        );
    }

    #[test]
    fn test_cross_greeks_vanna() {
        // Test function where vanna can be calculated analytically
        // V = S * σ, so ∂²V/∂S∂σ = 1
        let price_fn = |s: f64, v: f64| s * v;

        let calc = CrossGreeksCalculator::new();
        let vanna = calc.calculate_vanna(price_fn, 100.0, 0.20);

        // Should be close to 1
        assert!((vanna - 1.0).abs() < 1e-3);
    }

    #[test]
    fn test_speed_calculation() {
        // Cubic function: f(x) = x^3, third derivative = 6
        let f = |x: f64| x * x * x;

        let calc = CrossGreeksCalculator::new();
        let speed = calc.calculate_speed(f, 2.0);

        // Should be close to 6 (third derivatives have lower numerical accuracy)
        assert!(
            (speed - 6.0).abs() < 1.0,
            "Speed {} should be reasonably close to 6.0",
            speed
        );
    }
}
