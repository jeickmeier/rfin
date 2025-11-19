//! Swaption volatility surface calibration.
//!
//! Implements market-standard swaption volatility calibration supporting:
//! - Normal and lognormal volatility conventions
//! - Various ATM strike conventions
//! - SABR model calibration per expiry
//! - Accurate swap annuity calculations

use crate::calibration::methods::swaption_market_conventions::SwaptionMarketConvention;
use crate::calibration::quote::VolQuote;
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};
use crate::instruments::common::models::{SABRCalibrator, SABRModel, SABRParameters};
use finstack_core::dates::utils::add_months;
use finstack_core::dates::{BusinessDayConvention, Date, DayCountCtx, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::prelude::Currency;
use finstack_core::types::CurveId;
use finstack_core::Result;
use std::collections::BTreeMap;

/// Type alias for grouped quotes by expiry-tenor pairs
type QuotesByExpiryTenor = BTreeMap<(u64, u64), Vec<(f64, f64)>>;

/// Type alias for SABR parameters by expiry-tenor pairs  
type SABRParamsByExpiryTenor = BTreeMap<(u64, u64), SABRParameters>;

/// Convert a float to basis points for use as a map key
fn to_basis_points(value: f64) -> u64 {
    (value * 10000.0).round() as u64
}

/// Volatility quoting convention for swaptions.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SwaptionVolConvention {
    /// Normal (absolute) volatility in basis points
    Normal,
    /// Lognormal (Black) volatility as percentage
    Lognormal,
    /// Shifted lognormal for negative rates
    ShiftedLognormal {
        /// Shift amount for negative rate handling
        shift: f64,
    },
}

/// ATM strike convention for swaptions.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AtmStrikeConvention {
    /// ATM = forward swap rate (standard market convention)
    SwapRate,
    /// ATM = par swap rate (same as forward for zero-cost swap)
    ParRate,
}

/// Swaption volatility surface calibrator.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SwaptionVolCalibrator {
    /// Surface identifier
    pub surface_id: String,
    /// Volatility convention
    pub vol_convention: SwaptionVolConvention,
    /// ATM strike convention
    pub atm_convention: AtmStrikeConvention,
    /// Fixed SABR beta (0 for normal, 1 for lognormal)
    pub sabr_beta: f64,
    /// Base date for calculations
    pub base_date: Date,
    /// Discount curve ID for swap pricing
    pub discount_curve_id: CurveId,
    /// Forward curve ID (if different from discount)
    pub forward_id: Option<String>,
    /// Currency for market conventions
    pub currency: Currency,
    /// Market conventions configuration
    pub market_conventions: SwaptionMarketConvention,
    /// Calibration configuration
    pub config: CalibrationConfig,
    /// Optional calendar identifier for schedule generation
    pub calendar_id: Option<String>,
}

impl SwaptionVolCalibrator {
    /// Get market-standard SABR beta by currency for lognormal convention.
    ///
    /// Market practice for interest rate swaptions typically uses:
    /// - USD/EUR: β ≈ 0.5 (captures empirical smile dynamics)
    /// - Other G10: β ≈ 0.5
    /// - Can be overridden via builder method
    fn default_sabr_beta_lognormal(currency: Currency) -> f64 {
        match currency {
            Currency::USD | Currency::EUR | Currency::GBP | Currency::CHF | Currency::JPY => 0.5,
            _ => 0.5, // Conservative default for other currencies
        }
    }

    /// Create a new swaption volatility calibrator.
    pub fn new(
        surface_id: impl Into<String>,
        vol_convention: SwaptionVolConvention,
        atm_convention: AtmStrikeConvention,
        base_date: Date,
        discount_curve_id: impl Into<CurveId>,
        currency: Currency,
    ) -> Self {
        // Set SABR beta based on volatility convention and currency
        let sabr_beta = match vol_convention {
            SwaptionVolConvention::Normal => 0.0,
            SwaptionVolConvention::Lognormal | SwaptionVolConvention::ShiftedLognormal { .. } => {
                Self::default_sabr_beta_lognormal(currency)
            }
        };

        Self {
            surface_id: surface_id.into(),
            vol_convention,
            atm_convention,
            sabr_beta,
            base_date,
            discount_curve_id: discount_curve_id.into(),
            forward_id: None,
            currency,
            market_conventions: SwaptionMarketConvention::from_currency(currency),
            config: CalibrationConfig::default(),
            calendar_id: None,
        }
    }

