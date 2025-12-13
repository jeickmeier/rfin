//! Shared instrument pricing logic for curve calibration.
//!
//! Provides [`CalibrationPricer`] for pricing rate instruments during curve
//! calibration. This centralizes pricing logic that is shared between
//! discount curve and forward curve calibrators.
//!
//! # Features
//!
//! - **Settlement conventions**: Currency-specific T+0/T+2 handling
//! - **Configurable curves**: Separate discount and forward curve IDs
//! - **OIS support**: Optional OIS-specific compounding logic
//! - **Multi-instrument**: Deposits, FRAs, futures, swaps, basis swaps
//! - **Convexity adjustments**: Configurable futures convexity parameters
//!
//! # Example
//!
//! ```ignore
//! use finstack_valuations::calibration::methods::pricing::CalibrationPricer;
//! use finstack_core::currency::Currency;
//!
//! // For discount curve calibration (uses settlement date)
//! let pricer = CalibrationPricer::new(base_date, Currency::USD, "USD-OIS")
//!     .with_use_ois_logic(true);
//!
//! // For forward curve calibration (uses base date)
//! let pricer = CalibrationPricer::new(base_date, Currency::USD, "USD-3M-FWD")
//!     .with_discount_curve_id("USD-OIS")
//!     .with_use_settlement_start(false)
//!     .with_tenor_years(0.25);  // For 3M tenor
//!
//! let residual = pricer.price_instrument(&quote, &context)?;
//! ```

use super::convexity::ConvexityParameters;
use crate::calibration::quote::{
    default_calendar_for_currency, ois_compounding_for_index, settlement_days_for_currency,
    FutureSpecs, InstrumentConventions, RatesQuote,
};
use crate::instruments::basis_swap::{BasisSwap, BasisSwapLeg};
use crate::instruments::common::traits::Instrument;
use crate::instruments::deposit::Deposit;
use crate::instruments::fra::ForwardRateAgreement;
use crate::instruments::ir_future::InterestRateFuture;
use crate::instruments::irs::{FixedLegSpec, FloatLegSpec, FloatingLegCompounding, PayReceive};
use crate::instruments::InterestRateSwap;
use finstack_core::dates::{
    adjust, BusinessDayConvention, CalendarRegistry, Date, DateExt, DayCount, DayCountCtx,
    StubKind, Tenor,
};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::prelude::*;
use finstack_core::types::{CurveId, IndexId};

use serde::{Deserialize, Serialize};

/// Instrument pricer for curve calibration.
///
/// Encapsulates the configuration and logic needed to price rate instruments
/// during curve calibration. Used by both discount and forward curve calibrators.
///
/// # Configuration
///
/// The pricer needs to know:
/// - **Curve IDs**: Which curves to use for discounting and forward projection
/// - **Settlement**: How to compute settlement dates (currency conventions)
/// - **Start date mode**: Whether to use settlement date or base date for instruments
/// - **OIS logic**: Whether to use overnight-indexed compounding
/// - **Reset lag**: Business days between fixing and period start
/// - **Tenor**: For forward curve calibration, used in basis swap curve resolution
///
/// # Discount Curve Mode
///
/// ```ignore
/// let pricer = CalibrationPricer::new(base_date, Currency::USD, "USD-OIS")
///     .with_use_ois_logic(true);  // Uses settlement date for starts
/// ```
///
/// # Forward Curve Mode
///
/// ```ignore
/// let pricer = CalibrationPricer::new(base_date, Currency::USD, "USD-3M-FWD")
///     .with_discount_curve_id("USD-OIS")
///     .with_use_settlement_start(false)  // Uses base date for starts
///     .with_tenor_years(0.25);  // 3M tenor for basis swap resolution
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalibrationPricer {
    /// Base date for pricing
    pub base_date: Date,
    /// Currency for instruments
    pub currency: Currency,
    /// Discount curve ID for pricing
    pub discount_curve_id: CurveId,
    /// Forward curve ID for floating leg projections
    pub forward_curve_id: CurveId,
    /// Optional calendar identifier for settlement calculation
    pub calendar_id: Option<String>,
    /// Settlement lag in business days (None = use currency default)
    #[serde(default)]
    pub settlement_days: Option<i32>,
    /// Payment delay in business days after period end
    #[serde(default)]
    pub payment_delay_days: i32,
    /// Reset lag in business days for floating rate fixings
    #[serde(default = "default_reset_lag")]
    pub reset_lag: i32,
    /// Use OIS-specific logic for swap pricing
    #[serde(default = "default_use_ois_logic")]
    pub use_ois_logic: bool,
    /// Allow calendar-day settlement fallback
    #[serde(default)]
    pub allow_calendar_fallback: bool,
    /// Use settlement date as instrument start (true for discount curves)
    /// When false, uses base_date directly (for forward curve calibration)
    #[serde(default = "default_use_settlement_start")]
    pub use_settlement_start: bool,
    /// Optional convexity parameters for futures pricing
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convexity_params: Option<ConvexityParameters>,
    /// Tenor in years for forward curve (used for basis swap curve resolution)
    /// e.g., 0.25 for 3M, 0.5 for 6M
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenor_years: Option<f64>,
    /// Enable verbose logging during pricing
    #[serde(default)]
    pub verbose: bool,
}

fn default_reset_lag() -> i32 {
    2
}

fn default_use_ois_logic() -> bool {
    true
}

fn default_use_settlement_start() -> bool {
    true
}

