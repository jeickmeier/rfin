//! Inflation curve bootstrapping from ZC inflation swaps and ILBs.
//!
//! Implements market-standard inflation curve calibration using zero-coupon
//! inflation swaps to build forward CPI level curves.

use crate::calibration::config::ValidationMode;
use crate::calibration::quotes::InflationQuote;
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};
use crate::instruments::common::traits::Instrument;
use crate::instruments::inflation_swap::{InflationSwap, PayReceiveInflation};
use finstack_core::config::FinstackConfig;
use finstack_core::dates::DayCount;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::inflation_index::{InflationInterpolation, InflationLag};
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_core::prelude::*;
use finstack_core::types::CurveId;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Inflation curve bootstrapper using ZC inflation swaps.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InflationCurveCalibrator {
    /// Curve identifier
    pub curve_id: CurveId,
    /// Base date for the curve
    pub base_date: finstack_core::dates::Date,
    /// Currency
    pub currency: Currency,
    /// Base CPI level at calibration date
    pub base_cpi: f64,
    /// Discount curve ID for valuation
    pub discount_id: CurveId,
    /// Day count used for mapping calendar dates to time-axis (knots)
    pub time_dc: DayCount,
    /// Day count used for accrual estimations within calibration (e.g., analytical guess)
    pub accrual_dc: DayCount,
    /// Interpolation used during solving and for the final curve
    pub solve_interp: InterpStyle,
    /// Inflation lag (typically 3 months for CPI)
    pub inflation_lag: InflationLag,
    /// Monthly seasonality adjustment factors (12 values, one per month)
    pub seasonality_adjustments: Option<[f64; 12]>,
    /// Interpolation method for inflation index
    pub inflation_interpolation: InflationInterpolation,
    /// Calibration configuration
    pub config: CalibrationConfig,
}

impl InflationCurveCalibrator {
    /// Create a new inflation curve calibrator.
    pub fn new(
        curve_id: impl Into<CurveId>,
        base_date: finstack_core::dates::Date,
        currency: Currency,
        base_cpi: f64,
        discount_id: impl Into<CurveId>,
    ) -> Self {
        Self {
            curve_id: curve_id.into(),
            base_date,
            currency,
            base_cpi,
            discount_id: discount_id.into(),
            time_dc: DayCount::ActAct,
            accrual_dc: DayCount::ActAct,
            solve_interp: InterpStyle::LogLinear,
            inflation_lag: InflationLag::Months(3), // Standard 3-month lag for CPI
            seasonality_adjustments: None,
            inflation_interpolation: InflationInterpolation::Linear,
            config: CalibrationConfig::default(),
        }
    }

    /// Set calibration configuration from a `FinstackConfig`.
    ///
    /// Resolves `CalibrationConfig` from `FinstackConfig.extensions["valuations.calibration.v1"]`.
    pub fn with_finstack_config(mut self, cfg: &FinstackConfig) -> Result<Self> {
        self.config = CalibrationConfig::from_finstack_config_or_default(cfg)?;
        Ok(self)
    }

    /// Set the interpolation used both during solving and for the final curve.
    pub fn with_solve_interp(mut self, interpolation: InterpStyle) -> Self {
        self.solve_interp = interpolation;
        self
    }

    /// Set the time-axis day count used for CPI knot placement.
    pub fn with_time_dc(mut self, dc: DayCount) -> Self {
        self.time_dc = dc;
        self
    }

    /// Set the accrual day count used for analytical guesses and instrument accrual.
    pub fn with_accrual_dc(mut self, dc: DayCount) -> Self {
        self.accrual_dc = dc;
        self
    }

    /// Set the inflation lag (e.g., 3-month lag for CPI).
    pub fn with_inflation_lag(mut self, lag: InflationLag) -> Self {
        self.inflation_lag = lag;
        self
    }

    /// Set monthly seasonality adjustment factors (12 values, one per month).
    /// Factors should be close to 1.0 (e.g., 0.98 to 1.02 for ±2% adjustment).
    pub fn with_seasonality_adjustments(mut self, factors: [f64; 12]) -> Self {
        self.seasonality_adjustments = Some(factors);
        self
    }