    /// Set the forward curve ID (if different from discount).
    pub fn with_forward_id(mut self, forward_id: impl Into<String>) -> Self {
        self.forward_id = Some(forward_id.into());
        self
    }

    /// Set custom market conventions.
    pub fn with_market_conventions(mut self, conventions: SwaptionMarketConvention) -> Self {
        self.market_conventions = conventions;
        self
    }

    /// Set the calibration configuration.
    pub fn with_config(mut self, config: CalibrationConfig) -> Self {
        self.config = config;
        self
    }

    /// Override the default SABR beta parameter.
    /// By default, beta is currency-aware: 0.5 for USD/EUR rates, 0.0 for normal vols.
    pub fn with_sabr_beta(mut self, beta: f64) -> Self {
        self.sabr_beta = beta;
        self
    }

    /// Set an optional calendar identifier for schedule generation.
    pub fn with_calendar_id(mut self, calendar_id: impl Into<String>) -> Self {
        self.calendar_id = Some(calendar_id.into());
        self
    }

    /// Calculate forward swap rate for a given expiry and tenor.
    ///
    /// In multi-curve mode (when `forward_id` is set), this properly computes
    /// the floating leg PV using the forward curve and returns `float_pv / pv01`.
    /// In single-curve mode, it uses the traditional formula `(DF_start - DF_end) / PV01`.
    fn calculate_forward_swap_rate(
        &self,
        expiry: Date,
        tenor_years: f64,
        context: &MarketContext,
    ) -> Result<f64> {
        let disc = context.get_discount_ref(self.discount_curve_id.as_str())?;
        let swap_start = expiry;
        let swap_end = add_months(expiry, (tenor_years * 12.0) as i32);

        let t_start = self.market_conventions.day_count.year_fraction(
            self.base_date,
            swap_start,
            DayCountCtx::default(),
        )?;
        let t_end = self.market_conventions.day_count.year_fraction(
            self.base_date,
            swap_end,
            DayCountCtx::default(),
        )?;

        if t_start < 0.0 || t_end <= t_start {
            // TODO: Add field context - include expiry/tenor info to help debug invalid date ranges
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::InvalidDateRange,
            ));
        }

        // Calculate annuity using proper schedule
        let pv01 = self.calculate_pv01_proper(swap_start, swap_end, disc)?;

        if pv01 <= self.market_conventions.zero_threshold {
            // TODO: Add field context - specify which swap (expiry/tenor) has invalid PV01
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            ));
        }

        // Multi-curve mode: use forward curve for floating leg if configured
        if let Some(ref forward_id) = self.forward_id {
            let fwd = context.get_forward_ref(forward_id)?;

            // Build floating leg schedule
            let float_sched = crate::cashflow::builder::build_dates(
                swap_start,
                swap_end,
                self.market_conventions.float_freq,
                StubKind::None,
                BusinessDayConvention::Following,
                self.calendar_id.as_deref(),
            );

            if float_sched.dates.len() < 2 {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }

            // Calculate floating leg PV using forward curve
            let mut float_pv = 0.0;
            let mut prev = float_sched.dates[0];

            for &pay_date in &float_sched.dates[1..] {
                // Accrual fraction for this period
                let accrual = self.market_conventions.day_count.year_fraction(
                    prev,
                    pay_date,
                    DayCountCtx::default(),
                )?;

                // Time to payment
                let t_pay = self.market_conventions.day_count.year_fraction(
                    self.base_date,
                    pay_date,
                    DayCountCtx::default(),
                )?;

                // Time to period start
                let t_prev = self.market_conventions.day_count.year_fraction(
                    self.base_date,
                    prev,
                    DayCountCtx::default(),
                )?;

                // Forward rate for this period (using the forward curve)
                let forward_rate = fwd.rate_period(t_prev, t_pay);

                // Payment = forward_rate * accrual * discount_factor
                float_pv += forward_rate * accrual * disc.df(t_pay);

                prev = pay_date;
            }

            // Par rate = floating leg PV / annuity (PV01)
            Ok(float_pv / pv01)
        } else {
            // Single-curve mode: traditional formula
            let df_start = disc.df(t_start);
            let df_end = disc.df(t_end);
            Ok((df_start - df_end) / pv01)
        }
    }

    /// Calculate PV01 using the centralized cashflow::builder date generation.
    fn calculate_pv01_proper(
        &self,
        start: Date,
        end: Date,
        disc: &dyn finstack_core::market_data::traits::Discounting,
    ) -> Result<f64> {
        // Use shared builder to avoid duplication and ensure consistency
        let sched = crate::cashflow::builder::build_dates(
            start,
            end,
            self.market_conventions.fixed_freq,
            StubKind::None,
            BusinessDayConvention::Following,
            self.calendar_id.as_deref(),
        );
        let dates = sched.dates;
        if dates.len() < 2 {
            return Ok(0.0);
        }

        let mut pv01 = 0.0;
        let mut prev = dates[0];
        for &d in &dates[1..] {
            let dcf =
                self.market_conventions
                    .day_count
                    .year_fraction(prev, d, DayCountCtx::default())?;
            let t = self.market_conventions.day_count.year_fraction(
                self.base_date,
                d,
                DayCountCtx::default(),
            )?;
            pv01 += disc.df(t) * dcf;
            prev = d;
        }

        Ok(pv01)
    }

    /// Convert volatility between conventions.
    fn convert_volatility(
        &self,
        vol: f64,
        from_convention: SwaptionVolConvention,
        to_convention: SwaptionVolConvention,
        forward_rate: f64,
        time_to_expiry: f64,
    ) -> f64 {
        // For robust cross-convention conversion, equate option prices
        // under a unit annuity using Bachelier (normal) and Black (lognormal)
        // models. This avoids off-ATM distortions of simple ratios.
        use crate::calibration::{solve_1d, SolverKind};
        use finstack_core::math::{norm_cdf, norm_pdf};

        // Helper: Bachelier (normal) call price with unit annuity
        fn bachelier_price(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
            if t <= 0.0 {
                return (forward - strike).max(0.0);
            }
            if sigma_n <= 0.0 {
                return (forward - strike).max(0.0);
            }
            let st = sigma_n * t.sqrt();
            if st <= 0.0 {
                return (forward - strike).max(0.0);
            }
            let d = (forward - strike) / st;
            (forward - strike) * norm_cdf(d) + st * norm_pdf(d)
        }

        // Helper: Black (lognormal) call price with unit annuity
        fn black_price(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
            if t <= 0.0 {
                return (forward - strike).max(0.0);
            }
            if sigma <= 0.0 || forward <= 0.0 || strike <= 0.0 {
                return (forward - strike).max(0.0);
            }
            let st = sigma * t.sqrt();
            let d1 = ((forward / strike).ln() + 0.5 * st * st) / st;
            let d2 = d1 - st;
            forward * norm_cdf(d1) - strike * norm_cdf(d2)
        }

        // Helper: Black with shift (for shifted lognormal)
        fn black_shifted_price(forward: f64, strike: f64, sigma: f64, t: f64, shift: f64) -> f64 {
            black_price(forward + shift, strike + shift, sigma, t)
        }

        // Early returns for identical convention (including same shift)
        if std::mem::discriminant(&from_convention) == std::mem::discriminant(&to_convention) {
            // If both are shifted, check shift equality
            if let (
                SwaptionVolConvention::ShiftedLognormal { shift: s1 },
                SwaptionVolConvention::ShiftedLognormal { shift: s2 },
            ) = (from_convention, to_convention)
            {
                if (s1 - s2).abs() < 1e-12 {
                    return vol;
                }
            } else {
                return vol;
            }
        }

        // Preserve fast ATM analytic mapping for Normal <-> Lognormal
        if let (SwaptionVolConvention::Normal, SwaptionVolConvention::Lognormal) =
            (from_convention, to_convention)
        {
            if forward_rate.abs() > self.market_conventions.zero_threshold {
                return vol / forward_rate;
            } else {
                return vol;
            }
        }
        if let (SwaptionVolConvention::Lognormal, SwaptionVolConvention::Normal) =
            (from_convention, to_convention)
        {
            return vol * forward_rate;
        }

        // Compute price under source convention with unit annuity
        let price_from = match from_convention {
            SwaptionVolConvention::Normal => {
                bachelier_price(forward_rate, forward_rate, vol, time_to_expiry)
            }
            SwaptionVolConvention::Lognormal => {
                black_price(forward_rate, forward_rate, vol, time_to_expiry)
            }
            SwaptionVolConvention::ShiftedLognormal { shift } => {
                black_shifted_price(forward_rate, forward_rate, vol, time_to_expiry, shift)
            }
        };

        // General-strike conversion: use the actual forward_rate as strike for ATM
        // For non-ATM strikes elsewhere in code, the caller provides strike-specific vols.
        // We preserve that behavior by using K = F here.
        let f = forward_rate;
        let t = time_to_expiry.max(0.0);

        // Invert price to target convention by solving for sigma
        let objective = |sigma: f64| -> f64 {
            let sigma_pos = sigma.abs();
            let p = match to_convention {
                SwaptionVolConvention::Normal => bachelier_price(f, f, sigma_pos, t),
                SwaptionVolConvention::Lognormal => black_price(f, f, sigma_pos, t),
                SwaptionVolConvention::ShiftedLognormal { shift } => {
                    black_shifted_price(f, f, sigma_pos, t, shift)
                }
            };
            p - price_from
        };

        // Initial guesses derived from simple ATM approximations
        let mut guess = match (from_convention, to_convention) {
            (SwaptionVolConvention::Normal, SwaptionVolConvention::Lognormal) => {
                if f.abs() > self.market_conventions.zero_threshold {
                    vol / f
                } else {
                    vol
                }
            }
            (SwaptionVolConvention::Lognormal, SwaptionVolConvention::Normal) => vol * f,
            (SwaptionVolConvention::Normal, SwaptionVolConvention::ShiftedLognormal { shift }) => {
                if (f + shift).abs() > self.market_conventions.zero_threshold {
                    vol / (f + shift)
                } else {
                    vol
                }
            }
            (SwaptionVolConvention::ShiftedLognormal { shift }, SwaptionVolConvention::Normal) => {
                vol * (f + shift)
            }
            (
                SwaptionVolConvention::Lognormal,
                SwaptionVolConvention::ShiftedLognormal { shift },
            ) => {
                if (f + shift).abs() > self.market_conventions.zero_threshold {
                    vol * f / (f + shift)
                } else {
                    vol
                }
            }
            (
                SwaptionVolConvention::ShiftedLognormal { shift },
                SwaptionVolConvention::Lognormal,
            ) => {
                if f.abs() > self.market_conventions.zero_threshold {
                    vol * (f + shift) / f
                } else {
                    vol
                }
            }
            _ => vol,
        };
        if !guess.is_finite() || guess <= 0.0 {
            guess = (vol.abs() + 1e-6).max(1e-6);
        }

        // Solve with a robust 1D solver
        let solved = solve_1d(SolverKind::Brent, 1e-8, 100, objective, guess).unwrap_or(guess);
        let out = solved.abs();
        if out.is_finite() && out > 0.0 {
            out
        } else {
            vol
        }
    }

    /// Determine ATM strike based on convention.
    fn determine_atm_strike(
        &self,
        forward_rate: f64,
        _expiry: Date,
        _tenor_years: f64,
        _context: &MarketContext,
    ) -> Result<f64> {
        match self.atm_convention {
            AtmStrikeConvention::SwapRate | AtmStrikeConvention::ParRate => {
                // For vanilla swaps, par rate = forward rate
                // This is the standard market convention
                Ok(forward_rate)
            }
        }
    }

    /// Build volatility grid from calibrated SABR parameters.
    fn build_vol_grid(
        &self,
        sabr_params: &SABRParamsByExpiryTenor,
        context: &MarketContext,
    ) -> Result<Vec<f64>> {
        let target_expiries = &self.market_conventions.standard_expiries;
        let target_tenors = &self.market_conventions.standard_tenors;
        let mut vol_grid = Vec::with_capacity(target_expiries.len() * target_tenors.len());

        for &expiry_years in target_expiries {
            for &tenor_years in target_tenors {
                let key = (to_basis_points(expiry_years), to_basis_points(tenor_years));

                if let Some(params) = sabr_params.get(&key) {
                    // Have exact calibrated parameters
                    let model = SABRModel::new(params.clone());
                    let expiry = add_months(self.base_date, (expiry_years * 12.0) as i32);

                    match self.calculate_forward_swap_rate(expiry, tenor_years, context) {
                        Ok(forward) => {
                            match self.determine_atm_strike(forward, expiry, tenor_years, context) {
                                Ok(strike) => {
                                    match model.implied_volatility(forward, strike, expiry_years) {
                                        Ok(vol) => {
                                            vol_grid.push(vol);
                                        }
                                        Err(_) => {
                                            vol_grid.push(self.market_conventions.default_vol);
                                        }
                                    }
                                }
                                Err(_) => {
                                    vol_grid.push(self.market_conventions.default_vol);
                                }
                            }
                        }
                        Err(_) => {
                            vol_grid.push(self.market_conventions.default_vol);
                        }
                    }
                } else {
                    // Interpolate from nearby points
                    match self.interpolate_sabr_vol(expiry_years, tenor_years, sabr_params, context)
                    {
                        Ok(vol) => {
                            vol_grid.push(vol);
                        }
                        Err(_) => {
                            vol_grid.push(self.market_conventions.default_vol);
                        }
                    }
                }
            }
        }

        Ok(vol_grid)
    }

    /// Interpolate SABR volatility for points without direct calibration.
    fn interpolate_sabr_vol(
        &self,
        target_expiry: f64,
        target_tenor: f64,
        sabr_params: &SABRParamsByExpiryTenor,
        context: &MarketContext,
    ) -> Result<f64> {
        // Find closest calibrated point using min_by instead of sorting entire list
        let closest = sabr_params
            .iter()
            .map(|((exp_bp, ten_bp), params)| {
                let exp = *exp_bp as f64 / 10000.0;
                let ten = *ten_bp as f64 / 10000.0;
                let distance =
                    ((exp - target_expiry).powi(2) + (ten - target_tenor).powi(2)).sqrt();
                (distance, params)
            })
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        match closest {
            Some((_, params)) => {
                let model = SABRModel::new(params.clone());
                let expiry = add_months(self.base_date, (target_expiry * 12.0) as i32);
                let forward = self.calculate_forward_swap_rate(expiry, target_tenor, context)?;
                let strike = self.determine_atm_strike(forward, expiry, target_tenor, context)?;

                model.implied_volatility(forward, strike, target_expiry)
            }
            None => Ok(self.market_conventions.default_vol), // Use configured default
        }
    }
}

