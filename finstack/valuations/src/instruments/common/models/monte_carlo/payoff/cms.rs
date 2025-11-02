//! Constant Maturity Swap (CMS) cap and floor payoffs for Monte Carlo pricing.
//!
//! CMS products reference swap rates rather than LIBOR/SOFR, requiring
//! simulation of swap rates via Hull-White or other short rate models.

use super::super::pricer::swap_rate_utils::ForwardSwapRate;
use super::swaption::SwapSchedule;
use crate::instruments::common::mc::process::ou::HullWhite1FParams;
use crate::instruments::common::mc::traits::PathState;
use crate::instruments::common::models::monte_carlo::traits::Payoff;
use finstack_core::currency::Currency;
use finstack_core::money::Money;

/// CMS cap payoff (portfolio of CMS caplets).
///
/// Each caplet pays max(S_CMS(t) - K, 0) at fixing dates, where S_CMS(t)
/// is the CMS swap rate computed from the short rate.
#[derive(Clone, Debug)]
pub struct CmsCapPayoff {
    /// Strike rate (e.g., 0.04 for 4%)
    pub strike_rate: f64,
    /// CMS tenor (e.g., 10.0 for 10Y swap)
    pub cms_tenor: f64,
    /// Fixing dates (time in years)
    pub fixing_dates: Vec<f64>,
    /// Accrual fractions for each period
    pub accrual_fractions: Vec<f64>,
    /// Discount factors for each payment
    pub discount_factors: Vec<f64>,
    /// Notional amount
    pub notional: f64,
    /// Currency
    pub currency: Currency,
    /// Swap schedule for computing CMS rate
    pub swap_schedule: SwapSchedule,
    /// Hull-White parameters (needed for swap rate calculation)
    pub hw_params: HullWhite1FParams,

    // State variables
    accumulated_pv: f64,
    next_fixing_idx: usize,
}

impl CmsCapPayoff {
    /// Create a new CMS cap payoff.
    ///
    /// # Arguments
    ///
    /// * `strike_rate` - Cap strike rate
    /// * `cms_tenor` - Tenor of the CMS swap (e.g., 10.0 for 10Y)
    /// * `fixing_dates` - Dates when CMS rate is fixed
    /// * `accrual_fractions` - Daycount fractions
    /// * `discount_factors` - Discount factors for payments
    /// * `notional` - Notional amount
    /// * `currency` - Currency
    /// * `swap_schedule` - Schedule for the CMS swap
    /// * `hw_params` - Hull-White parameters for swap rate calculation
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        strike_rate: f64,
        cms_tenor: f64,
        fixing_dates: Vec<f64>,
        accrual_fractions: Vec<f64>,
        discount_factors: Vec<f64>,
        notional: f64,
        currency: Currency,
        swap_schedule: SwapSchedule,
        hw_params: HullWhite1FParams,
    ) -> Self {
        assert_eq!(
            fixing_dates.len(),
            accrual_fractions.len(),
            "Fixing dates and accrual fractions must match"
        );
        assert_eq!(
            fixing_dates.len(),
            discount_factors.len(),
            "Fixing dates and discount factors must match"
        );

        Self {
            strike_rate,
            cms_tenor,
            fixing_dates,
            accrual_fractions,
            discount_factors,
            notional,
            currency,
            swap_schedule,
            hw_params,
            accumulated_pv: 0.0,
            next_fixing_idx: 0,
        }
    }

    /// Compute convexity adjustment (simplified approximation).
    ///
    /// Convexity adjustment accounts for the difference between CMS rate
    /// and forward swap rate due to volatility.
    ///
    /// Delegates to `ForwardSwapRate::convexity_adjustment`.
    pub fn compute_convexity_adjustment(volatility: f64, tenor: f64, swap_tenor: f64) -> f64 {
        ForwardSwapRate::convexity_adjustment(volatility, tenor, swap_tenor)
    }
}

impl Payoff for CmsCapPayoff {
    fn on_event(&mut self, state: &PathState) {
        if self.next_fixing_idx < self.fixing_dates.len() {
            let target_time = self.fixing_dates[self.next_fixing_idx];

            // Check if we're at a fixing date
            if (state.time - target_time).abs() < 1e-6 || state.time >= target_time {
                // Get short rate from state
                let short_rate = state
                    .vars
                    .get(crate::instruments::common::mc::traits::state_keys::SHORT_RATE)
                    .copied()
                    .unwrap_or(0.0);

                // Compute CMS swap rate from short rate
                // Simple discount curve: DF(t) = exp(-r * t) for now
                // In production, use proper discount curve function
                let discount_fn = |t: f64| (-short_rate * t).exp();

                let cms_rate = ForwardSwapRate::compute(
                    &self.hw_params,
                    short_rate,
                    target_time,
                    &self.swap_schedule,
                    discount_fn,
                );

                // Caplet payoff: max(S_CMS - K, 0) * τ * N * DF
                let caplet = (cms_rate - self.strike_rate).max(0.0)
                    * self.accrual_fractions[self.next_fixing_idx]
                    * self.notional
                    * self.discount_factors[self.next_fixing_idx];

                self.accumulated_pv += caplet;
                self.next_fixing_idx += 1;
            }
        }
    }

    fn value(&self, currency: Currency) -> Money {
        Money::new(self.accumulated_pv, currency)
    }

