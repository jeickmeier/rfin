//! Swaption volatility surface calibration.
//!
//! Implements market-standard swaption volatility calibration supporting:
//! - Normal and lognormal volatility conventions
//! - Various ATM strike conventions
//! - SABR model calibration per expiry
//! - Accurate swap annuity calculations
//!
use crate::calibration::quotes::VolQuote;
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};
use crate::instruments::common::models::{SABRCalibrator, SABRModel, SABRParameters};
use finstack_core::config::FinstackConfig;
use finstack_core::dates::DateExt;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, DayCountCtx, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::prelude::Currency;
use finstack_core::types::CurveId;
use finstack_core::volatility::{convert_atm_volatility, VolatilityConvention};
use finstack_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// =============================================================================
// Swaption Market Conventions (moved from market_standards)
// =============================================================================

/// Get the default settlement calendar ID for a currency.
fn default_calendar_for_currency(currency: Currency) -> &'static str {
    match currency {
        Currency::USD => "usny",
        Currency::EUR => "target2",
        Currency::GBP => "gblo",
        Currency::JPY => "jpto",
        Currency::CHF => "chzu",
        Currency::AUD => "ausy",
        Currency::CAD => "cato",
        _ => "usny",
    }
}

/// Method for estimating swap payments
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PaymentEstimation {
    /// Use proper schedule generation
    ProperSchedule,
}

/// Market convention configuration for swaption calibration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwaptionMarketConvention {
    /// Fixed leg day count (used for swap annuity / PV01).
    pub fixed_day_count: DayCount,
    /// Floating leg day count (used for float PV in multi-curve mode).
    pub float_day_count: DayCount,
    /// Fixed leg business day convention.
    pub fixed_bdc: BusinessDayConvention,
    /// Floating leg business day convention.
    pub float_bdc: BusinessDayConvention,
    /// Fixed leg frequency
    pub fixed_freq: Tenor,
    /// Float leg frequency
    pub float_freq: Tenor,
    /// Standard expiry points (in years)
    pub standard_expiries: Vec<f64>,
    /// Standard tenor points (in years)
    pub standard_tenors: Vec<f64>,
    /// Minimum points for SABR calibration
    pub min_sabr_points: usize,
    /// Default volatility for missing data
    pub default_vol: f64,
    /// Reporting tolerance used to determine calibration success.
    pub vol_tolerance: f64,
    /// Zero threshold for rate checks
    pub zero_threshold: f64,
    /// Payment estimation method
    pub payment_estimation: PaymentEstimation,
}

impl SwaptionMarketConvention {
    /// USD market conventions
    pub fn usd() -> Self {
        Self {
            fixed_day_count: DayCount::Thirty360,
            float_day_count: DayCount::Act360,
            fixed_bdc: BusinessDayConvention::ModifiedFollowing,
            float_bdc: BusinessDayConvention::ModifiedFollowing,
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
            standard_expiries: vec![
                0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0,
            ],
            standard_tenors: vec![1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0],
            min_sabr_points: 3,
            default_vol: 0.2,
            vol_tolerance: 0.0015,
            zero_threshold: 1e-8,
            payment_estimation: PaymentEstimation::ProperSchedule,
        }
    }

    /// EUR market conventions
    pub fn eur() -> Self {
        Self {
            fixed_day_count: DayCount::ThirtyE360,
            float_day_count: DayCount::Act360,
            fixed_bdc: BusinessDayConvention::ModifiedFollowing,
            float_bdc: BusinessDayConvention::ModifiedFollowing,
            fixed_freq: Tenor::annual(),
            float_freq: Tenor::semi_annual(),
            standard_expiries: vec![
                0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0,
            ],
            standard_tenors: vec![1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0],
            min_sabr_points: 3,
            default_vol: 0.2,
            vol_tolerance: 0.0015,
            zero_threshold: 1e-8,
            payment_estimation: PaymentEstimation::ProperSchedule,
        }
    }