impl CalibrationPricer {
    /// Create a new calibration pricer with defaults.
    ///
    /// Default settings:
    /// - Discount and forward curves default to the provided curve_id
    /// - Settlement: Currency-specific (T+2 for USD/EUR, T+0 for GBP)
    /// - Reset lag: 2 business days
    /// - OIS logic: enabled
    /// - Use settlement start: true (for discount curve calibration)
    pub fn new(base_date: Date, currency: Currency, curve_id: impl Into<CurveId>) -> Self {
        let curve_id = curve_id.into();
        Self {
            base_date,
            currency,
            discount_curve_id: curve_id.clone(),
            forward_curve_id: curve_id,
            calendar_id: None,
            settlement_days: None,
            payment_delay_days: 0,
            reset_lag: 2,
            use_ois_logic: true,
            allow_calendar_fallback: false,
            use_settlement_start: true,
            convexity_params: None,
            tenor_years: None,
            verbose: false,
        }
    }

    /// Create a pricer configured for forward curve calibration.
    ///
    /// This sets:
    /// - `use_settlement_start = false` (uses base_date for instrument starts)
    /// - `use_ois_logic = false` (no OIS compounding for forward curves)
    /// - Tenor for basis swap resolution
    pub fn for_forward_curve(
        base_date: Date,
        currency: Currency,
        forward_curve_id: impl Into<CurveId>,
        discount_curve_id: impl Into<CurveId>,
        tenor_years: f64,
    ) -> Self {
        Self {
            base_date,
            currency,
            discount_curve_id: discount_curve_id.into(),
            forward_curve_id: forward_curve_id.into(),
            calendar_id: None,
            settlement_days: None,
            payment_delay_days: 0,
            reset_lag: 2,
            use_ois_logic: false,
            allow_calendar_fallback: false,
            use_settlement_start: false,
            convexity_params: None,
            tenor_years: Some(tenor_years),
            verbose: false,
        }
    }

    /// Set the discount curve ID.
    pub fn with_discount_curve_id(mut self, curve_id: impl Into<CurveId>) -> Self {
        self.discount_curve_id = curve_id.into();
        self
    }

    /// Set the forward curve ID.
    pub fn with_forward_curve_id(mut self, curve_id: impl Into<CurveId>) -> Self {
        self.forward_curve_id = curve_id.into();
        self
    }

    /// Set the calendar ID for settlement calculation.
    pub fn with_calendar_id(mut self, calendar_id: impl Into<String>) -> Self {
        self.calendar_id = Some(calendar_id.into());
        self
    }

    /// Set explicit settlement days (overrides currency default).
    pub fn with_settlement_days(mut self, days: i32) -> Self {
        self.settlement_days = Some(days);
        self
    }

    /// Set payment delay in business days.
    pub fn with_payment_delay(mut self, days: i32) -> Self {
        self.payment_delay_days = days;
        self
    }

    /// Set the reset lag in business days.
    pub fn with_reset_lag(mut self, days: i32) -> Self {
        self.reset_lag = days;
        self
    }

    /// Enable or disable OIS-specific swap pricing logic.
    pub fn with_use_ois_logic(mut self, use_ois: bool) -> Self {
        self.use_ois_logic = use_ois;
        self
    }

    /// Allow (or disallow) calendar-day settlement fallback.
    pub fn with_allow_calendar_fallback(mut self, allow: bool) -> Self {
        self.allow_calendar_fallback = allow;
        self
    }

    /// Set whether to use settlement date as instrument start.
    ///
    /// - `true` (default): Use settlement date (T+N) for instrument starts.
    ///   Appropriate for discount curve calibration.
    /// - `false`: Use base_date directly for instrument starts.
    ///   Appropriate for forward curve calibration.
    pub fn with_use_settlement_start(mut self, use_settlement: bool) -> Self {
        self.use_settlement_start = use_settlement;
        self
    }

    /// Set convexity parameters for futures pricing.
    pub fn with_convexity_params(mut self, params: ConvexityParameters) -> Self {
        self.convexity_params = Some(params);
        self
    }

    /// Set tenor in years for forward curve (used in basis swap resolution).
    pub fn with_tenor_years(mut self, tenor: f64) -> Self {
        self.tenor_years = Some(tenor);
        self
    }

    /// Enable or disable verbose logging.
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Get the effective start date for instruments.
    ///
    /// Returns settlement date if `use_settlement_start` is true,
    /// otherwise returns base_date.
    pub fn effective_start_date(
        &self,
        conventions: &InstrumentConventions,
    ) -> finstack_core::Result<Date> {
        if self.use_settlement_start {
            self.settlement_date_for_quote(conventions)
        } else {
            Ok(self.base_date)
        }
    }

    /// Resolve forward curve ID for basis swap legs.
    ///
    /// For forward curve calibration, determines which curve ID to use
    /// based on the index name and the calibrator's tenor.
    pub fn resolve_forward_curve_id(&self, index_name: &str) -> CurveId {
        if let Some(tenor) = self.tenor_years {
            // Use .round() to avoid float precision issues (e.g., 0.25 * 12 = 2.9999...)
            let tenor_months = (tenor * 12.0).round() as i32;
            let token = format!("{}M", tenor_months).to_ascii_uppercase();

            // Tokenize on non-alphanumerics to avoid substring traps ("13M" contains "3M")
            let normalized = index_name.to_ascii_uppercase();
            let tokens: Vec<&str> = normalized
                .split(|c: char| !c.is_ascii_alphanumeric())
                .filter(|t| !t.is_empty())
                .collect();

            let matches_tenor =
                tokens.contains(&token.as_str()) || (tenor_months == 12 && tokens.contains(&"1Y"));

            if matches_tenor {
                return self.forward_curve_id.clone();
            }
        }
        // Default: derive from index name
        format!("FWD_{}", index_name).into()
    }

    /// Get effective settlement days (explicit or currency default).
    pub fn effective_settlement_days(&self) -> i32 {
        self.settlement_days
            .unwrap_or_else(|| settlement_days_for_currency(self.currency))
    }

    // =========================================================================
    // Per-Quote Convention Resolution
    // =========================================================================

    /// Resolve effective settlement days for a specific quote.
    ///
    /// Priority: quote convention > pricer setting > currency default
    #[inline]
    fn resolve_settlement_days(&self, quote_conventions: &InstrumentConventions) -> i32 {
        quote_conventions
            .settlement_days
            .or(self.settlement_days)
            .unwrap_or_else(|| settlement_days_for_currency(self.currency))
    }