impl Calibrator<VolQuote, VolSurface> for SwaptionVolCalibrator {
    fn calibrate(
        &self,
        instruments: &[VolQuote],
        base_context: &MarketContext,
    ) -> Result<(VolSurface, CalibrationReport)> {
        // 1. Filter for SwaptionVol quotes only
        let swaption_quotes: Vec<_> = instruments
            .iter()
            .filter(|q| matches!(q, VolQuote::SwaptionVol { .. }))
            .collect();

        if swaption_quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        // 2. Group by expiry-tenor pairs
        let mut grouped_quotes: QuotesByExpiryTenor = BTreeMap::new();

        for quote in &swaption_quotes {
            if let VolQuote::SwaptionVol {
                expiry,
                tenor,
                strike,
                vol,
                ..
            } = quote
            {
                let expiry_years = self
                    .market_conventions
                    .day_count
                    .year_fraction(self.base_date, *expiry, DayCountCtx::default())
                    .unwrap_or(0.0);
                let tenor_years = self
                    .market_conventions
                    .day_count
                    .year_fraction(*expiry, *tenor, DayCountCtx::default())
                    .unwrap_or(0.0);

                if expiry_years > 0.0 && tenor_years > 0.0 {
                    grouped_quotes
                        .entry((to_basis_points(expiry_years), to_basis_points(tenor_years)))
                        .or_default()
                        .push((*strike, *vol));
                }
            }
        }

        // 3. Calibrate SABR parameters for each expiry-tenor combination
        let mut sabr_params: SABRParamsByExpiryTenor = BTreeMap::new();
        let mut all_residuals = BTreeMap::new();
        let mut residual_counter = 0;

        let sabr_calibrator = SABRCalibrator::new()
            .with_tolerance(self.config.tolerance)
            .with_max_iterations(self.config.max_iterations)
            .with_fd_gradients(self.config.use_fd_sabr_gradients);

        for ((expiry_bp, tenor_bp), strikes_vols) in &grouped_quotes {
            if strikes_vols.len() < self.market_conventions.min_sabr_points {
                continue; // Need minimum points for SABR
            }

            let expiry_years = *expiry_bp as f64 / 10000.0;
            let tenor_years = *tenor_bp as f64 / 10000.0;
            let expiry_date = add_months(self.base_date, (expiry_years * 12.0) as i32);

            // Calculate forward swap rate
            let forward =
                match self.calculate_forward_swap_rate(expiry_date, tenor_years, base_context) {
                    Ok(f) => {
                        if f <= 0.0 || !f.is_finite() {
                            continue;
                        }
                        f
                    }
                    Err(_) => continue,
                };

            // Extract strikes and vols
            let mut strikes: Vec<f64> = Vec::new();
            let mut vols: Vec<f64> = Vec::new();

            for &(strike, vol) in strikes_vols {
                strikes.push(strike);
                // Convert quoted vol to calibration convention if needed
                let converted_vol = self.convert_volatility(
                    vol,
                    self.vol_convention, // Assume quotes are in our convention
                    self.vol_convention,
                    forward,
                    expiry_years,
                );
                vols.push(converted_vol);
            }

            // Handle negative rates with shift if needed
            let params = match self.vol_convention {
                SwaptionVolConvention::ShiftedLognormal { shift } => {
                    // For shifted SABR, we need to shift the forward and strikes
                    let shifted_forward = forward + shift;
                    let shifted_strikes: Vec<f64> = strikes.iter().map(|&s| s + shift).collect();

                    // Calibrate with shifted values
                    let mut calibrated_params = sabr_calibrator.calibrate(
                        shifted_forward,
                        &shifted_strikes,
                        &vols,
                        expiry_years,
                        self.sabr_beta,
                    )?;

                    // Store the shift in the parameters
                    calibrated_params.shift = Some(shift);
                    Ok(calibrated_params)
                }
                _ => {
                    // Standard SABR calibration
                    if forward > self.market_conventions.zero_threshold
                        || self.vol_convention == SwaptionVolConvention::Normal
                    {
                        sabr_calibrator.calibrate(
                            forward,
                            &strikes,
                            &vols,
                            expiry_years,
                            self.sabr_beta,
                        )
                    } else {
                        // Auto-detect shift for negative rates
                        sabr_calibrator.calibrate_auto_shift(
                            forward,
                            &strikes,
                            &vols,
                            expiry_years,
                            self.sabr_beta,
                        )
                    }
                }
            };

            match params {
                Ok(p) => {
                    sabr_params.insert((*expiry_bp, *tenor_bp), p.clone());

                    // Calculate residuals
                    let model = SABRModel::new(p);
                    for (i, &strike) in strikes.iter().enumerate() {
                        let key = format!(
                            "SWPT-exp{:.2}y-ten{:.2}y-K{:.4}-{:06}",
                            expiry_years, tenor_years, strike, residual_counter
                        );
                        let residual = match model.implied_volatility(forward, strike, expiry_years)
                        {
                            Ok(model_vol) => model_vol - vols[i],
                            Err(_) => crate::calibration::PENALTY,
                        };
                        residual_counter += 1;
                        all_residuals.insert(key, residual);
                    }
                }
                Err(_) => continue,
            }
        }

        if sabr_params.is_empty() {
            return Err(finstack_core::Error::Calibration {
                message: "Failed to calibrate any swaption expiry-tenor combinations".to_string(),
                category: "swaption_vol_calibration".to_string(),
            });
        }

        // 4. Build volatility surface on target grid
        let vol_grid = self.build_vol_grid(&sabr_params, base_context)?;

        // 5. Create 2D surface (expiry x tenor)
        let target_expiries = &self.market_conventions.standard_expiries;
        let target_tenors = &self.market_conventions.standard_tenors;

        let surface =
            VolSurface::from_grid(&self.surface_id, target_expiries, target_tenors, &vol_grid)?;

        // 6. Create calibration report
        let report = CalibrationReport::new(
            all_residuals,
            sabr_params.len(),
            true,
            "Swaption vol calibration completed",
        )
        .with_metadata("calibrator", "SwaptionVolCalibrator")
        .with_metadata("vol_convention", format!("{:?}", self.vol_convention))
        .with_metadata("atm_convention", format!("{:?}", self.atm_convention))
        .with_metadata("num_expiry_tenor_pairs", sabr_params.len().to_string());

        Ok((surface, report))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_normal_vs_lognormal_conversion() {
        let calibrator = SwaptionVolCalibrator::new(
            "TEST",
            SwaptionVolConvention::Normal,
            AtmStrikeConvention::SwapRate,
            Date::from_calendar_date(2025, Month::January, 1).expect("valid date"),
            "USD-OIS",
            Currency::USD,
        );

        let forward = 0.05; // 5% forward rate
        let normal_vol = 0.01; // 100bp normal vol

        // Convert to lognormal
        let lognormal_vol = calibrator.convert_volatility(
            normal_vol,
            SwaptionVolConvention::Normal,
            SwaptionVolConvention::Lognormal,
            forward,
            1.0,
        );

        assert!((lognormal_vol - 0.2).abs() < 1e-6); // Should be 20% lognormal vol

        // Convert back
        let recovered_normal = calibrator.convert_volatility(
            lognormal_vol,
            SwaptionVolConvention::Lognormal,
            SwaptionVolConvention::Normal,
            forward,
            1.0,
        );

        assert!((recovered_normal - normal_vol).abs() < 1e-10);
    }

    #[test]
    fn test_atm_strike_conventions() {
        let calibrator = SwaptionVolCalibrator::new(
            "TEST",
            SwaptionVolConvention::Lognormal,
            AtmStrikeConvention::SwapRate,
            Date::from_calendar_date(2025, Month::January, 1).expect("valid date"),
            "USD-OIS",
            Currency::USD,
        );

        let forward_rate = 0.035; // 3.5%
        let expiry = Date::from_calendar_date(2026, Month::January, 1).expect("valid date");

        // Create a dummy context
        let context = MarketContext::new();

        // ATM = forward for swap rate convention
        let atm = calibrator
            .determine_atm_strike(forward_rate, expiry, 5.0, &context)
            .expect("should determine ATM strike");

        assert!((atm - forward_rate).abs() < 1e-10);
    }
}