    /// GBP market conventions
    pub fn gbp() -> Self {
        Self {
            fixed_day_count: DayCount::Act365F,
            float_day_count: DayCount::Act365F,
            fixed_bdc: BusinessDayConvention::ModifiedFollowing,
            float_bdc: BusinessDayConvention::ModifiedFollowing,
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
            standard_expiries: vec![
                0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0,
            ],
            standard_tenors: vec![1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0],
            min_sabr_points: 3,
            default_vol: 0.2,
            vol_tolerance: 0.0015,
            zero_threshold: 1e-8,
            payment_estimation: PaymentEstimation::ProperSchedule,
        }
    }

    /// JPY market conventions
    pub fn jpy() -> Self {
        Self {
            fixed_day_count: DayCount::Act365F,
            float_day_count: DayCount::Act365F,
            fixed_bdc: BusinessDayConvention::ModifiedFollowing,
            float_bdc: BusinessDayConvention::ModifiedFollowing,
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
            standard_expiries: vec![
                0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0,
            ],
            standard_tenors: vec![1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0],
            min_sabr_points: 3,
            default_vol: 0.2,
            vol_tolerance: 0.0015,
            zero_threshold: 1e-8,
            payment_estimation: PaymentEstimation::ProperSchedule,
        }
    }

    /// CHF market conventions
    pub fn chf() -> Self {
        Self {
            fixed_day_count: DayCount::ThirtyE360,
            float_day_count: DayCount::Act360,
            fixed_bdc: BusinessDayConvention::ModifiedFollowing,
            float_bdc: BusinessDayConvention::ModifiedFollowing,
            fixed_freq: Tenor::annual(),
            float_freq: Tenor::semi_annual(),
            standard_expiries: vec![
                0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0,
            ],
            standard_tenors: vec![1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0],
            min_sabr_points: 3,
            default_vol: 0.2,
            vol_tolerance: 0.0015,
            zero_threshold: 1e-8,
            payment_estimation: PaymentEstimation::ProperSchedule,
        }
    }

    /// Create from currency using standard market conventions
    pub fn from_currency(currency: Currency) -> Self {
        match currency {
            Currency::USD => Self::usd(),
            Currency::EUR => Self::eur(),
            Currency::GBP => Self::gbp(),
            Currency::JPY => Self::jpy(),
            Currency::CHF => Self::chf(),
            _ => Self::usd(),
        }
    }

    /// Builder pattern for customization
    pub fn with_expiries(mut self, expiries: Vec<f64>) -> Self {
        self.standard_expiries = expiries;
        self
    }

    /// Set standard tenors for swaption matrix
    pub fn with_tenors(mut self, tenors: Vec<f64>) -> Self {
        self.standard_tenors = tenors;
        self
    }

    /// Set day count convention for both fixed and floating legs.
    pub fn with_day_count(mut self, day_count: DayCount) -> Self {
        self.fixed_day_count = day_count;
        self.float_day_count = day_count;
        self
    }

    /// Set fixed leg day count convention.
    pub fn with_fixed_day_count(mut self, day_count: DayCount) -> Self {
        self.fixed_day_count = day_count;
        self
    }

    /// Set floating leg day count convention.
    pub fn with_float_day_count(mut self, day_count: DayCount) -> Self {
        self.float_day_count = day_count;
        self
    }

    /// Set fixed leg frequency
    pub fn with_fixed_freq(mut self, freq: Tenor) -> Self {
        self.fixed_freq = freq;
        self
    }

    /// Set floating leg frequency
    pub fn with_float_freq(mut self, freq: Tenor) -> Self {
        self.float_freq = freq;
        self
    }

    /// Set fixed leg business day convention.
    pub fn with_fixed_bdc(mut self, bdc: BusinessDayConvention) -> Self {
        self.fixed_bdc = bdc;
        self
    }

    /// Set floating leg business day convention.
    pub fn with_float_bdc(mut self, bdc: BusinessDayConvention) -> Self {
        self.float_bdc = bdc;
        self
    }

    /// Set default volatility for missing points
    pub fn with_default_vol(mut self, vol: f64) -> Self {
        self.default_vol = vol;
        self
    }

    /// Set the reporting tolerance used to declare calibration success.
    pub fn with_vol_tolerance(mut self, tol: f64) -> Self {
        self.vol_tolerance = tol;
        self
    }