    /// Resolve effective payment delay for a specific quote.
    ///
    /// Priority: quote convention > pricer setting
    #[inline]
    fn resolve_payment_delay(&self, quote_conventions: &InstrumentConventions) -> i32 {
        quote_conventions
            .payment_delay_days
            .unwrap_or(self.payment_delay_days)
    }

    /// Resolve effective reset lag for a specific quote.
    ///
    /// Priority: quote convention > pricer setting
    #[inline]
    fn resolve_reset_lag(&self, quote_conventions: &InstrumentConventions) -> i32 {
        quote_conventions.reset_lag.unwrap_or(self.reset_lag)
    }

    /// Resolve effective calendar ID for a specific quote.
    ///
    /// Priority: quote convention > pricer setting > currency default
    #[inline]
    fn resolve_calendar_id<'a>(&'a self, quote_conventions: &'a InstrumentConventions) -> &'a str {
        quote_conventions
            .calendar_id
            .as_deref()
            .or(self.calendar_id.as_deref())
            .unwrap_or_else(|| default_calendar_for_currency(self.currency))
    }

    /// Calculate settlement date from base date using business-day calendar.
    ///
    /// Uses the configured calendar (or currency default) to properly compute
    /// the spot/settlement date by adding business days and adjusting to the
    /// next business day if needed.
    ///
    /// # Market Conventions
    ///
    /// - USD/EUR/JPY/CHF: T+2 business days
    /// - GBP: T+0 (same-day settlement)
    /// - AUD/CAD: T+1 business day
    pub fn settlement_date(&self) -> finstack_core::Result<Date> {
        self.settlement_date_for_quote(&InstrumentConventions::default())
    }

    /// Calculate settlement date for a specific quote's conventions.
    ///
    /// Uses per-quote conventions if specified, falling back to pricer defaults.
    pub fn settlement_date_for_quote(
        &self,
        quote_conventions: &InstrumentConventions,
    ) -> finstack_core::Result<Date> {
        let days = self.resolve_settlement_days(quote_conventions);
        let calendar_id = self.resolve_calendar_id(quote_conventions);

        let registry = CalendarRegistry::global();

        // If we have a valid calendar, use business-day arithmetic
        if let Some(calendar) = registry.resolve_str(calendar_id) {
            if days == 0 {
                // T+0: just ensure base_date is a business day (use consistent BDC)
                adjust(
                    self.base_date,
                    BusinessDayConvention::ModifiedFollowing,
                    calendar,
                )
            } else {
                // Add business days and adjust result
                let spot = self.base_date.add_business_days(days, calendar)?;
                // Final adjustment ensures we land on a business day
                adjust(spot, BusinessDayConvention::ModifiedFollowing, calendar)
            }
        } else if self.allow_calendar_fallback {
            // Fallback: calendar not found, use calendar-day addition with warning.
            tracing::warn!(
                calendar_id = calendar_id,
                currency = ?self.currency,
                "Calendar not found, falling back to calendar-day settlement"
            );
            Ok(if days == 0 {
                self.base_date
            } else {
                self.base_date + time::Duration::days(days as i64)
            })
        } else {
            Err(finstack_core::Error::Input(
                finstack_core::error::InputError::NotFound {
                    id: format!("calendar '{}'", calendar_id),
                },
            ))
        }
    }

    // =========================================================================
    // Per-Instrument Pricing Functions
    // =========================================================================

    /// Price a deposit quote for calibration.
    ///
    /// Uses per-quote conventions if provided, otherwise falls back to pricer defaults.
    /// Start date is determined by `use_settlement_start` setting.
    pub fn price_deposit(
        &self,
        maturity: Date,
        rate: f64,
        day_count: DayCount,
        conventions: &InstrumentConventions,
        context: &MarketContext,
    ) -> Result<f64> {
        let start = self.effective_start_date(conventions)?;
        let calendar_id = self.resolve_calendar_id(conventions);

        let dep = Deposit {
            id: format!("CALIB_DEP_{}", maturity).into(),
            notional: Money::new(1_000_000.0, self.currency),
            start,
            end: maturity,
            day_count,
            quote_rate: Some(rate),
            discount_curve_id: self.discount_curve_id.clone(),
            attributes: Default::default(),
            // Only set spot_lag_days when use_settlement_start=true; otherwise
            // rely on the explicit start date to avoid double-application
            spot_lag_days: if self.use_settlement_start {
                Some(self.resolve_settlement_days(conventions))
            } else {
                None
            },
            bdc: Some(BusinessDayConvention::ModifiedFollowing),
            calendar_id: Some(calendar_id.to_string()),
        };

        let pv = dep.value(context, self.base_date)?;
        Ok(pv.amount() / dep.notional.amount())
    }

