//! First-passage time default calculator for credit instruments.
//!
//! Implements market-standard first-passage time default modeling using
//! cumulative hazard tracking. Separates default detection logic from
//! cashflow generation for cleaner architecture and testability.
//!
//! # Mathematical Foundation
//!
//! Default occurs when cumulative hazard exceeds an exponential threshold:
//! ```text
//! Λ(t) = ∫₀ᵗ λ(s) ds > E, where E ~ Exp(1) = -ln(U), U ~ Uniform(0,1)
//! ```
//!
//! The hazard rate λ(t) is derived from credit spreads:
//! ```text
//! λ(t) = s(t) / (1 - R)
//! where s(t) is the credit spread and R is the recovery rate
//! ```
//!
//! # References
//!
//! - Hull, J. C. (2018). *Options, Futures, and Other Derivatives* (10th ed.).
//!   Chapter 24: Credit Derivatives.
//! - Brigo, D., & Mercurio, F. (2006). *Interest Rate Models - Theory and Practice*
//!   (2nd ed.). Chapter 21: Credit Risk Modeling.

/// Event returned by default calculator after each update.
#[derive(Clone, Debug, PartialEq)]
pub enum DefaultEvent {
    /// No default occurred in this update
    NoDefault,
    
    /// Default occurred at the specified time
    DefaultOccurred {
        /// Time when default occurred
        time: f64,
        /// Recovery rate (fraction of outstanding recovered)
        recovery_fraction: f64,
    },
}

/// First-passage time default calculator.
///
/// Tracks cumulative hazard rate and detects default when the integrated
/// hazard exceeds a randomly drawn exponential threshold. This is the
/// industry-standard approach for credit risk modeling in Monte Carlo.
///
/// # Example
///
/// ```rust
/// use finstack_valuations::instruments::common::models::monte_carlo::payoff::default_calculator::{
///     FirstPassageCalculator, DefaultEvent
/// };
///
/// let mut calc = FirstPassageCalculator::new(0.4); // 40% recovery
/// calc.set_threshold(2.5); // E ~ Exp(1)
///
/// // Update with credit spread over a time period
/// let event = calc.update(0.02, 0.25, 0.25); // 200bp spread, 3 months
/// match event {
///     DefaultEvent::NoDefault => println!("No default yet"),
///     DefaultEvent::DefaultOccurred { time, recovery_fraction } => {
///         println!("Defaulted at t={}, recovery={}", time, recovery_fraction);
///     }
/// }
/// ```
#[derive(Clone, Debug)]
pub struct FirstPassageCalculator {
    /// Recovery rate (e.g., 0.4 for 40% recovery)
    recovery_rate: f64,
    
    /// Exponential threshold: E ~ Exp(1) = -ln(U)
    threshold: f64,
    
    /// Cumulative hazard: Λ(t) = ∫₀ᵗ λ(s) ds
    cumulative_hazard: f64,
    
    /// Whether default has occurred
    defaulted: bool,
    
    /// Time when default occurred (if any)
    default_time: Option<f64>,
}