    /// Set threshold below which volatilities are treated as zero
    pub fn with_zero_threshold(mut self, threshold: f64) -> Self {
        self.zero_threshold = threshold;
        self
    }

    /// Set minimum number of points required for SABR calibration
    pub fn with_min_sabr_points(mut self, min_points: usize) -> Self {
        self.min_sabr_points = min_points;
        self
    }

    /// Set payment estimation method for swaption pricing
    pub fn with_payment_estimation(mut self, method: PaymentEstimation) -> Self {
        self.payment_estimation = method;
        self
    }
}

// =============================================================================
// Type Aliases
// =============================================================================

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

impl From<SwaptionVolConvention> for VolatilityConvention {
    fn from(val: SwaptionVolConvention) -> Self {
        match val {
            SwaptionVolConvention::Normal => VolatilityConvention::Normal,
            SwaptionVolConvention::Lognormal => VolatilityConvention::Lognormal,
            SwaptionVolConvention::ShiftedLognormal { shift } => {
                VolatilityConvention::ShiftedLognormal { shift }
            }
        }
    }
}

/// ATM strike convention for swaptions.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AtmStrikeConvention {
    /// ATM = forward swap rate (standard market convention)
    SwapRate,
    /// ATM = par swap rate (same as forward for zero-cost swap)
    ParRate,
}

