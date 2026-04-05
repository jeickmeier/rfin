//! Nelson-Siegel / Nelson-Siegel-Svensson parametric curve calibration target.
//!
//! Implements [`GlobalSolveTarget`] to fit parametric yield curves from
//! market instruments using the Levenberg-Marquardt optimizer.

use crate::calibration::api::schema::ParametricCurveParams;
use crate::calibration::config::CalibrationConfig;
use crate::calibration::prepared::CalibrationQuote;
use crate::calibration::solver::global::GlobalFitOptimizer;
use crate::calibration::solver::traits::GlobalSolveTarget;
use crate::calibration::CalibrationReport;
use crate::market::quotes::market_quote::{ExtractQuotes, MarketQuote};
use crate::market::quotes::rates::RateQuote;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{NelsonSiegelModel, NsVariant, ParametricCurve};
use finstack_core::market_data::traits::Discounting;
use finstack_core::types::CurveId;
use finstack_core::Result;

/// Parameters for constructing a [`ParametricCurveTarget`].
#[derive(Clone)]
pub struct ParametricCurveTargetParams {
    /// Base date for the calibration.
    pub base_date: Date,
    /// Curve identifier.
    pub curve_id: CurveId,
    /// NS or NSS variant.
    pub variant: NsVariant,
    /// Optional initial parameter guesses.
    pub initial_params: Option<NelsonSiegelModel>,
    /// Base market context.
    pub base_context: MarketContext,
}

/// Calibration target for parametric (NS/NSS) curves.
///
/// Uses global optimization to fit 4 (NS) or 6 (NSS) parameters
/// from rate instrument quotes.
pub struct ParametricCurveTarget {
    params: ParametricCurveTargetParams,
}

impl ParametricCurveTarget {
    /// Create a new parametric curve target.
    pub fn new(params: ParametricCurveTargetParams) -> Self {
        Self { params }
    }

    /// Execute the full calibration for a parametric curve step.
    pub fn solve(
        schema_params: &ParametricCurveParams,
        quotes: &[MarketQuote],
        context: &MarketContext,
        global_config: &CalibrationConfig,
    ) -> Result<(MarketContext, CalibrationReport)> {
        let rates_quotes: Vec<RateQuote> = quotes.extract_quotes();
        if rates_quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::InputError::TooFewPoints,
            ));
        }

        let curve_dc = finstack_core::dates::DayCount::Act365F;
        let mut curve_ids = finstack_core::HashMap::default();
        let discount_id = schema_params
            .discount_curve_id
            .as_ref()
            .unwrap_or(&schema_params.curve_id);
        curve_ids.insert("discount".to_string(), discount_id.to_string());

        let build_ctx = crate::market::build::context::BuildCtx::new(
            schema_params.base_date,
            1_000_000.0,
            curve_ids,
        );

        let pillar_policy = crate::market::build::prepared::PillarPolicy::default();
        let mut prepared_quotes: Vec<CalibrationQuote> = Vec::with_capacity(rates_quotes.len());
        for q in rates_quotes {
            let prepared = crate::market::build::prepared::prepare_rate_quote(
                q,
                &build_ctx,
                curve_dc,
                schema_params.base_date,
                &pillar_policy,
            )?;
            prepared_quotes.push(CalibrationQuote::Rates(prepared));
        }

        let initial_params = schema_params.initial_params.clone().or_else(|| {
            Some(match schema_params.model {
                NsVariant::Ns => NelsonSiegelModel::Ns {
                    beta0: 0.03,
                    beta1: -0.02,
                    beta2: 0.01,
                    tau: 1.5,
                },
                NsVariant::Nss => NelsonSiegelModel::Nss {
                    beta0: 0.03,
                    beta1: -0.02,
                    beta2: 0.01,
                    beta3: 0.01,
                    tau1: 1.5,
                    tau2: 5.0,
                },
            })
        });

        let target = Self::new(ParametricCurveTargetParams {
            base_date: schema_params.base_date,
            curve_id: schema_params.curve_id.clone(),
            variant: schema_params.model,
            initial_params: initial_params.clone(),
            base_context: context.clone(),
        });

        let config = global_config.clone();
        let success_tolerance = Some(config.discount_curve.validation_tolerance);
        let (curve, report) =
            GlobalFitOptimizer::optimize(&target, &prepared_quotes, &config, success_tolerance)?;

        let new_context = context.clone().insert(curve);
        Ok((new_context, report))
    }

    fn default_guesses(&self) -> Vec<f64> {
        if let Some(ref model) = self.params.initial_params {
            return model.to_params_vec();
        }
        match self.params.variant {
            NsVariant::Ns => vec![0.03, -0.02, 0.01, 1.5],
            NsVariant::Nss => vec![0.03, -0.02, 0.01, 0.01, 1.5, 5.0],
        }
    }
}