    /// Price a FRA quote for calibration.
    ///
    /// Uses per-quote conventions if provided, otherwise falls back to pricer defaults.
    pub fn price_fra(
        &self,
        start: Date,
        end: Date,
        rate: f64,
        day_count: DayCount,
        conventions: &InstrumentConventions,
        context: &MarketContext,
    ) -> Result<f64> {
        let reset_lag = self.resolve_reset_lag(conventions);
        let calendar_id = self.resolve_calendar_id(conventions);
        let registry = CalendarRegistry::global();

        // Use business-day subtraction for fixing date calculation
        let fixing_date = if let Some(calendar) = registry.resolve_str(calendar_id) {
            // Business day subtraction using negative offset
            if start >= self.base_date {
                start.add_business_days(-reset_lag, calendar)?
            } else {
                self.base_date
            }
        } else if self.allow_calendar_fallback {
            // Fallback to calendar days if no calendar available (explicitly enabled).
            if start >= self.base_date + time::Duration::days(reset_lag as i64) {
                start - time::Duration::days(reset_lag as i64)
            } else {
                self.base_date
            }
        } else {
            return Err(finstack_core::Error::calendar_not_found_with_suggestions(
                calendar_id.to_string(),
                registry.available_ids(),
            ));
        };

        if self.verbose {
            tracing::debug!(
                fra_start = %start,
                fra_end = %end,
                fixing_date = %fixing_date,
                reset_lag = reset_lag,
                calendar = ?calendar_id,
                "FRA fixing date calculation"
            );
        }

        let fra = ForwardRateAgreement::builder()
            .id(format!("CALIB_FRA_{}_{}", start, end).into())
            .notional(Money::new(1_000_000.0, self.currency))
            .fixing_date(fixing_date)
            .start_date(start)
            .end_date(end)
            .fixed_rate(rate)
            .day_count(day_count)
            .reset_lag(reset_lag)
            .fixing_calendar_id_opt(Some(calendar_id.to_string()))
            .discount_curve_id(self.discount_curve_id.clone())
            .forward_id(self.forward_curve_id.clone())
            .build()
            .map_err(|_| finstack_core::Error::Internal)?;

        let pv = fra.value(context, self.base_date)?;
        Ok(pv.amount() / fra.notional.amount())
    }

