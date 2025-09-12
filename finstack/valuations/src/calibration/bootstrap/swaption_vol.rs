//! Swaption volatility surface calibration.
//!
//! Implements market-standard swaption volatility calibration supporting:
//! - Normal and lognormal volatility conventions
//! - Various ATM strike conventions
//! - SABR model calibration per expiry
//! - Accurate swap annuity calculations

use crate::calibration::quote::VolQuote;
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};
use crate::instruments::options::models::{SABRCalibrator, SABRModel, SABRParameters};
use crate::instruments::options::swaption::Swaption;
use finstack_core::dates::{Date, DayCount, DayCountCtx, Frequency};
use finstack_core::dates::utils::add_months;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::money::Money;
use finstack_core::prelude::Currency;
use finstack_core::{Result, F};
use ordered_float::OrderedFloat;
use std::collections::BTreeMap;
use time::Duration;

/// Type alias for grouped quotes by expiry-tenor pairs
type QuotesByExpiryTenor = BTreeMap<(OrderedFloat<F>, OrderedFloat<F>), Vec<(F, F)>>;

/// Type alias for SABR parameters by expiry-tenor pairs  
type SABRParamsByExpiryTenor = BTreeMap<(OrderedFloat<F>, OrderedFloat<F>), SABRParameters>;

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
    /// ATM = forward swap rate
    SwapRate,
    /// ATM = par swap rate (same as forward for zero-cost swap)
    ParRate,
    /// ATM = delta-neutral strike
    DeltaNeutral,
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
    /// Target expiry grid (in years)
    pub target_expiries: Vec<F>,
    /// Target tenor grid (in years)
    pub target_tenors: Vec<F>,
    /// Base date for calculations
    pub base_date: Date,
    /// Discount curve ID for swap pricing
    pub disc_id: &'static str,
    /// Forward curve ID (if different from discount)
    pub forward_id: Option<&'static str>,
    /// Day count convention for time calculations
    pub day_count: DayCount,
    /// Fixed leg frequency for swap calculations
    pub fixed_freq: Frequency,
    /// Float leg frequency for swap calculations
    pub float_freq: Frequency,
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
    ) -> Self {
        // Set SABR beta based on volatility convention
        let sabr_beta = match vol_convention {
            SwaptionVolConvention::Normal => 0.0,
            SwaptionVolConvention::Lognormal | SwaptionVolConvention::ShiftedLognormal { .. } => 1.0,
        };

        Self {
            surface_id: surface_id.into(),
            vol_convention,
            atm_convention,
            sabr_beta,
            target_expiries: vec![0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0],
            target_tenors: vec![1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0],
            base_date,
            disc_id,
            forward_id: None,
            day_count: DayCount::Act365F,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            config: CalibrationConfig::default(),
        }
    }

    /// Set the forward curve ID (if different from discount).
    pub fn with_forward_id(mut self, forward_id: &'static str) -> Self {
        self.forward_id = Some(forward_id);
        self
    }

    /// Set the target expiry grid.
    pub fn with_expiries(mut self, expiries: Vec<F>) -> Self {
        self.target_expiries = expiries;
        self
    }

    /// Set the target tenor grid.
    pub fn with_tenors(mut self, tenors: Vec<F>) -> Self {
        self.target_tenors = tenors;
        self
    }

    /// Set the day count convention.
    pub fn with_day_count(mut self, day_count: DayCount) -> Self {
        self.day_count = day_count;
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
        // Simple implementation: use discount curve to calculate forward swap rate
        // For a swap starting at expiry and ending at expiry + tenor:
        // Forward rate ≈ (DF(start) - DF(end)) / (PV01 of fixed leg)
        
        let disc = context.disc(self.disc_id)?;
        let swap_start = expiry;
        let swap_end = self.add_years_approx(expiry, tenor_years);
        
        // Calculate time to expiry and time to swap end
        let t_start = self.day_count
            .year_fraction(self.base_date, swap_start, DayCountCtx::default())?;
        let t_end = self.day_count
            .year_fraction(self.base_date, swap_end, DayCountCtx::default())?;
        
        if t_start < 0.0 || t_end <= t_start {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::InvalidDateRange,
            ));
        }
        
        let df_start = disc.df(t_start);
        let df_end = disc.df(t_end);
        
        // Simple approximation: assume semi-annual payments for fixed leg
        let num_payments = (tenor_years * 2.0).ceil() as usize;
        let payment_interval = tenor_years / num_payments as F;
        
        // Calculate approximate PV01 (present value of $1 per payment)
        let mut pv01 = 0.0;
        for i in 1..=num_payments {
            let payment_time = t_start + (i as F) * payment_interval;
            pv01 += disc.df(payment_time) * payment_interval;
        }
        
        if pv01 <= 0.0 {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            ));
        }
        
        // Forward swap rate approximation
        let forward_rate = (df_start - df_end) / pv01;
        
        Ok(forward_rate)
    }
    
    /// Calculate swap annuity for a given expiry and tenor.
    #[allow(dead_code)] // Used for future swaption pricing functionality
    fn calculate_swap_annuity(
        &self,
        expiry: Date,
        tenor_years: F,
        context: &MarketContext,
    ) -> Result<F> {
        let swap_start = expiry;
        let swap_end = self.add_years_approx(expiry, tenor_years);
        
        let swaption = Swaption {
            id: "temp".to_string(),
            option_type: crate::instruments::options::OptionType::Call,
            notional: Money::new(1_000_000.0, Currency::USD),
            strike_rate: 0.0,
            expiry,
            swap_start,
            swap_end,
            fixed_freq: self.fixed_freq,
            float_freq: self.float_freq,
            day_count: self.day_count,
            exercise: crate::instruments::options::swaption::SwaptionExercise::European,
            settlement: crate::instruments::options::swaption::SwaptionSettlement::Physical,
            disc_id: self.disc_id,
            forward_id: self.forward_id.unwrap_or(self.disc_id),
            vol_id: "dummy",
            implied_vol: None,
            sabr_params: None,
            attributes: Default::default(),
        };
        
        let disc = context.disc(self.disc_id)?;
        swaption.swap_annuity(disc.as_ref())
    }
    
    /// Helper to add years to a date (approximate).
    fn add_years_approx(&self, date: Date, years: F) -> Date {
        let full_years = years.floor() as i32;
        let remaining_days = ((years - full_years as F) * 365.25) as i64;
        
        // Add full years by converting to months
        let date_with_years = add_months(date, full_years * 12);
        
        // Add remaining days
        date_with_years + Duration::days(remaining_days)
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
                if forward_rate.abs() > 1e-8 {
                    vol / forward_rate
                } else {
                    vol // Avoid division by zero
                }
            }
            (SwaptionVolConvention::Lognormal, SwaptionVolConvention::Normal) => {
                // Lognormal to normal: σ_N = σ_LN * F
                vol * forward_rate
            }
            (SwaptionVolConvention::ShiftedLognormal { shift: s1 }, SwaptionVolConvention::ShiftedLognormal { shift: s2 }) if (s1 - s2).abs() < 1e-10 => {
                vol // Same shift
            }
            _ => vol, // Same convention or complex conversion - keep as is
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
                Ok(forward_rate)
            }
            AtmStrikeConvention::DeltaNeutral => {
                // Simplified: use forward rate as approximation
                // Full implementation would solve for delta-neutral strike
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
        let mut vol_grid = Vec::with_capacity(self.target_expiries.len() * self.target_tenors.len());
        
        for &expiry_years in &self.target_expiries {
            for &tenor_years in &self.target_tenors {
                let key = (expiry_years.into(), tenor_years.into());
                
                if let Some(params) = sabr_params.get(&key) {
                    // Have exact calibrated parameters
                    let model = SABRModel::new(params.clone());
                    let expiry = self.add_years_approx(self.base_date, expiry_years);
                    let forward = self.calculate_forward_swap_rate(expiry, tenor_years, context)?;
                    let strike = self.determine_atm_strike(forward, expiry, tenor_years, context)?;
                    
                    let vol = model.implied_volatility(forward, strike, expiry_years)?;
                    vol_grid.push(vol);
                } else {
                    // Interpolate from nearby points
                    let vol = self.interpolate_sabr_vol(expiry_years, tenor_years, sabr_params, context)?;
                    vol_grid.push(vol);
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
        // Find closest calibrated points
        let mut closest_points: Vec<((F, F), F, &SABRParameters)> = Vec::new();
        
        for ((exp, ten), params) in sabr_params {
            let distance = ((exp.0 - target_expiry).powi(2) + (ten.0 - target_tenor).powi(2)).sqrt();
            closest_points.push(((exp.0, ten.0), distance, params));
        }
        
        closest_points.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        
        if closest_points.is_empty() {
            return Ok(0.2); // Default volatility
        }
        
        // Use closest point for simplicity
        let params = closest_points[0].2;
        let model = SABRModel::new(params.clone());
        let expiry = self.add_years_approx(self.base_date, target_expiry);
        let forward = self.calculate_forward_swap_rate(expiry, target_tenor, context)?;
        let strike = self.determine_atm_strike(forward, expiry, target_tenor, context)?;
        
        model.implied_volatility(forward, strike, target_expiry)
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
            if let VolQuote::SwaptionVol { expiry, tenor, strike, vol, .. } = quote {
                let expiry_years = self.day_count
                    .year_fraction(self.base_date, *expiry, DayCountCtx::default())
                    .unwrap_or(0.0);
                let tenor_years = self.day_count
                    .year_fraction(*expiry, *tenor, DayCountCtx::default())
                    .unwrap_or(0.0);
                    
                    
                if expiry_years > 0.0 && tenor_years > 0.0 {
                    grouped_quotes
                        .entry((expiry_years.into(), tenor_years.into()))
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
        
        for ((expiry_key, tenor_key), strikes_vols) in &grouped_quotes {
            if strikes_vols.len() < 3 {
                continue; // Need at least 3 points for SABR
            }
            
            let expiry_years = expiry_key.into_inner();
            let tenor_years = tenor_key.into_inner();
            let expiry_date = self.add_years_approx(self.base_date, expiry_years);
            
            // Calculate forward swap rate
            let forward = match self.calculate_forward_swap_rate(expiry_date, tenor_years, base_context) {
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
                    if forward > 0.0 || self.vol_convention == SwaptionVolConvention::Normal {
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
                    sabr_params.insert((*expiry_key, *tenor_key), p.clone());
                    
                    // Calculate residuals
                    let model = SABRModel::new(p);
                    for (i, &strike) in strikes.iter().enumerate() {
                        match model.implied_volatility(forward, strike, expiry_years) {
                            Ok(model_vol) => {
                                let residual = model_vol - vols[i];
                                all_residuals.insert(format!("swaption_{}", residual_counter), residual);
                                residual_counter += 1;
                            }
                            Err(_) => {
                                all_residuals.insert(format!("swaption_{}", residual_counter), crate::calibration::penalize());
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
        let mut builder = VolSurface::builder(&self.surface_id)
            .expiries(&self.target_expiries)
            .strikes(&self.target_tenors); // Using tenors as "strikes" dimension
            
        // Add volatility rows
        let rows_per_expiry = self.target_tenors.len();
        for chunk in vol_grid.chunks(rows_per_expiry) {
            builder = builder.row(chunk);
        }
        
        let surface = builder.build()?;
        
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
        );
        
        let forward_rate = 0.035; // 3.5%
        let expiry = Date::from_calendar_date(2026, Month::January, 1).unwrap();
        
        // Create a dummy context
        let context = MarketContext::new();
        
        // ATM = forward for swap rate convention
        let atm = calibrator.determine_atm_strike(
            forward_rate,
            expiry,
            5.0,
            &context,
        ).unwrap();
        
        assert!((atm - forward_rate).abs() < 1e-10);
    }
}
