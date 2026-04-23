//! FX barrier option payoffs with quanto adjustments.
//!
//! Extends the barrier framework for FX options, including quanto barriers
//! where the barrier and/or payoff are in different currencies.

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::Result;
use finstack_monte_carlo::payoff::barrier::{BarrierOptionPayoff, BarrierType, OptionKind};
use finstack_monte_carlo::traits::PathState;
use finstack_monte_carlo::traits::Payoff;

/// FX barrier call option with quanto support.
///
/// Similar to `BarrierOptionPayoff` but designed for FX markets with optional quanto
/// adjustments for correlation between FX rate and domestic/foreign rates.
///
/// # FX Barrier Types
///
/// - **Up-and-out**: Option knocked out if FX rate rises above barrier
/// - **Up-and-in**: Option activated if FX rate rises above barrier
/// - **Down-and-out**: Option knocked out if FX rate falls below barrier
/// - **Down-and-in**: Option activated if FX rate falls below barrier
///
/// # Quanto Barriers
///
/// When barrier monitoring and payoff settlement are in different currencies,
/// the correlation between FX rate and underlying affects pricing. This is
/// handled via quanto adjustment in the drift of the FX process.
#[derive(Debug, Clone)]
pub struct FxBarrierCall {
    /// Underlying barrier call (reuses existing infrastructure)
    inner: BarrierOptionPayoff,
    /// Base currency (underlying currency, formerly foreign_currency)
    pub base_currency: Currency,
    /// Quote currency (settlement currency, formerly domestic_currency)
    pub quote_currency: Currency,
    /// Quanto adjustment factor (pre-computed)
    pub quanto_adjustment: f64,
}

impl FxBarrierCall {
    /// Create a new FX barrier call option.
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price (in foreign currency units)
    /// * `barrier` - Barrier level (in foreign currency units)
    /// * `barrier_type` - Type of barrier (up/down, in/out)
    /// * `notional` - Notional amount
    /// * `maturity_step` - Step index at maturity
    /// * `sigma` - FX volatility
    /// * `dt` - Time step size
    /// * `use_gobet_miri` - Use Gobet-Miri barrier adjustment
    /// * `domestic_currency` - Settlement currency
    /// * `foreign_currency` - Underlying currency
    /// * `quanto_adjustment` - Quanto adjustment factor (0.0 if not quanto)
    /// * `rebate` - Optional rebate paid at maturity if barrier condition met
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        strike: f64,
        barrier: f64,
        barrier_type: BarrierType,
        notional: f64,
        maturity_step: usize,
        sigma: f64,
        dt: f64,
        use_gobet_miri: bool,
        base_currency: Currency,
        quote_currency: Currency,
        quanto_adjustment: f64,
        rebate: Option<f64>,
    ) -> Result<Self> {
        let time_grid = finstack_monte_carlo::time_grid::TimeGrid::uniform(
            dt * maturity_step as f64,
            maturity_step,
        )?;
        let inner = BarrierOptionPayoff::new(
            strike,
            barrier,
            barrier_type,
            OptionKind::Call,
            rebate,
            notional,
            maturity_step,
            sigma,
            &time_grid,
            use_gobet_miri,
        );

        Ok(Self {
            inner,
            base_currency,
            quote_currency,
            quanto_adjustment,
        })
    }

    /// Create a standard FX barrier (no quanto adjustment).
    #[allow(clippy::too_many_arguments)]
    pub fn standard(
        strike: f64,
        barrier: f64,
        barrier_type: BarrierType,
        notional: f64,
        maturity_step: usize,
        sigma: f64,
        dt: f64,
        base_currency: Currency,
        quote_currency: Currency,
    ) -> Result<Self> {
        Self::new(
            strike,
            barrier,
            barrier_type,
            notional,
            maturity_step,
            sigma,
            dt,
            true, // Use Gobet-Miri by default
            base_currency,
            quote_currency,
            0.0, // No quanto adjustment
            None,
        )
    }

    /// Create a quanto FX barrier with adjustment.
    ///
    /// # Arguments
    ///
    /// * `quanto_adjustment` - Pre-computed quanto adjustment: r_for - q - ρ σ_FX σ_S
    #[allow(clippy::too_many_arguments)]
    pub fn quanto(
        strike: f64,
        barrier: f64,
        barrier_type: BarrierType,
        notional: f64,
        maturity_step: usize,
        sigma: f64,
        dt: f64,
        base_currency: Currency,
        quote_currency: Currency,
        quanto_adjustment: f64,
    ) -> Result<Self> {
        Self::new(
            strike,
            barrier,
            barrier_type,
            notional,
            maturity_step,
            sigma,
            dt,
            true,
            base_currency,
            quote_currency,
            quanto_adjustment,
            None,
        )
    }
}

impl Payoff for FxBarrierCall {
    fn on_event(&mut self, state: &mut PathState) {
        // Delegate to inner barrier call
        // FX rate should be stored in state as "spot" or "fx_rate"
        self.inner.on_event(state);
    }

    fn value(&self, currency: Currency) -> Money {
        // Get base payoff from inner barrier call
        // For quanto barriers, adjustment is already applied to drift
        // So we just return the base payoff
        // In more sophisticated implementations, we might need to apply
        // additional quanto corrections here
        self.inner.value(currency)
    }

    fn reset(&mut self) {
        self.inner.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fx_barrier_standard_creation() {
        let fx_barrier = FxBarrierCall::standard(
            1.15,
            1.20,
            BarrierType::UpAndOut,
            1_000_000.0,
            100,
            0.12,
            0.01,
            Currency::EUR,
            Currency::USD,
        )
        .expect("valid standard FX barrier should construct");

        assert_eq!(fx_barrier.base_currency, Currency::EUR);
        assert_eq!(fx_barrier.quote_currency, Currency::USD);
        assert_eq!(fx_barrier.quanto_adjustment, 0.0);
    }

    #[test]
    fn test_fx_barrier_quanto_creation() {
        let quanto_adj = 0.0172;
        let fx_barrier = FxBarrierCall::quanto(
            1.15,
            1.20,
            BarrierType::UpAndOut,
            1_000_000.0,
            100,
            0.12,
            0.01,
            Currency::EUR,
            Currency::USD,
            quanto_adj,
        )
        .expect("valid quanto FX barrier should construct");

        assert_eq!(fx_barrier.quanto_adjustment, quanto_adj);
    }
}