impl GlobalSolveTarget for ParametricCurveTarget {
    type Quote = CalibrationQuote;
    type Curve = ParametricCurve;

    fn build_time_grid_and_guesses(
        &self,
        quotes: &[Self::Quote],
    ) -> Result<(Vec<f64>, Vec<f64>, Vec<Self::Quote>)> {
        let guesses = self.default_guesses();
        // Dummy time grid - one entry per parameter
        let times: Vec<f64> = (0..guesses.len()).map(|i| i as f64).collect();
        Ok((times, guesses, quotes.to_vec()))
    }

    fn build_curve_from_params(&self, _times: &[f64], params: &[f64]) -> Result<Self::Curve> {
        let model = NelsonSiegelModel::from_params_vec(self.params.variant, params)?;
        ParametricCurve::builder(self.params.curve_id.clone())
            .base_date(self.params.base_date)
            .model(model)
            .build()
    }

    fn build_curve_for_solver_from_params(
        &self,
        _times: &[f64],
        params: &[f64],
    ) -> Result<Self::Curve> {
        // For NS/NSS, clamp tau values to avoid invalid parameters during solver iterations
        let mut p = params.to_vec();
        match self.params.variant {
            NsVariant::Ns => {
                if p.len() == 4 {
                    p[3] = p[3].max(0.01);
                }
            }
            NsVariant::Nss => {
                if p.len() == 6 {
                    p[4] = p[4].max(0.01);
                    p[5] = p[5].max(0.01);
                    // Ensure tau1 != tau2
                    if (p[4] - p[5]).abs() < 0.01 {
                        p[5] = p[4] + 0.5;
                    }
                }
            }
        }
        let model = NelsonSiegelModel::from_params_vec(self.params.variant, &p)?;
        ParametricCurve::builder(self.params.curve_id.clone())
            .base_date(self.params.base_date)
            .model(model)
            .build()
    }

    fn calculate_residuals(
        &self,
        curve: &Self::Curve,
        quotes: &[Self::Quote],
        residuals: &mut [f64],
    ) -> Result<()> {
        // Build a sampled DiscountCurve from the parametric curve to price instruments.
        let sample_times: Vec<f64> = {
            let mut times = vec![0.0];
            for q in quotes {
                let t = q.pillar_time();
                if t > 0.0 {
                    times.push(t);
                }
            }
            times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            times.dedup_by(|a, b| (*a - *b).abs() < 1e-10);
            // Add intermediate points for curve shape
            let max_t = times.last().copied().unwrap_or(30.0);
            let mut extended = times;
            let mut t = 0.5;
            while t < max_t {
                extended.push(t);
                t += 0.5;
            }
            extended.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            extended.dedup_by(|a, b| (*a - *b).abs() < 1e-10);
            extended
        };

        let knots: Vec<(f64, f64)> = sample_times.iter().map(|&t| (t, curve.df(t))).collect();
        let disc_curve = finstack_core::market_data::term_structures::DiscountCurve::builder(
            self.params.curve_id.clone(),
        )
        .base_date(self.params.base_date)
        .knots(knots)
        .allow_non_monotonic()
        .build_for_solver()?;

        let temp_context = self.params.base_context.clone().insert(disc_curve);

        for (i, q) in quotes.iter().enumerate() {
            let pv = q
                .get_instrument()
                .value_raw(&temp_context, self.params.base_date)?;
            residuals[i] = pv;
        }
        Ok(())
    }

    fn lower_bounds(&self) -> Option<Vec<f64>> {
        Some(match self.params.variant {
            NsVariant::Ns => vec![
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
                0.01,
            ],
            NsVariant::Nss => vec![
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
                0.01,
                0.01,
            ],
        })
    }

    fn upper_bounds(&self) -> Option<Vec<f64>> {
        Some(match self.params.variant {
            NsVariant::Ns => vec![f64::INFINITY, f64::INFINITY, f64::INFINITY, 30.0],
            NsVariant::Nss => {
                vec![
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::INFINITY,
                    30.0,
                    30.0,
                ]
            }
        })
    }
}
