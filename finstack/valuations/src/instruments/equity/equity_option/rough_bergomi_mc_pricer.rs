//! Monte Carlo pricer for the rough Bergomi (rBergomi) stochastic volatility model.
//!
//! Uses [`RoughBergomiProcess`] with the [`RoughBergomiEuler`] discretization and
//! pre-generated fractional Brownian motion increments via [`simulate_path_fractional`].
//! The rBergomi model requires fBM because the variance is driven by a Volterra
//! representation of fractional Brownian motion, unlike the rough Heston model
//! which uses standard Brownian motion with a singular kernel.

#[cfg(feature = "mc")]
use super::pricer::{collect_inputs_extended, option_currency};
#[cfg(feature = "mc")]
use super::types::EquityOption;
#[cfg(feature = "mc")]
use crate::instruments::common_impl::parameters::OptionType;
#[cfg(feature = "mc")]
use crate::instruments::common_impl::traits::Instrument;
#[cfg(feature = "mc")]
use finstack_core::dates::Date;
#[cfg(feature = "mc")]
use finstack_core::market_data::context::MarketContext;
#[cfg(feature = "mc")]
use finstack_core::money::Money;

/// Monte Carlo pricer for the rough Bergomi model.
///
/// Simulates spot paths under rBergomi dynamics where the instantaneous variance
/// is a functional of a fractional Brownian motion path:
///
/// ```text
/// V_t = xi_0(t) * exp(eta * W_H(t) - 0.5 * eta^2 * t^{2H})
/// ```
///
/// Model parameters are read from market scalars:
///
/// | Scalar Key | Default | Description |
/// |---|---|---|
/// | `RBERGOMI_ETA` | 1.9 | Vol-of-vol scaling |
/// | `RBERGOMI_HURST` | 0.07 | Hurst exponent |
/// | `RBERGOMI_RHO` | -0.9 | Spot-vol correlation |
#[cfg(feature = "mc")]
pub(crate) struct EquityOptionRoughBergomiMcPricer {
    /// Number of Monte Carlo paths.
    num_paths: usize,
    /// Number of time steps per path.
    num_steps: usize,
}

#[cfg(feature = "mc")]
impl EquityOptionRoughBergomiMcPricer {
    /// Create a new rBergomi MC pricer with explicit configuration.
    pub(crate) fn new(num_paths: usize, num_steps: usize) -> Self {
        Self {
            num_paths,
            num_steps,
        }
    }
}

#[cfg(feature = "mc")]
impl Default for EquityOptionRoughBergomiMcPricer {
    fn default() -> Self {
        Self::new(100_000, 100)
    }
}

/// Extract a unitless scalar from market data, falling back to a default.
#[cfg(feature = "mc")]
fn get_scalar(market: &MarketContext, key: &str, default: f64) -> f64 {
    crate::instruments::common_impl::helpers::get_unitless_scalar(market, key, default)
}

/// Run the fractional MC simulation loop for a concrete payoff type.
#[cfg(feature = "mc")]
#[allow(clippy::too_many_arguments)]
fn simulate_rbergomi<F: finstack_monte_carlo::traits::Payoff>(
    num_paths: usize,
    rng: &mut finstack_monte_carlo::rng::philox::PhiloxRng,
    time_grid: &finstack_monte_carlo::time_grid::TimeGrid,
    process: &finstack_monte_carlo::process::rough_bergomi::RoughBergomiProcess,
    disc: &finstack_monte_carlo::discretization::rough_bergomi::RoughBergomiEuler,
    initial_state: &[f64],
    payoff_template: &F,
    ccy: finstack_core::currency::Currency,
    fbm_gen: &dyn finstack_monte_carlo::rng::fbm::FractionalNoiseGenerator,
    num_steps: usize,
    err_ctx: crate::pricer::PricingErrorContext,
) -> crate::pricer::PricingResult<f64> {
    use finstack_monte_carlo::traits::{Discretization, RandomStream, StochasticProcess};

    let num_factors = process.num_factors();
    let work_size = disc.work_size(process);
    let mut state = vec![0.0; process.dim()];
    let mut z = vec![0.0; num_factors];
    let mut work = vec![0.0; work_size];
    let mut fbm_normals = vec![0.0; num_steps];
    let mut fbm_increments = vec![0.0; num_steps];
    let fbm_z_index = 1;

    let mut sum = 0.0;

    for _ in 0..num_paths {
        // Generate fBM increments for this path
        rng.fill_std_normals(&mut fbm_normals);
        fbm_gen.generate(&fbm_normals, &mut fbm_increments);

        let mut payoff = payoff_template.clone();
        payoff.reset();

        let pv = finstack_monte_carlo::engine_fractional::simulate_path_fractional(
            rng,
            time_grid,
            process,
            disc,
            initial_state,
            &mut payoff,
            ccy,
            &fbm_increments,
            fbm_z_index,
            &mut state,
            &mut z,
            &mut work,
        )
        .map_err(|e| crate::pricer::PricingError::from_core(e, err_ctx.clone()))?;

        sum += pv;
    }

    Ok(sum / num_paths as f64)
}

