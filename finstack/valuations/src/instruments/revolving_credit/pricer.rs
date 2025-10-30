//! Pricing engine for revolving credit facilities.
//!
//! Provides both deterministic and Monte Carlo pricing implementations.

use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

use super::cashflows::generate_deterministic_cashflows;
use super::types::RevolvingCredit;

/// Discounting pricer for revolving credit facilities with deterministic cashflows.
///
/// This pricer generates cashflows using the facility's deterministic schedule
/// and discounts them using the discount curve.
#[derive(Default)]
pub struct RevolvingCreditDiscountingPricer;

impl RevolvingCreditDiscountingPricer {
    /// Create a new revolving credit discounting pricer.
    pub fn new() -> Self {
        Self
    }

    /// Price a revolving credit facility using deterministic cashflows. 
    /// TODO: This is a placeholder for the actual mc pricing logic
    pub fn price_deterministic(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        // Generate cashflows
        let schedule = generate_deterministic_cashflows(facility, as_of)?;

        // Get discount curve
        let disc = market.get_discount_ref(facility.disc_id.as_str())?;

        // Discount all cashflows
        let mut pv = Money::new(0.0, facility.commitment_amount.currency());

        let disc_dc = disc.day_count();
        let base_date = disc.base_date();

        // Compute as_of discount factor
        let t_as_of = disc_dc.year_fraction(
            base_date,
            as_of,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let df_as_of = disc.df(t_as_of);

        for cf in &schedule.flows {
            // Skip past cashflows
            if cf.date <= as_of {
                continue;
            }

            // Compute discount factor from as_of
            let t_cf = disc_dc.year_fraction(
                base_date,
                cf.date,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let df_cf_abs = disc.df(t_cf);
            let df = if df_as_of != 0.0 {
                df_cf_abs / df_as_of
            } else {
                1.0
            };

            let discounted = cf.amount * df;
            pv = pv.checked_add(discounted)?;
        }

        Ok(pv)
    }
}

impl Pricer for RevolvingCreditDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let facility = instrument
            .as_any()
            .downcast_ref::<RevolvingCredit>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::RevolvingCredit,
                got: instrument.key(),
            })?;

        // Validate that we have a deterministic spec
        if !facility.is_deterministic() {
            return Err(PricingError::ModelFailure(
                "RevolvingCreditDiscountingPricer requires deterministic cashflows".to_string(),
            ));
        }

        // Extract valuation date from discount curve
        let disc = market
            .get_discount_ref(facility.disc_id.as_str())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Price the facility
        let pv = Self::price_deterministic(facility, market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(facility.id(), as_of, pv))
    }
}

/// Monte Carlo pricer for revolving credit facilities with stochastic utilization.
///
/// This pricer simulates utilization paths using a mean-reverting process and
/// averages the discounted cashflows across all paths to compute the expected PV.
///
/// **Note**: Requires the `mc` feature to be enabled.
#[cfg(feature = "mc")]
#[derive(Default)]
pub struct RevolvingCreditMcPricer;

#[cfg(feature = "mc")]
impl RevolvingCreditMcPricer {
    /// Create a new revolving credit Monte Carlo pricer.
    pub fn new() -> Self {
        Self
    }

