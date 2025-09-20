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
use crate::instruments::models::{SABRCalibrator, SABRModel, SABRParameters};
use crate::instruments::swaption::Swaption;
use crate::instruments::PricingOverrides;
use finstack_core::dates::utils::add_months;
use finstack_core::dates::{Date, DayCountCtx, Frequency};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::money::Money;
use finstack_core::prelude::Currency;
use finstack_core::{Result, F};
use std::collections::BTreeMap;

/// Type alias for grouped quotes by expiry-tenor pairs
type QuotesByExpiryTenor = BTreeMap<(u64, u64), Vec<(F, F)>>;

/// Type alias for SABR parameters by expiry-tenor pairs  
type SABRParamsByExpiryTenor = BTreeMap<(u64, u64), SABRParameters>;

/// Convert a float to basis points for use as a map key
fn to_basis_points(value: F) -> u64 {
    (value * 10000.0).round() as u64
}

/// Volatility quoting convention for swaptions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SwaptionVolConvention {
    /// Normal (absolute) volatility in basis points
    Normal,
    /// Lognormal (Black) volatility as percentage
    Lognormal,
    /// Shifted lognormal for negative rates
    ShiftedLognormal { shift: F },
}

/// ATM strike convention for swaptions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AtmStrikeConvention {
    /// ATM = forward swap rate (standard market convention)
    SwapRate,
    /// ATM = par swap rate (same as forward for zero-cost swap)
    ParRate,
}

/// Swaption volatility surface calibrator.
#[derive(Clone, Debug)]
pub struct SwaptionVolCalibrator {
    /// Surface identifier
    pub surface_id: String,
    /// Volatility convention
    pub vol_convention: SwaptionVolConvention,
    /// ATM strike convention
    pub atm_convention: AtmStrikeConvention,
    /// Fixed SABR beta (0 for normal, 1 for lognormal)
    pub sabr_beta: F,
    /// Base date for calculations
    pub base_date: Date,
    /// Discount curve ID for swap pricing
    pub disc_id: &'static str,
    /// Forward curve ID (if different from discount)
    pub forward_id: Option<&'static str>,
    /// Currency for market conventions
    pub currency: Currency,
    /// Market conventions configuration
    pub market_conventions: SwaptionMarketConvention,
    /// Calibration configuration
    pub config: CalibrationConfig,
}

impl SwaptionVolCalibrator {
    /// Create a new swaption volatility calibrator.
    pub fn new(
        surface_id: impl Into<String>,
        vol_convention: SwaptionVolConvention,
        atm_convention: AtmStrikeConvention,
        base_date: Date,
        disc_id: &'static str,
        currency: Currency,
    ) -> Self {
        // Set SABR beta based on volatility convention
        let sabr_beta = match vol_convention {
            SwaptionVolConvention::Normal => 0.0,
            SwaptionVolConvention::Lognormal | SwaptionVolConvention::ShiftedLognormal { .. } => {
                1.0
            }
        };

        Self {
            surface_id: surface_id.into(),
            vol_convention,
            atm_convention,
            sabr_beta,
            base_date,
            disc_id,
            forward_id: None,
            currency,
            market_conventions: SwaptionMarketConvention::from_currency(currency),
            config: CalibrationConfig::default(),
        }
    }

