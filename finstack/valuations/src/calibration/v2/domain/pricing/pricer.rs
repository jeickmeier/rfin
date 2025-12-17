//! Shared instrument pricing logic for curve calibration.
//!
//! Note: Copied from v1 for parallel implementation.

use super::convexity::ConvexityParameters;
use super::conventions as conv;
use super::super::quotes::{FutureSpecs, InstrumentConventions, RatesQuote};
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

// =============================================================================
// Default Convention Helpers
// =============================================================================

/// Get the OIS compounding method for a rate index.
fn ois_compounding_for_index(index: &IndexId, currency: Currency) -> FloatingLegCompounding {
    let upper = index.as_str().to_ascii_uppercase();

    // Index-name driven overrides
    if upper.contains("SONIA") {
        return FloatingLegCompounding::sonia();
    }
    if upper.contains("ESTR") || upper.contains("€STR") {
        return FloatingLegCompounding::estr();
    }
    if upper.contains("TONA") || upper.contains("TONAR") {
        return FloatingLegCompounding::tona();
    }
    if upper.contains("SOFR") {
        return FloatingLegCompounding::sofr();
    }

    // Currency fallback for generic ids like "USD-OIS"
    match currency {
        Currency::GBP => FloatingLegCompounding::sonia(),
        Currency::EUR => FloatingLegCompounding::estr(),
        Currency::JPY => FloatingLegCompounding::tona(),
        _ => FloatingLegCompounding::sofr(),
    }
}

// =============================================================================
// Quote Validation Types
// =============================================================================

/// Specifies the intended use case for rate quote validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RatesQuoteUseCase {
    /// Validation for discount curve calibration.
    DiscountCurve {
        /// If true, error on forward-dependent instruments; if false, warn only.
        enforce_separation: bool,
    },
    /// Validation for forward curve calibration.
    ForwardCurve,
}

/// Instrument pricer for curve calibration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalibrationPricer {
    /// Base date for pricing
    pub base_date: Date,
    /// Discount curve ID for pricing
    pub discount_curve_id: CurveId,
    /// Forward curve ID for floating leg projections
    pub forward_curve_id: CurveId,
    /// Settlement lag in business days (None = use quote convention or currency default)
    #[serde(default)]
    pub settlement_days: Option<i32>,
    /// Schedule/calendar identifier for settlement and date adjustments.
    ///
    /// If `None`, pricing will use the quote convention (if provided) or the
    /// market default for the calibration currency.
    #[serde(default)]
    pub calendar_id: Option<String>,
    /// Business day convention for settlement and schedule date adjustments.
    ///
    /// If `None`, pricing will use the quote convention (if provided) or the
    /// market default for the calibration currency.
    #[serde(default)]
    pub business_day_convention: Option<BusinessDayConvention>,
    /// Allow calendar-day settlement fallback
    #[serde(default)]
    pub allow_calendar_fallback: bool,
    /// Use settlement date as instrument start (true for discount curves)
    #[serde(default = "default_use_settlement_start")]
    pub use_settlement_start: bool,
    /// Optional convexity parameters for futures pricing
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convexity_params: Option<ConvexityParameters>,
    /// Tenor in years for forward curve (used for basis swap curve resolution)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenor_years: Option<f64>,
    /// Enable verbose logging during pricing
    #[serde(default)]
    pub verbose: bool,
}

fn default_use_settlement_start() -> bool {
    true
}

