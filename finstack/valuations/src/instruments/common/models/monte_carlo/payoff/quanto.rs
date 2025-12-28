//! Quanto option payoffs for Monte Carlo pricing.
//!
//! Quanto options have payoffs that depend on an underlying asset in one currency
//! but are settled in another currency, creating FX exposure.

use crate::instruments::common::mc::traits::{state_keys, PathState};
use crate::instruments::common::models::monte_carlo::traits::Payoff;
use finstack_core::currency::Currency;
use finstack_core::money::Money;

/// Quanto call option payoff.
///
/// A quanto call pays max(S_T - K, 0) in domestic currency, where:
/// - S_T is the equity price in foreign currency
/// - K is the strike in foreign currency
/// - The payoff is converted to domestic currency via FX rate
///
/// The quanto adjustment accounts for correlation between equity and FX rates.
#[derive(Clone, Debug)]
pub struct QuantoCallPayoff {
    /// Equity strike price (in foreign currency)
    pub equity_strike: f64,
    /// Notional amount
    pub notional: f64,
    /// Domestic currency (settlement currency)
    pub domestic_currency: Currency,
    /// Foreign currency (underlying currency)
    pub foreign_currency: Currency,
    /// Quanto adjustment factor: r_for - q - ρ σ_S σ_FX
    /// Pre-computed to avoid repeated calculation
    pub quanto_adjustment: f64,

    // State variables (tracked during path simulation)
    /// Terminal equity spot (in foreign currency)
    terminal_equity: f64,
    /// Terminal FX rate (domestic/foreign, e.g., USD/EUR)
    terminal_fx: f64,
}

impl QuantoCallPayoff {
    /// Create a new quanto call payoff.
    ///
    /// # Arguments
    ///
    /// * `equity_strike` - Strike price in foreign currency
    /// * `notional` - Notional amount
    /// * `domestic_currency` - Settlement currency (e.g., USD)
    /// * `foreign_currency` - Underlying currency (e.g., EUR)
    /// * `quanto_adjustment` - Pre-computed quanto adjustment: r_for - q - ρ σ_S σ_FX
    pub fn new(
        equity_strike: f64,
        notional: f64,
        domestic_currency: Currency,
        foreign_currency: Currency,
        quanto_adjustment: f64,
    ) -> Self {
        Self {
            equity_strike,
            notional,
            domestic_currency,
            foreign_currency,
            quanto_adjustment,
            terminal_equity: 0.0,
            terminal_fx: 0.0,
        }
    }

    /// Compute quanto adjustment factor.
    ///
    /// # Arguments
    ///
    /// * `r_foreign` - Foreign risk-free rate
    /// * `q` - Dividend yield
    /// * `rho` - Correlation between equity and FX
    /// * `sigma_equity` - Equity volatility
    /// * `sigma_fx` - FX volatility
    ///
    /// # Returns
    ///
    /// Quanto adjustment: r_for - q - ρ σ_S σ_FX
    pub fn compute_quanto_adjustment(
        r_foreign: f64,
        q: f64,
        rho: f64,
        sigma_equity: f64,
        sigma_fx: f64,
    ) -> f64 {
        r_foreign - q - rho * sigma_equity * sigma_fx
    }
}

impl Payoff for QuantoCallPayoff {
    fn on_event(&mut self, state: &mut PathState) {
        // Track equity spot and FX rate at maturity
        if let Some(&equity) = state.vars.get(state_keys::SPOT) {
            self.terminal_equity = equity;
        }

        // FX rate is stored using state_keys::FX_RATE for multi-asset processes
        // Check for FX rate key
        if let Some(&fx_rate) = state.vars.get(state_keys::FX_RATE) {
            self.terminal_fx = fx_rate;
        } else if let Some(&fx_rate) = state.vars.get("fx_rate") {
            self.terminal_fx = fx_rate;
        } else {
            // Default: assume FX rate of 1.0 if not found
            // In practice, this should be set by the process/discretization
            // For multi-asset processes, the pricer/engine should set FX_RATE
            self.terminal_fx = 1.0;
        }
    }

    fn value(&self, currency: Currency) -> Money {
        // Payoff in foreign currency units: max(S_T - K, 0)
        let payoff_fx_units = (self.terminal_equity - self.equity_strike).max(0.0);

        // Convert to domestic currency: payoff_dom = payoff_fx * FX_T * notional
        // Note: quanto adjustment is already applied to the drift, so we just convert here
        let payoff_dom = payoff_fx_units * self.terminal_fx * self.notional;

        Money::new(payoff_dom, currency)
    }

    fn reset(&mut self) {
        self.terminal_equity = 0.0;
        self.terminal_fx = 0.0;
    }
}