    /// Set the forward curve ID (if different from discount).
    pub fn with_forward_id(mut self, forward_id: &'static str) -> Self {
        self.forward_id = Some(forward_id);
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

    /// Calculate forward swap rate for a given expiry and tenor.
    fn calculate_forward_swap_rate(
        &self,
        expiry: Date,
        tenor_years: F,
        context: &MarketContext,
    ) -> Result<F> {
        let disc = context
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            self.disc_id,
        )?;
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
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::InvalidDateRange,
            ));
        }

        let df_start = disc.df(t_start);
        let df_end = disc.df(t_end);

        // Calculate annuity using proper schedule
        let pv01 = self.calculate_pv01_proper(swap_start, swap_end, disc)?;

        if pv01 <= self.market_conventions.zero_threshold {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            ));
        }

        // Forward swap rate = (DF(start) - DF(end)) / PV01
        Ok((df_start - df_end) / pv01)
    }

    /// Calculate PV01 using proper schedule generation.
    fn calculate_pv01_proper(
        &self,
        start: Date,
        end: Date,
        disc: &dyn finstack_core::market_data::traits::Discounting,
    ) -> Result<F> {
        // Generate payment dates based on frequency
        let mut dates = vec![start];
        let mut current = start;

        while current < end {
            current = match self.market_conventions.fixed_freq {
                Frequency::Months(m) => add_months(current, m as i32),
                Frequency::Days(d) => current + time::Duration::days(d as i64),
                _ => add_months(current, 3), // Default to quarterly
            };
            if current > end {
                break;
            }
            dates.push(current);
        }
        if dates.last() != Some(&end) {
            dates.push(end);
        }

        let mut pv01 = 0.0;
        for i in 1..dates.len() {
            let period_start = dates[i - 1];
            let period_end = dates[i];
            let dcf = self.market_conventions.day_count.year_fraction(
                period_start,
                period_end,
                DayCountCtx::default(),
            )?;
            let t = self.market_conventions.day_count.year_fraction(
                self.base_date,
                period_end,
                DayCountCtx::default(),
            )?;
            pv01 += disc.df(t) * dcf;
        }

        Ok(pv01)
    }

    /// Calculate swap annuity for a given expiry and tenor.
    #[allow(dead_code)] // Will be used for future swaption pricing enhancements
    fn calculate_swap_annuity(
        &self,
        expiry: Date,
        tenor_years: F,
        context: &MarketContext,
    ) -> Result<F> {
        let swap_start = expiry;
        let swap_end = add_months(expiry, (tenor_years * 12.0) as i32);

        let swaption = Swaption {
            id: "temp".to_string(),
            option_type: crate::instruments::OptionType::Call,
            notional: Money::new(1_000_000.0, self.currency),
            strike_rate: 0.0,
            expiry,
            swap_start,
            swap_end,
            fixed_freq: self.market_conventions.fixed_freq,
            float_freq: self.market_conventions.float_freq,
            day_count: self.market_conventions.day_count,
            exercise: crate::instruments::swaption::SwaptionExercise::European,
            settlement: crate::instruments::swaption::SwaptionSettlement::Physical,
            disc_id: self.disc_id,
            forward_id: self.forward_id.unwrap_or(self.disc_id),
            vol_id: "dummy",
            pricing_overrides: PricingOverrides::default(),
            sabr_params: None,
            attributes: Default::default(),
        };

        let disc = context
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            self.disc_id,
        )?;
        crate::instruments::swaption::pricing::SwaptionPricer
            .swap_annuity(&swaption, disc, self.base_date)
    }

    /// Convert volatility between conventions.
    fn convert_volatility(
        &self,
        vol: F,
        from_convention: SwaptionVolConvention,
        to_convention: SwaptionVolConvention,
        forward_rate: F,
        _time_to_expiry: F,
    ) -> F {
        match (from_convention, to_convention) {
            (SwaptionVolConvention::Normal, SwaptionVolConvention::Lognormal) => {
                // Normal to lognormal: σ_LN = σ_N / F
                if forward_rate.abs() > self.market_conventions.zero_threshold {
                    vol / forward_rate
                } else {
                    vol // Avoid division by zero
                }
            }
            (SwaptionVolConvention::Lognormal, SwaptionVolConvention::Normal) => {
                // Lognormal to normal: σ_N = σ_LN * F
                vol * forward_rate
            }
            (
                SwaptionVolConvention::ShiftedLognormal { shift: s1 },
                SwaptionVolConvention::ShiftedLognormal { shift: s2 },
            ) => {
                if (s1 - s2).abs() < 1e-10 {
                    vol // Same shift
                } else {
                    // Convert between different shifts
                    // σ_shifted2 = σ_shifted1 * (F + s1) / (F + s2)
                    vol * (forward_rate + s1) / (forward_rate + s2)
                }
            }
            (SwaptionVolConvention::Normal, SwaptionVolConvention::ShiftedLognormal { shift }) => {
                // Normal to shifted lognormal
                if (forward_rate + shift).abs() > self.market_conventions.zero_threshold {
                    vol / (forward_rate + shift)
                } else {
                    vol
                }
            }
            (SwaptionVolConvention::ShiftedLognormal { shift }, SwaptionVolConvention::Normal) => {
                // Shifted lognormal to normal
                vol * (forward_rate + shift)
            }
            (
                SwaptionVolConvention::Lognormal,
                SwaptionVolConvention::ShiftedLognormal { shift },
            ) => {
                // Lognormal to shifted lognormal: use Black's approximation
                vol * forward_rate / (forward_rate + shift)
            }
            (
                SwaptionVolConvention::ShiftedLognormal { shift },
                SwaptionVolConvention::Lognormal,
            ) => {
                // Shifted lognormal to lognormal
                if forward_rate.abs() > self.market_conventions.zero_threshold {
                    vol * (forward_rate + shift) / forward_rate
                } else {
                    vol
                }
            }
            _ => vol, // Same convention
        }
    }

    /// Determine ATM strike based on convention.
    fn determine_atm_strike(
        &self,
        forward_rate: F,
        _expiry: Date,
        _tenor_years: F,
        _context: &MarketContext,
    ) -> Result<F> {
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
    ) -> Result<Vec<F>> {
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
        target_expiry: F,
        target_tenor: F,
        sabr_params: &SABRParamsByExpiryTenor,
        context: &MarketContext,
    ) -> Result<F> {
        // Find closest calibrated point using min_by instead of sorting entire list
        let closest = sabr_params
            .iter()
            .map(|((exp_bp, ten_bp), params)| {
                let exp = *exp_bp as F / 10000.0;
                let ten = *ten_bp as F / 10000.0;
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
            .with_max_iterations(self.config.max_iterations);

        for ((expiry_bp, tenor_bp), strikes_vols) in &grouped_quotes {
            if strikes_vols.len() < self.market_conventions.min_sabr_points {
                continue; // Need minimum points for SABR
            }

            let expiry_years = *expiry_bp as F / 10000.0;
            let tenor_years = *tenor_bp as F / 10000.0;
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
            let mut strikes: Vec<F> = Vec::new();
            let mut vols: Vec<F> = Vec::new();

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
                    let shifted_strikes: Vec<F> = strikes.iter().map(|&s| s + shift).collect();

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
                        match model.implied_volatility(forward, strike, expiry_years) {
                            Ok(model_vol) => {
                                let residual = model_vol - vols[i];
                                all_residuals
                                    .insert(format!("swaption_{}", residual_counter), residual);
                                residual_counter += 1;
                            }
                            Err(_) => {
                                all_residuals.insert(
                                    format!("swaption_{}", residual_counter),
                                    crate::calibration::penalize(),
                                );
                                residual_counter += 1;
                            }
                        }
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
        let report = CalibrationReport::success_simple(all_residuals, 1)
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
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
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
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            "USD-OIS",
            Currency::USD,
        );

        let forward_rate = 0.035; // 3.5%
        let expiry = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        // Create a dummy context
        let context = MarketContext::new();

        // ATM = forward for swap rate convention
        let atm = calibrator
            .determine_atm_strike(forward_rate, expiry, 5.0, &context)
            .unwrap();

        assert!((atm - forward_rate).abs() < 1e-10);
    }
}