    /// Set the interpolation method for the inflation index.
    pub fn with_inflation_interpolation(mut self, interp: InflationInterpolation) -> Self {
        self.inflation_interpolation = interp;
        self
    }

    /// Apply seasonality adjustment to a CPI value based on the month.
    fn apply_seasonality(&self, cpi_value: f64, date: finstack_core::dates::Date) -> f64 {
        if let Some(factors) = &self.seasonality_adjustments {
            let month_idx = (date.month() as usize) - 1;
            cpi_value * factors[month_idx]
        } else {
            cpi_value
        }
    }
}

use finstack_core::market_data::term_structures::InflationCurve;

impl Calibrator<InflationQuote, InflationCurve> for InflationCurveCalibrator {
    fn calibrate(
        &self,
        instruments: &[InflationQuote],
        base_context: &MarketContext,
    ) -> Result<(InflationCurve, CalibrationReport)> {
        let mut warnings: Vec<String> = Vec::new();

        // Extract relevant inflation swap quotes for this index and sort by maturity
        let mut quotes: Vec<(finstack_core::dates::Date, f64, String)> = instruments
            .iter()
            .filter_map(|q| match q {
                InflationQuote::InflationSwap {
                    maturity,
                    rate,
                    index,
                    ..
                } => Some((*maturity, *rate, index.clone())),
                _ => None,
            })
            .filter(|(_, _, index)| index == self.curve_id.as_str())
            .collect();

        if quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        quotes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        if self.config.verbose {
            tracing::debug!(
                curve_id = %self.curve_id.as_str(),
                base_date = %self.base_date,
                quotes = quotes.len(),
                base_cpi = self.base_cpi,
                "Starting inflation curve calibration"
            );
        }

        // Start knots with CPI at base date
        let mut knots: Vec<(f64, f64)> = vec![(0.0, self.base_cpi)];
        let mut residuals = BTreeMap::new();
        // Use configured solver via factory to honor tolerance and iteration settings consistently
        // Use solve_1d helper directly
        {
            // Internal IDs used only for solving. Final curve will use self.curve_id
            const CALIB_INDEX_ID: &str = "CALIB_INFLATION";

            // Ensure discount curve exists in base context (best-effort; pricing will use context provided by caller)
            let _ = base_context.get_discount_ref(&self.discount_id)?;

            // Clone discount id for use in closures (avoids memory leak from Box::leak)
            let discount_curve_id = self.discount_id.clone();

            // Note: We don't require an inflation index during calibration; the index is provided by caller when repricing.

            for (maturity, par_rate, _idx) in quotes {
                // Consistent time-axis for CPI knot (use original maturity for curve construction)
                let t = match self.time_dc.year_fraction(
                    self.base_date,
                    maturity,
                    finstack_core::dates::DayCountCtx::default(),
                ) {
                    Ok(v) => v,
                    Err(e) => {
                        let msg = format!(
                            "Inflation quote maturity={maturity}: invalid time_dc year_fraction: {e:?}"
                        );
                        warnings.push(msg.clone());
                        residuals.insert(format!("ZCIS-{}", maturity), crate::calibration::PENALTY);
                        if self.config.validation_mode == ValidationMode::Error {
                            return Err(finstack_core::Error::Calibration {
                                message: msg,
                                category: "inflation_curve_calibration".to_string(),
                            });
                        }
                        continue;
                    }
                };
                if t <= 0.0 {
                    warnings.push(format!(
                        "Skipping inflation quote with non-positive time to maturity: maturity={maturity} t={t:.6}"
                    ));
                    residuals.insert(format!("ZCIS-{}", maturity), crate::calibration::PENALTY);
                    continue;
                }

                // Initial guess: compound last CPI by par rate over accrual time
                let tau = match self.accrual_dc.year_fraction(
                    self.base_date,
                    maturity,
                    finstack_core::dates::DayCountCtx::default(),
                ) {
                    Ok(v) => v,
                    Err(e) => {
                        let msg = format!(
                            "Inflation quote maturity={maturity}: invalid accrual_dc year_fraction: {e:?}"
                        );
                        warnings.push(msg.clone());
                        residuals.insert(format!("ZCIS-{}", maturity), crate::calibration::PENALTY);
                        if self.config.validation_mode == ValidationMode::Error {
                            return Err(finstack_core::Error::Calibration {
                                message: msg,
                                category: "inflation_curve_calibration".to_string(),
                            });
                        }
                        continue;
                    }
                };
                // Use analytical breakeven CPI for initial guess to ensure f(x0)=0
                let mut initial_guess = self.base_cpi * (1.0 + par_rate).powf(tau);

                // Apply seasonality adjustment to initial guess if applicable
                initial_guess = self.apply_seasonality(initial_guess, maturity);
                if self.config.verbose {
                    tracing::debug!(
                        maturity = %maturity,
                        t = t,
                        rate = par_rate,
                        tau = tau,
                        guess = initial_guess,
                        "Processing inflation swap quote"
                    );
                }

                // Objective priced via instrument pricer
                // Pre-allocate knots buffer to reduce allocations in objective
                let mut knots_with_capacity = Vec::with_capacity(knots.len() + 1);
                knots_with_capacity.extend_from_slice(&knots);

                let base_ctx_ref = base_context;
                let notional = Money::new(1_000_000.0, self.currency);
                let disc_id_clone = discount_curve_id.clone();

                let base_date = self.base_date;
                let objective = move |cpi_guess: f64| -> f64 {
                    if !cpi_guess.is_finite() || cpi_guess <= 0.0 {
                        return crate::calibration::PENALTY;
                    }

                    // Build temporary inflation curve with current knots + guessed point
                    // Reuse pre-allocated buffer
                    let mut temp_knots = knots_with_capacity.clone();
                    temp_knots.push((t, cpi_guess));
                    // Sort by time using total_cmp for safe float comparison
                    temp_knots.sort_by(|a, b| a.0.total_cmp(&b.0));

                    let temp_curve = match InflationCurve::builder(CALIB_INDEX_ID)
                        .base_cpi(self.base_cpi)
                        .knots(temp_knots)
                        .set_interp(self.solve_interp)
                        .build()
                    {
                        Ok(c) => c,
                        Err(_) => return crate::calibration::PENALTY,
                    };

                    // Build synthetic ZC inflation swap matching the quote
                    let swap = match InflationSwap::builder()
                        .id(format!("CALIB_ZCIS_{}", maturity).into())
                        .notional(notional)
                        .start(base_date)
                        .maturity(maturity)
                        .fixed_rate(par_rate)
                        .inflation_index_id(CALIB_INDEX_ID.into())
                        .discount_curve_id(disc_id_clone.clone())
                        .dc(self.accrual_dc)
                        .side(PayReceiveInflation::PayFixed)
                        .build()
                    {
                        Ok(s) => s,
                        Err(_) => return crate::calibration::PENALTY,
                    };

                    // Update market context with temp inflation curve
                    let temp_ctx = base_ctx_ref.clone().insert_inflation(temp_curve);

                    match swap.value(&temp_ctx, base_date) {
                        Ok(pv) => pv.amount() / notional.amount(),
                        Err(_) => crate::calibration::PENALTY,
                    }
                };

                // Use solve_1d helper directly
                use crate::calibration::solve_1d;
                let mut solved_cpi = match solve_1d(
                    self.config.solver_kind.clone(),
                    self.config.tolerance,
                    self.config.max_iterations,
                    &objective,
                    initial_guess,
                ) {
                    Ok(root) => root,
                    Err(e) => {
                        let msg =
                            format!("Inflation solve_1d failed for maturity={maturity}: {e:?}");
                        warnings.push(msg.clone());
                        residuals.insert(format!("ZCIS-{}", maturity), crate::calibration::PENALTY);
                        if self.config.validation_mode == ValidationMode::Error {
                            return Err(finstack_core::Error::Calibration {
                                message: msg,
                                category: "inflation_curve_calibration".to_string(),
                            });
                        }
                        continue;
                    }
                };
                if !solved_cpi.is_finite() || solved_cpi <= 0.0 {
                    let msg =
                        format!("Solved CPI is invalid for maturity={maturity}: {solved_cpi}");
                    warnings.push(msg.clone());
                    residuals.insert(format!("ZCIS-{}", maturity), crate::calibration::PENALTY);
                    if self.config.validation_mode == ValidationMode::Error {
                        return Err(finstack_core::Error::Calibration {
                            message: msg,
                            category: "inflation_curve_calibration".to_string(),
                        });
                    }
                    continue;
                }

                // Stamp seasonality before evaluating residuals and committing knot
                solved_cpi = self.apply_seasonality(solved_cpi, maturity);

                // Record residual and commit the knot
                let res = objective(solved_cpi).abs();
                if !res.is_finite() || res >= crate::calibration::PENALTY * 0.5 {
                    let msg = format!(
                        "Inflation objective returned invalid/penalty residual at maturity={maturity}: {res}"
                    );
                    warnings.push(msg.clone());
                    residuals.insert(format!("ZCIS-{}", maturity), crate::calibration::PENALTY);
                    if self.config.validation_mode == ValidationMode::Error {
                        return Err(finstack_core::Error::Calibration {
                            message: msg,
                            category: "inflation_curve_calibration".to_string(),
                        });
                    }
                    continue;
                }
                if self.config.verbose {
                    tracing::debug!(
                        solved_cpi = solved_cpi,
                        residual = res,
                        "Solved CPI for maturity"
                    );
                }
                residuals.insert(format!("ZCIS-{}", maturity), res);
                knots.push((t, solved_cpi));
            }

            // Build final curve with requested identifier
            let mut final_knots = knots;
            if final_knots.len() < 2 {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::TooFewPoints,
                ));
            }
            final_knots.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