/// Quanto put option payoff.
///
/// A quanto put pays max(K - S_T, 0) in domestic currency.
#[derive(Clone, Debug)]
pub struct QuantoPutPayoff {
    /// Equity strike price (in foreign currency)
    pub equity_strike: f64,
    /// Notional amount
    pub notional: f64,
    /// Domestic currency (settlement currency)
    pub domestic_currency: Currency,
    /// Foreign currency (underlying currency)
    pub foreign_currency: Currency,
    /// Quanto adjustment factor
    pub quanto_adjustment: f64,

    // State variables
    terminal_equity: f64,
    terminal_fx: f64,
}

impl QuantoPutPayoff {
    /// Create a new quanto put payoff.
    pub fn new(
        equity_strike: f64,
        notional: f64,
        domestic_currency: Currency,
        foreign_currency: Currency,
        quanto_adjustment: f64,
    ) -> Self {
        Self {
            equity_strike,
            notional,
            domestic_currency,
            foreign_currency,
            quanto_adjustment,
            terminal_equity: 0.0,
            terminal_fx: 0.0,
        }
    }
}

impl Payoff for QuantoPutPayoff {
    fn on_event(&mut self, state: &mut PathState) {
        if let Some(&equity) = state.vars.get(state_keys::SPOT) {
            self.terminal_equity = equity;
        }

        if let Some(&fx_rate) = state.vars.get(state_keys::FX_RATE) {
            self.terminal_fx = fx_rate;
        } else if let Some(&fx_rate) = state.vars.get("fx_rate") {
            self.terminal_fx = fx_rate;
        } else {
            self.terminal_fx = 1.0;
        }
    }

    fn value(&self, currency: Currency) -> Money {
        // Payoff in foreign currency units: max(K - S_T, 0)
        let payoff_fx_units = (self.equity_strike - self.terminal_equity).max(0.0);

        // Convert to domestic currency
        let payoff_dom = payoff_fx_units * self.terminal_fx * self.notional;

        Money::new(payoff_dom, currency)
    }

    fn reset(&mut self) {
        self.terminal_equity = 0.0;
        self.terminal_fx = 0.0;
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common::mc::traits::{state_keys, PathState};

    #[test]
    fn test_quanto_adjustment_computation() {
        let adj = QuantoCallPayoff::compute_quanto_adjustment(
            0.03, // r_foreign
            0.02, // q
            -0.3, // rho (negative correlation)
            0.20, // sigma_equity
            0.12, // sigma_fx
        );

        // adj = 0.03 - 0.02 - (-0.3) * 0.20 * 0.12
        //     = 0.01 + 0.0072 = 0.0172
        let expected = 0.03 - 0.02 - (-0.3) * 0.20 * 0.12;
        assert!((adj - expected).abs() < 1e-10);
    }

    #[test]
    fn test_quanto_call_payoff() {
        let mut payoff = QuantoCallPayoff::new(
            4000.0, // Strike
            1.0,    // Notional
            Currency::USD,
            Currency::EUR,
            0.0172, // Quanto adjustment
        );

        // Simulate terminal state
        let mut state = PathState::new(100, 1.0);
        state.set(state_keys::SPOT, 4200.0); // Equity spot
        state.set("fx_rate", 1.10); // EUR/USD = 1.10

        payoff.on_event(&mut state);

        let value = payoff.value(Currency::USD);
        // Payoff = max(4200 - 4000, 0) * 1.10 * 1.0 = 200 * 1.10 = 220
        assert!((value.amount() - 220.0).abs() < 1e-10);
    }

    #[test]
    fn test_quanto_put_payoff() {
        let mut payoff = QuantoPutPayoff::new(4000.0, 1.0, Currency::USD, Currency::EUR, 0.0172);

        let mut state = PathState::new(100, 1.0);
        state.set(state_keys::SPOT, 3800.0); // Equity below strike
        state.set("fx_rate", 1.10);

        payoff.on_event(&mut state);

        let value = payoff.value(Currency::USD);
        // Payoff = max(4000 - 3800, 0) * 1.10 * 1.0 = 200 * 1.10 = 220
        assert!((value.amount() - 220.0).abs() < 1e-10);
    }

    #[test]
    fn test_quanto_reset() {
        let mut payoff = QuantoCallPayoff::new(4000.0, 1.0, Currency::USD, Currency::EUR, 0.0172);

        let mut state = PathState::new(100, 1.0);
        state.set(state_keys::SPOT, 4200.0);
        state.set("fx_rate", 1.10);

        payoff.on_event(&mut state);
        assert_eq!(payoff.terminal_equity, 4200.0);
        assert_eq!(payoff.terminal_fx, 1.10);

        payoff.reset();
        assert_eq!(payoff.terminal_equity, 0.0);
        assert_eq!(payoff.terminal_fx, 0.0);
    }
}
