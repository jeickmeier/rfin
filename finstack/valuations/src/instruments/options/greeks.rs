//! Greeks calculation for option instruments.

use finstack_core::F;

/// Container for option Greeks
#[derive(Clone, Debug, Default)]
pub struct Greeks {
    /// Delta: price sensitivity to underlying price
    pub delta: F,
    /// Gamma: delta sensitivity to underlying price
    pub gamma: F,
    /// Vega: price sensitivity to volatility (per 1% change)
    pub vega: F,
    /// Theta: price sensitivity to time decay (daily)
    pub theta: F,
    /// Rho: price sensitivity to interest rate (per 1% change)
    pub rho: F,
    /// Volga/Vomma: vega sensitivity to volatility
    pub volga: Option<F>,
    /// Vanna: delta sensitivity to volatility
    pub vanna: Option<F>,
    /// Charm: delta sensitivity to time
    pub charm: Option<F>,
    /// Color: gamma sensitivity to time
    pub color: Option<F>,
    /// Speed: gamma sensitivity to underlying price
    pub speed: Option<F>,
}

impl Greeks {
    /// Create a new Greeks instance with basic Greeks
    pub fn new(delta: F, gamma: F, vega: F, theta: F, rho: F) -> Self {
        Self {
            delta,
            gamma,
            vega,
            theta,
            rho,
            volga: None,
            vanna: None,
            charm: None,
            color: None,
            speed: None,
        }
    }
    
    /// Create a Greeks instance with all Greeks
    #[allow(clippy::too_many_arguments)]
    pub fn full(
        delta: F,
        gamma: F,
        vega: F,
        theta: F,
        rho: F,
        volga: F,
        vanna: F,
        charm: F,
        color: F,
        speed: F,
    ) -> Self {
        Self {
            delta,
            gamma,
            vega,
            theta,
            rho,
            volga: Some(volga),
            vanna: Some(vanna),
            charm: Some(charm),
            color: Some(color),
            speed: Some(speed),
        }
    }
}

/// Calculator for option Greeks using finite differences
pub struct GreeksCalculator {
    /// Bump size for delta calculation (in underlying units)
    pub delta_bump: F,
    /// Bump size for vega calculation (in volatility percentage points)
    pub vega_bump: F,
    /// Bump size for rho calculation (in rate percentage points)
    pub rho_bump: F,
    /// Bump size for theta calculation (in days)
    pub theta_bump: F,
}

impl Default for GreeksCalculator {
    fn default() -> Self {
        Self {
            delta_bump: 0.01, // 1% of underlying
            vega_bump: 0.01,  // 1% volatility
            rho_bump: 0.01,   // 1% rate
            theta_bump: 1.0,  // 1 day
        }
    }
}

impl GreeksCalculator {
    /// Create a new Greeks calculator with default settings
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Calculate Greeks using finite differences
    /// 
    /// # Arguments
    /// * `price_fn` - Function that calculates option price given parameters
    /// * `spot` - Current spot price
    /// * `r` - Risk-free rate
    /// * `sigma` - Implied volatility
    /// * `t` - Time to maturity
    /// * `q` - Dividend yield or foreign rate
    pub fn calculate_greeks<F: Fn(f64, f64, f64, f64, f64) -> f64>(
        &self,
        price_fn: F,
        spot: f64,
        r: f64,
        sigma: f64,
        t: f64,
        q: f64,
    ) -> Greeks {
        // Base price
        let base_price = price_fn(spot, r, sigma, t, q);
        
        // Delta: first derivative with respect to spot
        let spot_bump = spot * self.delta_bump;
        let price_up = price_fn(spot + spot_bump, r, sigma, t, q);
        let price_down = price_fn(spot - spot_bump, r, sigma, t, q);
        let delta = (price_up - price_down) / (2.0 * spot_bump);
        
        // Gamma: second derivative with respect to spot
        let gamma = (price_up - 2.0 * base_price + price_down) / (spot_bump * spot_bump);
        
        // Vega: derivative with respect to volatility
        let vega_up = price_fn(spot, r, sigma + self.vega_bump, t, q);
        let vega_down = price_fn(spot, r, sigma - self.vega_bump, t, q);
        let vega = (vega_up - vega_down) / (2.0 * self.vega_bump);
        
        // Theta: derivative with respect to time (negative of time derivative)
        let t_bump = self.theta_bump / 365.25; // Convert days to years
        let theta = if t > t_bump {
            let price_later = price_fn(spot, r, sigma, t - t_bump, q);
            -(base_price - price_later) / t_bump * 365.25 // Convert to daily theta
        } else {
            0.0
        };
        
        // Rho: derivative with respect to interest rate
        let rho_up = price_fn(spot, r + self.rho_bump, sigma, t, q);
        let rho_down = price_fn(spot, r - self.rho_bump, sigma, t, q);
        let rho = (rho_up - rho_down) / (2.0 * self.rho_bump);
        
        Greeks::new(delta, gamma, vega, theta, rho)
    }
    