            if self.config.verbose {
                tracing::debug!(
                    final_knots = final_knots.len(),
                    "Building final inflation curve"
                );
            }
            let curve = match InflationCurve::builder(self.curve_id.to_owned())
                .base_cpi(self.base_cpi)
                .knots(final_knots.clone())
                .set_interp(self.solve_interp)
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    return Err(finstack_core::Error::Calibration {
                        message: format!(
                            "Failed to build inflation curve {}: {e}",
                            self.curve_id.as_str()
                        ),
                        category: "inflation_curve_build".to_string(),
                    });
                }
            };

            // Validate the calibrated inflation curve (honor config.validation + validation_mode).
            use crate::calibration::validation::CurveValidator;
            let mut validation_status = "passed";
            let mut validation_error: Option<String> = None;
            if let Err(e) = curve.validate(&self.config.validation) {
                validation_status = "failed";
                validation_error = Some(e.to_string());
                match self.config.validation_mode {
                    ValidationMode::Warn => {
                        tracing::warn!(
                            curve_id = %self.curve_id.as_str(),
                            error = %e,
                            "Calibrated inflation curve failed validation (continuing due to Warn mode)"
                        );
                    }
                    ValidationMode::Error => {
                        return Err(finstack_core::Error::Calibration {
                            message: format!(
                                "Calibrated inflation curve {} failed validation: {}",
                                self.curve_id.as_str(),
                                e
                            ),
                            category: "inflation_curve_validation".to_string(),
                        });
                    }
                }
            }

            let report = CalibrationReport::for_type_with_tolerance(
                "inflation_curve",
                residuals,
                final_knots.len(),
                self.config.tolerance,
            )
            .with_metadata("solve_interp", format!("{:?}", self.solve_interp))
            .with_metadata("time_dc", format!("{:?}", self.time_dc))
            .with_metadata("accrual_dc", format!("{:?}", self.accrual_dc))
            .with_metadata("inflation_lag", format!("{:?}", self.inflation_lag))
            .with_metadata(
                "inflation_interpolation",
                format!("{:?}", self.inflation_interpolation),
            )
            .with_metadata(
                "has_seasonality",
                format!("{}", self.seasonality_adjustments.is_some()),
            )
            .with_metadata("validation", validation_status)
            .with_validation_result(validation_status == "passed", validation_error)
            .with_metadata("warnings_count", warnings.len().to_string())
            .with_metadata(
                "warnings",
                warnings
                    .iter()
                    .take(50)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n"),
            );

            Ok((curve, report))
        }
    }
}