    /// Price a futures quote for calibration.
    ///
    /// Uses per-quote conventions if provided, otherwise falls back to pricer defaults.
    /// Convexity adjustment priority:
    /// 1. Quote-level override (specs.convexity_adjustment)
    /// 2. Pricer-level custom params (self.convexity_params)
    /// 3. Currency-specific defaults
    pub fn price_future(
        &self,
        expiry: Date,
        price: f64,
        specs: &FutureSpecs,
        _conventions: &InstrumentConventions,
        context: &MarketContext,
    ) -> Result<f64> {
        let period_start = expiry;
        let period_end = expiry.add_months(specs.delivery_months as i32);
        let fixing_date = expiry; // Typically same as expiry for futures

        // Calculate convexity adjustment using priority:
        // 1. Quote-level override
        // 2. Pricer-level custom params
        // 3. Currency-specific defaults
        let convexity_adj = if let Some(adj) = specs.convexity_adjustment {
            Some(adj)
        } else {
            let params = self
                .convexity_params
                .clone()
                .unwrap_or_else(|| ConvexityParameters::for_currency(self.currency));

            let adj =
                params.calculate_for_future(self.base_date, expiry, period_end, specs.day_count);

            if self.verbose {
                let dc_ctx = DayCountCtx::default();
                let time_to_expiry = specs
                    .day_count
                    .year_fraction(self.base_date, expiry, dc_ctx)
                    .unwrap_or(0.0);
                let time_to_maturity = specs
                    .day_count
                    .year_fraction(self.base_date, period_end, dc_ctx)
                    .unwrap_or(0.0);
                tracing::debug!(
                    future_expiry = %expiry,
                    time_to_expiry = time_to_expiry,
                    time_to_maturity = time_to_maturity,
                    convexity_adjustment = adj,
                    "Futures convexity adjustment"
                );
            }

            Some(adj)
        };

        let future = InterestRateFuture::builder()
            .id(format!("CALIB_FUT_{}", expiry).into())
            .notional(Money::new(specs.face_value, self.currency))
            .expiry_date(expiry)
            .fixing_date(fixing_date)
            .period_start(period_start)
            .period_end(period_end)
            .quoted_price(price)
            .day_count(specs.day_count)
            .position(crate::instruments::ir_future::Position::Long)
            .contract_specs(crate::instruments::ir_future::FutureContractSpecs {
                face_value: specs.face_value,
                tick_size: specs.tick_size,
                tick_value: specs.tick_value,
                delivery_months: specs.delivery_months,
                convexity_adjustment: convexity_adj,
            })
            .discount_curve_id(self.discount_curve_id.clone())
            .forward_id(self.forward_curve_id.clone())
            .build()
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!("IRFuture builder failed for expiry {}: {}", expiry, e),
                category: "yield_curve_bootstrap".to_string(),
            })?;

        let pv = future.value(context, self.base_date)?;
        Ok(pv.amount() / future.notional.amount())
    }

    /// Price a swap quote for calibration.
    ///
    /// Uses per-quote conventions if provided, otherwise falls back to pricer defaults.
    /// Start date is determined by `use_settlement_start` setting.
    #[allow(clippy::too_many_arguments)]
    pub fn price_swap(
        &self,
        maturity: Date,
        rate: f64,
        fixed_freq: Tenor,
        float_freq: Tenor,
        fixed_dc: DayCount,
        float_dc: DayCount,
        index: &IndexId,
        is_ois_quote: bool,
        conventions: &InstrumentConventions,
        context: &MarketContext,
    ) -> Result<f64> {
        let start = self.effective_start_date(conventions)?;
        let payment_delay = self.resolve_payment_delay(conventions);
        let reset_lag = self.resolve_reset_lag(conventions);
        let calendar_id = self.resolve_calendar_id(conventions).to_string();

        let fixed_spec = FixedLegSpec {
            discount_curve_id: self.discount_curve_id.clone(),
            rate,
            freq: fixed_freq,
            dc: fixed_dc,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some(calendar_id.clone()),
            stub: StubKind::None,
            par_method: None,
            compounding_simple: true,
            start,
            end: maturity,
            payment_delay_days: payment_delay,
        };

        // Determine floating leg curve IDs and compounding based on:
        // 1. Whether OIS logic is enabled for this pricer
        // 2. Whether the quote itself is OIS-suitable (overnight index)
        let use_ois_pricing = self.use_ois_logic && is_ois_quote;

        let (float_discount_id, float_forward_id, compounding) = if use_ois_pricing {
            // OIS pricing: use same curve for discount and forward, with OIS compounding
            (
                self.discount_curve_id.clone(),
                self.discount_curve_id.clone(),
                ois_compounding_for_index(index, self.currency),
            )
        } else {
            // Standard pricing: separate discount and forward curves, simple compounding
            (
                self.discount_curve_id.clone(),
                self.forward_curve_id.clone(),
                FloatingLegCompounding::Simple,
            )
        };

        let float_spec = FloatLegSpec {
            discount_curve_id: float_discount_id,
            forward_curve_id: float_forward_id,
            spread_bp: 0.0,
            freq: float_freq,
            dc: float_dc,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some(calendar_id.clone()),
            fixing_calendar_id: Some(calendar_id),
            stub: StubKind::None,
            reset_lag_days: reset_lag,
            start,
            end: maturity,
            compounding,
            payment_delay_days: payment_delay,
        };

        let swap = InterestRateSwap {
            id: format!("CALIB_SWAP_{}", maturity).into(),
            notional: Money::new(1_000_000.0, self.currency),
            side: PayReceive::ReceiveFixed,
            fixed: fixed_spec,
            float: float_spec,
            margin_spec: None,
            attributes: Default::default(),
        };

        let pv = swap.value(context, self.base_date)?;
        Ok(pv.amount() / swap.notional.amount())
    }

    /// Price a basis swap quote for calibration.
    ///
    /// Uses per-quote conventions if provided, otherwise falls back to pricer defaults.
    /// For forward curve calibration, uses `resolve_forward_curve_id` to determine
    /// which leg should use the curve being calibrated.
    #[allow(clippy::too_many_arguments)]
    pub fn price_basis_swap(
        &self,
        maturity: Date,
        primary_index: &str,
        reference_index: &str,
        spread_bp: f64,
        primary_freq: Tenor,
        reference_freq: Tenor,
        primary_dc: DayCount,
        reference_dc: DayCount,
        currency: Currency,
        conventions: &InstrumentConventions,
        context: &MarketContext,
    ) -> Result<f64> {
        let start = self.effective_start_date(conventions)?;
        let reset_lag = self.resolve_reset_lag(conventions);
        let payment_delay = self.resolve_payment_delay(conventions);
        let calendar_id = self.resolve_calendar_id(conventions);

        // Use resolve_forward_curve_id which handles tenor matching for forward curve calibration
        let primary_forward_id = self.resolve_forward_curve_id(primary_index);
        let reference_forward_id = self.resolve_forward_curve_id(reference_index);

        let primary_leg = BasisSwapLeg {
            forward_curve_id: primary_forward_id.clone(),
            frequency: primary_freq,
            day_count: primary_dc,
            bdc: BusinessDayConvention::ModifiedFollowing,
            payment_lag_days: payment_delay,
            reset_lag_days: reset_lag,
            spread: spread_bp / 10_000.0,
        };

        let reference_leg = BasisSwapLeg {
            forward_curve_id: reference_forward_id.clone(),
            frequency: reference_freq,
            day_count: reference_dc,
            bdc: BusinessDayConvention::ModifiedFollowing,
            payment_lag_days: payment_delay,
            reset_lag_days: reset_lag,
            spread: 0.0,
        };

        let basis_swap = BasisSwap::new(
            format!("CALIB_BASIS_{}", maturity),
            Money::new(1_000_000.0, currency),
            start,
            maturity,
            primary_leg,
            reference_leg,
            self.discount_curve_id.as_str(),
        )
        .with_allow_calendar_fallback(self.allow_calendar_fallback)
        .with_calendar(calendar_id.to_string());

        // For forward curve calibration, one of the curves is being calibrated
        // and may not be in the context yet - that's expected
        // For discount curve calibration, both forward curves must exist
        if self.use_settlement_start {
            // Discount curve mode: both forward curves must exist
            if context
                .get_forward_ref(primary_forward_id.as_str())
                .is_err()
                || context
                    .get_forward_ref(reference_forward_id.as_str())
                    .is_err()
            {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::NotFound {
                        id: "forward curves".to_string(),
                    },
                ));
            }
        }

        let pv = basis_swap.value(context, self.base_date)?;
        Ok(pv.amount() / basis_swap.notional.amount())
    }

    // =========================================================================
    // Instrument Construction (for testing/repricing)
    // =========================================================================

    /// Create an OIS swap from a quote, matching calibration instrument construction.
    ///
    /// This method creates a swap instrument exactly as constructed during calibration,
    /// ensuring consistent pricing for test repricing and verification.
    ///
    /// Uses the pricer's configuration for curve IDs, calendar, reset lag, and payment delay.
    ///
    /// # Arguments
    ///
    /// * `quote` - A `RatesQuote::Swap` variant
    /// * `notional` - Notional amount for the swap
    ///
    /// # Returns
    ///
    /// An `InterestRateSwap` configured identically to calibration instruments.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let pricer = CalibrationPricer::new(base_date, Currency::USD, "USD-OIS")
    ///     .with_payment_delay(0);
    ///
    /// let swap = pricer.create_ois_swap(&quote, Money::new(1_000_000.0, Currency::USD))?;
    /// ```
    pub fn create_ois_swap(&self, quote: &RatesQuote, notional: Money) -> Result<InterestRateSwap> {
        let (maturity, rate, fixed_freq, float_freq, fixed_dc, float_dc, index, conventions) =
            match quote {
                RatesQuote::Swap {
                    maturity,
                    rate,
                    fixed_freq,
                    float_freq,
                    fixed_dc,
                    float_dc,
                    index,
                    conventions,
                } => (
                    *maturity,
                    *rate,
                    *fixed_freq,
                    *float_freq,
                    *fixed_dc,
                    *float_dc,
                    index,
                    conventions,
                ),
                _ => {
                    return Err(finstack_core::Error::Input(
                        finstack_core::error::InputError::Invalid,
                    ))
                }
            };

        let start = self.effective_start_date(conventions)?;
        let payment_delay = self.resolve_payment_delay(conventions);
        let reset_lag = self.resolve_reset_lag(conventions);
        let calendar_id = self.resolve_calendar_id(conventions).to_string();

        // Determine compounding based on OIS logic setting
        let compounding = if self.use_ois_logic {
            ois_compounding_for_index(index, self.currency)
        } else {
            FloatingLegCompounding::Simple
        };

        let fixed_spec = FixedLegSpec {
            discount_curve_id: self.discount_curve_id.clone(),
            rate,
            freq: fixed_freq,
            dc: fixed_dc,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some(calendar_id.clone()),
            stub: StubKind::None,
            par_method: None,
            compounding_simple: true,
            start,
            end: maturity,
            payment_delay_days: payment_delay,
        };

        let float_spec = FloatLegSpec {
            discount_curve_id: self.discount_curve_id.clone(),
            forward_curve_id: self.forward_curve_id.clone(),
            spread_bp: 0.0,
            freq: float_freq,
            dc: float_dc,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some(calendar_id.clone()),
            fixing_calendar_id: Some(calendar_id),
            stub: StubKind::None,
            reset_lag_days: reset_lag,
            start,
            end: maturity,
            compounding,
            payment_delay_days: payment_delay,
        };

        InterestRateSwap::builder()
            .id(format!("SWAP-{}", maturity).into())
            .notional(notional)
            .side(PayReceive::ReceiveFixed)
            .fixed(fixed_spec)
            .float(float_spec)
            .build()
    }

    // =========================================================================
    // Main Instrument Pricing Dispatch
    // =========================================================================

    /// Price an instrument using the given market context.
    ///
    /// Returns the pricing error (PV for par instruments) that should be zero
    /// when the curve is correctly calibrated.
    ///
    /// Uses per-instrument conventions if specified on the quote, otherwise
    /// falls back to pricer defaults and currency conventions.
    ///
    /// # Settlement Handling
    ///
    /// Deposits use currency-specific settlement dates unless overridden:
    /// - USD/EUR/JPY/CHF: T+2
    /// - GBP: T+0 (same-day settlement)
    /// - AUD/CAD: T+1
    pub fn price_instrument(&self, quote: &RatesQuote, context: &MarketContext) -> Result<f64> {
        match quote {
            RatesQuote::Deposit {
                maturity,
                rate,
                day_count,
                conventions,
            } => self.price_deposit(*maturity, *rate, *day_count, conventions, context),

            RatesQuote::FRA {
                start,
                end,
                rate,
                day_count,
                conventions,
            } => self.price_fra(*start, *end, *rate, *day_count, conventions, context),

            RatesQuote::Future {
                expiry,
                price,
                specs,
                conventions,
            } => self.price_future(*expiry, *price, specs, conventions, context),

            RatesQuote::Swap {
                maturity,
                rate,
                fixed_freq,
                float_freq,
                fixed_dc,
                float_dc,
                index,
                conventions,
            } => {
                let is_ois = quote.is_ois_suitable();
                self.price_swap(
                    *maturity,
                    *rate,
                    *fixed_freq,
                    *float_freq,
                    *fixed_dc,
                    *float_dc,
                    index,
                    is_ois,
                    conventions,
                    context,
                )
            }

            RatesQuote::BasisSwap {
                maturity,
                primary_index,
                reference_index,
                spread_bp,
                primary_freq,
                reference_freq,
                primary_dc,
                reference_dc,
                currency,
                conventions,
            } => self.price_basis_swap(
                *maturity,
                primary_index,
                reference_index,
                *spread_bp,
                *primary_freq,
                *reference_freq,
                *primary_dc,
                *reference_dc,
                *currency,
                conventions,
                context,
            ),
        }
    }

    // =========================================================================
    // Quote Validation
    // =========================================================================

    /// Extract rate from quote.
    ///
    /// Converts different quote types to their effective rate representation:
    /// - Deposits/FRAs/Swaps: direct rate
    /// - Futures: price converted to rate (100 - price) / 100
    /// - Basis swaps: spread in bp converted to decimal
    pub fn get_rate(quote: &RatesQuote) -> f64 {
        match quote {
            RatesQuote::Deposit { rate, .. } => *rate,
            RatesQuote::FRA { rate, .. } => *rate,
            RatesQuote::Future { price, .. } => (100.0 - price) / 100.0,
            RatesQuote::Swap { rate, .. } => *rate,
            RatesQuote::BasisSwap { spread_bp, .. } => *spread_bp / 10_000.0,
        }
    }

    /// Validate quote sequence for no-arbitrage and completeness.
    ///
    /// Performs basic validation:
    /// - Non-empty quote list
    /// - No duplicate maturities
    /// - Rates within configured bounds
    ///
    /// # Arguments
    ///
    /// * `quotes` - The quote sequence to validate
    /// * `rate_bounds` - Min/max rate bounds for sanity checking
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails.
    pub fn validate_quotes(
        quotes: &[RatesQuote],
        rate_bounds: &crate::calibration::config::RateBounds,
    ) -> Result<()> {
        if quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        // Check for duplicate (quote_type, maturity) combinations
        // This allows different instrument types with the same maturity
        let mut seen = std::collections::HashSet::new();
        for quote in quotes {
            let key = (quote.get_type(), quote.maturity_date());
            if !seen.insert(key) {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }
        }

        // Check rates are reasonable (basic sanity check)
        for quote in quotes {
            let rate = Self::get_rate(quote);
            if !rate.is_finite() || !rate_bounds.contains(rate) {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }
        }

        Ok(())
    }

    /// Pre-validate that all required curves exist for the quote set.
    ///
    /// Fails fast with a clear error if dependencies are missing, rather than
    /// returning PENALTY values during calibration.
    ///
    /// # Arguments
    ///
    /// * `quotes` - The quote sequence to validate
    /// * `context` - Market context to check for required curves
    ///
    /// # Errors
    ///
    /// Returns an error if required forward curves are missing for basis swaps.
    pub fn validate_curve_dependencies(
        &self,
        quotes: &[RatesQuote],
        context: &MarketContext,
    ) -> Result<()> {
        for quote in quotes {
            if let RatesQuote::BasisSwap {
                primary_index,
                reference_index,
                ..
            } = quote
            {
                // Use resolver for consistent curve ID derivation
                let primary_fwd = self.resolve_forward_curve_id(primary_index);
                let ref_fwd = self.resolve_forward_curve_id(reference_index);

                if context.get_forward_ref(primary_fwd.as_str()).is_err() {
                    return Err(finstack_core::Error::Input(
                        finstack_core::error::InputError::NotFound {
                            id: format!(
                                "Forward curve '{}' required for basis swap calibration. \
                                 Please calibrate the forward curve first.",
                                primary_fwd
                            ),
                        },
                    ));
                }
                if context.get_forward_ref(ref_fwd.as_str()).is_err() {
                    return Err(finstack_core::Error::Input(
                        finstack_core::error::InputError::NotFound {
                            id: format!(
                                "Forward curve '{}' required for basis swap calibration. \
                                 Please calibrate the forward curve first.",
                                ref_fwd
                            ),
                        },
                    ));
                }
            }
        }
        Ok(())
    }

    /// Validate quote suitability for discount curve calibration.
    ///
    /// Checks that the quote set is appropriate for discount curve calibration
    /// under multi-curve framework principles. Warns (or errors in strict mode)
    /// when forward-dependent instruments are used without OIS-suitable quotes.
    ///
    /// ## Multi-Curve Framework Guidance
    ///
    /// **Appropriate for discount curve calibration:**
    /// - OIS swaps (e.g., SOFR, ESTR, SONIA): overnight compounded, collateral-aligned
    /// - Deposits: short-end risk-free rates
    ///
    /// **Not recommended for discount curves (use dedicated forward curve calibration):**
    /// - FRAs: reference LIBOR/term rates, require forward curve for pricing
    /// - Futures: reference term rates, convexity-adjusted
    /// - Tenor swaps (3M, 6M LIBOR-based): require forward curves per tenor
    /// - Basis swaps: used for cross-tenor calibration, not discount
    ///
    /// # Arguments
    ///
    /// * `quotes` - The quote sequence to validate
    /// * `enforce_separation` - If true, error on invalid mix; if false, warn only
    ///
    /// # Errors
    ///
    /// Returns an error if `enforce_separation` is true and quotes are unsuitable.
    pub fn validate_discount_curve_quotes(
        quotes: &[RatesQuote],
        enforce_separation: bool,
    ) -> Result<()> {
        let mut has_forward_dependent = false;
        let mut has_ois_suitable = false;

        for quote in quotes {
            if quote.requires_forward_curve() {
                has_forward_dependent = true;
            }
            if quote.is_ois_suitable() {
                has_ois_suitable = true;
            }
        }

        // Enforce separation if configured: do not allow forward-dependent instruments for discount curve
        if has_forward_dependent && !has_ois_suitable {
            if enforce_separation {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            } else {
                tracing::warn!(
                    "Discount curve calibration using forward-dependent instruments (FRA, Future, non-OIS Swap). \
                     Best practice: use OIS swaps (SOFR/ESTR/SONIA) or deposits for discount curves, \
                     and calibrate forward curves separately."
                );
            }
        }

        Ok(())
    }
}