    /// Calculate extended Greeks including second-order sensitivities
    pub fn calculate_extended_greeks<F: Fn(f64, f64, f64, f64, f64) -> f64>(
        &self,
        price_fn: F,
        spot: f64,
        r: f64,
        sigma: f64,
        t: f64,
        q: f64,
    ) -> Greeks {
        // Calculate basic Greeks first
        let basic = self.calculate_greeks(&price_fn, spot, r, sigma, t, q);
        
        // Volga: second derivative with respect to volatility
        let base_price = price_fn(spot, r, sigma, t, q);
        let vega_up = price_fn(spot, r, sigma + self.vega_bump, t, q);
        let vega_down = price_fn(spot, r, sigma - self.vega_bump, t, q);
        let volga = (vega_up - 2.0 * base_price + vega_down) / (self.vega_bump * self.vega_bump);
        
        // Vanna: cross derivative of delta with respect to volatility
        let spot_bump = spot * self.delta_bump;
        let price_up_vol_up = price_fn(spot + spot_bump, r, sigma + self.vega_bump, t, q);
        let price_up_vol_down = price_fn(spot + spot_bump, r, sigma - self.vega_bump, t, q);
        let price_down_vol_up = price_fn(spot - spot_bump, r, sigma + self.vega_bump, t, q);
        let price_down_vol_down = price_fn(spot - spot_bump, r, sigma - self.vega_bump, t, q);
        
        let vanna = ((price_up_vol_up - price_up_vol_down) - (price_down_vol_up - price_down_vol_down))
            / (4.0 * spot_bump * self.vega_bump);
        
        // Charm: cross derivative of delta with respect to time
        let t_bump = self.theta_bump / 365.25;
        let charm = if t > t_bump {
            let price_up_later = price_fn(spot + spot_bump, r, sigma, t - t_bump, q);
            let price_down_later = price_fn(spot - spot_bump, r, sigma, t - t_bump, q);
            let delta_later = (price_up_later - price_down_later) / (2.0 * spot_bump);
            -(basic.delta - delta_later) / t_bump * 365.25
        } else {
            0.0
        };
        
        // Color: cross derivative of gamma with respect to time
        let color = if t > t_bump {
            let _price_up = price_fn(spot + spot_bump, r, sigma, t, q);
            let _price_down = price_fn(spot - spot_bump, r, sigma, t, q);
            let price_up_later = price_fn(spot + spot_bump, r, sigma, t - t_bump, q);
            let price_down_later = price_fn(spot - spot_bump, r, sigma, t - t_bump, q);
            let base_later = price_fn(spot, r, sigma, t - t_bump, q);
            
            let gamma_later = (price_up_later - 2.0 * base_later + price_down_later) / (spot_bump * spot_bump);
            -(basic.gamma - gamma_later) / t_bump * 365.25
        } else {
            0.0
        };
        
        // Speed: third derivative with respect to spot
        let h = spot_bump / 2.0;
        let price_3up = price_fn(spot + 3.0 * h, r, sigma, t, q);
        let price_up = price_fn(spot + h, r, sigma, t, q);
        let price_down = price_fn(spot - h, r, sigma, t, q);
        let price_3down = price_fn(spot - 3.0 * h, r, sigma, t, q);
        
        let speed = (price_3up - 3.0 * price_up + 3.0 * price_down - price_3down) / (8.0 * h * h * h);
        
        Greeks::full(
            basic.delta,
            basic.gamma,
            basic.vega,
            basic.theta,
            basic.rho,
            volga,
            vanna,
            charm,
            color,
            speed,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greeks_creation() {
        let greeks = Greeks::new(0.5, 0.02, 0.1, -0.05, 0.03);
        
        assert_eq!(greeks.delta, 0.5);
        assert_eq!(greeks.gamma, 0.02);
        assert_eq!(greeks.vega, 0.1);
        assert_eq!(greeks.theta, -0.05);
        assert_eq!(greeks.rho, 0.03);
        assert!(greeks.volga.is_none());
    }
    
    #[test]
    fn test_extended_greeks() {
        let greeks = Greeks::full(0.5, 0.02, 0.1, -0.05, 0.03, 0.01, 0.005, -0.002, -0.001, 0.0005);
        
        assert_eq!(greeks.volga, Some(0.01));
        assert_eq!(greeks.vanna, Some(0.005));
        assert_eq!(greeks.charm, Some(-0.002));
        assert_eq!(greeks.color, Some(-0.001));
        assert_eq!(greeks.speed, Some(0.0005));
    }
    
    #[test]
    fn test_greeks_calculator() {
        let calc = GreeksCalculator::new();
        
        // Simple test function: linear in spot
        let price_fn = |spot: f64, _r: f64, _sigma: f64, _t: f64, _q: f64| -> f64 {
            spot * 0.5
        };
        
        let greeks = calc.calculate_greeks(price_fn, 100.0, 0.05, 0.25, 1.0, 0.02);
        
        // For a linear function, delta should be constant (0.5) and gamma should be 0
        assert!((greeks.delta - 0.5).abs() < 0.01);
        assert!(greeks.gamma.abs() < 0.01);
    }
}