    /// Price a revolving credit facility using Monte Carlo simulation.
    ///
    /// Simulates the utilization rate evolution using a mean-reverting OU process,
    /// generates cashflows for each path, and returns the average discounted value.
    pub fn price_stochastic(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        use super::types::{DrawRepaySpec, UtilizationProcess};

        // Extract stochastic spec
        let stoch_spec = match &facility.draw_repay_spec {
            DrawRepaySpec::Stochastic(spec) => spec,
            _ => return Err(finstack_core::error::InputError::Invalid.into()),
        };

        // Extract mean-reverting parameters
        let (target_rate, speed, volatility) = match &stoch_spec.utilization_process {
            UtilizationProcess::MeanReverting {
                target_rate,
                speed,
                volatility,
            } => (*target_rate, *speed, *volatility),
        };

        // Get discount curve
        let disc = market.get_discount_ref(facility.disc_id.as_str())?;
        let disc_dc = disc.day_count();
        let base_date = disc.base_date();

        // Compute as_of discount factor
        let t_as_of = disc_dc.year_fraction(
            base_date,
            as_of,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let df_as_of = disc.df(t_as_of);

        // Time horizon in years
        let t_start = disc_dc.year_fraction(
            base_date,
            facility.commitment_date,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let t_end = disc_dc.year_fraction(
            base_date,
            facility.maturity_date,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let time_horizon = t_end - t_start;

        if time_horizon <= 0.0 {
            return Ok(Money::new(0.0, facility.commitment_amount.currency()));
        }

        // Setup MC simulation
        let num_paths = stoch_spec.num_paths;
        let seed = stoch_spec.seed.unwrap_or(42);

        // Use quarterly time steps for cashflow generation
        let dt = 0.25; // 3 months
        let num_steps = (time_horizon / dt).ceil() as usize;

        // Initialize RNG
        use crate::instruments::common::mc::rng::philox::PhiloxRng;
        use crate::instruments::common::mc::traits::RandomStream;

        let base_rng = PhiloxRng::new(seed);

        // Run MC simulation
        let mut sum_pv = 0.0;

        for path_idx in 0..num_paths {
            let mut rng = base_rng.split(path_idx as u64);

            // Simulate utilization path
            let mut utilization = facility.utilization_rate();
            let mut path_pv = 0.0;

            // Get base rate for interest calculation
            let base_rate = match &facility.base_rate_spec {
                super::types::BaseRateSpec::Fixed { rate } => *rate,
                super::types::BaseRateSpec::Floating {
                    index_id,
                    margin_bp,
                    ..
                } => {
                    let fwd = market.get_forward_ref(index_id.as_str())?;
                    fwd.rate(0.25) + (margin_bp * 1e-4)
                }
            };

            for step in 0..num_steps {
                let t = t_start + (step as f64) * dt;
                let t_next = (t + dt).min(t_end);
                let actual_dt = t_next - t;

                if actual_dt <= 0.0 {
                    break;
                }

                // Current drawn and undrawn amounts based on utilization
                let commitment = facility.commitment_amount.amount();
                let drawn = commitment * utilization;
                let undrawn = commitment - drawn;

                // Calculate cashflows for this period
                // Interest on drawn
                let interest = drawn * base_rate * actual_dt;

                // Commitment fee on undrawn
                let commitment_fee = undrawn * (facility.fees.commitment_fee_bp * 1e-4) * actual_dt;

                // Usage fee on drawn
                let usage_fee = drawn * (facility.fees.usage_fee_bp * 1e-4) * actual_dt;

                // Facility fee on total commitment
                let facility_fee = commitment * (facility.fees.facility_fee_bp * 1e-4) * actual_dt;

                // Total cashflow for this period
                let total_cf = interest + commitment_fee + usage_fee + facility_fee;

                // Discount to valuation date
                let df_abs = disc.df(t_next);
                let df = if df_as_of != 0.0 {
                    df_abs / df_as_of
                } else {
                    1.0
                };

                path_pv += total_cf * df;

                // Evolve utilization using Euler-Maruyama discretization
                // dU = speed * (target - U) * dt + volatility * sqrt(dt) * dW
                if step < num_steps - 1 {
                    let drift = speed * (target_rate - utilization) * actual_dt;
                    let diffusion = volatility * actual_dt.sqrt() * rng.next_std_normal();
                    utilization += drift + diffusion;

                    // Clamp utilization to [0, 1]
                    utilization = utilization.clamp(0.0, 1.0);
                }
            }

            sum_pv += path_pv;
        }

        // Average across paths
        let mean_pv = sum_pv / (num_paths as f64);

        Ok(Money::new(mean_pv, facility.commitment_amount.currency()))
    }
}

#[cfg(feature = "mc")]
impl Pricer for RevolvingCreditMcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::MonteCarloGBM)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let facility = instrument
            .as_any()
            .downcast_ref::<RevolvingCredit>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::RevolvingCredit,
                got: instrument.key(),
            })?;

        // Validate that we have a stochastic spec
        if !facility.is_stochastic() {
            return Err(PricingError::ModelFailure(
                "RevolvingCreditMcPricer requires stochastic specification".to_string(),
            ));
        }

        // Extract valuation date from discount curve
        let disc = market
            .get_discount_ref(facility.disc_id.as_str())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Price the facility using MC
        let pv = Self::price_stochastic(facility, market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(facility.id(), as_of, pv))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pricer_key() {
        let pricer = RevolvingCreditDiscountingPricer::new();
        assert_eq!(
            pricer.key(),
            PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::Discounting)
        );
    }

    #[cfg(feature = "mc")]
    #[test]
    fn test_mc_pricer_key() {
        let pricer = RevolvingCreditMcPricer::new();
        assert_eq!(
            pricer.key(),
            PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::MonteCarloGBM)
        );
    }
}
