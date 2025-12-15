//! Cross-currency (XCCY) basis calibration.
//!
//! Market-standard approach implemented:
//! - Build a dedicated multi-currency XCCY swap instrument with explicit curve + FX conventions.
//! - Bootstrap a foreign discount curve such that each quoted XCCY basis swap reprices to PV=0.
//!
//! This calibrator intentionally does **not** model an abstract "basis spread curve" detached
//! from an instrument; instead it solves directly for discount factors (or equivalently zero rates)
//! on the target discount curve.

use crate::calibration::config::ValidationMode;
pub use crate::calibration::quotes::xccy::{SpreadOn, XccyBasisQuote};
use crate::calibration::validation::CurveValidator;
use crate::calibration::{CalibrationConfig, CalibrationReport};
use crate::instruments::xccy_swap::{LegSide, XccySwap, XccySwapLeg};
use finstack_core::dates::calendar::registry::CalendarRegistry;
use finstack_core::dates::{
    adjust, BusinessDayConvention, Date, DayCount, DayCountCtx, HolidayCalendar,
};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_core::math::Solver;
use finstack_core::money::Money;
use finstack_core::prelude::*;
use finstack_core::types::{Currency, CurveId};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

/// XCCY Basis Calibrator.
#[derive(Clone, Debug)]
pub struct XccyBasisCalibrator {
    /// Curve identifier for the resulting basis-adjusted discount curve
    pub curve_id: CurveId,
    /// Base date
    pub base_date: Date,
    /// Currency of the curve being calibrated (usually the foreign currency in the pair)
    pub currency: Currency,
    /// Configuration
    pub config: CalibrationConfig,
    /// Curve day count for time mapping (None = currency standard).
    pub curve_day_count: Option<DayCount>,
    /// Interpolation used during solving and for the final curve.
    pub solve_interp: InterpStyle,
    /// Extrapolation policy for the final curve.
    pub extrapolation: ExtrapolationPolicy,
}

impl XccyBasisCalibrator {
    /// Create a new XCCY Basis Calibrator.
    pub fn new(curve_id: impl Into<CurveId>, base_date: Date, currency: Currency) -> Self {
        Self {
            curve_id: curve_id.into(),
            base_date,
            currency,
            config: CalibrationConfig::default(),
            curve_day_count: None,
            solve_interp: InterpStyle::MonotoneConvex,
            extrapolation: ExtrapolationPolicy::FlatForward,
        }
    }

    /// Set the curve day count used for time mapping.
    pub fn with_curve_day_count(mut self, dc: DayCount) -> Self {
        self.curve_day_count = Some(dc);
        self
    }

    /// Set the interpolation used during solving and for the final curve.
    pub fn with_solve_interp(mut self, interpolation: InterpStyle) -> Self {
        self.solve_interp = interpolation;
        self
    }

    /// Set the extrapolation policy for the final curve.
    pub fn with_extrapolation(mut self, extrapolation: ExtrapolationPolicy) -> Self {
        self.extrapolation = extrapolation;
        self
    }

    fn effective_curve_day_count(&self) -> DayCount {
        // Default to Act/365F for curve time. Override via with_curve_day_count().
        self.curve_day_count.unwrap_or(DayCount::Act365F)
    }

