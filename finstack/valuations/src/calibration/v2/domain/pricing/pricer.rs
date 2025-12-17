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
        let calendar_id = common.calendar_id;
        let registry = CalendarRegistry::global();

        // Track whether we actually found the calendar (for FRA builder)
        let calendar_found = registry.resolve_str(calendar_id).is_some();

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
            .map_err(|_| finstack_core::Error::Internal)?;

        let pv = fra.value(context, self.base_date)?;
        Ok(pv.amount() / fra.notional.amount())
    }

    /// Price a futures quote for calibration.
    pub fn price_future(
        &self,
        expiry: Date,
        price: f64,
        specs: &FutureSpecs,
        _conventions: &InstrumentConventions,
        currency: Currency,
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
                .unwrap_or_else(|| ConvexityParameters::for_currency(currency));

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
            .notional(Money::new(specs.face_value, currency))
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
        let calendar_id = common.calendar_id.to_string();

        let fixed_spec = FixedLegSpec {
            discount_curve_id: self.discount_curve_id.clone(),
            rate,
            freq: fixed_freq,
            dc: fixed_dc,
            bdc: common.bdc,
            calendar_id: Some(calendar_id.clone()),
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
        let calendar_id = common.calendar_id;

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
        let calendar_id = resolved.common.calendar_id.to_string();

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
            freq: resolved.float_freq,
            dc: resolved.float_dc,
            bdc: resolved.common.bdc,
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
                price,
                specs,
                conventions,
            } => self.price_future(*expiry, *price, specs, conventions, currency, context),

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
                let calendar_id = common.calendar_id;
                let registry = CalendarRegistry::global();
                let calendar = registry.resolve_str(calendar_id).ok_or_else(|| {
                    finstack_core::Error::calendar_not_found_with_suggestions(
                        calendar_id.to_string(),
                        registry.available_ids(),
                    )
                })?;
                let fixing_date = if *start >= self.base_date {
                    start.add_business_days(-reset_lag, calendar)?
                } else {
                    self.base_date
                };
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
                    .map_err(|_| finstack_core::Error::Internal)?;
                let pv = fra.value(context, self.base_date)?;
                Ok(pv.amount() / fra.notional.amount())
            }
            RatesQuote::Future {
                expiry,
                price,
                specs,
                conventions,
            } => self.price_future(*expiry, *price, specs, conventions, currency, context),
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
                let calendar_id = resolved.common.calendar_id.to_string();

                let fixed_spec = FixedLegSpec {
                    discount_curve_id: self.discount_curve_id.clone(),
                    rate: *rate,
                    freq: resolved.fixed_freq,
                    dc: resolved.fixed_dc,
                    bdc: resolved.common.bdc,
                    calendar_id: Some(calendar_id.clone()),
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
                    calendar_id: Some(calendar_id.clone()),
                    fixing_calendar_id: Some(calendar_id),
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
                .with_calendar(common.calendar_id.to_string());
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

    /// Pre-validate that all required curves exist for the quote set.
    pub fn validate_curve_dependencies(
        &self,
        quotes: &[RatesQuote],
        context: &MarketContext,
    ) -> Result<()> {
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

        // 2. Duplicate (type, maturity) detection
        let mut seen = std::collections::HashSet::new();
        for quote in quotes {
            let key = (quote.get_type(), quote.maturity_date());
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