#[cfg(feature = "mc")]
impl crate::pricer::Pricer for EquityOptionRoughBergomiMcPricer {
    fn key(&self) -> crate::pricer::PricerKey {
        crate::pricer::PricerKey::new(
            crate::pricer::InstrumentType::EquityOption,
            crate::pricer::ModelKey::MonteCarloRoughBergomi,
        )
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common_impl::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> crate::pricer::PricingResult<crate::results::ValuationResult> {
        let equity_option = instrument
            .as_any()
            .downcast_ref::<EquityOption>()
            .ok_or_else(|| {
                crate::pricer::PricingError::type_mismatch(
                    crate::pricer::InstrumentType::EquityOption,
                    instrument.key(),
                )
            })?;

        let inputs = collect_inputs_extended(equity_option, market, as_of).map_err(|e| {
            crate::pricer::PricingError::model_failure_with_context(
                e.to_string(),
                crate::pricer::PricingErrorContext::default(),
            )
        })?;
        let (spot, r, q, sigma, t) = (inputs.spot, inputs.r, inputs.q, inputs.sigma, inputs.t_vol);

        if t <= 0.0 {
            let intrinsic = match equity_option.option_type {
                OptionType::Call => (spot - equity_option.strike).max(0.0),
                OptionType::Put => (equity_option.strike - spot).max(0.0),
            };
            return Ok(crate::results::ValuationResult::stamped(
                equity_option.id(),
                as_of,
                Money::new(
                    intrinsic * equity_option.notional.amount(),
                    option_currency(equity_option),
                ),
            ));
        }

        // Fetch rBergomi parameters
        let eta = get_scalar(market, "RBERGOMI_ETA", 1.9);
        let hurst = get_scalar(market, "RBERGOMI_HURST", 0.07);
        let rho = get_scalar(market, "RBERGOMI_RHO", -0.9);

        let err_ctx = crate::pricer::PricingErrorContext::from_instrument(equity_option)
            .model(crate::pricer::ModelKey::MonteCarloRoughBergomi);

        // Build a flat forward variance curve from the implied vol
        let xi =
            finstack_core::market_data::term_structures::ForwardVarianceCurve::flat(sigma * sigma)
                .map_err(|e| crate::pricer::PricingError::from_core(e, err_ctx.clone()))?;

        let hurst_exp = finstack_core::math::fractional::HurstExponent::new(hurst)
            .map_err(|e| crate::pricer::PricingError::from_core(e, err_ctx.clone()))?;
        let params = finstack_monte_carlo::process::rough_bergomi::RoughBergomiParams::new(
            r, q, hurst_exp, eta, rho, xi,
        )
        .map_err(|e| crate::pricer::PricingError::from_core(e, err_ctx.clone()))?;
        let process =
            finstack_monte_carlo::process::rough_bergomi::RoughBergomiProcess::new(params);

        // Build time grid
        let time_grid = finstack_monte_carlo::time_grid::TimeGrid::uniform(t, self.num_steps)
            .map_err(|e| crate::pricer::PricingError::from_core(e, err_ctx.clone()))?;
        let times: Vec<f64> = (0..=self.num_steps)
            .map(|i| t * i as f64 / self.num_steps as f64)
            .collect();

        // Build discretization and fBM generator
        let disc =
            finstack_monte_carlo::discretization::rough_bergomi::RoughBergomiEuler::new(hurst_exp);
        let fbm_config = finstack_monte_carlo::rng::fbm::FractionalNoiseConfig {
            hurst,
            generator_type: finstack_monte_carlo::rng::fbm::FbmGeneratorType::Auto,
        };
        let fbm_gen = finstack_monte_carlo::rng::fbm::create_fbm_generator(&times, &fbm_config)
            .map_err(|e| crate::pricer::PricingError::from_core(e, err_ctx.clone()))?;

        let ccy = option_currency(equity_option);
        let initial_state = [spot];

        // Derive deterministic seed from instrument id
        let seed_val =
            if let Some(ref scenario) = equity_option.pricing_overrides.metrics.mc_seed_scenario {
                finstack_monte_carlo::seed::derive_seed(&equity_option.id, scenario)
            } else {
                finstack_monte_carlo::seed::derive_seed(&equity_option.id, "base")
            };
        let mut rng = finstack_monte_carlo::rng::philox::PhiloxRng::new(seed_val);

        let mean_pv = match equity_option.option_type {
            OptionType::Call => {
                let payoff = finstack_monte_carlo::payoff::vanilla::EuropeanCall::new(
                    equity_option.strike,
                    equity_option.notional.amount(),
                    self.num_steps,
                );
                simulate_rbergomi(
                    self.num_paths,
                    &mut rng,
                    &time_grid,
                    &process,
                    &disc,
                    &initial_state,
                    &payoff,
                    ccy,
                    fbm_gen.as_ref(),
                    self.num_steps,
                    err_ctx.clone(),
                )?
            }
            OptionType::Put => {
                let payoff = finstack_monte_carlo::payoff::vanilla::EuropeanPut::new(
                    equity_option.strike,
                    equity_option.notional.amount(),
                    self.num_steps,
                );
                simulate_rbergomi(
                    self.num_paths,
                    &mut rng,
                    &time_grid,
                    &process,
                    &disc,
                    &initial_state,
                    &payoff,
                    ccy,
                    fbm_gen.as_ref(),
                    self.num_steps,
                    err_ctx,
                )?
            }
        };

        // simulate_path_fractional already returns discounted PV — do not
        // multiply by discount_factor again.
        let pv = Money::new(mean_pv, ccy);
        Ok(crate::results::ValuationResult::stamped(
            equity_option.id(),
            as_of,
            pv,
        ))
    }
}
