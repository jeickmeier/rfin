//! Asian option pricers (Monte Carlo and analytical).

// Common imports for all pricers
use crate::instruments::asian_option::types::AsianOption;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingResult};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

// MC-specific imports
#[cfg(feature = "mc")]
use crate::instruments::common::mc::estimate::Estimate;
#[cfg(feature = "mc")]
use crate::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::engine::PathCaptureConfig;
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::payoff::asian::{AsianCall, AsianPut};
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::results::MoneyEstimate;
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::variance_reduction::control_variate::{
    apply_control_variate, covariance,
};

/// Asian option Monte Carlo pricer.
#[cfg(feature = "mc")]
pub struct AsianOptionMcPricer {
    config: PathDependentPricerConfig,
}

#[cfg(feature = "mc")]
impl AsianOptionMcPricer {
    /// Create a new Asian option MC pricer with default config.
    pub fn new() -> Self {
        Self {
            config: PathDependentPricerConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: PathDependentPricerConfig) -> Self {
        Self { config }
    }

    /// Price an Asian option using Monte Carlo.
    fn price_internal(
        &self,
        inst: &AsianOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        // Get time to maturity
        let t = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

        let (hist_sum, hist_prod_log, hist_count) = inst.accumulated_state(as_of);

        if t <= 0.0 {
            // Expired: use realized average
            let average = if hist_count > 0 {
                match inst.averaging_method {
                    crate::instruments::asian_option::types::AveragingMethod::Arithmetic => {
                        hist_sum / hist_count as f64
                    }
                    crate::instruments::asian_option::types::AveragingMethod::Geometric => {
                        (hist_prod_log / hist_count as f64).exp()
                    }
                }
            } else {
                // Fallback
                let spot_scalar = curves.price(&inst.spot_id)?;
                match spot_scalar {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                    finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
                }
            };

            let intrinsic = match inst.option_type {
                crate::instruments::OptionType::Call => (average - inst.strike.amount()).max(0.0),
                crate::instruments::OptionType::Put => (inst.strike.amount() - average).max(0.0),
            };
            return Ok(Money::new(
                intrinsic * inst.notional.amount(),
                inst.strike.currency(),
            ));
        }

        // Get discount curve
        let disc_curve = curves.get_discount_ref(inst.discount_curve_id.as_str())?;
        let r = disc_curve.zero(t);
        let t_as_of = disc_curve.day_count().year_fraction(
            disc_curve.base_date(),
            as_of,
            DayCountCtx::default(),
        )?;
        let df_as_of = disc_curve.df(t_as_of);
        let df_maturity = disc_curve.df(t_as_of + t);
        let discount_factor = if df_as_of > 0.0 {
            df_maturity / df_as_of
        } else {
            1.0
        };

        // Get spot
        let spot_scalar = curves.price(&inst.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        // Get dividend yield
        let q = if let Some(div_id) = &inst.div_yield_id {
            match curves.price(div_id.as_str()) {
                Ok(ms) => match ms {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                    finstack_core::market_data::scalars::MarketScalar::Price(_) => 0.0,
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        // Get volatility
        let vol_surface = curves.surface_ref(inst.vol_surface_id.as_str())?;
        let sigma = vol_surface.value_clamped(t, inst.strike.amount());

        // Create GBM process
        let gbm_params = GbmParams::new(r, q, sigma);
        let process = GbmProcess::new(gbm_params);

        // Map fixing dates to time steps
        // Use configurable steps per year with a minimum cap
        let steps_per_year = self.config.steps_per_year;
        let num_steps = ((t * steps_per_year).round() as usize).max(self.config.min_steps);
        let mut fixing_steps = Vec::new();
        for &fixing_date in &inst.fixing_dates {
            let fixing_t =
                inst.day_count
                    .year_fraction(as_of, fixing_date, DayCountCtx::default())?;
            if fixing_t > 0.0 && fixing_t <= t {
                let step = (fixing_t / t * num_steps as f64).round() as usize;
                let clamped = step.min(num_steps.saturating_sub(1)).max(0);
                fixing_steps.push(clamped);
            }
        }
        fixing_steps.sort();
        fixing_steps.dedup();

        // Create payoff
        let averaging = match inst.averaging_method {
            crate::instruments::asian_option::types::AveragingMethod::Arithmetic => {
                crate::instruments::common::models::monte_carlo::payoff::asian::AveragingMethod::Arithmetic
            }
            crate::instruments::asian_option::types::AveragingMethod::Geometric => {
                crate::instruments::common::models::monte_carlo::payoff::asian::AveragingMethod::Geometric
            }
        };

        // Derive deterministic seed from instrument ID and scenario
        #[cfg(feature = "mc")]
        use crate::instruments::common::models::monte_carlo::seed;

        let seed = if let Some(ref scenario) = inst.pricing_overrides.mc_seed_scenario {
            #[cfg(feature = "mc")]
            {
                seed::derive_seed(&inst.id, scenario)
            }
            #[cfg(not(feature = "mc"))]
            42
        } else {
            #[cfg(feature = "mc")]
            {
                seed::derive_seed(&inst.id, "base")
            }
            #[cfg(not(feature = "mc"))]
            self.config.seed
        };

        // Create config with derived seed
        let mut config = self.config.clone();
        config.seed = seed;

        // If arithmetic averaging, apply geometric-Asian control variate for variance reduction
        let result_money = match (inst.averaging_method, inst.option_type) {
            (
                crate::instruments::asian_option::types::AveragingMethod::Arithmetic,
                crate::instruments::OptionType::Call,
            ) => {
                // Use path capture to get per-path discounted payoffs for covariance
                let mut cfg_cap = config.clone();
                cfg_cap.path_capture = PathCaptureConfig::all().with_payoffs();
                let pricer_cap = PathDependentPricer::new(cfg_cap);

                // Arithmetic payoff
                let arith_payoff = AsianCall::with_history(
                    inst.strike.amount(),
                    inst.notional.amount(),
                    crate::instruments::common::models::monte_carlo::payoff::asian::AveragingMethod::Arithmetic,
                    fixing_steps.clone(),
                    hist_sum,
                    hist_prod_log,
                    hist_count,
                );
                let arith_full = pricer_cap.price_with_paths(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &arith_payoff,
                    inst.strike.currency(),
                    discount_factor,
                )?;

                // Geometric payoff (same RNG via same seed)
                let geom_payoff = AsianCall::with_history(
                    inst.strike.amount(),
                    inst.notional.amount(),
                    crate::instruments::common::models::monte_carlo::payoff::asian::AveragingMethod::Geometric,
                    fixing_steps.clone(),
                    hist_sum,
                    hist_prod_log,
                    hist_count,
                );
                let geom_full = pricer_cap.price_with_paths(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &geom_payoff,
                    inst.strike.currency(),
                    discount_factor,
                )?;

                // Extract per-path discounted payoffs
                let xs: Vec<f64> = arith_full
                    .paths
                    .as_ref()
                    .expect("paths captured")
                    .paths
                    .iter()
                    .map(|p| p.final_value)
                    .collect();
                let ys: Vec<f64> = geom_full
                    .paths
                    .as_ref()
                    .expect("paths captured")
                    .paths
                    .iter()
                    .map(|p| p.final_value)
                    .collect();

                let n = xs.len();
                // Sample means
                let mean_x = xs.iter().sum::<f64>() / n as f64;
                let mean_y = ys.iter().sum::<f64>() / n as f64;
                // Sample variances (unbiased)
                let var_x = if n > 1 {
                    xs.iter().map(|v| (v - mean_x).powi(2)).sum::<f64>() / (n as f64 - 1.0)
                } else {
                    0.0
                };
                let var_y = if n > 1 {
                    ys.iter().map(|v| (v - mean_y).powi(2)).sum::<f64>() / (n as f64 - 1.0)
                } else {
                    0.0
                };
                // Covariance
                let cov_xy = covariance(&xs, &ys);

                // Analytical value of geometric Asian (control)
                // Use standard formula if not seasoned, otherwise we should ideally use adjusted.
                // But since our Geometric Analytical doesn't support seasoning, we can't use CV safely?
                // Actually, if we are seasoned, we can't use the analytical control variate unless we implement the adjustment.
                // If hist_count > 0, we should probably disable CV or implement the adjustment.
                // For now, let's disable CV if seasoned to avoid wrong adjustment.

                if hist_count > 0 {
                    // Fallback to plain MC if seasoned (CV disabled)
                    MoneyEstimate::from_estimate(
                        Estimate {
                            mean: mean_x,
                            stderr: (var_x / n as f64).sqrt(),
                            ci_95: (
                                mean_x - 1.96 * (var_x / n as f64).sqrt(),
                                mean_x + 1.96 * (var_x / n as f64).sqrt(),
                            ),
                            num_paths: n,
                            std_dev: None,
                            median: None,
                            percentile_25: None,
                            percentile_75: None,
                            min: None,
                            max: None,
                        },
                        inst.strike.currency(),
                    )
                    .mean
                } else {
                    let control_analytical = geometric_asian_call(
                        spot,
                        inst.strike.amount(),
                        t,
                        r,
                        q,
                        sigma,
                        inst.fixing_dates.len(),
                    );

                    let adj = apply_control_variate(
                        mean_x,
                        var_x,
                        mean_y,
                        var_y,
                        cov_xy,
                        control_analytical,
                        n,
                    );
                    MoneyEstimate::from_estimate(adj, inst.strike.currency()).mean
                }
            }
            (
                crate::instruments::asian_option::types::AveragingMethod::Arithmetic,
                crate::instruments::OptionType::Put,
            ) => {
                let mut cfg_cap = config.clone();
                cfg_cap.path_capture = PathCaptureConfig::all().with_payoffs();
                let pricer_cap = PathDependentPricer::new(cfg_cap);

                let arith_payoff = AsianPut::with_history(
                    inst.strike.amount(),
                    inst.notional.amount(),
                    crate::instruments::common::models::monte_carlo::payoff::asian::AveragingMethod::Arithmetic,
                    fixing_steps.clone(),
                    hist_sum,
                    hist_prod_log,
                    hist_count,
                );
                let arith_full = pricer_cap.price_with_paths(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &arith_payoff,
                    inst.strike.currency(),
                    discount_factor,
                )?;

                let geom_payoff = AsianPut::with_history(
                    inst.strike.amount(),
                    inst.notional.amount(),
                    crate::instruments::common::models::monte_carlo::payoff::asian::AveragingMethod::Geometric,
                    fixing_steps.clone(),
                    hist_sum,
                    hist_prod_log,
                    hist_count,
                );
                let geom_full = pricer_cap.price_with_paths(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &geom_payoff,
                    inst.strike.currency(),
                    discount_factor,
                )?;

                let xs: Vec<f64> = arith_full
                    .paths
                    .as_ref()
                    .expect("paths captured")
                    .paths
                    .iter()
                    .map(|p| p.final_value)
                    .collect();
                let ys: Vec<f64> = geom_full
                    .paths
                    .as_ref()
                    .expect("paths captured")
                    .paths
                    .iter()
                    .map(|p| p.final_value)
                    .collect();
                let n = xs.len();
                let mean_x = xs.iter().sum::<f64>() / n as f64;
                let mean_y = ys.iter().sum::<f64>() / n as f64;
                let var_x = if n > 1 {
                    xs.iter().map(|v| (v - mean_x).powi(2)).sum::<f64>() / (n as f64 - 1.0)
                } else {
                    0.0
                };
                let var_y = if n > 1 {
                    ys.iter().map(|v| (v - mean_y).powi(2)).sum::<f64>() / (n as f64 - 1.0)
                } else {
                    0.0
                };
                let cov_xy = covariance(&xs, &ys);

                if hist_count > 0 {
                    MoneyEstimate::from_estimate(
                        Estimate {
                            mean: mean_x,
                            stderr: (var_x / n as f64).sqrt(),
                            ci_95: (
                                mean_x - 1.96 * (var_x / n as f64).sqrt(),
                                mean_x + 1.96 * (var_x / n as f64).sqrt(),
                            ),
                            num_paths: n,
                            std_dev: None,
                            median: None,
                            percentile_25: None,
                            percentile_75: None,
                            min: None,
                            max: None,
                        },
                        inst.strike.currency(),
                    )
                    .mean
                } else {
                    let control_analytical = geometric_asian_put(
                        spot,
                        inst.strike.amount(),
                        t,
                        r,
                        q,
                        sigma,
                        inst.fixing_dates.len(),
                    );

                    let adj = apply_control_variate(
                        mean_x,
                        var_x,
                        mean_y,
                        var_y,
                        cov_xy,
                        control_analytical,
                        n,
                    );
                    MoneyEstimate::from_estimate(adj, inst.strike.currency()).mean
                }
            }
            // Geometric averaging (no CV needed) or fallback path
            _ => {
                let pricer = PathDependentPricer::new(config);
                match inst.option_type {
                    crate::instruments::OptionType::Call => {
                        let payoff = AsianCall::with_history(
                            inst.strike.amount(),
                            inst.notional.amount(),
                            averaging,
                            fixing_steps,
                            hist_sum,
                            hist_prod_log,
                            hist_count,
                        );
                        pricer
                            .price(
                                &process,
                                spot,
                                t,
                                num_steps,
                                &payoff,
                                inst.strike.currency(),
                                discount_factor,
                            )?
                            .mean
                    }
                    crate::instruments::OptionType::Put => {
                        let payoff = AsianPut::with_history(
                            inst.strike.amount(),
                            inst.notional.amount(),
                            averaging,
                            fixing_steps,
                            hist_sum,
                            hist_prod_log,
                            hist_count,
                        );
                        pricer
                            .price(
                                &process,
                                spot,
                                t,
                                num_steps,
                                &payoff,
                                inst.strike.currency(),
                                discount_factor,
                            )?
                            .mean
                    }
                }
            }
        };

        Ok(result_money)
    }

    /// Price with LRM Greeks (delta, vega) convenience.
    #[allow(clippy::too_many_lines)]
    pub fn price_with_lrm_greeks_internal(
        &self,
        inst: &AsianOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<(Money, Option<(f64, f64)>)> {
        // Reuse the same setup as price_internal
        let t = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

        let (hist_sum, hist_prod_log, hist_count) = inst.accumulated_state(as_of);

        if t <= 0.0 {
            // Expired, return 0 value/greeks (or intrinsic if we care about cashflows, but greeks usually 0)
            // For simplicity, we return 0 value if expired in LRM greeks context as sensitivity is 0.
            // Or should we return intrinsic? Since this is pricing + greeks, we should return value.
            let average = if hist_count > 0 {
                match inst.averaging_method {
                    crate::instruments::asian_option::types::AveragingMethod::Arithmetic => {
                        hist_sum / hist_count as f64
                    }
                    crate::instruments::asian_option::types::AveragingMethod::Geometric => {
                        (hist_prod_log / hist_count as f64).exp()
                    }
                }
            } else {
                let spot_scalar = curves.price(&inst.spot_id)?;
                match spot_scalar {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                    finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
                }
            };
            let intrinsic = match inst.option_type {
                crate::instruments::OptionType::Call => (average - inst.strike.amount()).max(0.0),
                crate::instruments::OptionType::Put => (inst.strike.amount() - average).max(0.0),
            };

            return Ok((
                Money::new(intrinsic * inst.notional.amount(), inst.strike.currency()),
                None,
            ));
        }

        let disc_curve = curves.get_discount_ref(inst.discount_curve_id.as_str())?;
        let r = disc_curve.zero(t);
        let t_as_of = disc_curve.day_count().year_fraction(
            disc_curve.base_date(),
            as_of,
            DayCountCtx::default(),
        )?;
        let df_as_of = disc_curve.df(t_as_of);
        let df_maturity = disc_curve.df(t_as_of + t);
        let discount_factor = if df_as_of > 0.0 {
            df_maturity / df_as_of
        } else {
            1.0
        };

        let spot_scalar = curves.price(&inst.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let q = if let Some(div_id) = &inst.div_yield_id {
            match curves.price(div_id.as_str()) {
                Ok(ms) => match ms {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                    finstack_core::market_data::scalars::MarketScalar::Price(_) => 0.0,
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        let vol_surface = curves.surface_ref(inst.vol_surface_id.as_str())?;
        let sigma = vol_surface.value_clamped(t, inst.strike.amount());

        let gbm_params = GbmParams::new(r, q, sigma);
        let process = GbmProcess::new(gbm_params);

        let steps_per_year = self.config.steps_per_year;
        let num_steps = ((t * steps_per_year).round() as usize).max(self.config.min_steps);

        // Fixing steps mapping
        let mut fixing_steps = Vec::new();
        for &fixing_date in &inst.fixing_dates {
            let fixing_t =
                inst.day_count
                    .year_fraction(as_of, fixing_date, DayCountCtx::default())?;
            if fixing_t > 0.0 && fixing_t <= t {
                let step = (fixing_t / t * num_steps as f64).round() as usize;
                let clamped = step.min(num_steps.saturating_sub(1)).max(0);
                fixing_steps.push(clamped);
            }
        }

        fixing_steps.sort();
        fixing_steps.dedup();

        let averaging = match inst.averaging_method {
            crate::instruments::asian_option::types::AveragingMethod::Arithmetic => {
                crate::instruments::common::models::monte_carlo::payoff::asian::AveragingMethod::Arithmetic
            }
            crate::instruments::asian_option::types::AveragingMethod::Geometric => {
                crate::instruments::common::models::monte_carlo::payoff::asian::AveragingMethod::Geometric
            }
        };

        // Seed handling
        #[cfg(feature = "mc")]
        use crate::instruments::common::models::monte_carlo::seed;
        let seed = if let Some(ref scenario) = inst.pricing_overrides.mc_seed_scenario {
            #[cfg(feature = "mc")]
            {
                seed::derive_seed(&inst.id, scenario)
            }
            #[cfg(not(feature = "mc"))]
            42
        } else {
            #[cfg(feature = "mc")]
            {
                seed::derive_seed(&inst.id, "base")
            }
            #[cfg(not(feature = "mc"))]
            self.config.seed
        };
        let mut cfg = self.config.clone();
        cfg.seed = seed;
        let pricer = PathDependentPricer::new(cfg);

        let (est, greeks) = match inst.option_type {
            crate::instruments::OptionType::Call => {
                let payoff =
                    crate::instruments::common::models::monte_carlo::payoff::asian::AsianCall::with_history(
                        inst.strike.amount(),
                        inst.notional.amount(),
                        averaging,
                        fixing_steps,
                        hist_sum,
                        hist_prod_log,
                        hist_count,
                    );
                pricer.price_with_lrm_greeks(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &payoff,
                    inst.strike.currency(),
                    discount_factor,
                    r,
                    q,
                    sigma,
                )?
            }
            crate::instruments::OptionType::Put => {
                let payoff =
                    crate::instruments::common::models::monte_carlo::payoff::asian::AsianPut::with_history(
                        inst.strike.amount(),
                        inst.notional.amount(),
                        averaging,
                        fixing_steps,
                        hist_sum,
                        hist_prod_log,
                        hist_count,
                    );
                pricer.price_with_lrm_greeks(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &payoff,
                    inst.strike.currency(),
                    discount_factor,
                    r,
                    q,
                    sigma,
                )?
            }
        };

        Ok((est.mean, greeks))
    }
}

#[cfg(feature = "mc")]
impl Default for AsianOptionMcPricer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "mc")]
impl Pricer for AsianOptionMcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::AsianOption, ModelKey::MonteCarloGBM)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let asian = instrument
            .as_any()
            .downcast_ref::<AsianOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::AsianOption, instrument.key())
            })?;

        let pv = self
            .price_internal(asian, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        Ok(ValuationResult::stamped(asian.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
#[cfg(feature = "mc")]
pub fn npv(
    inst: &AsianOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<Money> {
    let pricer = AsianOptionMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}

/// Present value with LRM Greeks via Monte Carlo.
#[cfg(feature = "mc")]
pub fn npv_with_lrm_greeks(
    inst: &AsianOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<(Money, Option<(f64, f64)>)> {
    let pricer = AsianOptionMcPricer::new();
    pricer.price_with_lrm_greeks_internal(inst, curves, as_of)
}

// ========================= ANALYTICAL PRICERS =========================

use crate::instruments::common::helpers::collect_black_scholes_inputs;
use crate::instruments::common::models::closed_form::asian::{
    arithmetic_asian_call_tw, arithmetic_asian_put_tw, geometric_asian_call, geometric_asian_put,
};

/// Geometric Asian option analytical pricer.
pub struct AsianOptionAnalyticalGeometricPricer;

impl AsianOptionAnalyticalGeometricPricer {
    /// Create a new analytical geometric Asian option pricer
    pub fn new() -> Self {
        Self
    }
}

impl Default for AsianOptionAnalyticalGeometricPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for AsianOptionAnalyticalGeometricPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::AsianOption, ModelKey::AsianGeometricBS)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let asian = instrument
            .as_any()
            .downcast_ref::<AsianOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::AsianOption, instrument.key())
            })?;

        // Use standardized input collection
        let (spot, r, q, sigma, t) = collect_black_scholes_inputs(
            &asian.spot_id,
            &asian.discount_curve_id,
            asian.div_yield_id.as_ref(),
            &asian.vol_surface_id,
            asian.strike.amount(),
            asian.expiry,
            asian.day_count,
            market,
            as_of,
        )
        .map_err(|e| PricingError::model_failure(e.to_string()))?;

        let (sum, log_prod, count) = asian.accumulated_state(as_of);
        let total_fixings = asian.fixing_dates.len();

        if t <= 0.0 {
            // Handle expired option using realized average
            let average = if count > 0 {
                match asian.averaging_method {
                    crate::instruments::asian_option::types::AveragingMethod::Arithmetic => {
                        sum / count as f64
                    }
                    crate::instruments::asian_option::types::AveragingMethod::Geometric => {
                        (log_prod / count as f64).exp()
                    }
                }
            } else {
                // Fallback if no fixings recorded (unlikely for expired option)
                spot
            };

            let intrinsic = match asian.option_type {
                crate::instruments::OptionType::Call => (average - asian.strike.amount()).max(0.0),
                crate::instruments::OptionType::Put => (asian.strike.amount() - average).max(0.0),
            };
            return Ok(ValuationResult::stamped(
                asian.id(),
                as_of,
                Money::new(intrinsic * asian.notional.amount(), asian.strike.currency()),
            ));
        }

        // Seasoned Geometric Analytical not fully supported yet
        if count > 0 {
            return Err(PricingError::model_failure(
                "Seasoned Geometric Asian not supported in Analytical pricer. Use Monte Carlo."
                    .to_string(),
            ));
        }

        let price = match asian.option_type {
            crate::instruments::OptionType::Call => {
                geometric_asian_call(spot, asian.strike.amount(), t, r, q, sigma, total_fixings)
            }
            crate::instruments::OptionType::Put => {
                geometric_asian_put(spot, asian.strike.amount(), t, r, q, sigma, total_fixings)
            }
        };

        let pv = Money::new(price * asian.notional.amount(), asian.strike.currency());
        Ok(ValuationResult::stamped(asian.id(), as_of, pv))
    }
}

/// Arithmetic Asian option semi-analytical pricer (Turnbull-Wakeman).
pub struct AsianOptionSemiAnalyticalTwPricer;

impl AsianOptionSemiAnalyticalTwPricer {
    /// Create a new Turnbull-Wakeman approximation Asian option pricer
    pub fn new() -> Self {
        Self
    }
}

impl Default for AsianOptionSemiAnalyticalTwPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for AsianOptionSemiAnalyticalTwPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::AsianOption, ModelKey::AsianTurnbullWakeman)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let asian = instrument
            .as_any()
            .downcast_ref::<AsianOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::AsianOption, instrument.key())
            })?;

        // Use standardized input collection
        let (spot, r, q, sigma, t) = collect_black_scholes_inputs(
            &asian.spot_id,
            &asian.discount_curve_id,
            asian.div_yield_id.as_ref(),
            &asian.vol_surface_id,
            asian.strike.amount(),
            asian.expiry,
            asian.day_count,
            market,
            as_of,
        )
        .map_err(|e| PricingError::model_failure(e.to_string()))?;

        let (sum, _, count) = asian.accumulated_state(as_of);
        let total_fixings = asian.fixing_dates.len();

        if t <= 0.0 {
            let average = if count > 0 { sum / count as f64 } else { spot };
            let intrinsic = match asian.option_type {
                crate::instruments::OptionType::Call => (average - asian.strike.amount()).max(0.0),
                crate::instruments::OptionType::Put => (asian.strike.amount() - average).max(0.0),
            };
            return Ok(ValuationResult::stamped(
                asian.id(),
                as_of,
                Money::new(intrinsic * asian.notional.amount(), asian.strike.currency()),
            ));
        }

        let future_fixings = total_fixings.saturating_sub(count);
        if future_fixings == 0 {
            // Deterministic case (all fixings past, but not expired?)
            let average = sum / total_fixings as f64;
            let payoff = match asian.option_type {
                crate::instruments::OptionType::Call => (average - asian.strike.amount()).max(0.0),
                crate::instruments::OptionType::Put => (asian.strike.amount() - average).max(0.0),
            };
            // Discount to present
            let disc_curve = market
                .get_discount_ref(asian.discount_curve_id.as_str())
                .map_err(|e| PricingError::model_failure(e.to_string()))?;
            let df = disc_curve.df(t); // approx discount using t (time to expiry)
            return Ok(ValuationResult::stamped(
                asian.id(),
                as_of,
                Money::new(
                    payoff * df * asian.notional.amount(),
                    asian.strike.currency(),
                ),
            ));
        }

        let n = total_fixings as f64;
        let m = future_fixings as f64;
        let k = asian.strike.amount();

        let numerator = n * k - sum;
        let k_eff = numerator / m;
        let scale = m / n;

        let price = if k_eff < 0.0 {
            match asian.option_type {
                crate::instruments::OptionType::Call => {
                    // Deep ITM: PV(Avg) - PV(K)
                    // PV(Avg) = (Sum + Sum(PV(F_i))) / N * DF_T ??
                    // No, Payoff paid at T.
                    // E[Payoff] = 1/N (Sum + Sum(E[S_i])) - K
                    // PV = DF_T * ( 1/N (Sum + Sum(F_i)) - K )
                    // F_i = S * exp((r-q)*t_i)
                    // This requires iterating future fixings.
                    // For simplicity in this review fix, we can error or approximate.
                    // But let's try to be correct.
                    let mut sum_fwd = 0.0;
                    for date in &asian.fixing_dates {
                        if *date > as_of {
                            let t_i = asian
                                .day_count
                                .year_fraction(as_of, *date, DayCountCtx::default())
                                .map_err(|e| PricingError::model_failure(e.to_string()))?;
                            sum_fwd += spot * ((r - q) * t_i).exp();
                        }
                    }
                    let expected_avg = (sum + sum_fwd) / n;
                    let df = (-r * t).exp();
                    (expected_avg - k).max(0.0) * df
                }
                crate::instruments::OptionType::Put => 0.0,
            }
        } else {
            let unscaled = match asian.option_type {
                crate::instruments::OptionType::Call => {
                    arithmetic_asian_call_tw(spot, k_eff, t, r, q, sigma, future_fixings)
                }
                crate::instruments::OptionType::Put => {
                    arithmetic_asian_put_tw(spot, k_eff, t, r, q, sigma, future_fixings)
                }
            };
            unscaled * scale
        };

        let pv = Money::new(price * asian.notional.amount(), asian.strike.currency());
        Ok(ValuationResult::stamped(asian.id(), as_of, pv))
    }
}