// =============================================================================
// Standalone Helper (backward compatibility)
// =============================================================================

/// Creates an OIS swap from a rates quote with specified curve IDs.
///
/// This is a convenience wrapper around [`CalibrationPricer::create_ois_swap`]
/// for backward compatibility. For new code, prefer using the pricer directly.
///
/// # Arguments
///
/// * `quote` - The swap quote containing rate and frequency parameters
/// * `discount_curve_id` - Curve ID for discounting (e.g., "USD-OIS")
/// * `forward_curve_id` - Curve ID for forward projection (same as discount for OIS)
/// * `base_date` - Start date of the swap
/// * `notional` - Notional amount
/// * `calendar_id` - Optional calendar for schedule generation
/// * `payment_delay_days` - Payment delay in business days after period end
///
/// # Returns
///
/// An `InterestRateSwap` configured identically to calibration instruments.
///
/// # Example
///
/// ```ignore
/// use finstack_valuations::calibration::methods::pricing::create_ois_swap_from_quote;
///
/// let swap = create_ois_swap_from_quote(
///     &quote,
///     "USD-OIS",
///     "USD-OIS",
///     base_date,
///     Money::new(1_000_000.0, Currency::USD),
///     None,
///     0, // payment_delay_days
/// )?;
/// ```
///
/// # See Also
///
/// - [`CalibrationPricer::create_ois_swap`] for the preferred API with full configuration
pub fn create_ois_swap_from_quote(
    quote: &RatesQuote,
    discount_curve_id: &str,
    forward_curve_id: &str,
    base_date: Date,
    notional: Money,
    calendar_id: Option<&str>,
    payment_delay_days: i32,
) -> Result<InterestRateSwap> {
    let currency = notional.currency();
    let mut pricer = CalibrationPricer::new(base_date, currency, discount_curve_id)
        .with_forward_curve_id(forward_curve_id)
        .with_payment_delay(payment_delay_days)
        .with_use_settlement_start(false) // Use base_date as start (legacy behavior)
        .with_use_ois_logic(true); // Enable OIS compounding

    if let Some(cal) = calendar_id {
        pricer = pricer.with_calendar_id(cal);
    }

    pricer.create_ois_swap(quote, notional)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pricer_builder() {
        let base_date =
            Date::from_calendar_date(2024, time::Month::January, 15).expect("valid date");

        let pricer = CalibrationPricer::new(base_date, Currency::USD, "USD-OIS")
            .with_discount_curve_id("USD-DISC")
            .with_forward_curve_id("USD-3M")
            .with_reset_lag(0)
            .with_use_ois_logic(false);

        assert_eq!(pricer.discount_curve_id.as_str(), "USD-DISC");
        assert_eq!(pricer.forward_curve_id.as_str(), "USD-3M");
        assert_eq!(pricer.reset_lag, 0);
        assert!(!pricer.use_ois_logic);
    }

    #[test]
    fn test_effective_settlement_days() {
        let base_date =
            Date::from_calendar_date(2024, time::Month::January, 15).expect("valid date");

        // USD defaults to T+2
        let usd_pricer = CalibrationPricer::new(base_date, Currency::USD, "USD-OIS");
        assert_eq!(usd_pricer.effective_settlement_days(), 2);

        // GBP defaults to T+0
        let gbp_pricer = CalibrationPricer::new(base_date, Currency::GBP, "GBP-SONIA");
        assert_eq!(gbp_pricer.effective_settlement_days(), 0);

        // Explicit override
        let custom_pricer =
            CalibrationPricer::new(base_date, Currency::USD, "USD-OIS").with_settlement_days(1);
        assert_eq!(custom_pricer.effective_settlement_days(), 1);
    }

    // =========================================================================
    // Tests for market-standards fixes
    // =========================================================================

    #[test]
    fn test_forward_mode_uses_base_date_start() {
        // Finding 1: use_settlement_start=false should result in base_date as start
        let base_date =
            Date::from_calendar_date(2024, time::Month::January, 15).expect("valid date");

        let pricer = CalibrationPricer::for_forward_curve(
            base_date,
            Currency::USD,
            "USD-3M-FWD",
            "USD-OIS",
            0.25,
        );

        // Forward mode should use base_date directly
        assert!(!pricer.use_settlement_start);
        let start = pricer
            .effective_start_date(&InstrumentConventions::default())
            .expect("should succeed");
        assert_eq!(
            start, base_date,
            "Forward mode should use base_date as start"
        );
    }

    #[test]
    fn test_forward_curve_routing_uses_round() {
        // Finding 4: Tenor routing should use round() and suffix matching
        let base_date =
            Date::from_calendar_date(2024, time::Month::January, 15).expect("valid date");

        // 3M tenor (0.25 years)
        let pricer = CalibrationPricer::for_forward_curve(
            base_date,
            Currency::USD,
            "USD-3M-FWD",
            "USD-OIS",
            0.25,
        );

        // Should match indices ending with "3M" or containing "-3M"
        assert_eq!(
            pricer.resolve_forward_curve_id("USD-SOFR-3M").as_str(),
            "USD-3M-FWD",
            "Should route 3M index to forward curve"
        );
        assert_eq!(
            pricer.resolve_forward_curve_id("3M").as_str(),
            "USD-3M-FWD",
            "Should route bare 3M suffix to forward curve"
        );

        // Should NOT match different tenors
        assert_ne!(
            pricer.resolve_forward_curve_id("USD-SOFR-6M").as_str(),
            "USD-3M-FWD",
            "Should NOT route 6M index to 3M forward curve"
        );
    }

    #[test]
    fn test_duplicate_maturity_allows_different_types() {
        // Finding 6: Different quote types with same maturity should be allowed
        use crate::calibration::config::RateBounds;

        let maturity =
            Date::from_calendar_date(2025, time::Month::January, 15).expect("valid date");

        let quotes = vec![
            RatesQuote::Deposit {
                maturity,
                rate: 0.04,
                day_count: DayCount::Act360,
                conventions: InstrumentConventions::default(),
            },
            RatesQuote::Swap {
                maturity,
                rate: 0.045,
                fixed_freq: finstack_core::dates::Tenor::new(
                    6,
                    finstack_core::dates::TenorUnit::Months,
                ), // Semi-annual
                float_freq: finstack_core::dates::Tenor::new(
                    3,
                    finstack_core::dates::TenorUnit::Months,
                ), // Quarterly
                fixed_dc: DayCount::Thirty360,
                float_dc: DayCount::Act360,
                index: "SOFR".into(),
                conventions: InstrumentConventions::default(),
            },
        ];

        let bounds = RateBounds::default();
        let result = CalibrationPricer::validate_quotes(&quotes, &bounds);
        assert!(
            result.is_ok(),
            "Different quote types with same maturity should be valid"
        );
    }

    #[test]
    fn test_duplicate_maturity_rejects_same_type() {
        // Finding 6: Same quote type with same maturity should be rejected
        use crate::calibration::config::RateBounds;

        let maturity =
            Date::from_calendar_date(2025, time::Month::January, 15).expect("valid date");

        let quotes = vec![
            RatesQuote::Deposit {
                maturity,
                rate: 0.04,
                day_count: DayCount::Act360,
                conventions: InstrumentConventions::default(),
            },
            RatesQuote::Deposit {
                maturity,
                rate: 0.041,
                day_count: DayCount::Act360,
                conventions: InstrumentConventions::default(),
            },
        ];

        let bounds = RateBounds::default();
        let result = CalibrationPricer::validate_quotes(&quotes, &bounds);
        assert!(
            result.is_err(),
            "Same quote type with same maturity should be invalid"
        );
    }

    #[test]
    fn test_future_specs_tick_conventions() {
        // Finding 7: FutureSpecs should use configurable tick values
        let specs = FutureSpecs::default();
        assert!(
            (specs.tick_size - 0.0025).abs() < 1e-10,
            "Default tick size should be 0.0025"
        );
        assert!(
            (specs.tick_value - 6.25).abs() < 1e-10,
            "Default tick value should be 6.25"
        );

        // Custom specs should work
        let custom_specs = FutureSpecs {
            tick_size: 0.005,
            tick_value: 12.5,
            ..FutureSpecs::default()
        };
        assert!((custom_specs.tick_size - 0.005).abs() < 1e-10);
        assert!((custom_specs.tick_value - 12.5).abs() < 1e-10);
    }
}