/// Interpolation method for SABR parameters across the expiry–tenor grid.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SabrInterpolationMethod {
    /// Bilinear interpolation in (expiry, tenor) over SABR parameters.
    #[default]
    Bilinear,
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
    /// Interpolation method used to infer SABR parameters between calibrated slices.
    #[serde(default)]
    pub sabr_interpolation: SabrInterpolationMethod,
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
            sabr_interpolation: SabrInterpolationMethod::Bilinear,
            // Default to the market-standard settlement calendar for the currency.
            calendar_id: Some(default_calendar_for_currency(currency).to_string()),
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

    /// Set calibration configuration from a `FinstackConfig`.
    ///
    /// Resolves `CalibrationConfig` from `FinstackConfig.extensions["valuations.calibration.v1"]`.
    pub fn with_finstack_config(mut self, cfg: &FinstackConfig) -> Result<Self> {
        self.config = CalibrationConfig::from_finstack_config_or_default(cfg)?;
        Ok(self)
    }

    /// Set the SABR interpolation method for expiry–tenor points without direct calibration.
    pub fn with_sabr_interpolation_method(mut self, method: SabrInterpolationMethod) -> Self {
        self.sabr_interpolation = method;
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
        let swap_end = expiry.add_months((tenor_years * 12.0) as i32);

        let t_start =
            disc.day_count()
                .year_fraction(disc.base_date(), swap_start, DayCountCtx::default())?;
        let t_end =
            disc.day_count()
                .year_fraction(disc.base_date(), swap_end, DayCountCtx::default())?;

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
            let float_sched = crate::cashflow::builder::date_generation::build_dates_checked(
                swap_start,
                swap_end,
                self.market_conventions.float_freq,
                StubKind::None,
                self.market_conventions.float_bdc,
                self.calendar_id.as_deref(),
            )?;

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
                let accrual = self.market_conventions.float_day_count.year_fraction(
                    prev,
                    pay_date,
                    DayCountCtx::default(),
                )?;

                // Time to payment
                let t_pay_disc = disc.day_count().year_fraction(
                    disc.base_date(),
                    pay_date,
                    DayCountCtx::default(),
                )?;

                // Time to period start
                let t_prev_fwd =
                    fwd.day_count()
                        .year_fraction(fwd.base_date(), prev, DayCountCtx::default())?;
                let t_pay_fwd = fwd.day_count().year_fraction(
                    fwd.base_date(),
                    pay_date,
                    DayCountCtx::default(),
                )?;

                // Forward rate for this period (using the forward curve)
                let forward_rate = fwd.rate_period(t_prev_fwd, t_pay_fwd);

                // Payment = forward_rate * accrual * discount_factor
                float_pv += forward_rate * accrual * disc.df(t_pay_disc);

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
        // Use shared builder to avoid duplication and ensure consistency.
        let sched = crate::cashflow::builder::date_generation::build_dates_checked(
            start,
            end,
            self.market_conventions.fixed_freq,
            StubKind::None,
            self.market_conventions.fixed_bdc,
            self.calendar_id.as_deref(),
        )?;
        let dates = &sched.dates;
        if dates.len() < 2 {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            ));
        }

        let mut pv01 = 0.0;
        let mut prev = dates[0];
        for &d in dates.iter().skip(1) {
            let dcf = self.market_conventions.fixed_day_count.year_fraction(
                prev,
                d,
                DayCountCtx::default(),
            )?;
            let t = disc
                .day_count()
                .year_fraction(disc.base_date(), d, DayCountCtx::default())?;
            pv01 += disc.df(t) * dcf;
            prev = d;
        }

        Ok(pv01)
    }

    /// Convert ATM volatility between conventions.
    fn convert_volatility(
        &self,
        vol: f64,
        from_convention: SwaptionVolConvention,
        to_convention: SwaptionVolConvention,
        forward_rate: f64,
        time_to_expiry: f64,
    ) -> Result<f64> {
        convert_atm_volatility(
            vol,
            from_convention.into(),
            to_convention.into(),
            forward_rate,
            time_to_expiry,
        )
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
    ///
    /// # Errors
    ///
    /// Returns an error if SABR implied volatility cannot be computed for any
    /// expiry/tenor combination. Silent fallbacks are not allowed as they can
    /// mask calibration failures and produce invalid risk surfaces.
    fn build_vol_grid(
        &self,
        sabr_params: &SABRParamsByExpiryTenor,
        context: &MarketContext,
    ) -> Result<Vec<f64>> {
        let target_expiries = &self.market_conventions.standard_expiries;
        let target_tenors = &self.market_conventions.standard_tenors;
        let mut vol_grid = Vec::with_capacity(target_expiries.len() * target_tenors.len());
        let mut failed_points: Vec<(f64, f64, String)> = Vec::new();

        for &expiry_years in target_expiries {
            for &tenor_years in target_tenors {
                let key = (to_basis_points(expiry_years), to_basis_points(tenor_years));

                let vol_result = if let Some(params) = sabr_params.get(&key) {
                    // Have exact calibrated parameters
                    let model = SABRModel::new(params.clone());
                    let expiry = self.base_date.add_months((expiry_years * 12.0) as i32);

                    (|| -> Result<f64> {
                        let forward =
                            self.calculate_forward_swap_rate(expiry, tenor_years, context)?;
                        let strike =
                            self.determine_atm_strike(forward, expiry, tenor_years, context)?;
                        model
                            .implied_volatility(forward, strike, expiry_years)
                            .map_err(|e| finstack_core::Error::Calibration {
                                message: format!("SABR implied volatility failed: {e:?}"),
                                category: "swaption_vol_surface".to_string(),
                            })
                    })()
                } else {
                    // Interpolate from nearby points
                    self.interpolate_sabr_vol(expiry_years, tenor_years, sabr_params, context)
                };

                match vol_result {
                    Ok(vol) => vol_grid.push(vol),
                    Err(e) => {
                        // Track the failed point for error reporting
                        failed_points.push((expiry_years, tenor_years, format!("{e:?}")));
                        // Push placeholder to maintain grid structure; will error below
                        vol_grid.push(f64::NAN);
                    }
                }
            }
        }

        // Fail calibration if any vol computations failed
        if !failed_points.is_empty() {
            let failed_desc: Vec<String> = failed_points
                .iter()
                .take(10) // Limit error message size
                .map(|(exp, ten, err)| format!("expiry={exp:.2}y tenor={ten:.2}y: {err}"))
                .collect();
            let suffix = if failed_points.len() > 10 {
                format!(" (and {} more)", failed_points.len() - 10)
            } else {
                String::new()
            };
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Swaption vol grid failed at {} point(s): [{}]{}. \
                    Check that all expiry/tenor combinations have valid SABR parameters or can be interpolated.",
                    failed_points.len(),
                    failed_desc.join("; "),
                    suffix
                ),
                category: "swaption_vol_grid_build".to_string(),
            });
        }

        Ok(vol_grid)
    }

    /// Extract sorted unique expiries and tenors (in years) from SABR parameter grid.
    fn sabr_grid_axes(sabr_params: &SABRParamsByExpiryTenor) -> (Vec<f64>, Vec<f64>) {
        let mut expiries_bp = Vec::new();
        let mut tenors_bp = Vec::new();

        for (key, _) in sabr_params.iter() {
            let (exp_bp, ten_bp) = *key;
            expiries_bp.push(exp_bp);
            tenors_bp.push(ten_bp);
        }

        expiries_bp.sort_unstable();
        expiries_bp.dedup();
        tenors_bp.sort_unstable();
        tenors_bp.dedup();

        let expiries = expiries_bp
            .into_iter()
            .map(|bp| bp as f64 / 10000.0)
            .collect();
        let tenors = tenors_bp
            .into_iter()
            .map(|bp| bp as f64 / 10000.0)
            .collect();

        (expiries, tenors)
    }

    /// Locate bracketing indices for a target value within a sorted axis.
    ///
    /// Returns (i_lo, i_hi) as indices into `axis`. If the target lies outside
    /// the axis range, both indices collapse to the nearest endpoint.
    fn bracket_axis(axis: &[f64], target: f64) -> Option<(usize, usize)> {
        if axis.is_empty() {
            return None;
        }
        if axis.len() == 1 {
            return Some((0, 0));
        }

        // If target is before the first point or after the last, clamp to edges.
        if target <= axis[0] {
            return Some((0, 0));
        }
        if target >= axis[axis.len() - 1] {
            let last = axis.len() - 1;
            return Some((last, last));
        }

        // Find segment such that axis[i_lo] <= target <= axis[i_hi]
        for i in 0..axis.len() - 1 {
            if target >= axis[i] && target <= axis[i + 1] {
                return Some((i, i + 1));
            }
        }
        // Fallback: shouldn't happen with sorted axis, but be defensive
        Some((axis.len() - 1, axis.len() - 1))
    }

    /// Bilinear interpolation of SABR parameters across expiry–tenor grid.
    ///
    /// Returns interpolated parameters if a suitable neighborhood is found,
    /// otherwise None (caller should fall back to nearest/default behavior).
    fn interpolate_sabr_params_bilinear(
        &self,
        target_expiry: f64,
        target_tenor: f64,
        sabr_params: &SABRParamsByExpiryTenor,
    ) -> Option<SABRParameters> {
        if sabr_params.is_empty() {
            return None;
        }

        let (expiries, tenors) = Self::sabr_grid_axes(sabr_params);
        if expiries.is_empty() || tenors.is_empty() {
            return None;
        }

        let (ei_lo, ei_hi) = Self::bracket_axis(&expiries, target_expiry)?;
        let (ti_lo, ti_hi) = Self::bracket_axis(&tenors, target_tenor)?;

        let e_lo = expiries[ei_lo];
        let e_hi = expiries[ei_hi];
        let t_lo = tenors[ti_lo];
        let t_hi = tenors[ti_hi];

        // Helper to fetch parameters at given (expiry, tenor) years.
        let fetch = |e: f64, t: f64| -> Option<&SABRParameters> {
            let key = (to_basis_points(e), to_basis_points(t));
            sabr_params.get(&key)
        };

        // Exact node case: both axes collapsed → nearest grid point.
        if ei_lo == ei_hi && ti_lo == ti_hi {
            return fetch(e_lo, t_lo).cloned();
        }

        // 1D interpolation along tenor only (single expiry).
        if ei_lo == ei_hi && ti_lo != ti_hi {
            let p_lo = fetch(e_lo, t_lo)?;
            let p_hi = fetch(e_lo, t_hi).unwrap_or(p_lo);
            let wy = if (t_hi - t_lo).abs() > 0.0 {
                (target_tenor - t_lo) / (t_hi - t_lo)
            } else {
                0.0
            };
            return Some(Self::interpolate_sabr_linear(p_lo, p_hi, wy));
        }

        // 1D interpolation along expiry only (single tenor).
        if ti_lo == ti_hi && ei_lo != ei_hi {
            let p_lo = fetch(e_lo, t_lo)?;
            let p_hi = fetch(e_hi, t_lo).unwrap_or(p_lo);
            let wx = if (e_hi - e_lo).abs() > 0.0 {
                (target_expiry - e_lo) / (e_hi - e_lo)
            } else {
                0.0
            };
            return Some(Self::interpolate_sabr_linear(p_lo, p_hi, wx));
        }

        // Full bilinear interpolation requires all four corners.
        let p_00 = fetch(e_lo, t_lo)?;
        let p_10 = fetch(e_hi, t_lo).unwrap_or(p_00);
        let p_01 = fetch(e_lo, t_hi).unwrap_or(p_00);
        let p_11 = fetch(e_hi, t_hi).unwrap_or(p_10);

        let wx = if (e_hi - e_lo).abs() > 0.0 {
            (target_expiry - e_lo) / (e_hi - e_lo)
        } else {
            0.0
        };
        let wy = if (t_hi - t_lo).abs() > 0.0 {
            (target_tenor - t_lo) / (t_hi - t_lo)
        } else {
            0.0
        };

        Some(Self::interpolate_sabr_bilinear(
            p_00, p_10, p_01, p_11, wx, wy,
        ))
    }

    /// Linear interpolation between two SABR parameter sets in parameter space.
    ///
    /// - alpha, nu interpolated in log-space to preserve positivity.
    /// - rho interpolated linearly and clamped to (-1, 1).
    fn interpolate_sabr_linear(p0: &SABRParameters, p1: &SABRParameters, w: f64) -> SABRParameters {
        let w_clamped = w.clamp(0.0, 1.0);

        let log_alpha0 = p0.alpha.ln();
        let log_alpha1 = p1.alpha.ln();
        let log_nu0 = p0.nu.ln();
        let log_nu1 = p1.nu.ln();

        let alpha = (log_alpha0 * (1.0 - w_clamped) + log_alpha1 * w_clamped).exp();
        let nu = (log_nu0 * (1.0 - w_clamped) + log_nu1 * w_clamped).exp();

        let rho_raw = p0.rho * (1.0 - w_clamped) + p1.rho * w_clamped;
        let rho = rho_raw.clamp(-0.999, 0.999);

        SABRParameters {
            alpha,
            beta: p0.beta, // beta is fixed in calibrator; keep from base
            nu,
            rho,
            shift: p0.shift, // shift is constant for the surface
        }
    }

    /// Bilinear interpolation between four SABR parameter sets on a rectangle.
    fn interpolate_sabr_bilinear(
        p_00: &SABRParameters,
        p_10: &SABRParameters,
        p_01: &SABRParameters,
        p_11: &SABRParameters,
        wx: f64,
        wy: f64,
    ) -> SABRParameters {
        let wx_clamped = wx.clamp(0.0, 1.0);
        let wy_clamped = wy.clamp(0.0, 1.0);

        // First interpolate along expiry for each tenor.
        let p0 = Self::interpolate_sabr_linear(p_00, p_10, wx_clamped);
        let p1 = Self::interpolate_sabr_linear(p_01, p_11, wx_clamped);

        // Then interpolate along tenor between the two intermediate parameters.
        Self::interpolate_sabr_linear(&p0, &p1, wy_clamped)
    }
    /// Interpolate SABR volatility for points without direct calibration.
    ///
    /// # Errors
    ///
    /// Returns an error if interpolation is not possible (no nearby calibrated points)
    /// or if SABR implied volatility computation fails.
    fn interpolate_sabr_vol(
        &self,
        target_expiry: f64,
        target_tenor: f64,
        sabr_params: &SABRParamsByExpiryTenor,
        context: &MarketContext,
    ) -> Result<f64> {
        let params = self
            .interpolate_sabr_params_bilinear(target_expiry, target_tenor, sabr_params)
            .ok_or_else(|| finstack_core::Error::Calibration {
                message: format!(
                    "Cannot interpolate SABR params for expiry={target_expiry:.2}y tenor={target_tenor:.2}y: \
                    insufficient calibrated points for bilinear interpolation"
                ),
                category: "swaption_vol_interpolation".to_string(),
            })?;

        let model = SABRModel::new(params);
        let expiry = self.base_date.add_months((target_expiry * 12.0) as i32);
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
        let mut warnings: Vec<String> = Vec::new();

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
                let expiry_years = match self.market_conventions.fixed_day_count.year_fraction(
                    self.base_date,
                    *expiry,
                    DayCountCtx::default(),
                ) {
                    Ok(v) => v,
                    Err(e) => {
                        warnings.push(format!(
                            "Skipping swaption vol quote with invalid expiry year fraction: expiry={expiry:?} strike={strike:.6}: {e:?}"
                        ));
                        continue;
                    }
                };
                let tenor_years = match self.market_conventions.fixed_day_count.year_fraction(
                    *expiry,
                    *tenor,
                    DayCountCtx::default(),
                ) {
                    Ok(v) => v,
                    Err(e) => {
                        warnings.push(format!(
                            "Skipping swaption vol quote with invalid tenor year fraction: expiry={expiry:?} tenor={tenor:?} strike={strike:.6}: {e:?}"
                        ));
                        continue;
                    }
                };

                if expiry_years > 0.0 && tenor_years > 0.0 {
                    grouped_quotes
                        .entry((to_basis_points(expiry_years), to_basis_points(tenor_years)))
                        .or_default()
                        .push((*strike, *vol));
                } else {
                    warnings.push(format!(
                        "Skipping swaption vol quote with non-positive expiry/tenor: expiry={expiry:?} tenor={tenor:?} strike={strike:.6} (expiry_years={expiry_years:.6}, tenor_years={tenor_years:.6})"
                    ));
                }
            }
        }

        if grouped_quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        // 3. Calibrate SABR parameters for each expiry-tenor combination
        let mut sabr_params: SABRParamsByExpiryTenor = BTreeMap::new();
        let mut all_residuals = BTreeMap::new();
        let mut residual_counter = 0;
        let mut skipped_pairs: Vec<String> = Vec::new();

        let sabr_calibrator = SABRCalibrator::new()
            .with_tolerance(self.config.tolerance)
            .with_max_iterations(self.config.max_iterations)
            .with_fd_gradients(self.config.use_fd_sabr_gradients);

        for ((expiry_bp, tenor_bp), strikes_vols) in &grouped_quotes {
            if strikes_vols.len() < self.market_conventions.min_sabr_points {
                skipped_pairs.push(format!(
                    "exp={:.2}y tenor={:.2}y: insufficient points for SABR (have {}, need {})",
                    *expiry_bp as f64 / 10000.0,
                    *tenor_bp as f64 / 10000.0,
                    strikes_vols.len(),
                    self.market_conventions.min_sabr_points
                ));
                continue;
            }

            let expiry_years = *expiry_bp as f64 / 10000.0;
            let tenor_years = *tenor_bp as f64 / 10000.0;
            let expiry_date = self.base_date.add_months((expiry_years * 12.0) as i32);

            // Calculate forward swap rate
            let forward = match self.calculate_forward_swap_rate(
                expiry_date,
                tenor_years,
                base_context,
            ) {
                Ok(f) if f.is_finite() && f > 0.0 => f,
                Ok(f) => {
                    skipped_pairs.push(format!(
                        "exp={expiry_years:.2}y tenor={tenor_years:.2}y: invalid forward rate {f}"
                    ));
                    continue;
                }
                Err(e) => {
                    skipped_pairs.push(format!(
                        "exp={expiry_years:.2}y tenor={tenor_years:.2}y: forward swap rate failed: {e:?}"
                    ));
                    continue;
                }
            };

            // Extract strikes and vols
            let mut strikes: Vec<f64> = Vec::new();
            let mut vols: Vec<f64> = Vec::new();
            let mut conversion_failed = false;

            for &(strike, vol) in strikes_vols {
                strikes.push(strike);
                // Convert quoted vol to calibration convention if needed
                let converted_vol = match self.convert_volatility(
                    vol,
                    self.vol_convention, // Assume quotes are in our convention
                    self.vol_convention,
                    forward,
                    expiry_years,
                ) {
                    Ok(v) => v,
                    Err(e) => {
                        skipped_pairs.push(format!(
                            "exp={expiry_years:.2}y tenor={tenor_years:.2}y strike={strike:.6}: invalid volatility quote {vol:.6}: {e:?}"
                        ));
                        conversion_failed = true;
                        break;
                    }
                };
                vols.push(converted_vol);
            }
            if conversion_failed {
                continue;
            }

            // Handle negative rates with shift if needed
            let params = match self.vol_convention {
                SwaptionVolConvention::ShiftedLognormal { shift } => {
                    // For shifted SABR, we need to shift the forward and strikes
                    let shifted_forward = forward + shift;
                    let shifted_strikes: Vec<f64> = strikes.iter().map(|&s| s + shift).collect();

                    // Calibrate with shifted values
                    let calibrated_params = sabr_calibrator.calibrate(
                        shifted_forward,
                        &shifted_strikes,
                        &vols,
                        expiry_years,
                        self.sabr_beta,
                    );

                    // Store the shift in the parameters
                    calibrated_params.map(|mut p| {
                        p.shift = Some(shift);
                        p
                    })
                }
                _ => {
                    // Standard SABR calibration with ATM pinning (market-standard approach)
                    // This ensures ATM vol matches exactly, then fits nu/rho to smile
                    if forward > self.market_conventions.zero_threshold
                        || self.vol_convention == SwaptionVolConvention::Normal
                    {
                        sabr_calibrator.calibrate_with_atm_pinning(
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
                Err(e) => {
                    skipped_pairs.push(format!(
                        "exp={expiry_years:.2}y tenor={tenor_years:.2}y: SABR calibration failed: {e:?}"
                    ));
                    continue;
                }
            }
        }

        if sabr_params.is_empty() {
            return Err(finstack_core::Error::Calibration {
                message: "Failed to calibrate any swaption expiry-tenor combinations".to_string(),
                category: "swaption_vol_calibration".to_string(),
            });
        }

        // 4. Build volatility surface on target grid (strict: errors on any failed point)
        let vol_grid = self.build_vol_grid(&sabr_params, base_context)?;

        // 5. Create 2D surface (expiry x tenor)
        let target_expiries = &self.market_conventions.standard_expiries;
        let target_tenors = &self.market_conventions.standard_tenors;

        let surface =
            VolSurface::from_grid(&self.surface_id, target_expiries, target_tenors, &vol_grid)?;

        // 6. Create calibration report
        let mut report = CalibrationReport::for_type_with_tolerance(
            "swaption_vol",
            all_residuals,
            sabr_params.len(),
            self.market_conventions.vol_tolerance,
        )
        .with_metadata("calibrator", "SwaptionVolCalibrator")
        .with_metadata("vol_convention", format!("{:?}", self.vol_convention))
        .with_metadata("atm_convention", format!("{:?}", self.atm_convention))
        .with_metadata("num_expiry_tenor_pairs", sabr_params.len().to_string())
        .with_metadata(
            "fixed_day_count",
            format!("{:?}", self.market_conventions.fixed_day_count),
        )
        .with_metadata(
            "float_day_count",
            format!("{:?}", self.market_conventions.float_day_count),
        )
        .with_metadata(
            "fixed_bdc",
            format!("{:?}", self.market_conventions.fixed_bdc),
        )
        .with_metadata(
            "float_bdc",
            format!("{:?}", self.market_conventions.float_bdc),
        )
        .with_metadata("warnings_count", warnings.len().to_string())
        .with_metadata("skipped_pairs_count", skipped_pairs.len().to_string());

        if let Some(id) = self.calendar_id.as_deref() {
            report.update_metadata("calendar_id", id);
        }
        if !skipped_pairs.is_empty() {
            report.update_metadata(
                "skipped_pairs",
                skipped_pairs
                    .iter()
                    .take(50)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
        if !warnings.is_empty() {
            report.update_metadata(
                "warnings",
                warnings
                    .iter()
                    .take(50)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }

        Ok((surface, report))
    }
}