    fn resolve_calendar_strict(id: &str) -> Result<&'static dyn HolidayCalendar> {
        CalendarRegistry::global().resolve_str(id).ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: format!("calendar '{}'", id),
            })
        })
    }

    fn adjust_joint_calendar_strict(
        mut date: Date,
        bdc: BusinessDayConvention,
        dom: &'static dyn HolidayCalendar,
        for_cal: &'static dyn HolidayCalendar,
    ) -> Result<Date> {
        for _ in 0..5 {
            let adj_dom = adjust(date, bdc, dom)?;
            let adj_for = adjust(adj_dom, bdc, for_cal)?;
            if adj_for == date {
                return Ok(adj_for);
            }
            date = adj_for;
        }
        Ok(date)
    }

    fn add_joint_business_days(
        start: Date,
        days: u32,
        dom: &'static dyn HolidayCalendar,
        for_cal: &'static dyn HolidayCalendar,
    ) -> Result<Date> {
        if days == 0 {
            return Ok(start);
        }
        let mut d = start;
        let mut remaining = days;
        while remaining > 0 {
            d += time::Duration::days(1);
            if dom.is_business_day(d) && for_cal.is_business_day(d) {
                remaining -= 1;
            }
        }
        Ok(d)
    }

    fn spot_date(&self, q: &XccyBasisQuote) -> Result<Date> {
        let dom = Self::resolve_calendar_strict(q.domestic_calendar_id().unwrap_or(""))?;
        let for_cal = Self::resolve_calendar_strict(q.foreign_calendar_id().unwrap_or(""))?;
        let prelim =
            Self::add_joint_business_days(self.base_date, q.spot_lag_days(), dom, for_cal)?;
        Self::adjust_joint_calendar_strict(prelim, q.spot_bdc(), dom, for_cal)
    }

    fn quote_to_instrument(&self, q: &XccyBasisQuote) -> Result<XccySwap> {
        if q.foreign_currency != self.currency {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            ));
        }
        let start_date = self.spot_date(q)?;

        let spread = q.spread_bp * 1e-4;
        if !spread.is_finite() {
            return Err(finstack_core::Error::Validation(
                "XCCY quote spread must be finite".to_string(),
            ));
        }

        let (dom_spread, for_spread) = match q.spread_on {
            SpreadOn::Domestic => (spread, 0.0),
            SpreadOn::Foreign => (0.0, spread),
        };

        let domestic_leg = XccySwapLeg {
            currency: q.domestic_currency,
            notional: Money::new(q.domestic_notional, q.domestic_currency),
            side: LegSide::Receive,
            forward_curve_id: q.domestic_forward_curve_id.clone(),
            discount_curve_id: q.domestic_discount_curve_id.clone(),
            frequency: q.domestic_frequency(),
            day_count: q.domestic_day_count(),
            bdc: q.domestic_bdc(),
            spread: dom_spread,
            payment_lag_days: q.domestic_payment_lag(),
            calendar_id: q.domestic_calendar_id().map(|s| s.to_string()),
            allow_calendar_fallback: false,
        };

        let foreign_leg = XccySwapLeg {
            currency: q.foreign_currency,
            notional: Money::new(q.foreign_notional, q.foreign_currency),
            side: LegSide::Pay,
            forward_curve_id: q.foreign_forward_curve_id.clone(),
            discount_curve_id: self.curve_id.clone(),
            frequency: q.foreign_frequency(),
            day_count: q.foreign_day_count(),
            bdc: q.foreign_bdc(),
            spread: for_spread,
            payment_lag_days: q.foreign_payment_lag(),
            calendar_id: q.foreign_calendar_id().map(|s| s.to_string()),
            allow_calendar_fallback: false,
        };

        Ok(XccySwap::new(
            format!("XCCY-{}-{}", q.domestic_currency, q.foreign_currency),
            start_date,
            q.maturity,
            domestic_leg,
            foreign_leg,
            q.domestic_currency,
        )
        .with_notional_exchange(q.notional_exchange))
    }

    /// Bootstrap basis curve from quotes.
    ///
    /// Calibrates a foreign discount curve directly to reprice XCCY basis swaps to PV=0.
    ///
    /// # Arguments
    /// * `quotes`: List of XCCY basis swap quotes
    /// * `solver`: Numerical solver
    /// * `base_context`: Market context containing:
    ///     - Domestic Discount Curve
    ///     - Domestic Forward Curve (if needed)
    ///     - Foreign Forward Curve (for the foreign leg projection)
    ///     - Spot FX Rate
    pub fn bootstrap<S: Solver>(
        &self,
        quotes: &[XccyBasisQuote],
        solver: &S,
        base_context: &MarketContext,
    ) -> Result<(DiscountCurve, CalibrationReport)> {
        if quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            ));
        }

        // Sort quotes by maturity
        let mut sorted = quotes.to_vec();
        sorted.sort_by_key(|q| q.maturity);

        // Fail-fast validation: required market data + calendars must exist.
        if base_context.fx.is_none() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::NotFound {
                    id: "fx_matrix".to_string(),
                },
            ));
        }
        for q in &sorted {
            if q.foreign_currency != self.currency {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }
            let _ = base_context.get_discount_ref(&q.domestic_discount_curve_id)?;
            let _ = base_context.get_forward_ref(&q.domestic_forward_curve_id)?;
            let _ = base_context.get_forward_ref(&q.foreign_forward_curve_id)?;
            if let Some(cal_id) = q.domestic_calendar_id() {
                let _ = Self::resolve_calendar_strict(cal_id)?;
            }
            if let Some(cal_id) = q.foreign_calendar_id() {
                let _ = Self::resolve_calendar_strict(cal_id)?;
            }
        }

        // Pre-build instruments for pricing
        let instruments: Vec<XccySwap> = sorted
            .iter()
            .map(|q| self.quote_to_instrument(q))
            .collect::<Result<Vec<_>>>()?;

        let curve_dc = self.effective_curve_day_count();
        let bounds = self.config.effective_rate_bounds(self.currency);
        let min_rate = bounds.min_rate;
        let max_rate = bounds.max_rate;

        // Base knot
        let mut knots: Vec<(f64, f64)> = vec![(0.0, 1.0)];
        let mut residuals = BTreeMap::new();
        let mut total_iterations = 0usize;

        // Use RefCell to avoid cloning context on each objective evaluation.
        let ctx_rc = Rc::new(RefCell::new(base_context.clone()));

        let mut last_rate_guess = 0.0_f64;
        let dc_ctx = DayCountCtx::default();

        for (idx, (q, inst)) in sorted.iter().zip(instruments.iter()).enumerate() {
            // Curve time mapping uses curve day count (consistent knot basis).
            let t = curve_dc
                .year_fraction(self.base_date, q.maturity, dc_ctx)
                .map_err(|e| finstack_core::Error::Calibration {
                    message: format!(
                        "XCCY bootstrap failed to compute time for maturity {}: {}",
                        q.maturity, e
                    ),
                    category: "xccy_bootstrap".to_string(),
                })?
                .max(1e-12);

            // Prepare a mutable knots buffer (existing + candidate point).
            let existing_len = knots.len();
            let temp_knots = Rc::new(RefCell::new({
                let mut v = Vec::with_capacity(knots.len() + 1);
                v.extend_from_slice(&knots);
                v
            }));

            let curve_id = self.curve_id.clone();
            let base_date = self.base_date;
            let solve_interp = self.solve_interp;
            let extrapolation = self.extrapolation;
            let inst_clone = inst.clone();
            let ctx_rc_clone = ctx_rc.clone();

            let objective = move |rate: f64| -> f64 {
                if !rate.is_finite() || rate < min_rate || rate > max_rate {
                    return crate::calibration::PENALTY;
                }

                let df = (-rate * t).exp();
                if !df.is_finite() || df <= 0.0 {
                    return crate::calibration::PENALTY;
                }

                {
                    let mut k = temp_knots.borrow_mut();
                    k.truncate(existing_len);
                    k.push((t, df));
                    let temp_curve = match DiscountCurve::builder(curve_id.clone())
                        .base_date(base_date)
                        .day_count(curve_dc)
                        .extrapolation(extrapolation)
                        .allow_non_monotonic()
                        .knots(k.iter().copied())
                        .set_interp(solve_interp)
                        .build_for_solver()
                    {
                        Ok(c) => c,
                        Err(_) => return crate::calibration::PENALTY,
                    };

                    ctx_rc_clone.borrow_mut().insert_mut(Arc::new(temp_curve));
                }

                let ctx = ctx_rc_clone.borrow();
                match inst_clone.npv(&ctx, base_date) {
                    Ok(v) => v.amount(),
                    Err(_) => crate::calibration::PENALTY,
                }
            };

            // Scan grid for bracketing: include endpoints and a dense grid around the last guess.
            let mut scan = Vec::with_capacity(48);
            scan.push(min_rate);
            scan.push(max_rate);
            let center = last_rate_guess.clamp(min_rate, max_rate);
            for i in -10..=10 {
                let r = center + (i as f64) * 0.0025;
                if r >= min_rate && r <= max_rate {
                    scan.push(r);
                }
            }
            // Coarse linear grid across bounds for robustness.
            let coarse_n = 24usize;
            for i in 0..=coarse_n {
                let w = i as f64 / coarse_n as f64;
                scan.push(min_rate + w * (max_rate - min_rate));
            }
            scan.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            scan.dedup_by(|a, b| (*a - *b).abs() < 1e-12);

            let (tentative, diag) = crate::calibration::bracket_solve_1d_with_diagnostics(
                &objective,
                center,
                &scan,
                self.config.tolerance,
                self.config.max_iterations,
            )?;
            total_iterations += diag.eval_count;

            let solved_rate = if let Some(r) = tentative {
                r
            } else {
                if diag.valid_eval_count == 0 {
                    return Err(finstack_core::Error::Calibration {
                        message: format!(
                            "XCCY bootstrap failed at {}: all {} objective evaluations were invalid/penalized. \
                             This usually indicates missing market data (FX/curves) or inconsistent conventions.",
                            q.maturity, diag.eval_count
                        ),
                        category: "xccy_bootstrap".to_string(),
                    });
                }
                let start_point = diag.best_point.unwrap_or(center);
                solver.solve(&objective, start_point).map_err(|e| {
                    finstack_core::Error::Calibration {
                        message: format!(
                            "XCCY bootstrap solver failed at {}: {} (no bracket). Best candidate: rate={:.6} residual={:.2e}",
                            q.maturity,
                            e,
                            diag.best_point.unwrap_or(f64::NAN),
                            diag.best_value.unwrap_or(f64::NAN)
                        ),
                        category: "xccy_bootstrap".to_string(),
                    }
                })?
            };

            if !solved_rate.is_finite() {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "XCCY bootstrap produced non-finite rate at {}: {:?}",
                        q.maturity, solved_rate
                    ),
                    category: "xccy_bootstrap".to_string(),
                });
            }

            last_rate_guess = solved_rate;
            let solved_df = (-solved_rate * t).exp();
            knots.push((t, solved_df));

            // Compute final residual at solved point.
            let residual = objective(solved_rate);
            let key = format!(
                "xccy:{}{}:{}bp@{}",
                q.domestic_currency, q.foreign_currency, q.spread_bp, q.maturity
            );
            residuals.insert(key, residual);

            if self.config.verbose {
                tracing::debug!(
                    step = idx + 1,
                    total = sorted.len(),
                    maturity = %q.maturity,
                    t = t,
                    rate = solved_rate,
                    df = solved_df,
                    residual = residual,
                    "Bootstrapped XCCY point"
                );
            }
        }

        // Build final curve.
        let curve = DiscountCurve::builder(self.curve_id.to_owned())
            .base_date(self.base_date)
            .day_count(curve_dc)
            .extrapolation(self.extrapolation)
            .allow_non_monotonic()
            .knots(knots)
            .set_interp(self.solve_interp)
            .build()
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!(
                    "final XCCY DiscountCurve build failed for {}: {}",
                    self.curve_id, e
                ),
                category: "xccy_bootstrap".to_string(),
            })?;

        // Validate the calibrated curve (honor config.validation + validation_mode).
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
                        "Calibrated XCCY discount curve failed validation (continuing due to Warn mode)"
                    );
                }
                ValidationMode::Error => {
                    return Err(finstack_core::Error::Calibration {
                        message: format!(
                            "Calibrated XCCY discount curve {} failed validation: {}",
                            self.curve_id, e
                        ),
                        category: "xccy_validation".to_string(),
                    });
                }
            }
        }

        let mut report = CalibrationReport::for_type_with_tolerance(
            "xccy_discount_curve",
            residuals,
            total_iterations,
            self.config.tolerance,
        )
        .with_metadata("currency", self.currency.to_string())
        .with_metadata("curve_day_count", format!("{:?}", curve_dc))
        .with_metadata("solve_interp", format!("{:?}", self.solve_interp))
        .with_metadata("extrapolation", format!("{:?}", self.extrapolation))
        .with_metadata("validation", validation_status)
        .with_validation_result(validation_status == "passed", validation_error.clone());

        if let Some(err) = validation_error {
            report = report.with_metadata("validation_error", err);
        }

        Ok((curve, report))
    }
}
