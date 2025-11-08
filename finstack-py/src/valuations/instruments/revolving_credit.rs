use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::common::{extract_curve_id, extract_instrument_id, parse_frequency_label};
use finstack_valuations::instruments::revolving_credit::{
    BaseRateSpec, DrawRepayEvent, DrawRepaySpec, RevolvingCredit, RevolvingCreditFees,
    StochasticUtilizationSpec, UtilizationProcess,
};
use finstack_valuations::instruments::revolving_credit::types::{
    CreditSpreadProcessSpec, InterestRateProcessSpec, McConfig,
};
use crate::core::market_data::PyMarketContext;
use crate::valuations::common::mc::result::PyMonteCarloResult;
use crate::valuations::cashflow::builder::PyCashFlowSchedule;
use crate::core::error::core_to_py;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList, PyModule, PyType};
use pyo3::Bound;
use std::collections::HashMap;

/// Revolving credit facility instrument.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RevolvingCredit",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyRevolvingCredit {
    pub(crate) inner: RevolvingCredit,
}

impl PyRevolvingCredit {
    pub(crate) fn new(inner: RevolvingCredit) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRevolvingCredit {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, commitment_amount, drawn_amount, commitment_date, maturity_date, base_rate_spec, payment_frequency, fees, draw_repay_spec, discount_curve)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a revolving credit facility.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier.
    ///     commitment_amount: Total committed amount as :class:`finstack.core.money.Money`.
    ///     drawn_amount: Initial drawn amount as :class:`finstack.core.money.Money`.
    ///     commitment_date: Date when facility becomes available.
    ///     maturity_date: Date when facility expires.
    ///     base_rate_spec: Base rate specification (dict with 'type' and params).
    ///     payment_frequency: Payment frequency (e.g., 'quarterly').
    ///     fees: Fee structure dict.
    ///     draw_repay_spec: Draw/repayment specification (dict).
    ///     discount_curve: Discount curve identifier.
    ///
    /// Returns:
    ///     RevolvingCredit: Configured revolving credit instrument.
    fn builder(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        commitment_amount: Bound<'_, PyAny>,
        drawn_amount: Bound<'_, PyAny>,
        commitment_date: Bound<'_, PyAny>,
        maturity_date: Bound<'_, PyAny>,
        base_rate_spec: Bound<'_, PyAny>,
        payment_frequency: Option<&str>,
        fees: Bound<'_, PyAny>,
        draw_repay_spec: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        use finstack_core::dates::DayCount;

        let id = extract_instrument_id(&instrument_id)?;
        let commitment = extract_money(&commitment_amount)?;
        let drawn = extract_money(&drawn_amount)?;
        let commit_date = py_to_date(&commitment_date)?;
        let mat_date = py_to_date(&maturity_date)?;
        let discount_curve_id = extract_curve_id(&discount_curve)?;

        // Parse base rate spec
        let base_rate = if let Ok(dict) = base_rate_spec.downcast::<PyDict>() {
            let py_type_item = dict
                .get_item("type")?
                .ok_or_else(|| PyValueError::new_err("Missing 'type' key in base_rate_spec"))?;
            let py_type = py_type_item.extract::<String>()?;

            match py_type.to_lowercase().as_str() {
                "fixed" => {
                    let rate = dict
                        .get_item("rate")?
                        .ok_or_else(|| PyValueError::new_err("Missing 'rate' for fixed rate"))?
                        .extract::<f64>()?;
                    BaseRateSpec::Fixed { rate }
                }
                "floating" => {
                    let index_id_item = dict.get_item("index_id")?.ok_or_else(|| {
                        PyValueError::new_err("Missing 'index_id' for floating rate")
                    })?;
                    let index_id_str = index_id_item.extract::<String>()?;
                    let margin_bp = dict
                        .get_item("margin_bp")?
                        .and_then(|v| v.extract::<f64>().ok())
                        .unwrap_or(0.0);
                    let reset_freq_str = dict
                        .get_item("reset_freq")?
                        .and_then(|v| v.extract::<String>().ok());
                    let reset_freq = parse_frequency(reset_freq_str.as_deref())?;
                    BaseRateSpec::Floating {
                        index_id: finstack_core::types::CurveId::new(&index_id_str),
                        margin_bp,
                        reset_freq,
                    }
                }
                other => {
                    return Err(PyValueError::new_err(format!(
                        "Unknown base rate type: {other}"
                    )))
                }
            }
        } else {
            return Err(PyValueError::new_err(
                "base_rate_spec must be a dict with 'type' key",
            ));
        };

        // Parse payment frequency
        let pay_freq = parse_frequency(payment_frequency)?;

        // Parse fees
        let fees_struct = if let Ok(dict) = fees.downcast::<PyDict>() {
            RevolvingCreditFees {
                upfront_fee: dict
                    .get_item("upfront_fee")?
                    .and_then(|v| extract_money(&v).ok()),
                commitment_fee_bp: dict
                    .get_item("commitment_fee_bp")?
                    .and_then(|v| v.extract::<f64>().ok())
                    .unwrap_or(0.0),
                usage_fee_bp: dict
                    .get_item("usage_fee_bp")?
                    .and_then(|v| v.extract::<f64>().ok())
                    .unwrap_or(0.0),
                facility_fee_bp: dict
                    .get_item("facility_fee_bp")?
                    .and_then(|v| v.extract::<f64>().ok())
                    .unwrap_or(0.0),
            }
        } else {
            RevolvingCreditFees::default()
        };

        // Parse draw/repay spec
        let draw_repay =
            if let Ok(dict) = draw_repay_spec.downcast::<PyDict>() {
                if let Ok(Some(deterministic)) = dict.get_item("deterministic") {
                    let events_list = deterministic
                        .downcast::<PyList>()
                        .map_err(|_| PyValueError::new_err("deterministic must be a list"))?;
                    let mut events = Vec::new();
                    for item in events_list.iter() {
                        let event_dict = item.downcast::<PyDict>()?;
                        let date =
                            py_to_date(&event_dict.get_item("date")?.ok_or_else(|| {
                                PyValueError::new_err("Missing 'date' in event")
                            })?)?;
                        let amount =
                            extract_money(&event_dict.get_item("amount")?.ok_or_else(|| {
                                PyValueError::new_err("Missing 'amount' in event")
                            })?)?;
                        let is_draw = event_dict
                            .get_item("is_draw")?
                            .and_then(|v| v.extract::<bool>().ok())
                            .unwrap_or(true);
                        events.push(DrawRepayEvent {
                            date,
                            amount,
                            is_draw,
                        });
                    }
                    DrawRepaySpec::Deterministic(events)
                } else if let Ok(Some(stochastic)) = dict.get_item("stochastic") {
                    let stoch_dict = stochastic
                        .downcast::<PyDict>()
                        .map_err(|_| PyValueError::new_err("stochastic must be a dict"))?;
                    let process_dict_item = stoch_dict
                        .get_item("utilization_process")?
                        .ok_or_else(|| PyValueError::new_err("Missing 'utilization_process'"))?;
                    let process_dict = process_dict_item.downcast::<PyDict>()?;
                    let process_type_val = process_dict.get_item("type")?.ok_or_else(|| {
                        PyValueError::new_err("Missing 'type' in utilization_process")
                    })?;
                    let process_type = process_type_val.extract::<String>()?;

                    let utilization_process = match process_type.to_lowercase().as_str() {
                        "mean_reverting" | "meanreverting" => {
                            let target_rate = process_dict
                                .get_item("target_rate")?
                                .ok_or_else(|| PyValueError::new_err("Missing 'target_rate'"))?
                                .extract::<f64>()?;
                            let speed = process_dict
                                .get_item("speed")?
                                .ok_or_else(|| PyValueError::new_err("Missing 'speed'"))?
                                .extract::<f64>()?;
                            let volatility = process_dict
                                .get_item("volatility")?
                                .ok_or_else(|| PyValueError::new_err("Missing 'volatility'"))?
                                .extract::<f64>()?;
                            UtilizationProcess::MeanReverting {
                                target_rate,
                                speed,
                                volatility,
                            }
                        }
                        other => {
                            return Err(PyValueError::new_err(format!(
                                "Unknown utilization process: {other}"
                            )))
                        }
                    };

                    let num_paths = stoch_dict
                        .get_item("num_paths")?
                        .ok_or_else(|| PyValueError::new_err("Missing 'num_paths'"))?
                        .extract::<usize>()?;
                    let seed = stoch_dict
                        .get_item("seed")?
                        .and_then(|v| v.extract::<Option<u64>>().ok())
                        .flatten();

                    // Optional Monte Carlo config (utilization + credit, correlation, etc.)
                    let mc_config_opt: Option<McConfig> = if let Some(mc_obj) = stoch_dict.get_item("mc_config")? {
                        let mc = mc_obj
                            .downcast::<PyDict>()
                            .map_err(|_| PyValueError::new_err("mc_config must be a dict"))?;

                        // recovery_rate (required)
                        let recovery_rate = mc
                            .get_item("recovery_rate")?
                            .ok_or_else(|| PyValueError::new_err("mc_config.recovery_rate is required"))?
                            .extract::<f64>()?;

                        // credit_spread_process (required): one-of keys
                        let csp_item = mc
                            .get_item("credit_spread_process")?
                            .ok_or_else(|| PyValueError::new_err("mc_config.credit_spread_process is required"))?;
                        let csp_dict = csp_item
                            .downcast::<PyDict>()
                            .map_err(|_| PyValueError::new_err("credit_spread_process must be a dict"))?;

                        let credit_spread_process = if let Ok(Some(cir_any)) = csp_dict.get_item("cir") {
                            let cir = cir_any.downcast::<PyDict>()?;
                            let kappa = cir
                                .get_item("kappa")?
                                .ok_or_else(|| PyValueError::new_err("cir.kappa is required"))?
                                .extract::<f64>()?;
                            let theta = cir
                                .get_item("theta")?
                                .ok_or_else(|| PyValueError::new_err("cir.theta is required"))?
                                .extract::<f64>()?;
                            let sigma = cir
                                .get_item("sigma")?
                                .ok_or_else(|| PyValueError::new_err("cir.sigma is required"))?
                                .extract::<f64>()?;
                            let initial = cir
                                .get_item("initial")?
                                .ok_or_else(|| PyValueError::new_err("cir.initial is required"))?
                                .extract::<f64>()?;
                            CreditSpreadProcessSpec::Cir { kappa, theta, sigma, initial }
                        } else if let Ok(Some(const_any)) = csp_dict.get_item("constant") {
                            let spread = const_any
                                .downcast::<PyAny>()
                                .map_err(|_| PyValueError::new_err("constant must be a float"))?
                                .extract::<f64>()?;
                            CreditSpreadProcessSpec::Constant(spread)
                        } else if let Ok(Some(ma_any)) = csp_dict.get_item("market_anchored") {
                            let ma = ma_any.downcast::<PyDict>()?;
                            let hazard_curve_id = ma
                                .get_item("hazard_curve_id")?
                                .ok_or_else(|| PyValueError::new_err("market_anchored.hazard_curve_id is required"))?
                                .extract::<String>()?;
                            let kappa = ma
                                .get_item("kappa")?
                                .ok_or_else(|| PyValueError::new_err("market_anchored.kappa is required"))?
                                .extract::<f64>()?;
                            let implied_vol = ma
                                .get_item("implied_vol")?
                                .ok_or_else(|| PyValueError::new_err("market_anchored.implied_vol is required"))?
                                .extract::<f64>()?;
                            let tenor_years = ma
                                .get_item("tenor_years")?
                                .and_then(|v| v.extract::<Option<f64>>().ok())
                                .flatten();
                            CreditSpreadProcessSpec::MarketAnchored {
                                hazard_curve_id: finstack_core::types::CurveId::new(&hazard_curve_id),
                                kappa,
                                implied_vol,
                                tenor_years,
                            }
                        } else {
                            return Err(PyValueError::new_err(
                                "credit_spread_process must contain one of: 'cir', 'constant', 'market_anchored'",
                            ));
                        };

                        // interest_rate_process (optional)
                        let irp = if let Some(irp_any) = mc.get_item("interest_rate_process")? {
                            let irp_dict = irp_any
                                .downcast::<PyDict>()
                                .map_err(|_| PyValueError::new_err("interest_rate_process must be a dict"))?;
                            if let Ok(Some(hw_any)) = irp_dict.get_item("hull_white_1f") {
                                let hw = hw_any.downcast::<PyDict>()?;
                                let kappa = hw
                                    .get_item("kappa")?
                                    .ok_or_else(|| PyValueError::new_err("hull_white_1f.kappa is required"))?
                                    .extract::<f64>()?;
                                let sigma = hw
                                    .get_item("sigma")?
                                    .ok_or_else(|| PyValueError::new_err("hull_white_1f.sigma is required"))?
                                    .extract::<f64>()?;
                                let initial = hw
                                    .get_item("initial")?
                                    .ok_or_else(|| PyValueError::new_err("hull_white_1f.initial is required"))?
                                    .extract::<f64>()?;
                                let theta = hw
                                    .get_item("theta")?
                                    .ok_or_else(|| PyValueError::new_err("hull_white_1f.theta is required"))?
                                    .extract::<f64>()?;
                                Some(InterestRateProcessSpec::HullWhite1F { kappa, sigma, initial, theta })
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        // correlation: either full 3x3 matrix or util_credit_corr
                        let correlation_matrix = if let Some(corr_any) = mc.get_item("correlation_matrix")? {
                            let corr_list = corr_any
                                .downcast::<PyList>()
                                .map_err(|_| PyValueError::new_err("correlation_matrix must be a 3x3 list"))?;
                            if corr_list.len() != 3 {
                                return Err(PyValueError::new_err("correlation_matrix must have 3 rows"));
                            }
                            let mut mat = [[0.0_f64; 3]; 3];
                            for (i, row_any) in corr_list.iter().enumerate() {
                                let row = row_any.downcast::<PyList>()?;
                                if row.len() != 3 {
                                    return Err(PyValueError::new_err("each correlation_matrix row must have 3 elements"));
                                }
                                for (j, val_any) in row.iter().enumerate() {
                                    mat[i][j] = val_any.extract::<f64>()?;
                                }
                            }
                            Some(mat)
                        } else {
                            None
                        };
                        let util_credit_corr = mc
                            .get_item("util_credit_corr")?
                            .and_then(|v| v.extract::<Option<f64>>().ok())
                            .flatten();

                        Some(McConfig {
                            correlation_matrix,
                            recovery_rate,
                            credit_spread_process,
                            interest_rate_process: irp,
                            util_credit_corr,
                        })
                    } else {
                        None
                    };

                    // Construct StochasticUtilizationSpec
                    let spec = StochasticUtilizationSpec {
                        utilization_process,
                        num_paths,
                        seed,
                        antithetic: false,
                        use_sobol_qmc: false,
                        default_model: None,
                        mc_config: mc_config_opt,
                    };
                    DrawRepaySpec::Stochastic(Box::new(spec))
                } else {
                    return Err(PyValueError::new_err(
                        "draw_repay_spec must have 'deterministic' or 'stochastic' key",
                    ));
                }
            } else {
                return Err(PyValueError::new_err("draw_repay_spec must be a dict"));
            };

        let mut builder = RevolvingCredit::builder();
        builder = builder.id(id);
        builder = builder.commitment_amount(commitment);
        builder = builder.drawn_amount(drawn);
        builder = builder.commitment_date(commit_date);
        builder = builder.maturity_date(mat_date);
        builder = builder.base_rate_spec(base_rate);
        builder = builder.payment_frequency(pay_freq);
        builder = builder.fees(fees_struct);
        builder = builder.draw_repay_spec(draw_repay);
        builder = builder.discount_curve_id(discount_curve_id);
        builder = builder.day_count(DayCount::Act365F);
        let rev_credit = builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to build RevolvingCredit: {e}"
            ))
        })?;
        Ok(Self::new(rev_credit))
    }

    #[pyo3(
        signature = (market, as_of=None, capture_mode="sample", sample_count=100, seed=42),
        text_signature = "(self, market, as_of=None, capture_mode='sample', sample_count=100, seed=42)"
    )]
    /// Run Monte Carlo for this facility and return paths (sampled) with cumulative payoff.
    ///
    /// Args:
    ///     market: MarketContext with discount/forward/hazard curves.
    ///     as_of: Optional valuation date; defaults to discount curve base date.
    ///     capture_mode: 'sample' or 'all' for path capture.
    ///     sample_count: Number of paths to capture when mode='sample'.
    ///     seed: RNG seed.
    ///
    /// Returns:
    ///     MonteCarloResult: Contains estimate and captured PathDataset (if configured).
    fn mc_paths(
        &self,
        py: pyo3::Python<'_>,
        market: &PyMarketContext,
        as_of: Option<pyo3::Bound<'_, pyo3::PyAny>>,
        capture_mode: &str,
        sample_count: usize,
        seed: u64,
    ) -> pyo3::PyResult<PyMonteCarloResult> {
        use finstack_valuations::instruments::common::mc::process::ProcessMetadata;
        use finstack_valuations::instruments::common::mc::traits::StochasticProcess;
        use finstack_valuations::instruments::common::mc::rng::philox::PhiloxRng;
        use finstack_valuations::instruments::common::mc::rng::sobol::SobolRng;
        use finstack_valuations::instruments::common::mc::time_grid::TimeGrid;
        use finstack_valuations::instruments::common::models::monte_carlo::discretization::revolving_credit::RevolvingCreditDiscretization;
        use finstack_valuations::instruments::common::models::monte_carlo::engine::{McEngineBuilder, PathCaptureConfig};
        use finstack_valuations::instruments::common::models::monte_carlo::payoff::revolving_credit::{FeeStructure, RevolvingCreditPayoff};
        use finstack_valuations::instruments::common::models::monte_carlo::process::revolving_credit::{
            CreditSpreadParams, InterestRateSpec, RevolvingCreditProcess, RevolvingCreditProcessParams, UtilizationParams,
        };

        // Ensure stochastic spec with mc_config exists
        let stoch_spec = match &self.inner.draw_repay_spec {
            finstack_valuations::instruments::revolving_credit::types::DrawRepaySpec::Stochastic(spec) => spec.as_ref(),
            _ => {
                return Err(PyValueError::new_err(
                    "mc_paths requires a stochastic draw_repay_spec with mc_config",
                ))
            }
        };
        let mc_config = stoch_spec.mc_config.as_ref().ok_or_else(|| {
            PyValueError::new_err("mc_paths requires mc_config in stochastic specification")
        })?;

        // Resolve valuation date from discount curve if not provided
        let disc = market
            .inner
            .get_discount_ref(self.inner.discount_curve_id.as_str())
            .map_err(crate::core::error::core_to_py)?;
        let as_of_date = match as_of {
            Some(d) => crate::core::utils::py_to_date(&d)?,
            None => disc.base_date(),
        };

        // Utilization params
        let util_params = match &stoch_spec.utilization_process {
            finstack_valuations::instruments::revolving_credit::types::UtilizationProcess::MeanReverting { target_rate, speed, volatility } => {
                UtilizationParams::new(*speed, *target_rate, *volatility)
            }
        };

        // Interest rate spec
        use finstack_valuations::instruments::revolving_credit::types::BaseRateSpec as RcBase;
        let interest_rate_spec = match &self.inner.base_rate_spec {
            RcBase::Fixed { rate } => InterestRateSpec::Fixed { rate: *rate },
            RcBase::Floating { .. } => {
                match &mc_config.interest_rate_process {
                    Some(finstack_valuations::instruments::revolving_credit::types::InterestRateProcessSpec::HullWhite1F{ kappa, sigma, initial, theta }) => {
                        use finstack_valuations::instruments::common::mc::process::ou::HullWhite1FParams;
                        InterestRateSpec::Floating { params: HullWhite1FParams::new(*kappa, *sigma, *theta), initial: *initial }
                    }
                    None => {
                        // Deterministic forward curve fallback
                        let fwd = market
                            .inner
                            .get_forward_ref(match &self.inner.base_rate_spec { RcBase::Floating { index_id, .. } => index_id.as_str(), _ => unreachable!() })
                            .map_err(crate::core::error::core_to_py)?;
                        let times = fwd.knots().to_vec();
                        let rates = fwd.forwards().to_vec();
                        InterestRateSpec::DeterministicForward { times, rates }
                    }
                }
            }
        };

        // Credit spread params (supports MarketAnchored and others)
        use finstack_valuations::instruments::revolving_credit::types::CreditSpreadProcessSpec as CsSpec;
        let credit_spread_params = match &mc_config.credit_spread_process {
            CsSpec::Cir { kappa, theta, sigma, initial } => CreditSpreadParams::new(*kappa, *theta, *sigma, *initial),
            CsSpec::Constant(spread) => CreditSpreadParams::new(0.01, *spread, 0.001, *spread),
            CsSpec::MarketAnchored { hazard_curve_id, kappa, implied_vol, tenor_years } => {
                let hazard = market
                    .inner
                    .get_hazard_ref(hazard_curve_id.as_str())
                    .map_err(crate::core::error::core_to_py)?;
                let dc = hazard.day_count();
                let base_date = hazard.base_date();
                let t_maturity = dc
                    .year_fraction(base_date, self.inner.maturity_date, finstack_core::dates::DayCountCtx::default())
                    .map_err(crate::core::error::core_to_py)?;
                let t = tenor_years.unwrap_or_else(|| t_maturity.max(1e-8));
                let sp_t = hazard.sp(t);
                let avg_lambda = if t > 0.0 { (-sp_t.ln()) / t } else { 0.0 };
                let mut first_lambda = None;
                if let Some((_, lambda)) = hazard.knot_points().next() { first_lambda = Some(lambda.max(0.0)); }
                let lambda0 = first_lambda.unwrap_or(avg_lambda).max(0.0);
                let one_minus_r = (1.0 - mc_config.recovery_rate).max(1e-6);
                let s0 = one_minus_r * lambda0;
                let s_bar = one_minus_r * avg_lambda;
                let k = *kappa;
                let a = if (k * t).abs() < 1e-8 { 1.0 - 0.5 * k * t } else { (1.0 - (-k * t).exp()) / (k * t) };
                let theta = if (1.0 - a).abs() < 1e-12 { s_bar } else { ((s_bar - a * s0) / (1.0 - a)).max(0.0) };
                let sigma = (*implied_vol) * s_bar.max(1e-12).sqrt();
                CreditSpreadParams::new(k, theta, sigma, s0)
            }
        };

        // Time grid setup (quarterly)
        let disc_curve = disc; // alias
        let disc_dc = disc_curve.day_count();
        let base_date = disc_curve.base_date();
        let t_start = disc_dc
            .year_fraction(base_date, self.inner.commitment_date.max(as_of_date), finstack_core::dates::DayCountCtx::default())
            .map_err(crate::core::error::core_to_py)?;
        let t_end = disc_dc
            .year_fraction(base_date, self.inner.maturity_date, finstack_core::dates::DayCountCtx::default())
            .map_err(crate::core::error::core_to_py)?;
        let time_horizon = (t_end - t_start).max(0.0);
        let num_steps = ((time_horizon / 0.25).ceil() as usize).max(1);
        let time_grid = TimeGrid::uniform(time_horizon, num_steps).map_err(crate::core::error::core_to_py)?;

        // Build process and discretization
        let mut process_params = RevolvingCreditProcessParams::new(util_params, interest_rate_spec, credit_spread_params);
        if let Some(corr) = mc_config.correlation_matrix {
            process_params = process_params.with_correlation(corr);
        } else if let Some(rho) = mc_config.util_credit_corr.or(Some(0.8)) {
            let correlation = [ [1.0, 0.0, rho], [0.0, 1.0, 0.0], [rho, 0.0, 1.0] ];
            process_params = process_params.with_correlation(correlation);
        }
        process_params = process_params.with_time_offset(t_start);
        let process = RevolvingCreditProcess::new(process_params);
        let discz = RevolvingCreditDiscretization::from_process(&process).map_err(crate::core::error::core_to_py)?;

        // Precompute discount factors for payoff (needed for internal PV accumulation)
        let disc_curve = market.inner.get_discount_ref(self.inner.discount_curve_id.as_str())
            .map_err(crate::core::error::core_to_py)?;
        let disc_dc = disc_curve.day_count();
        let base_date = disc_curve.base_date();
        
        let t_as_of = disc_dc
            .year_fraction(base_date, as_of_date, finstack_core::dates::DayCountCtx::default())
            .map_err(crate::core::error::core_to_py)?;
        let df_as_of = disc_curve.df(t_as_of);
        
        let mut discount_factors = Vec::with_capacity(num_steps + 1);
        discount_factors.push(if df_as_of > 0.0 { disc_curve.df(t_start) / df_as_of } else { 1.0 });
        for i in 0..num_steps {
            let t_abs = t_start + time_grid.time(i + 1);
            let df_abs = disc_curve.df(t_abs);
            discount_factors.push(if df_as_of > 0.0 { df_abs / df_as_of } else { 1.0 });
        }
        
        // Payoff
        let fees = FeeStructure::new(
            self.inner.fees.commitment_fee_bp,
            self.inner.fees.usage_fee_bp,
            self.inner.fees.facility_fee_bp,
        );
        let is_fixed_rate = matches!(self.inner.base_rate_spec, RcBase::Fixed { .. });
        let (fixed_rate, margin_bp) = match &self.inner.base_rate_spec {
            RcBase::Fixed { rate } => (*rate, 0.0),
            RcBase::Floating { margin_bp, .. } => (0.0, *margin_bp),
        };
        let payoff = RevolvingCreditPayoff::new(
            self.inner.commitment_amount.amount(),
            self.inner.day_count,
            is_fixed_rate,
            fixed_rate,
            margin_bp,
            fees,
            mc_config.recovery_rate,
            time_horizon,
            discount_factors,
        );

        // Engine with path capture (payoffs enabled)
        let path_capture = match capture_mode {
            "all" => PathCaptureConfig::all().with_payoffs(),
            _ => PathCaptureConfig::sample(sample_count, seed).with_payoffs(),
        };
        let engine = McEngineBuilder::new()
            .num_paths(stoch_spec.num_paths)
            .seed(stoch_spec.seed.unwrap_or(seed))
            .time_grid(time_grid)
            .parallel(false) // Controlled by user; parallel feature must be enabled separately
            .antithetic(stoch_spec.antithetic)
            .path_capture(path_capture)
            .build()
            .map_err(crate::core::error::core_to_py)?;

        // RNGs
        let rng_philox = PhiloxRng::new(seed);
        let sobol_dim = process.num_factors();
        let rng_sobol = SobolRng::new(sobol_dim, seed);
        let use_sobol = stoch_spec.use_sobol_qmc;

        // Initial state
        let initial_utilization = self.inner.utilization_rate();
        let initial_state = process.params().initial_state(initial_utilization);

        // Run with capture and return MonteCarloResult
        let mc_result = py.allow_threads(|| {
            if use_sobol {
                engine
                    .price_with_capture::<SobolRng, _, _, _>(
                        &rng_sobol,
                        &process,
                        &discz,
                        &initial_state,
                        &payoff,
                        self.inner.commitment_amount.currency(),
                        1.0,
                        process.metadata(),
                    )
                    .map_err(crate::core::error::core_to_py)
            } else {
                engine
                    .price_with_capture::<PhiloxRng, _, _, _>(
                        &rng_philox,
                        &process,
                        &discz,
                        &initial_state,
                        &payoff,
                        self.inner.commitment_amount.currency(),
                        1.0,
                        process.metadata(),
                    )
                    .map_err(crate::core::error::core_to_py)
            }
        })?;

        // Handle upfront fee at pricer level (one-time cashflow, not path-dependent)
        // Upfront fee is paid by lender at commitment, so it reduces facility value
        let upfront_fee_pv = if let Some(upfront_fee) = self.inner.fees.upfront_fee {
            // Recompute discount factors for upfront fee adjustment
            let disc_curve = market.inner.get_discount_ref(self.inner.discount_curve_id.as_str())
                .map_err(crate::core::error::core_to_py)?;
            let base_date = disc_curve.base_date();
            let disc_dc = disc_curve.day_count();
            
            let t_start = disc_dc
                .year_fraction(base_date, self.inner.commitment_date, finstack_core::dates::DayCountCtx::default())
                .map_err(crate::core::error::core_to_py)?;
            let t_as_of = disc_dc
                .year_fraction(base_date, as_of_date, finstack_core::dates::DayCountCtx::default())
                .map_err(crate::core::error::core_to_py)?;
            
            let df_as_of = disc_curve.df(t_as_of);
            let df_commitment = disc_curve.df(t_start);
            let df = if df_as_of > 0.0 { df_commitment / df_as_of } else { 1.0 };
            upfront_fee.amount() * df
        } else {
            0.0
        };

        // Adjust the PV: lender pays upfront fee (outflow), so subtract from PV
        let mut adjusted_result = mc_result;
        let adjusted_mean = adjusted_result.estimate.mean.amount() - upfront_fee_pv;
        adjusted_result.estimate.mean = finstack_core::money::Money::new(
            adjusted_mean,
            self.inner.commitment_amount.currency(),
        );

        Ok(PyMonteCarloResult::new(adjusted_result))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Commitment amount.
    #[getter]
    fn commitment_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.commitment_amount)
    }

    /// Drawn amount.
    #[getter]
    fn drawn_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.drawn_amount)
    }

    /// Commitment date.
    #[getter]
    fn commitment_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.commitment_date)
    }

    /// Maturity date.
    #[getter]
    fn maturity_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.maturity_date)
    }

    /// Price the facility using the appropriate method (deterministic or MC).
    ///
    /// Args:
    ///     market: MarketContext with curves
    ///     as_of: Valuation date
    ///
    /// Returns:
    ///     Money: Present value of the facility
    fn value(&self, market: &PyMarketContext, as_of: Bound<'_, PyAny>) -> PyResult<PyMoney> {
        use crate::core::error::core_to_py;
        use finstack_valuations::instruments::common::traits::Instrument;
        
        let date = py_to_date(&as_of)?;
        let pv = self.inner.value(&market.inner, date).map_err(core_to_py)?;
        Ok(PyMoney::new(pv))
    }

    /// Alias for value() - compute NPV of the facility.
    ///
    /// Args:
    ///     market: MarketContext with curves
    ///     as_of: Valuation date
    ///
    /// Returns:
    ///     Money: Net present value of the facility
    fn npv(&self, market: &PyMarketContext, as_of: Bound<'_, PyAny>) -> PyResult<PyMoney> {
        self.value(market, as_of)
    }

    /// Get utilization rate (drawn / commitment).
    ///
    /// Returns:
    ///     float: Current utilization rate (0.0 to 1.0)
    fn utilization_rate(&self) -> f64 {
        self.inner.utilization_rate()
    }

    /// Get undrawn amount.
    ///
    /// Returns:
    ///     Money: Undrawn commitment amount
    fn undrawn_amount(&self) -> PyResult<PyMoney> {
        use crate::core::error::core_to_py;
        let amount = self.inner.undrawn_amount().map_err(core_to_py)?;
        Ok(PyMoney::new(amount))
    }

    /// Check if facility uses deterministic cashflows.
    ///
    /// Returns:
    ///     bool: True if deterministic, False if stochastic
    fn is_deterministic(&self) -> bool {
        self.inner.is_deterministic()
    }

    /// Check if facility uses stochastic utilization.
    ///
    /// Returns:
    ///     bool: True if stochastic, False if deterministic
    fn is_stochastic(&self) -> bool {
        self.inner.is_stochastic()
    }

    /// Extract cashflows from Monte Carlo paths as a pandas DataFrame.
    ///
    /// Convenience method that runs mc_paths() and extracts cashflows into
    /// a pandas DataFrame for easy analysis.
    ///
    /// Args:
    ///     market: MarketContext with curves
    ///     as_of: Optional valuation date
    ///     num_paths: Number of paths to simulate (default: 1000)
    ///     capture_mode: 'all' or 'sample' (default: 'all')
    ///     seed: RNG seed for reproducibility (default: 42)
    ///
    /// Returns:
    ///     pd.DataFrame: Cashflows with columns:
    ///         - path_id: path identifier
    ///         - step: timestep index
    ///         - time_years: time in years
    ///         - amount: cashflow amount
    ///         - cashflow_type: type of cashflow (Principal, Interest, etc.)
    ///
    /// Example:
    ///     >>> df = revolver.cashflows_df(market, as_of)
    ///     >>> principal_flows = df[df['cashflow_type'] == 'Principal']
    ///     >>> interest_flows = df[df['cashflow_type'] == 'Interest']
    #[pyo3(signature = (market, as_of=None, num_paths=None, capture_mode="all", seed=42))]
    fn cashflows_df(
        &self,
        py: Python,
        market: &PyMarketContext,
        as_of: Option<Bound<'_, PyAny>>,
        num_paths: Option<usize>,
        capture_mode: &str,
        seed: u64,
    ) -> PyResult<PyObject> {
        // Determine sample count (all paths when capture_mode="all")
        let sample_count = num_paths.unwrap_or(1000);
        
        // Run MC simulation with path capture
        let mc_result = self.mc_paths(py, market, as_of, capture_mode, sample_count, seed)?;
        
        // Extract paths from result
        let path_dataset_inner = mc_result.inner.paths
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("No paths captured in MC result"))?;
        
        // Create Python wrapper for PathDataset
        use crate::valuations::common::mc::paths::PyPathDataset;
        let py_paths = Py::new(py, PyPathDataset { inner: path_dataset_inner.clone() })?;
        
        // Call the method on the Python object
        let df = py_paths.call_method1(py, "cashflows_to_dataframe", ())?;
        Ok(df)
    }

    /// Calculate IRR distribution from Monte Carlo paths.
    ///
    /// Computes the Internal Rate of Return for each simulated path using
    /// the path's principal cashflows (deployments and repayments).
    ///
    /// Args:
    ///     market: MarketContext with curves
    ///     as_of: Optional valuation date
    ///     num_paths: Number of paths to simulate (default: 1000)
    ///     seed: RNG seed (default: 42)
    ///
    /// Returns:
    ///     dict: Dictionary with keys:
    ///         - irrs: list of IRR values for each path (None if no sign change)
    ///         - mean: mean IRR across paths
    ///         - std: standard deviation of IRRs
    ///         - percentiles: dict with p10, p25, p50, p75, p90
    ///
    /// Example:
    ///     >>> irr_stats = revolver.irr_distribution(market, as_of)
    ///     >>> print(f"Mean IRR: {irr_stats['mean']:.2%}")
    ///     >>> print(f"Median IRR: {irr_stats['percentiles']['p50']:.2%}")
    #[pyo3(signature = (market, as_of=None, num_paths=1000, seed=42))]
    fn irr_distribution(
        &self,
        py: Python,
        market: &PyMarketContext,
        as_of: Option<Bound<'_, PyAny>>,
        num_paths: usize,
        seed: u64,
    ) -> PyResult<PyObject> {
        use crate::core::error::core_to_py;
        
        // Run MC simulation with all paths captured
        let mc_result = self.mc_paths(py, market, as_of.clone(), "all", num_paths, seed)?;
        
        // Extract paths (access inner field directly)
        let path_dataset = mc_result.inner.paths
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("No paths captured in MC result"))?;
        
        // Get base date for IRR calculation
        let disc = market.inner
            .get_discount_ref(self.inner.discount_curve_id.as_str())
            .map_err(core_to_py)?;
        let base_date = disc.base_date();
        
        // Calculate IRR for each path
        let mut irrs = Vec::new();
        
        for path in &path_dataset.paths {
            // Collect all cashflows for this path
            let mut path_cashflows: Vec<(f64, f64)> = Vec::new();
            
            for point in &path.points {
                for (time, amount, _cf_type) in &point.cashflows {
                    path_cashflows.push((*time, *amount));
                }
            }
            
            // Aggregate cashflows by time (sum amounts at same time)
            let mut time_map = std::collections::HashMap::new();
            for (time, amount) in path_cashflows {
                *time_map.entry((time * 1000.0).round() as i64).or_insert(0.0) += amount;
            }
            
            let mut aggregated: Vec<(f64, f64)> = time_map
                .into_iter()
                .map(|(time_key, amount)| (time_key as f64 / 1000.0, amount))
                .collect();
            aggregated.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            
            // Calculate IRR using core XIRR
            use finstack_valuations::instruments::revolving_credit::metrics::irr::calculate_path_irr;
            let irr = calculate_path_irr(&aggregated, base_date, self.inner.day_count);
            
            irrs.push(irr);
        }
        
        // Calculate statistics
        let valid_irrs: Vec<f64> = irrs.iter().filter_map(|&x| x).collect();
        
        if valid_irrs.is_empty() {
            return Err(PyValueError::new_err(
                "No valid IRRs calculated (all paths may have same-sign cashflows)"
            ));
        }
        
        let mean = valid_irrs.iter().sum::<f64>() / valid_irrs.len() as f64;
        let variance = valid_irrs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / valid_irrs.len() as f64;
        let std = variance.sqrt();
        
        // Calculate percentiles
        let mut sorted = valid_irrs.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let percentile = |p: f64| -> f64 {
            let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
            sorted[idx.min(sorted.len() - 1)]
        };
        
        // Create result dictionary
        use pyo3::types::PyDict;
        let result = PyDict::new(py);
        result.set_item("irrs", irrs)?;
        result.set_item("mean", mean)?;
        result.set_item("std", std)?;
        
        let percentiles = PyDict::new(py);
        percentiles.set_item("p10", percentile(10.0))?;
        percentiles.set_item("p25", percentile(25.0))?;
        percentiles.set_item("p50", percentile(50.0))?;
        percentiles.set_item("p75", percentile(75.0))?;
        percentiles.set_item("p90", percentile(90.0))?;
        result.set_item("percentiles", percentiles)?;
        
        Ok(result.into())
    }

    /// Build the cashflow schedule for this facility (deterministic only).
    ///
    /// Args:
    ///     market: MarketContext (not used for deterministic, but required for API consistency)
    ///     as_of: Optional valuation date; defaults to discount curve base date
    ///
    /// Returns:
    ///     CashFlowSchedule: The generated cashflow schedule
    #[pyo3(
        signature = (market, as_of=None),
        text_signature = "(self, market, as_of=None)"
    )]
    #[allow(unused_variables)]
    fn build_schedule(
        &self,
        market: &PyMarketContext,
        as_of: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyCashFlowSchedule> {
        // Only works for deterministic specs
        if !self.inner.is_deterministic() {
            return Err(PyValueError::new_err(
                "build_schedule only works for deterministic draw_repay_spec"
            ));
        }

        // Resolve as_of date
        let as_of_date = if let Some(as_of_obj) = as_of {
            py_to_date(&as_of_obj)?
        } else {
            // Default to commitment_date
            self.inner.commitment_date
        };

        // Generate cashflow schedule with curves (include floating base-rate projections)
        use finstack_valuations::instruments::revolving_credit::cashflows::generate_deterministic_cashflows_with_curves;
        let schedule = generate_deterministic_cashflows_with_curves(&self.inner, &market.inner, as_of_date)
            .map_err(core_to_py)?;

        Ok(PyCashFlowSchedule::new(schedule))
    }

    /// Compute per-period present values for this facility's cashflows.
    ///
    /// Args:
    ///     periods: PeriodPlan, list[Period], or period range string
    ///     market: MarketContext with discount/forward/hazard curves
    ///     discount_curve_id: Optional curve ID (defaults to facility's discount_curve_id)
    ///     hazard_curve_id: Optional hazard curve ID for credit adjustment
    ///     as_of: Optional valuation date (defaults to discount curve base_date)
    ///     day_count: Optional day count convention (defaults to discount curve day_count)
    ///
    /// Returns:
    ///     dict[str, float]: Map from period code to PV amount
    #[pyo3(
        signature = (periods, market, *, discount_curve_id=None, hazard_curve_id=None, as_of=None, day_count=None),
        text_signature = "(periods, market, /, *, discount_curve_id=None, hazard_curve_id=None, as_of=None, day_count=None)"
    )]
    fn per_period_pv(
        &self,
        py: Python<'_>,
        periods: Bound<'_, PyAny>,
        market: &PyMarketContext,
        discount_curve_id: Option<&str>,
        hazard_curve_id: Option<&str>,
        as_of: Option<Bound<'_, PyAny>>,
        day_count: Option<Bound<'_, PyAny>>,
    ) -> PyResult<HashMap<String, f64>> {
        // Build schedule first
        let schedule = self.build_schedule(market, as_of.clone())?;
        
        // Delegate to schedule's per_period_pv method
        // Need to convert PyMarketContext reference to Bound<PyAny>
        let disc_id = discount_curve_id.unwrap_or_else(|| self.inner.discount_curve_id.as_str());
        let market_py = Py::new(py, market.clone())?;
        let market_bound_temp = market_py.into_bound(py);
        let market_bound: Bound<'_, PyAny> = unsafe {
            <Bound<'_, PyAny> as Clone>::clone(&market_bound_temp).downcast_into_unchecked()
        };
        schedule.per_period_pv(py, periods, market_bound, Some(disc_id), hazard_curve_id, as_of, day_count)
    }

    /// Convert cashflow schedule to a period-aligned DataFrame.
    ///
    /// Returns a dict-of-arrays suitable for pandas/Polars with columns:
    /// Start Date, End Date, PayDate, CFType, Currency, Notional, YrFraq,
    /// Days, Amount, DiscountFactor, SurvivalProb (optional), PV,
    /// Unfunded Amount (optional), Commitment Amount (optional),
    /// Base Rate (optional), Spread (optional), allin_rate.
    ///
    /// Note: For revolving credit, Notional represents the drawn amount at each period,
    /// and Commitment Amount (when provided) represents the total facility commitment.
    ///
    /// Args:
    ///     periods: PeriodPlan, list[Period], or period range string
    ///     market: MarketContext with discount/forward/hazard curves
    ///     discount_curve_id: Optional curve ID (defaults to facility's discount_curve_id)
    ///     hazard_curve_id: Optional hazard curve ID for credit adjustment (adds SurvivalProb column)
    ///     forward_curve_id: Optional forward curve ID for floating rate decomposition
    ///     as_of: Optional valuation date (defaults to discount curve base_date)
    ///     day_count: Optional day count convention (defaults to schedule.day_count)
    ///     facility_limit: Optional facility limit Money for Unfunded Amount column
    ///     include_floating_decomposition: If True, adds Base Rate and Spread columns for floating cashflows
    ///
    /// Returns:
    ///     dict: Dictionary with column names as keys and lists as values
    #[pyo3(
        signature = (periods, market, *, discount_curve_id=None, hazard_curve_id=None, forward_curve_id=None, as_of=None, day_count=None, facility_limit=None, include_floating_decomposition=false),
        text_signature = "(periods, market, /, *, discount_curve_id=None, hazard_curve_id=None, forward_curve_id=None, as_of=None, day_count=None, facility_limit=None, include_floating_decomposition=False)"
    )]
    fn to_period_dataframe(
        &self,
        py: Python<'_>,
        periods: Bound<'_, PyAny>,
        market: &PyMarketContext,
        discount_curve_id: Option<&str>,
        hazard_curve_id: Option<&str>,
        forward_curve_id: Option<&str>,
        as_of: Option<Bound<'_, PyAny>>,
        day_count: Option<Bound<'_, PyAny>>,
        facility_limit: Option<Bound<'_, PyAny>>,
        include_floating_decomposition: bool,
    ) -> PyResult<PyObject> {
        // Build schedule first
        let schedule = self.build_schedule(market, as_of.clone())?;
        
        // Delegate to schedule's to_period_dataframe method
        let disc_id = discount_curve_id.unwrap_or_else(|| self.inner.discount_curve_id.as_str());
        
        // For floating rate decomposition, use forward_curve_id if provided, otherwise try to extract from base_rate_spec
        let fwd_id = if include_floating_decomposition {
            forward_curve_id.or_else(|| {
                match &self.inner.base_rate_spec {
                    finstack_valuations::instruments::revolving_credit::BaseRateSpec::Floating { index_id, .. } => {
                        Some(index_id.as_str())
                    }
                    _ => None
                }
            })
        } else {
            forward_curve_id
        };
        
        // Need to convert PyMarketContext reference to Bound<PyAny>
        let market_py = Py::new(py, market.clone())?;
        let market_bound_temp = market_py.into_bound(py);
        let market_bound: Bound<'_, PyAny> = unsafe {
            <Bound<'_, PyAny> as Clone>::clone(&market_bound_temp).downcast_into_unchecked()
        };
        let out = schedule.to_period_dataframe(
            py,
            periods,
            market_bound,
            Some(disc_id),
            hazard_curve_id,
            fwd_id,
            as_of,
            day_count,
            facility_limit,
            include_floating_decomposition,
        )?;

        // Post-process columns:
        // - allin_rate: effective coupon (Fixed rate, or Base Rate + Spread for FloatReset)
        // - Drop 'Rate' to avoid duplication when Base Rate/Spread are present
        let out_bound = out.into_bound(py);
        if let Ok(dict) = out_bound.clone().downcast_into::<PyDict>() {
            // Extract existing columns
            let cf_any = dict.get_item("CFType")?;
            let rate_any = dict.get_item("Rate")?;
            if let (Some(cf_any), Some(rate_any)) = (cf_any, rate_any) {
                if let (Ok(cf_list), Ok(rate_list)) = (cf_any.downcast::<PyList>(), rate_any.downcast::<PyList>()) {
                    let n = cf_list.len();
                    // Build allin_rate from existing 'Rate'
                    let mut allin_vals: Vec<PyObject> = Vec::with_capacity(n);
                    for i in 0..n {
                        allin_vals.push(rate_list.get_item(i).unwrap().clone().unbind());
                    }

                    // Set new column and drop 'Rate'
                    dict.set_item("allin_rate", PyList::new(py, allin_vals)?)?;
                    let _ = dict.del_item("Rate");
                    return Ok(dict.into_any().into());
                }
            }
            // If structure differs, fall through and return original
            return Ok(dict.into_any().into());
        }
        // Default return if not a dict
        Ok(out_bound.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "RevolvingCredit(id='{}', commitment={}, drawn={}, util={:.1}%)",
            self.inner.id.as_str(),
            self.inner.commitment_amount.amount(),
            self.inner.drawn_amount.amount(),
            self.inner.utilization_rate() * 100.0
        )
    }
}

fn parse_frequency(freq_str: Option<&str>) -> PyResult<finstack_core::dates::Frequency> {
    parse_frequency_label(freq_str)
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyRevolvingCredit>()?;
    Ok(vec!["RevolvingCredit"])
}