    fn reset(&mut self) {
        self.accumulated_pv = 0.0;
        self.next_fixing_idx = 0;
    }
}

/// CMS floor payoff (portfolio of CMS floorlets).
///
/// Each floorlet pays max(K - S_CMS(t), 0) at fixing dates.
#[derive(Clone, Debug)]
pub struct CmsFloorPayoff {
    /// Strike rate
    pub strike_rate: f64,
    /// CMS tenor
    pub cms_tenor: f64,
    /// Fixing dates
    pub fixing_dates: Vec<f64>,
    /// Accrual fractions
    pub accrual_fractions: Vec<f64>,
    /// Discount factors
    pub discount_factors: Vec<f64>,
    /// Notional amount
    pub notional: f64,
    /// Currency
    pub currency: Currency,
    /// Swap schedule
    pub swap_schedule: SwapSchedule,
    /// Hull-White parameters
    pub hw_params: HullWhite1FParams,

    // State variables
    accumulated_pv: f64,
    next_fixing_idx: usize,
}

impl CmsFloorPayoff {
    /// Create a new CMS floor payoff.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        strike_rate: f64,
        cms_tenor: f64,
        fixing_dates: Vec<f64>,
        accrual_fractions: Vec<f64>,
        discount_factors: Vec<f64>,
        notional: f64,
        currency: Currency,
        swap_schedule: SwapSchedule,
        hw_params: HullWhite1FParams,
    ) -> Self {
        assert_eq!(
            fixing_dates.len(),
            accrual_fractions.len(),
            "Fixing dates and accrual fractions must match"
        );
        assert_eq!(
            fixing_dates.len(),
            discount_factors.len(),
            "Fixing dates and discount factors must match"
        );

        Self {
            strike_rate,
            cms_tenor,
            fixing_dates,
            accrual_fractions,
            discount_factors,
            notional,
            currency,
            swap_schedule,
            hw_params,
            accumulated_pv: 0.0,
            next_fixing_idx: 0,
        }
    }
}

impl Payoff for CmsFloorPayoff {
    fn on_event(&mut self, state: &PathState) {
        if self.next_fixing_idx < self.fixing_dates.len() {
            let target_time = self.fixing_dates[self.next_fixing_idx];

            if (state.time - target_time).abs() < 1e-6 || state.time >= target_time {
                let short_rate = state
                    .vars
                    .get(crate::instruments::common::mc::traits::state_keys::SHORT_RATE)
                    .copied()
                    .unwrap_or(0.0);

                let discount_fn = |t: f64| (-short_rate * t).exp();

                let cms_rate = ForwardSwapRate::compute(
                    &self.hw_params,
                    short_rate,
                    target_time,
                    &self.swap_schedule,
                    discount_fn,
                );

                // Floorlet payoff: max(K - S_CMS, 0) * τ * N * DF
                let floorlet = (self.strike_rate - cms_rate).max(0.0)
                    * self.accrual_fractions[self.next_fixing_idx]
                    * self.notional
                    * self.discount_factors[self.next_fixing_idx];

                self.accumulated_pv += floorlet;
                self.next_fixing_idx += 1;
            }
        }
    }

    fn value(&self, currency: Currency) -> Money {
        Money::new(self.accumulated_pv, currency)
    }

    fn reset(&mut self) {
        self.accumulated_pv = 0.0;
        self.next_fixing_idx = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::common::mc::process::ou::HullWhite1FParams;

    #[test]
    fn test_cms_cap_creation() {
        let fixing_dates = vec![0.25, 0.5, 0.75, 1.0];
        let accruals = vec![0.25, 0.25, 0.25, 0.25];
        let dfs = vec![0.99, 0.98, 0.97, 0.96];

        // Swap schedule needs accruals matching payment dates length
        let payment_dates = vec![1.0, 1.25, 1.5, 1.75, 2.0];
        let schedule_accruals = vec![0.25, 0.25, 0.25, 0.25, 0.25];
        let schedule = SwapSchedule::new(1.0, 2.0, payment_dates, schedule_accruals);

        let hw_params = HullWhite1FParams::new(0.1, 0.01, 0.03);

        let cap = CmsCapPayoff::new(
            0.04, // Strike
            10.0, // 10Y CMS
            fixing_dates,
            accruals,
            dfs,
            10_000_000.0,
            Currency::USD,
            schedule,
            hw_params,
        );

        assert_eq!(cap.strike_rate, 0.04);
        assert_eq!(cap.cms_tenor, 10.0);
    }

    #[test]
    fn test_cms_cap_reset() {
        let fixing_dates = vec![0.25];
        let accruals = vec![0.25];
        let dfs = vec![0.99];
        // Swap schedule needs accruals matching payment dates length
        let payment_dates = vec![1.0, 1.25];
        let schedule_accruals = vec![0.25, 0.25];
        let schedule = SwapSchedule::new(1.0, 1.25, payment_dates, schedule_accruals);
        let hw_params = HullWhite1FParams::new(0.1, 0.01, 0.03);

        let mut cap = CmsCapPayoff::new(
            0.04,
            10.0,
            fixing_dates,
            accruals,
            dfs,
            1.0,
            Currency::USD,
            schedule,
            hw_params,
        );

        cap.accumulated_pv = 100.0;
        cap.next_fixing_idx = 1;

        cap.reset();

        assert_eq!(cap.accumulated_pv, 0.0);
        assert_eq!(cap.next_fixing_idx, 0);
    }
}