impl CalibrationPricer {
    /// Market-standard calendar identifier for rates by currency.
    ///
    /// These identifiers must exist in `CalendarRegistry`.
    pub fn market_calendar_id(currency: Currency) -> &'static str {
        match currency {
            Currency::USD => "usny",
            Currency::EUR => "target2",
            Currency::GBP => "gblo",
            Currency::JPY => "jpto",
            Currency::CHF => "chzu",
            Currency::AUD => "ausy",
            Currency::CAD => "cato",
            Currency::NZD => "nzau",
            Currency::HKD => "hkex",
            Currency::SGD => "sgex",
            _ => "usny",
        }
    }

    /// Market-standard spot settlement lag (business days) by currency.
    pub fn market_settlement_days(currency: Currency) -> i32 {
        match currency {
            Currency::GBP => 0,
            Currency::AUD | Currency::CAD => 1,
            _ => 2,
        }
    }

    /// Market-standard business day convention for rates scheduling.
    pub fn market_business_day_convention(_currency: Currency) -> BusinessDayConvention {
        BusinessDayConvention::ModifiedFollowing
    }

    /// Create a new calibration pricer with defaults.
    pub fn new(base_date: Date, curve_id: impl Into<CurveId>) -> Self {
        let curve_id = curve_id.into();
        Self {
            base_date,
            discount_curve_id: curve_id.clone(),
            forward_curve_id: curve_id,
            settlement_days: None,
            calendar_id: None,
            business_day_convention: None,
            allow_calendar_fallback: false,
            use_settlement_start: true,
            convexity_params: None,
            tenor_years: None,
            verbose: false,
        }
    }

    /// Resolve settlement date using strictly provided quote conventions (no defaults).
    pub fn settlement_date_for_quote_strict(
        &self,
        quote_conventions: &InstrumentConventions,
        currency: Currency,
    ) -> finstack_core::Result<Date> {
        let settled = conv::resolve_settlement_strict(quote_conventions, currency)?;
        let days = settled.settlement_days;
        let calendar_id = settled.calendar_id;

        let registry = CalendarRegistry::global();
        if let Some(calendar) = registry.resolve_str(calendar_id) {
            if days == 0 {
                adjust(self.base_date, settled.bdc, calendar)
            } else {
                let spot = self.base_date.add_business_days(days, calendar)?;
                adjust(spot, settled.bdc, calendar)
            }
        } else {
            Err(finstack_core::Error::Input(
                finstack_core::error::InputError::NotFound {
                    id: format!("calendar '{}'", calendar_id),
                },
            ))
        }
    }


    /// Create a pricer configured for forward curve calibration.
    pub fn for_forward_curve(
        base_date: Date,
        forward_curve_id: impl Into<CurveId>,
        discount_curve_id: impl Into<CurveId>,
        tenor_years: f64,
    ) -> Self {
        Self {
            base_date,
            discount_curve_id: discount_curve_id.into(),
            forward_curve_id: forward_curve_id.into(),
            settlement_days: None,
            calendar_id: None,
            business_day_convention: None,
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

    /// Set explicit settlement days (overrides quote convention and currency default).
    pub fn with_settlement_days(mut self, days: i32) -> Self {
        self.settlement_days = Some(days);
        self
    }

    /// Set the calendar identifier used for settlement/schedule generation.
    pub fn with_calendar_id(mut self, calendar_id: impl Into<String>) -> Self {
        self.calendar_id = Some(calendar_id.into());
        self
    }

    /// Set the business day convention used for settlement/schedule generation.
    pub fn with_business_day_convention(mut self, bdc: BusinessDayConvention) -> Self {
        self.business_day_convention = Some(bdc);
        self
    }

    /// Populate missing pricer-level conventions using market defaults for the given currency.
    ///
    /// Quote-level conventions still take precedence at pricing time.
    pub fn with_market_conventions(mut self, currency: Currency) -> Self {
        if self.settlement_days.is_none() {
            self.settlement_days = Some(Self::market_settlement_days(currency));
        }
        if self.calendar_id.is_none() {
            self.calendar_id = Some(Self::market_calendar_id(currency).to_string());
        }
        if self.business_day_convention.is_none() {
            self.business_day_convention = Some(Self::market_business_day_convention(currency));
        }
        self
    }

    /// Allow (or disallow) calendar-day settlement fallback.
    pub fn with_allow_calendar_fallback(mut self, allow: bool) -> Self {
        self.allow_calendar_fallback = allow;
        self
    }

    /// Set whether to use settlement date as instrument start.
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
    pub fn effective_start_date(
        &self,
        conventions: &InstrumentConventions,
        currency: Currency,
    ) -> finstack_core::Result<Date> {
        if self.use_settlement_start {
            self.settlement_date_for_quote(conventions, currency)
        } else {
            Ok(self.base_date)
        }
    }

    /// Resolve forward curve ID for basis swap legs.
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

    /// Calculate settlement date from base date using business-day calendar.
    pub fn settlement_date(&self, currency: Currency) -> finstack_core::Result<Date> {
        self.settlement_date_for_quote(&InstrumentConventions::default(), currency)
    }

    /// Calculate settlement date for a specific quote's conventions.
    pub fn settlement_date_for_quote(
        &self,
        quote_conventions: &InstrumentConventions,
        currency: Currency,
    ) -> finstack_core::Result<Date> {
        let common = conv::resolve_common(self, quote_conventions, currency);
        let days = common.settlement_days;
        let calendar_id = common.calendar_id;
        let bdc = common.bdc;

        let registry = CalendarRegistry::global();

        // If we have a valid calendar, use business-day arithmetic
        if let Some(calendar) = registry.resolve_str(calendar_id) {
            if days == 0 {
                // T+0: just ensure base_date is a business day (use consistent BDC)
                adjust(self.base_date, bdc, calendar)
            } else {
                // Add business days and adjust result
                let spot = self.base_date.add_business_days(days, calendar)?;
                // Final adjustment ensures we land on a business day
                adjust(spot, bdc, calendar)
            }
        } else if self.allow_calendar_fallback {
            // Fallback: calendar not found, use calendar-day addition with warning.
            tracing::warn!(
                calendar_id = calendar_id,
                currency = ?currency,
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
    pub fn price_deposit(
        &self,
        maturity: Date,
        rate: f64,
        day_count: DayCount,
        conventions: &InstrumentConventions,
        currency: Currency,
        context: &MarketContext,
    ) -> Result<f64> {
        let common = conv::resolve_common(self, conventions, currency);
        let start = self.effective_start_date(conventions, currency)?;

        let dep = Deposit {
            id: format!("CALIB_DEP_{}", maturity).into(),
            notional: Money::new(1_000_000.0, currency),
            start,
            end: maturity,
            day_count,
            quote_rate: Some(rate),
            discount_curve_id: self.discount_curve_id.clone(),
            attributes: Default::default(),
            // Only set spot_lag_days when use_settlement_start=true; otherwise
            // rely on the explicit start date to avoid double-application
            spot_lag_days: if self.use_settlement_start && common.settlement_days != 0 {
                Some(common.settlement_days)
            } else {
                None
            },
            bdc: if common.settlement_days == 0 {
                None
            } else {
                Some(common.bdc)
            },
            calendar_id: if common.settlement_days == 0 {
                None
            } else {
                Some(common.calendar_id.to_string())
            },
        };

        let pv = dep.value(context, self.base_date)?;
        Ok(pv.amount() / dep.notional.amount())
    }

    /// Price a FRA quote for calibration.
    #[allow(clippy::too_many_arguments)]
    pub fn price_fra(
        &self,
        start: Date,
        end: Date,
        rate: f64,
        day_count: DayCount,
        conventions: &InstrumentConventions,
        currency: Currency,
        context: &MarketContext,
    ) -> Result<f64> {
        let common = conv::resolve_common(self, conventions, currency);
        let reset_lag = common.reset_lag_days;
        let calendar_id = common.fixing_calendar_id;

        let (fixing_date, calendar_found) = self.compute_fra_fixing_date(
            start,
            reset_lag,
            calendar_id,
            self.allow_calendar_fallback,
        )?;

        if self.verbose {
            tracing::debug!(
                fra_start = %start,
                fra_end = %end,
                fixing_date = %fixing_date,
                reset_lag = reset_lag,
                calendar = ?calendar_id,
                calendar_found = calendar_found,
                "FRA fixing date calculation"
            );
        }

        // Only pass calendar ID to FRA if the calendar was actually found
        let fixing_calendar_id_opt = if calendar_found {
            Some(calendar_id.to_string())
        } else {
            None
        };

        let fra = ForwardRateAgreement::builder()
            .id(format!("CALIB_FRA_{}_{}", start, end).into())
            .notional(Money::new(1_000_000.0, currency))
            .fixing_date(fixing_date)
            .start_date(start)
            .end_date(end)
            .fixed_rate(rate)
            .day_count(day_count)
            .reset_lag(reset_lag)
            .fixing_calendar_id_opt(fixing_calendar_id_opt)
            .discount_curve_id(self.discount_curve_id.clone())
            .forward_id(self.forward_curve_id.clone())
            .pay_fixed(false) // Receive fixed, pay floating (consistent with forward curve calibration)
            .build()
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!(
                    "FRA builder failed for start {} end {}: {}",
                    start, end, e
                ),
                category: "fra_pricing".to_string(),
            })?;

        let pv = fra.value(context, self.base_date)?;
        Ok(pv.amount() / fra.notional.amount())
    }

    /// Price a futures quote for calibration.
    #[allow(clippy::too_many_arguments)]
    pub fn price_future(
        &self,
        expiry: Date,
        period_start: Date,
        period_end: Date,
        fixing_date: Option<Date>,
        price: f64,
        specs: &FutureSpecs,
        _conventions: &InstrumentConventions,
        currency: Currency,
        context: &MarketContext,
    ) -> Result<f64> {
        // Default fixing date to the underlying period start when not supplied.
        let fixing_date = fixing_date.unwrap_or(period_start);

        let dc_ctx = DayCountCtx::default();
        let time_to_expiry = specs
            .day_count
            .year_fraction(self.base_date, expiry, dc_ctx)
            .unwrap_or(0.0);
        let time_to_maturity = specs
            .day_count
            .year_fraction(self.base_date, period_end, dc_ctx)
            .unwrap_or(0.0);

        // Calculate convexity adjustment using priority:
        // 1. Quote-level override
        // 2. Pricer-level custom params
        // 3. Currency-specific defaults
        let convexity_adj =
            self.resolve_future_convexity(specs, currency, time_to_expiry, time_to_maturity);

        if self.verbose {
            tracing::debug!(
                future_expiry = %expiry,
                time_to_expiry = time_to_expiry,
                time_to_maturity = time_to_maturity,
                market_implied_vol = ?specs.market_implied_vol,
                convexity_adjustment = ?convexity_adj,
                "Futures convexity adjustment"
            );
        }

        let future = InterestRateFuture::builder()
            .id(format!("CALIB_FUT_{}", expiry).into())
            .notional(self.future_notional(specs, currency))
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

    /// Compute FRA fixing date using signed reset lag and calendars.
    ///
    /// Returns the fixing date and a flag indicating whether the calendar was found.
    fn compute_fra_fixing_date(
        &self,
        start: Date,
        reset_lag: i32,
        calendar_id: &str,
        allow_calendar_fallback: bool,
    ) -> finstack_core::Result<(Date, bool)> {
        let registry = CalendarRegistry::global();
        if let Some(calendar) = registry.resolve_str(calendar_id) {
            let fixing_date = if start >= self.base_date {
                start.add_business_days(reset_lag, calendar)?
            } else {
                self.base_date
            };
            Ok((fixing_date, true))
        } else if allow_calendar_fallback {
            let candidate = start + time::Duration::days(reset_lag as i64);
            let fixing_date = if candidate >= self.base_date {
                candidate
            } else {
                self.base_date
            };
            Ok((fixing_date, false))
        } else {
            Err(finstack_core::Error::calendar_not_found_with_suggestions(
                calendar_id.to_string(),
                registry.available_ids(),
            ))
        }
    }

    fn resolve_future_convexity(
        &self,
        specs: &FutureSpecs,
        currency: Currency,
        time_to_expiry: f64,
        time_to_maturity: f64,
    ) -> Option<f64> {
        if let Some(adj) = specs.convexity_adjustment {
            return Some(adj);
        }

        let params = self
            .convexity_params
            .clone()
            .unwrap_or_else(|| ConvexityParameters::for_currency(currency));

        Some(params.calculate_adjustment_with_market_vol(
            time_to_expiry,
            time_to_maturity,
            specs.market_implied_vol,
        ))
    }

    fn future_notional(&self, specs: &FutureSpecs, currency: Currency) -> Money {
        Money::new(specs.face_value * specs.multiplier, currency)
    }

    /// Price a swap quote for calibration.
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
        currency: Currency,
        context: &MarketContext,
    ) -> Result<f64> {
        let common = conv::resolve_common(self, conventions, currency);
        let start = self.effective_start_date(conventions, currency)?;
        let payment_delay = common.payment_delay_days;
        let reset_lag = common.reset_lag_days;
        let payment_calendar_id = common.payment_calendar_id.to_string();
        let fixing_calendar_id = common.fixing_calendar_id.to_string();

        let fixed_spec = FixedLegSpec {
            discount_curve_id: self.discount_curve_id.clone(),
            rate,
            freq: fixed_freq,
            dc: fixed_dc,
            bdc: common.bdc,
            calendar_id: Some(payment_calendar_id.clone()),
            stub: StubKind::None,
            par_method: None,
            compounding_simple: true,
            start,
            end: maturity,
            payment_delay_days: payment_delay,
        };

        // Determine whether to use OIS pricing:
        // - For discount curve calibration (tenor_years=None): use OIS pricing if quote is OIS-suitable
        // - For forward curve calibration (tenor_years=Some): always use standard pricing to project
        //   using the forward curve being calibrated
        let use_ois_pricing = is_ois_quote && self.tenor_years.is_none();

        let (float_discount_id, float_forward_id, compounding) = if use_ois_pricing {
            // OIS pricing: use same curve for discount and forward, with OIS compounding
            (
                self.discount_curve_id.clone(),
                self.discount_curve_id.clone(),
                ois_compounding_for_index(index, currency),
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
            bdc: common.bdc,
            calendar_id: Some(payment_calendar_id),
            fixing_calendar_id: Some(fixing_calendar_id),
            stub: StubKind::None,
            reset_lag_days: reset_lag,
            start,
            end: maturity,
            compounding,
            payment_delay_days: payment_delay,
        };

        let swap = InterestRateSwap {
            id: format!("CALIB_SWAP_{}", maturity).into(),
            notional: Money::new(1_000_000.0, currency),
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
        let common = conv::resolve_common(self, conventions, currency);
        let start = self.effective_start_date(conventions, currency)?;
        let reset_lag = common.reset_lag_days;
        let payment_delay = common.payment_delay_days;
        let calendar_id = common.payment_calendar_id;

        // Determine forward curve IDs based on calibration mode
        let (primary_forward_id, reference_forward_id) = if let Some(tenor) = self.tenor_years {
            // Forward curve calibration: match by index OR frequency
            let primary_matches = self.leg_matches_tenor(primary_index, &primary_freq, tenor);
            let reference_matches = self.leg_matches_tenor(reference_index, &reference_freq, tenor);

            match (primary_matches, reference_matches) {
                (true, false) => (
                    self.forward_curve_id.clone(),
                    self.derive_forward_curve_id(reference_index),
                ),
                (false, true) => (
                    self.derive_forward_curve_id(primary_index),
                    self.forward_curve_id.clone(),
                ),
                (true, true) => {
                    return Err(finstack_core::Error::Validation(format!(
                        "BasisSwap quote references calibrator tenor on both legs (ambiguous): \
                         primary_index='{}', reference_index='{}'",
                        primary_index, reference_index
                    )));
                }
                (false, false) => {
                    return Err(finstack_core::Error::Validation(format!(
                        "BasisSwap quote does not reference calibrator tenor: \
                         primary_index='{}', reference_index='{}'",
                        primary_index, reference_index
                    )));
                }
            }
        } else {
            // Discount curve calibration: derive both curve IDs from index names
            (
                self.derive_forward_curve_id(primary_index),
                self.derive_forward_curve_id(reference_index),
            )
        };

        let primary_leg = BasisSwapLeg {
            forward_curve_id: primary_forward_id.clone(),
            frequency: primary_freq,
            day_count: primary_dc,
            bdc: common.bdc,
            payment_lag_days: payment_delay,
            reset_lag_days: reset_lag,
            spread: spread_bp / 10_000.0,
        };

        let reference_leg = BasisSwapLeg {
            forward_curve_id: reference_forward_id.clone(),
            frequency: reference_freq,
            day_count: reference_dc,
            bdc: common.bdc,
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

        // For discount curve calibration (tenor_years=None), both forward curves must exist
        // For forward curve calibration, one curve is being calibrated and may not exist yet
        if self.tenor_years.is_none()
            && (context
                .get_forward_ref(primary_forward_id.as_str())
                .is_err()
                || context
                    .get_forward_ref(reference_forward_id.as_str())
                    .is_err())
        {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::NotFound {
                    id: "forward curves".to_string(),
                },
            ));
        }

        let pv = basis_swap.value(context, self.base_date)?;
        Ok(pv.amount() / basis_swap.notional.amount())
    }

    /// Check if a basis swap leg matches the calibrator's tenor.
    fn leg_matches_tenor(&self, index: &str, freq: &Tenor, tenor_years: f64) -> bool {
        // Check index name for tenor token
        let tenor_months = (tenor_years * 12.0).round() as i32;
        let token = format!("{}M", tenor_months).to_ascii_uppercase();

        let normalized = index.to_ascii_uppercase();
        let tokens: Vec<&str> = normalized
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|t| !t.is_empty())
            .collect();

        let index_matches =
            tokens.contains(&token.as_str()) || (tenor_months == 12 && tokens.contains(&"1Y"));

        // Check frequency match
        let freq_matches = if freq.unit == finstack_core::dates::TenorUnit::Months {
            let freq_years = freq.count as f64 / 12.0;
            // Use a small tolerance for floating-point comparison
            (freq_years - tenor_years).abs() < 1e-6
        } else {
            false
        };

        index_matches || freq_matches
    }

    /// Derive a forward curve ID from an index name (fallback for non-matching legs).
    fn derive_forward_curve_id(&self, index_name: &str) -> CurveId {
        format!("FWD_{}", index_name).into()
    }

    // =========================================================================
    // Instrument Construction (for testing/repricing)
    // =========================================================================

    /// Create an OIS swap from a quote, matching calibration instrument construction.
    pub fn create_ois_swap(
        &self,
        quote: &RatesQuote,
        notional: Money,
        currency: Currency,
    ) -> Result<InterestRateSwap> {
        let (maturity, rate, is_ois, conventions) = match quote {
            RatesQuote::Swap {
                maturity,
                rate,
                is_ois,
                conventions,
                ..
            } => (*maturity, *rate, *is_ois, conventions),
            _ => {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ))
            }
        };

        let resolved = conv::resolve_swap_conventions(self, quote, currency)?;
        let start = self.effective_start_date(conventions, currency)?;
        let payment_delay = resolved.common.payment_delay_days;
        let reset_lag = resolved.common.reset_lag_days;
        let payment_calendar_id = resolved.common.payment_calendar_id.to_string();
        let fixing_calendar_id = resolved.common.fixing_calendar_id.to_string();

        // Determine whether to use OIS pricing (same logic as price_swap)
        let use_ois_pricing = is_ois && self.tenor_years.is_none();

        let compounding = if use_ois_pricing {
            ois_compounding_for_index(resolved.index, currency)
        } else {
            FloatingLegCompounding::Simple
        };

        let fixed_spec = FixedLegSpec {
            discount_curve_id: self.discount_curve_id.clone(),
            rate,
            freq: resolved.fixed_freq,
            dc: resolved.fixed_dc,
            bdc: resolved.common.bdc,
            calendar_id: Some(payment_calendar_id.clone()),
            stub: StubKind::None,
            par_method: None,
            compounding_simple: true,
            start,
            end: maturity,
            payment_delay_days: payment_delay,
        };

        let (float_discount_id, float_forward_id) = if use_ois_pricing {
            (
                self.discount_curve_id.clone(),
                self.discount_curve_id.clone(),
            )
        } else {
            (
                self.discount_curve_id.clone(),
                self.forward_curve_id.clone(),
            )
        };

        let float_spec = FloatLegSpec {
            discount_curve_id: float_discount_id,
            forward_curve_id: float_forward_id,
            spread_bp: 0.0,
            freq: resolved.float_freq,
            dc: resolved.float_dc,
            bdc: resolved.common.bdc,
            calendar_id: Some(payment_calendar_id),
            fixing_calendar_id: Some(fixing_calendar_id),
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
    pub fn price_instrument(
        &self,
        quote: &RatesQuote,
        currency: Currency,
        context: &MarketContext,
    ) -> Result<f64> {
        match quote {
            RatesQuote::Deposit {
                maturity,
                rate,
                conventions,
            } => {
                let day_count = conv::resolve_money_market(self, conventions, currency).day_count;
                self.price_deposit(*maturity, *rate, day_count, conventions, currency, context)
            }

            RatesQuote::FRA {
                start,
                end,
                rate,
                conventions,
            } => {
                let day_count = conv::resolve_money_market(self, conventions, currency).day_count;
                self.price_fra(*start, *end, *rate, day_count, conventions, currency, context)
            }

            RatesQuote::Future {
                expiry,
                period_start,
                period_end,
                fixing_date,
                price,
                specs,
                conventions,
            } => self.price_future(
                *expiry,
                *period_start,
                *period_end,
                *fixing_date,
                *price,
                specs,
                conventions,
                currency,
                context,
            ),

            RatesQuote::Swap {
                maturity,
                rate,
                ..
            } => {
                let resolved = conv::resolve_swap_conventions(self, quote, currency)?;
                let is_ois = quote.is_ois_suitable();
                self.price_swap(
                    *maturity,
                    *rate,
                    resolved.fixed_freq,
                    resolved.float_freq,
                    resolved.fixed_dc,
                    resolved.float_dc,
                    resolved.index,
                    is_ois,
                    match quote {
                        RatesQuote::Swap { conventions, .. } => conventions,
                        _ => return Err(finstack_core::Error::Input(finstack_core::error::InputError::Invalid)),
                    },
                    currency,
                    context,
                )
            }

            RatesQuote::BasisSwap {
                maturity,
                spread_bp,
                conventions,
                ..
            } => {
                let resolved = conv::resolve_basis_swap_conventions(self, quote, currency)?;
                self.price_basis_swap(
                    *maturity,
                    resolved.primary_index.as_str(),
                    resolved.reference_index.as_str(),
                    *spread_bp,
                    resolved.primary_freq,
                    resolved.reference_freq,
                    resolved.primary_dc,
                    resolved.reference_dc,
                    resolved.currency,
                    conventions,
                    context,
                )
            }
        }
    }

    /// Price a rate instrument requiring all conventions to be explicitly provided.
    pub fn price_instrument_strict(
        &self,
        quote: &RatesQuote,
        currency: Currency,
        context: &MarketContext,
    ) -> Result<f64> {
        match quote {
            RatesQuote::Deposit {
                maturity,
                rate,
                conventions,
            } => {
                // For strict pricing we still honor quote-provided day count, but
                // allow settlement conventions to fall back to currency defaults.
                let resolved = conv::resolve_money_market(self, conventions, currency);
                let start = if self.use_settlement_start {
                    self.settlement_date_for_quote(conventions, currency)?
                } else {
                    self.base_date
                };
                let dep = Deposit {
                    id: format!("CALIB_DEP_{}", maturity).into(),
                    notional: Money::new(1_000_000.0, currency),
                    start,
                    end: *maturity,
                    day_count: resolved.day_count,
                    quote_rate: Some(*rate),
                    discount_curve_id: self.discount_curve_id.clone(),
                    attributes: Default::default(),
                    spot_lag_days: if self.use_settlement_start && resolved.common.settlement_days != 0 {
                        Some(resolved.common.settlement_days)
                    } else {
                        None
                    },
                    bdc: if resolved.common.settlement_days == 0 {
                        None
                    } else {
                        Some(resolved.common.bdc)
                    },
                    calendar_id: if resolved.common.settlement_days == 0 {
                        None
                    } else {
                        Some(resolved.common.calendar_id.to_string())
                    },
                };
                let pv = dep.value(context, self.base_date)?;
                Ok(pv.amount() / dep.notional.amount())
            }
            RatesQuote::FRA {
                start,
                end,
                rate,
                conventions,
            } => {
                // Require day count but allow settlement/reset conventions to default.
                let common = conv::resolve_common(self, conventions, currency);
                let day_count = conventions.day_count.ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "FRA quote requires conventions.day_count to be set".to_string(),
                    )
                })?;
                let reset_lag = common.reset_lag_days;
                let calendar_id = common.fixing_calendar_id;
                let (fixing_date, _) =
                    self.compute_fra_fixing_date(*start, reset_lag, calendar_id, false)?;
                let fra = ForwardRateAgreement::builder()
                    .id(format!("CALIB_FRA_{}_{}", start, end).into())
                    .notional(Money::new(1_000_000.0, currency))
                    .fixing_date(fixing_date)
                    .start_date(*start)
                    .end_date(*end)
                    .fixed_rate(*rate)
                    .day_count(day_count)
                    .reset_lag(reset_lag)
                    .fixing_calendar_id_opt(Some(calendar_id.to_string()))
                    .discount_curve_id(self.discount_curve_id.clone())
                    .forward_id(self.forward_curve_id.clone())
                    .pay_fixed(false)
                    .build()
                    .map_err(|e| finstack_core::Error::Calibration {
                        message: format!(
                            "FRA builder failed for start {} end {}: {}",
                            start, end, e
                        ),
                        category: "fra_pricing".to_string(),
                    })?;
                let pv = fra.value(context, self.base_date)?;
                Ok(pv.amount() / fra.notional.amount())
            }
            RatesQuote::Future {
                expiry,
                period_start,
                period_end,
                fixing_date,
                price,
                specs,
                conventions,
            } => self.price_future(
                *expiry,
                *period_start,
                *period_end,
                *fixing_date,
                *price,
                specs,
                conventions,
                currency,
                context,
            ),
            RatesQuote::Swap {
                maturity,
                rate,
                is_ois,
                conventions,
                ..
            } => {
                // Enforce leg day counts/tenors while allowing settlement defaults.
                let resolved = conv::resolve_swap_conventions(self, quote, currency)?;
                let start = self.effective_start_date(conventions, currency)?;
                let payment_delay = resolved.common.payment_delay_days;
                let reset_lag = resolved.common.reset_lag_days;
                let payment_calendar_id = resolved.common.payment_calendar_id.to_string();
                let fixing_calendar_id = resolved.common.fixing_calendar_id.to_string();

                let fixed_spec = FixedLegSpec {
                    discount_curve_id: self.discount_curve_id.clone(),
                    rate: *rate,
                    freq: resolved.fixed_freq,
                    dc: resolved.fixed_dc,
                    bdc: resolved.common.bdc,
                    calendar_id: Some(payment_calendar_id.clone()),
                    stub: StubKind::None,
                    par_method: None,
                    compounding_simple: true,
                    start,
                    end: *maturity,
                    payment_delay_days: payment_delay,
                };

                let use_ois_pricing = *is_ois && self.tenor_years.is_none();
                let (float_discount_id, float_forward_id, compounding) = if use_ois_pricing {
                    (
                        self.discount_curve_id.clone(),
                        self.discount_curve_id.clone(),
                        ois_compounding_for_index(resolved.index, currency),
                    )
                } else {
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
                    freq: resolved.float_freq,
                    dc: resolved.float_dc,
                    bdc: resolved.common.bdc,
                    calendar_id: Some(payment_calendar_id),
                    fixing_calendar_id: Some(fixing_calendar_id),
                    stub: StubKind::None,
                    reset_lag_days: reset_lag,
                    start,
                    end: *maturity,
                    compounding,
                    payment_delay_days: payment_delay,
                };

                let swap = InterestRateSwap {
                    id: format!("CALIB_SWAP_{}", maturity).into(),
                    notional: Money::new(1_000_000.0, currency),
                    side: PayReceive::ReceiveFixed,
                    fixed: fixed_spec,
                    float: float_spec,
                    margin_spec: None,
                    attributes: Default::default(),
                };
                let pv = swap.value(context, self.base_date)?;
                Ok(pv.amount() / swap.notional.amount())
            }
            RatesQuote::BasisSwap {
                maturity,
                spread_bp,
                conventions,
                ..
            } => {
                let resolved = conv::resolve_basis_swap_conventions_strict(quote, currency)?;
                let common = conv::resolve_common_strict(conventions, resolved.currency)?;
                let start = if self.use_settlement_start {
                    self.settlement_date_for_quote_strict(conventions, resolved.currency)?
                } else {
                    self.base_date
                };
                let basis_swap = BasisSwap::new(
                    format!("CALIB_BASIS_{}", maturity),
                    Money::new(1_000_000.0, resolved.currency),
                    start,
                    *maturity,
                    BasisSwapLeg {
                        forward_curve_id: self.resolve_forward_curve_id(resolved.primary_index.as_str()),
                        frequency: resolved.primary_freq,
                        day_count: resolved.primary_dc,
                        bdc: common.bdc,
                        payment_lag_days: common.payment_delay_days,
                        reset_lag_days: common.reset_lag_days,
                        spread: *spread_bp / 10_000.0,
                    },
                    BasisSwapLeg {
                        forward_curve_id: self.resolve_forward_curve_id(resolved.reference_index.as_str()),
                        frequency: resolved.reference_freq,
                        day_count: resolved.reference_dc,
                        bdc: common.bdc,
                        payment_lag_days: common.payment_delay_days,
                        reset_lag_days: common.reset_lag_days,
                        spread: 0.0,
                    },
                    self.discount_curve_id.as_str(),
                )
                .with_allow_calendar_fallback(self.allow_calendar_fallback)
                .with_calendar(common.payment_calendar_id.to_string());
                let pv = basis_swap.value(context, self.base_date)?;
                Ok(pv.amount() / basis_swap.notional.amount())
            }
        }
    }

    // =========================================================================
    // Quote Validation
    // =========================================================================

    /// Extract rate from quote.
    pub fn get_rate(quote: &RatesQuote) -> f64 {
        match quote {
            RatesQuote::Deposit { rate, .. } => *rate,
            RatesQuote::FRA { rate, .. } => *rate,
            RatesQuote::Future { price, .. } => (100.0 - price) / 100.0,
            RatesQuote::Swap { rate, .. } => *rate,
            RatesQuote::BasisSwap { spread_bp, .. } => *spread_bp / 10_000.0,
        }
    }

    /// Build a duplicate-detection key tailored to the quote type.
    fn dedupe_key(quote: &RatesQuote) -> String {
        match quote {
            RatesQuote::Deposit { maturity, .. } => format!("DEP|{}", maturity),
            RatesQuote::FRA { start, end, .. } => format!("FRA|{}|{}", start, end),
            RatesQuote::Future {
                expiry,
                period_start,
                period_end,
                ..
            } => format!("FUT|{}|{}|{}", expiry, period_start, period_end),
            RatesQuote::Swap {
                maturity,
                is_ois,
                float_leg_conventions,
                ..
            } => {
                let index = float_leg_conventions
                    .index
                    .as_ref()
                    .map(|i| i.as_str())
                    .unwrap_or("UNKNOWN");
                format!("SWAP|{}|{}|{}", maturity, index, is_ois)
            }
            RatesQuote::BasisSwap {
                maturity,
                primary_leg_conventions,
                reference_leg_conventions,
                ..
            } => {
                let primary_idx = primary_leg_conventions
                    .index
                    .as_ref()
                    .map(|i| i.as_str())
                    .unwrap_or("PRIMARY");
                let ref_idx = reference_leg_conventions
                    .index
                    .as_ref()
                    .map(|i| i.as_str())
                    .unwrap_or("REFERENCE");
                format!("BASIS|{}|{}|{}", maturity, primary_idx, ref_idx)
            }
        }
    }

    /// Pre-validate that all required curves exist for the quote set.
    pub fn validate_curve_dependencies(
        &self,
        quotes: &[RatesQuote],
        context: &MarketContext,
    ) -> Result<()> {
        let calibrating_forward_id = self.forward_curve_id.as_str();
        let allow_missing_calibrated_curve = self.tenor_years.is_some();

        for quote in quotes {
            if let RatesQuote::BasisSwap {
                primary_leg_conventions,
                reference_leg_conventions,
                ..
            } = quote
            {
                // Get index names from conventions
                let primary_index = primary_leg_conventions.index.as_ref()
                    .ok_or_else(|| finstack_core::Error::Validation(
                        "BasisSwap requires primary_leg_conventions.index".to_string()
                    ))?;
                let reference_index = reference_leg_conventions.index.as_ref()
                    .ok_or_else(|| finstack_core::Error::Validation(
                        "BasisSwap requires reference_leg_conventions.index".to_string()
                    ))?;
                
                // Use resolver for consistent curve ID derivation
                let primary_fwd = self.resolve_forward_curve_id(primary_index.as_str());
                let ref_fwd = self.resolve_forward_curve_id(reference_index.as_str());

                let primary_missing = context.get_forward_ref(primary_fwd.as_str()).is_err();
                let reference_missing = context.get_forward_ref(ref_fwd.as_str()).is_err();

                if allow_missing_calibrated_curve {
                    // In forward calibration we allow the calibrated curve to be absent.
                    if primary_missing && primary_fwd.as_str() != calibrating_forward_id {
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

                    if reference_missing && ref_fwd.as_str() != calibrating_forward_id {
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
                } else {
                    if primary_missing {
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
                    if reference_missing {
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
        }
        Ok(())
    }

    /// Unified validation for rate quotes with use-case-specific rules.
    pub fn validate_rates_quotes(
        quotes: &[RatesQuote],
        rate_bounds: &crate::calibration::config::RateBounds,
        base_date: Date,
        use_case: RatesQuoteUseCase,
    ) -> Result<()> {
        // 1. Non-empty check
        if quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        // 2. Duplicate detection with instrument-specific keys
        let mut seen = std::collections::HashSet::new();
        for quote in quotes {
            let key = Self::dedupe_key(quote);
            if !seen.insert(key) {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }
        }

        // 3. Per-quote validation (rate bounds, finite check, maturity, use-case constraints)
        // Also accumulate discount-curve "separation" violations for a single warn/error.
        let mut separation_violations: Vec<&'static str> = Vec::new();

        for quote in quotes {
            // Use-case specific: Forward curve does not support Deposit
            if let RatesQuoteUseCase::ForwardCurve = use_case {
                if matches!(quote, RatesQuote::Deposit { .. }) {
                    return Err(finstack_core::Error::Validation(
                        "ForwardCurveCalibrator does not support Deposit quotes (use DiscountCurveCalibrator)".into(),
                    ));
                }
            }

            // Use-case specific: Discount curve checks non-OIS forward-dependent instruments
            // Collect violations here; decide warn vs error below based on enforce_separation
            if let RatesQuoteUseCase::DiscountCurve { .. } = use_case {
                if !quote.is_ois_suitable()
                    && quote.requires_forward_curve()
                    && separation_violations.len() < 5
                {
                    separation_violations.push(quote.get_type());
                }
            }

            // Instrument-specific date sanity
            match quote {
                RatesQuote::FRA { start, end, .. } => {
                    if *start <= base_date {
                        return Err(finstack_core::Error::Calibration {
                            message: format!(
                                "FRA start {} is on or before base date {}",
                                start, base_date
                            ),
                            category: "quote_validation".to_string(),
                        });
                    }
                    if *end <= *start {
                        return Err(finstack_core::Error::Calibration {
                            message: format!(
                                "FRA end {} is on or before start {}",
                                end, start
                            ),
                            category: "quote_validation".to_string(),
                        });
                    }
                }
                RatesQuote::Future {
                    period_start,
                    period_end,
                    ..
                } => {
                    if *period_start <= base_date {
                        return Err(finstack_core::Error::Calibration {
                            message: format!(
                                "Future period_start {} is on or before base date {}",
                                period_start, base_date
                            ),
                            category: "quote_validation".to_string(),
                        });
                    }
                    if *period_end <= *period_start {
                        return Err(finstack_core::Error::Calibration {
                            message: format!(
                                "Future period_end {} is on or before period_start {}",
                                period_end, period_start
                            ),
                            category: "quote_validation".to_string(),
                        });
                    }
                }
                _ => {}
            }

            // Rate extraction and validation
            let rate = Self::get_rate(quote);
            if !rate.is_finite() {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }
            if !rate_bounds.contains(rate) {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Quote rate {:.4}% outside allowed bounds [{:.2}%, {:.2}%]. \
                        Use `with_rate_bounds()` to adjust bounds for this market regime.",
                        rate * 100.0,
                        rate_bounds.min_rate * 100.0,
                        rate_bounds.max_rate * 100.0
                    ),
                    category: "quote_validation".to_string(),
                });
            }

            // Maturity must be after base date
            let maturity = quote.maturity_date();
            if maturity <= base_date {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Quote maturity {} is on or before base date {}",
                        maturity, base_date
                    ),
                    category: "quote_validation".to_string(),
                });
            }
        }

        // 4. Use-case specific: Discount curve separation enforcement (warn vs error)
        if let RatesQuoteUseCase::DiscountCurve { enforce_separation } = use_case {
            if !separation_violations.is_empty() {
                let examples = separation_violations.join(", ");
                let msg = format!(
                    "Discount curve calibration received {} non-OIS forward-dependent quote(s) \
(e.g. {}). Best practice: use Deposits/OIS swaps for discount curves and calibrate forward curves separately.",
                    separation_violations.len(),
                    examples
                );

                if enforce_separation {
                    return Err(finstack_core::Error::Validation(msg));
                } else {
                    tracing::warn!("{msg}");
                }
            }
        }

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibration::config::RateBounds;
    use time::Month;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
    use finstack_core::dates::BusinessDayConvention;

    #[test]
    fn fra_fixing_date_respects_signed_reset_lag_negative() {
        let base_date =
            Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
        let pricer = CalibrationPricer::new(base_date, "USD-OIS");
        let start =
            Date::from_calendar_date(2024, Month::January, 9).expect("valid start date");
        let calendar = CalendarRegistry::global()
            .resolve_str("usny")
            .expect("usny calendar");
        let expected = start
            .add_business_days(-2, calendar)
            .expect("business day math");

        let (fixing, found) =
            pricer.compute_fra_fixing_date(start, -2, "usny", false).expect("fixing date");
        assert!(found);
        assert_eq!(fixing, expected);
    }

    #[test]
    fn fra_fixing_date_respects_signed_reset_lag_positive() {
        let base_date =
            Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
        let pricer = CalibrationPricer::new(base_date, "USD-OIS");
        let start =
            Date::from_calendar_date(2024, Month::January, 9).expect("valid start date");
        let calendar = CalendarRegistry::global()
            .resolve_str("usny")
            .expect("usny calendar");
        let expected = start
            .add_business_days(2, calendar)
            .expect("business day math");

        let (fixing, found) =
            pricer.compute_fra_fixing_date(start, 2, "usny", false).expect("fixing date");
        assert!(found);
        assert_eq!(fixing, expected);
    }

    #[test]
    fn fra_fixing_date_falls_back_to_calendar_days_when_allowed() {
        let base_date =
            Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
        let pricer = CalibrationPricer::new(base_date, "USD-OIS").with_allow_calendar_fallback(true);
        let start =
            Date::from_calendar_date(2024, Month::January, 9).expect("valid start date");
        let expected = start + time::Duration::days(-2);

        let (fixing, found) =
            pricer.compute_fra_fixing_date(start, -2, "missing-calendar", true).expect("fixing");
        assert!(!found);
        assert_eq!(fixing, expected);
    }

    #[test]
    fn create_ois_swap_matches_price_swap_pricing() {
        let base_date =
            Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (5.0, 0.80)])
            .build()
            .expect("discount curve");

        let ctx = MarketContext::new().insert_discount(disc);

        let pricer = CalibrationPricer::new(base_date, "USD-OIS");
        let maturity = base_date + time::Duration::days(365);
        let quote = RatesQuote::Swap {
            maturity,
            rate: 0.02,
            is_ois: true,
            conventions: Default::default(),
            fixed_leg_conventions: InstrumentConventions::default(),
            float_leg_conventions: InstrumentConventions::default().with_index("USD-SOFR"),
        };

        let pv_norm = pricer
            .price_instrument(&quote, Currency::USD, &ctx)
            .expect("price swap");

        let swap = pricer
            .create_ois_swap(&quote, Money::new(1_000_000.0, Currency::USD), Currency::USD)
            .expect("create ois swap");
        let pv_swap = swap
            .value(&ctx, base_date)
            .expect("value swap")
            .amount()
            / swap.notional.amount();

        assert!(
            (pv_norm - pv_swap).abs() < 1e-12,
            "OIS construction mismatch: pricer={} vs builder={}",
            pv_norm,
            pv_swap
        );
    }

    #[test]
    fn validate_quotes_allows_distinct_fras_with_same_end() {
        let base_date =
            Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
        let fra1 = RatesQuote::FRA {
            start: base_date + time::Duration::days(30),
            end: base_date + time::Duration::days(60),
            rate: 0.05,
            conventions: Default::default(),
        };
        let fra2 = RatesQuote::FRA {
            start: base_date + time::Duration::days(35),
            end: base_date + time::Duration::days(60),
            rate: 0.051,
            conventions: Default::default(),
        };
        let bounds = RateBounds::default();
        CalibrationPricer::validate_rates_quotes(
            &[fra1, fra2],
            &bounds,
            base_date,
            RatesQuoteUseCase::DiscountCurve {
                enforce_separation: false,
            },
        )
        .expect("distinct FRAs should not be considered duplicates");
    }

    #[test]
    fn validate_quotes_rejects_duplicate_fra() {
        let base_date =
            Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
        let fra = RatesQuote::FRA {
            start: base_date + time::Duration::days(30),
            end: base_date + time::Duration::days(60),
            rate: 0.05,
            conventions: Default::default(),
        };
        let bounds = RateBounds::default();
        let err = CalibrationPricer::validate_rates_quotes(
            &[fra.clone(), fra],
            &bounds,
            base_date,
            RatesQuoteUseCase::DiscountCurve {
                enforce_separation: false,
            },
        )
        .expect_err("duplicate FRA should be rejected");

        match err {
            finstack_core::Error::Input(finstack_core::error::InputError::Invalid) => {}
            other => panic!("Unexpected error: {:?}", other),
        }
    }

    #[test]
    fn validate_quotes_rejects_started_fra() {
        let base_date =
            Date::from_calendar_date(2024, Month::January, 10).expect("valid base date");
        let fra = RatesQuote::FRA {
            start: base_date, // same as base -> invalid
            end: base_date + time::Duration::days(30),
            rate: 0.05,
            conventions: Default::default(),
        };
        let bounds = RateBounds::default();
        let err = CalibrationPricer::validate_rates_quotes(
            &[fra],
            &bounds,
            base_date,
            RatesQuoteUseCase::DiscountCurve {
                enforce_separation: false,
            },
        )
        .expect_err("FRA starting on base date should be rejected");
        assert!(matches!(
            err,
            finstack_core::Error::Calibration { .. }
        ));
    }

    #[test]
    fn validate_quotes_rejects_fra_with_inverted_dates() {
        let base_date =
            Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
        let fra = RatesQuote::FRA {
            start: base_date + time::Duration::days(40),
            end: base_date + time::Duration::days(10), // ends before start
            rate: 0.05,
            conventions: Default::default(),
        };
        let bounds = RateBounds::default();
        let err = CalibrationPricer::validate_rates_quotes(
            &[fra],
            &bounds,
            base_date,
            RatesQuoteUseCase::DiscountCurve {
                enforce_separation: false,
            },
        )
        .expect_err("FRA with end before start should be rejected");
        assert!(matches!(
            err,
            finstack_core::Error::Calibration { .. }
        ));
    }

    #[test]
    fn settlement_respects_custom_bdc() {
        // Saturday base date; Preceding should move to prior Friday.
        let base_date =
            Date::from_calendar_date(2024, Month::January, 6).expect("valid base date");
        let pricer = CalibrationPricer::new(base_date, "USD-OIS");
        let conventions = InstrumentConventions::default()
            .with_settlement_days(0)
            .with_business_day_convention(BusinessDayConvention::Preceding)
            .with_calendar_id("usny");

        let settlement = pricer
            .settlement_date_for_quote(&conventions, Currency::USD)
            .expect("settlement");

        assert_eq!(
            settlement,
            Date::from_calendar_date(2024, Month::January, 5).expect("valid date")
        );
    }

    #[test]
    fn fixing_calendar_override_is_used() {
        let base_date =
            Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
        let pricer = CalibrationPricer::new(base_date, "USD-OIS");
        let conventions = InstrumentConventions::default()
            .with_calendar_id("missing-calendar")
            .with_fixing_calendar_id("usny")
            .with_reset_lag(-2);

        let common = conv::resolve_common(&pricer, &conventions, Currency::USD);
        assert_eq!(common.fixing_calendar_id, "usny");

        let (fixing_date, found) =
            pricer.compute_fra_fixing_date(base_date + time::Duration::days(5), -2, common.fixing_calendar_id, false).expect("fixing date");
        assert!(found, "should use fixing calendar even if general calendar is missing");
        assert_eq!(
            fixing_date,
            (base_date + time::Duration::days(5))
                .add_business_days(
                    -2,
                    CalendarRegistry::global()
                        .resolve_str("usny")
                        .expect("usny calendar")
                )
                .expect("business day math")
        );
    }

    #[test]
    fn validate_curve_dependencies_allows_missing_calibrated_forward() {
        let base_date =
            Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
        let pricer = CalibrationPricer::for_forward_curve(
            base_date,
            "FWD_USD-SOFR-3M",
            "USD-OIS",
            0.25,
        );

        // Only provide the reference forward curve; primary (the one being calibrated) is absent.
        let ref_curve = ForwardCurve::builder("FWD_USD-SOFR-6M", 0.5)
            .base_date(base_date)
            .knots([(0.0, 0.03), (5.0, 0.03)])
            .build()
            .expect("forward curve");

        let ctx = MarketContext::new().insert_forward(ref_curve);

        let quote = RatesQuote::BasisSwap {
            maturity: base_date + time::Duration::days(365),
            spread_bp: 0.0,
            conventions: InstrumentConventions::default().with_currency(Currency::USD),
            primary_leg_conventions: InstrumentConventions::default().with_index("USD-SOFR-3M"),
            reference_leg_conventions: InstrumentConventions::default().with_index("USD-SOFR-6M"),
        };

        pricer
            .validate_curve_dependencies(&[quote], &ctx)
            .expect("calibrated forward curve may be missing");
    }

    #[test]
    fn validate_curve_dependencies_rejects_missing_unrelated_forward() {
        let base_date =
            Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
        let pricer = CalibrationPricer::for_forward_curve(
            base_date,
            "FWD_USD-SOFR-3M",
            "USD-OIS",
            0.25,
        );

        // No forward curves in context; reference leg missing and not the calibrated curve.
        let ctx = MarketContext::new();

        let quote = RatesQuote::BasisSwap {
            maturity: base_date + time::Duration::days(365),
            spread_bp: 0.0,
            conventions: InstrumentConventions::default().with_currency(Currency::USD),
            primary_leg_conventions: InstrumentConventions::default().with_index("USD-SOFR-3M"),
            reference_leg_conventions: InstrumentConventions::default().with_index("USD-SOFR-6M"),
        };

        let err = pricer
            .validate_curve_dependencies(&[quote], &ctx)
            .expect_err("missing reference forward curve should error");

        match err {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound { .. }) => {}
            other => panic!("Unexpected error: {:?}", other),
        }
    }

    #[test]
    fn future_convexity_uses_market_vol_if_provided() {
        let base_date =
            Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
        let pricer = CalibrationPricer::new(base_date, "USD-OIS");
        let specs_default = FutureSpecs::default();
        let specs_with_vol = FutureSpecs {
            market_implied_vol: Some(0.02),
            ..FutureSpecs::default()
        };

        let t_exp = 0.5;
        let t_mat = 0.75;

        let adj_default = pricer
            .resolve_future_convexity(&specs_default, Currency::USD, t_exp, t_mat)
            .expect("convexity adjustment");
        let adj_market = pricer
            .resolve_future_convexity(&specs_with_vol, Currency::USD, t_exp, t_mat)
            .expect("convexity adjustment with vol");

        assert!(adj_market > adj_default, "market vol should increase convexity");
    }

    #[test]
    fn future_notional_scales_with_multiplier() {
        let base_date =
            Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
        let pricer = CalibrationPricer::new(base_date, "USD-OIS");
        let specs = FutureSpecs {
            face_value: 1_000_000.0,
            multiplier: 2.5,
            ..FutureSpecs::default()
        };

        let notional = pricer.future_notional(&specs, Currency::USD);
        assert!((notional.amount() - 2_500_000.0).abs() < 1e-6);
    }
}