impl FirstPassageCalculator {
    /// Create a new first-passage calculator.
    ///
    /// # Arguments
    ///
    /// * `recovery_rate` - Recovery rate on default (e.g., 0.4 for 40%)
    ///
    /// # Panics
    ///
    /// Panics if recovery_rate is not in [0, 1]
    pub fn new(recovery_rate: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&recovery_rate),
            "Recovery rate must be in [0, 1], got {}",
            recovery_rate
        );
        
        Self {
            recovery_rate,
            threshold: 0.0,
            cumulative_hazard: 0.0,
            defaulted: false,
            default_time: None,
        }
    }
    
    /// Set the default threshold for this path.
    ///
    /// Should be called at the start of each Monte Carlo path with a draw
    /// from Exp(1): `threshold = -ln(U)` where U ~ Uniform(0,1).
    ///
    /// # Arguments
    ///
    /// * `threshold` - Exponential random variable E ~ Exp(1)
    pub fn set_threshold(&mut self, threshold: f64) {
        self.threshold = threshold.max(1e-10);
    }
    
    /// Update cumulative hazard and check for default.
    ///
    /// Integrates the hazard rate over the time interval [t-dt, t] and
    /// checks if cumulative hazard exceeds the threshold.
    ///
    /// # Arguments
    ///
    /// * `credit_spread` - Current credit spread (annualized, e.g., 0.02 for 200bp)
    /// * `dt` - Time step size in years
    /// * `current_time` - Current time in years
    ///
    /// # Returns
    ///
    /// `DefaultEvent` indicating whether default occurred in this interval
    ///
    /// # Hazard Rate Conversion
    ///
    /// Converts credit spread to hazard rate using:
    /// ```text
    /// λ = s / (1 - R)
    /// ```
    /// where s is the credit spread and R is the recovery rate.
    pub fn update(
        &mut self,
        credit_spread: f64,
        dt: f64,
        current_time: f64,
    ) -> DefaultEvent {
        // If already defaulted, return NoDefault (no further events)
        if self.defaulted {
            return DefaultEvent::NoDefault;
        }
        
        // Skip negative or zero time steps
        if dt <= 0.0 {
            return DefaultEvent::NoDefault;
        }
        
        // Convert credit spread to hazard rate: λ = s / (1 - R)
        // Clamp denominator to prevent numerical instability at high recovery
        let spread = credit_spread.max(0.0);
        let one_minus_recovery = (1.0 - self.recovery_rate).max(1e-6);
        let hazard_rate = spread / one_minus_recovery;
        
        // Integrate hazard over time interval using simple rectangular rule
        // (credit spread assumed constant over dt)
        self.cumulative_hazard += hazard_rate * dt;
        
        // Check if default occurred
        if self.cumulative_hazard >= self.threshold {
            self.defaulted = true;
            self.default_time = Some(current_time);
            
            return DefaultEvent::DefaultOccurred {
                time: current_time,
                recovery_fraction: self.recovery_rate,
            };
        }
        
        DefaultEvent::NoDefault
    }
    
    /// Check if default has occurred.
    pub fn is_defaulted(&self) -> bool {
        self.defaulted
    }
    
    /// Get the time when default occurred (if any).
    pub fn default_time(&self) -> Option<f64> {
        self.default_time
    }
    
    /// Get current cumulative hazard.
    pub fn cumulative_hazard(&self) -> f64 {
        self.cumulative_hazard
    }
    
    /// Reset the calculator for a new path.
    ///
    /// Resets cumulative hazard and default state, but preserves the
    /// threshold (set by on_path_start) and recovery rate.
    ///
    /// NOTE: The threshold is set by `on_path_start()` BEFORE `reset()` is called,
    /// so we must preserve it. Only reset the accumulated state (hazard, default flag).
    pub fn reset(&mut self) {
        // DO NOT reset threshold - it's set by on_path_start() before reset()
        // self.threshold = 0.0;  // WRONG - would lose the random threshold!
        self.cumulative_hazard = 0.0;
        self.defaulted = false;
        self.default_time = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_new_calculator() {
        let calc = FirstPassageCalculator::new(0.4);
        assert_eq!(calc.recovery_rate, 0.4);
        assert_eq!(calc.cumulative_hazard, 0.0);
        assert!(!calc.is_defaulted());
        assert!(calc.default_time().is_none());
    }
    
    #[test]
    #[should_panic(expected = "Recovery rate must be in [0, 1]")]
    fn test_invalid_recovery_rate() {
        FirstPassageCalculator::new(1.5);
    }
    
    #[test]
    fn test_set_threshold() {
        let mut calc = FirstPassageCalculator::new(0.4);
        calc.set_threshold(2.5);
        assert_eq!(calc.threshold, 2.5);
        
        // Should clamp negative to small positive
        calc.set_threshold(-1.0);
        assert!(calc.threshold > 0.0);
    }
    
    #[test]
    fn test_no_default_low_hazard() {
        let mut calc = FirstPassageCalculator::new(0.4);
        calc.set_threshold(10.0); // High threshold
        
        // Low spread over short period
        let event = calc.update(0.01, 0.25, 0.25); // 100bp, 3 months
        assert_eq!(event, DefaultEvent::NoDefault);
        assert!(!calc.is_defaulted());
        
        // Cumulative hazard should be: 0.01 / (1 - 0.4) * 0.25 = 0.004167
        assert!((calc.cumulative_hazard() - 0.004167).abs() < 1e-5);
    }
    
    #[test]
    fn test_default_occurs() {
        let mut calc = FirstPassageCalculator::new(0.4);
        calc.set_threshold(0.1); // Low threshold
        
        // High spread over short period
        let event = calc.update(0.10, 1.0, 1.0); // 1000bp, 1 year
        
        // Hazard rate: 0.10 / (1 - 0.4) = 0.1667
        // Cumulative hazard: 0.1667 * 1.0 = 0.1667 > 0.1 threshold
        match event {
            DefaultEvent::DefaultOccurred { time, recovery_fraction } => {
                assert_eq!(time, 1.0);
                assert_eq!(recovery_fraction, 0.4);
            }
            _ => panic!("Expected default to occur"),
        }
        
        assert!(calc.is_defaulted());
        assert_eq!(calc.default_time(), Some(1.0));
    }
    
    #[test]
    fn test_no_further_defaults_after_first() {
        let mut calc = FirstPassageCalculator::new(0.4);
        calc.set_threshold(0.05);
        
        // First update triggers default
        let event1 = calc.update(0.05, 1.0, 1.0);
        assert!(matches!(event1, DefaultEvent::DefaultOccurred { .. }));
        
        // Second update should return NoDefault (already defaulted)
        let event2 = calc.update(0.05, 1.0, 2.0);
        assert_eq!(event2, DefaultEvent::NoDefault);
    }
    
    #[test]
    fn test_reset() {
        let mut calc = FirstPassageCalculator::new(0.4);
        calc.set_threshold(0.05);  // Low threshold to trigger default
        
        // Trigger default (hazard = 0.10/0.6 * 1.0 = 0.1667 > 0.05)
        let _ = calc.update(0.10, 1.0, 1.0);
        assert!(calc.is_defaulted());
        
        // Reset
        calc.reset();
        assert!(!calc.is_defaulted());
        assert_eq!(calc.cumulative_hazard(), 0.0);
        assert!(calc.default_time().is_none());
        
        // Threshold should be PRESERVED (not reset to 0)
        assert_eq!(calc.threshold, 0.05);
        
        // Recovery rate should be preserved
        assert_eq!(calc.recovery_rate, 0.4);
    }
    
    #[test]
    fn test_zero_spread_no_default() {
        let mut calc = FirstPassageCalculator::new(0.4);
        calc.set_threshold(0.01);
        
        // Zero spread means zero hazard
        let event = calc.update(0.0, 10.0, 10.0);
        assert_eq!(event, DefaultEvent::NoDefault);
        assert_eq!(calc.cumulative_hazard(), 0.0);
    }
    
    #[test]
    fn test_cumulative_hazard_builds_up() {
        let mut calc = FirstPassageCalculator::new(0.4);
        calc.set_threshold(1.0);
        
        // Multiple updates accumulate hazard
        calc.update(0.01, 0.25, 0.25); // += 0.01/0.6 * 0.25 = 0.004167
        calc.update(0.02, 0.25, 0.50); // += 0.02/0.6 * 0.25 = 0.008333
        calc.update(0.03, 0.25, 0.75); // += 0.03/0.6 * 0.25 = 0.0125
        
        // Total: 0.004167 + 0.008333 + 0.0125 = 0.025
        assert!((calc.cumulative_hazard() - 0.025).abs() < 1e-5);
    }
    
    #[test]
    fn test_high_recovery_rate() {
        let mut calc = FirstPassageCalculator::new(0.99); // Very high recovery
        calc.set_threshold(0.1);
        
        // High recovery means higher hazard for same spread
        // λ = 0.01 / (1 - 0.99) = 0.01 / 0.01 = 1.0
        let event = calc.update(0.01, 0.15, 0.15); // Should trigger: 1.0 * 0.15 = 0.15 > 0.1
        
        match event {
            DefaultEvent::DefaultOccurred { recovery_fraction, .. } => {
                assert_eq!(recovery_fraction, 0.99);
            }
            _ => panic!("Expected default with high recovery rate"),
        }
    }
    
    #[test]
    fn test_realistic_low_spread_no_default() {
        let mut calc = FirstPassageCalculator::new(0.40);
        
        // Typical threshold from Exp(1) distribution (mean = 1.0)
        calc.set_threshold(1.0);
        
        // Very low spread: 1bp = 0.0001
        // λ = 0.0001 / 0.6 = 0.000167
        // Over 1 year (4 quarterly steps): cumulative = 0.000167
        // P(default) = 1 - exp(-0.000167) ≈ 0.0167% (basically zero)
        
        for _ in 0..4 {
            let event = calc.update(0.0001, 0.25, 0.25);
            assert_eq!(event, DefaultEvent::NoDefault, "Should not default with 1bp spread");
        }
        
        // Cumulative should be tiny
        assert!(calc.cumulative_hazard() < 0.001, 
                "Cumulative hazard should be very small, got {}", calc.cumulative_hazard());
    }
    
    #[test]
    fn test_realistic_moderate_spread_low_default() {
        let mut calc = FirstPassageCalculator::new(0.40);
        
        // Moderate threshold
        calc.set_threshold(1.0);
        
        // 150bp spread
        // λ = 0.015 / 0.6 = 0.025
        // Over 1 year: cumulative = 0.025
        // P(default) = 1 - exp(-0.025) ≈ 2.47%
        
        let event = calc.update(0.015, 1.0, 1.0);
        assert_eq!(event, DefaultEvent::NoDefault, "Should not default with threshold=1.0 > cumulative=0.025");
        
        assert!((calc.cumulative_hazard() - 0.025).abs() < 1e-5,
                "Cumulative hazard should be 0.025, got {}", calc.cumulative_hazard());
    }
}


