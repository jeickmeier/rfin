//! Rates instrument pricing for `CalibrationPricer`.

use super::CalibrationPricer;
use crate::calibration::quotes::{InstrumentConventions, RatesQuote};
use crate::calibration::quotes::rate_index::{RateIndexConventions, RateIndexKind};
use crate::instruments::basis_swap::{BasisSwap, BasisSwapLeg};
use crate::instruments::common::traits::Instrument;
use crate::instruments::deposit::Deposit;
use crate::instruments::fra::ForwardRateAgreement;
use crate::instruments::irs::{FixedLegSpec, FloatLegSpec, FloatingLegCompounding, PayReceive};
use crate::instruments::InterestRateSwap;
use finstack_core::dates::{Date, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{Currency, CurveId};
use finstack_core::Result;

use super::super::convention_resolution as conv;

impl CalibrationPricer {
    /// Price a rate instrument for calibration (strict when enabled).
    pub fn price_instrument_for_calibration(
        &self,
        quote: &RatesQuote,
        currency: Currency,
        context: &MarketContext,
    ) -> Result<f64> {
        if self.strict_pricing {
            self.price_instrument_strict(quote, currency, context)
        } else {
            self.price_instrument(quote, currency, context)
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
                message: format!("FRA builder failed for start {} end {}: {}", start, end, e),
                category: "fra_pricing".to_string(),
            })?;

        let pv = fra.value(context, self.base_date)?;
        Ok(pv.amount() / fra.notional.amount())
    }

    /// Price a swap quote for calibration.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn price_swap(
        &self,
        maturity: Date,
        rate: f64,
        resolved: &conv::ResolvedSwapConventions<'_>,
        is_ois_quote: bool,
        conventions: &InstrumentConventions,
        currency: Currency,
        context: &MarketContext,
    ) -> Result<f64> {
        let start = self.effective_start_date(conventions, currency)?;
        let payment_delay = resolved.common.payment_delay_days;
        let reset_lag = resolved.common.reset_lag_days;
        let payment_calendar_id = resolved.common.payment_calendar_id.to_string();
        let fixing_calendar_id = resolved.common.fixing_calendar_id.to_string();

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
                RateIndexConventions::for_index_with_currency(resolved.index, currency)
                    .ois_compounding
                    .unwrap_or(FloatingLegCompounding::sofr()),
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

    /// Create an OIS swap from a quote, matching calibration instrument construction.
    pub fn create_ois_swap(
        &self,
        quote: &RatesQuote,
        notional: Money,
        currency: Currency,
    ) -> Result<InterestRateSwap> {
        let (maturity, rate, conventions) = match quote {
            RatesQuote::Swap {
                maturity,
                rate,
                conventions,
                ..
            } => (*maturity, *rate, conventions),
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
        let use_ois_pricing = quote.is_ois_suitable() && self.tenor_years.is_none();

        let compounding = if use_ois_pricing {
            RateIndexConventions::for_index_with_currency(resolved.index, currency)
                .ois_compounding
                .unwrap_or(FloatingLegCompounding::sofr())
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
                let day_count = conventions.day_count.unwrap_or_else(|| {
                    InstrumentConventions::default_money_market_day_count(currency)
                });
                self.price_deposit(*maturity, *rate, day_count, conventions, currency, context)
            }

            RatesQuote::FRA {
                start,
                end,
                rate,
                conventions,
            } => {
                let day_count = conventions.day_count.unwrap_or_else(|| {
                    InstrumentConventions::default_money_market_day_count(currency)
                });
                self.price_fra(
                    *start,
                    *end,
                    *rate,
                    day_count,
                    conventions,
                    currency,
                    context,
                )
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

            RatesQuote::Swap { maturity, rate, .. } => {
                let resolved = conv::resolve_swap_conventions(self, quote, currency)?;
                let is_ois = quote.is_ois_suitable();
                self.price_swap(
                    *maturity,
                    *rate,
                    &resolved,
                    is_ois,
                    match quote {
                        RatesQuote::Swap { conventions, .. } => conventions,
                        _ => {
                            return Err(finstack_core::Error::Input(
                                finstack_core::error::InputError::Invalid,
                            ))
                        }
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
                let day_count = conventions.day_count.ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "Deposit quote requires conventions.day_count to be set in strict pricing"
                            .to_string(),
                    )
                })?;
                let start = self.effective_start_date(conventions, currency)?;
                let settlement_days = conventions
                    .settlement_days
                    .or(self.settlement_days)
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires settlement_days to be set (quote or step)"
                                .to_string(),
                        )
                    })?;
                let bdc = conventions
                    .business_day_convention
                    .or(self.business_day_convention)
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires business_day_convention to be set (quote or step)"
                                .to_string(),
                        )
                    })?;
                let calendar_id = conventions
                    .calendar_id
                    .as_deref()
                    .or(self.calendar_id.as_deref())
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires calendar_id to be set (quote or step)"
                                .to_string(),
                        )
                    })?;
                let dep = Deposit {
                    id: format!("CALIB_DEP_{}", maturity).into(),
                    notional: Money::new(1_000_000.0, currency),
                    start,
                    end: *maturity,
                    day_count,
                    quote_rate: Some(*rate),
                    discount_curve_id: self.discount_curve_id.clone(),
                    attributes: Default::default(),
                    spot_lag_days: if self.use_settlement_start && settlement_days != 0 {
                        Some(settlement_days)
                    } else {
                        None
                    },
                    bdc: if settlement_days == 0 { None } else { Some(bdc) },
                    calendar_id: if settlement_days == 0 {
                        None
                    } else {
                        Some(calendar_id.to_string())
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
                let day_count = conventions.day_count.ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "FRA quote requires conventions.day_count to be set".to_string(),
                    )
                })?;
                let reset_lag = conventions
                    .reset_lag
                    .or(self.default_reset_lag_days)
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires reset_lag to be set (quote or step)"
                                .to_string(),
                        )
                    })?;
                let calendar_id = conventions
                    .effective_fixing_calendar_id()
                    .or(self.calendar_id.as_deref())
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires a fixing calendar to be set (quote or step)"
                                .to_string(),
                        )
                    })?;
                let (fixing_date, _) = self.compute_fra_fixing_date(
                    *start,
                    reset_lag,
                    calendar_id,
                    self.allow_calendar_fallback,
                )?;
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
                conventions,
                ..
            } => {
                let (fixed_leg_conventions, float_leg_conventions) = match quote {
                    RatesQuote::Swap {
                        fixed_leg_conventions,
                        float_leg_conventions,
                        ..
                    } => (fixed_leg_conventions, float_leg_conventions),
                    _ => {
                        return Err(finstack_core::Error::Input(
                            finstack_core::error::InputError::Invalid,
                        ))
                    }
                };

                let index = float_leg_conventions.index.as_ref().ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "Swap quote requires float_leg_conventions.index to be set".to_string(),
                    )
                })?;
                let index_conv = RateIndexConventions::for_index_with_currency(index, currency);

                let fixed_freq = fixed_leg_conventions.payment_frequency.ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "Strict pricing requires fixed_leg_conventions.payment_frequency to be set"
                            .to_string(),
                    )
                })?;
                let fixed_dc = fixed_leg_conventions.day_count.ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "Strict pricing requires fixed_leg_conventions.day_count to be set"
                            .to_string(),
                    )
                })?;

                let float_freq = float_leg_conventions
                    .payment_frequency
                    .unwrap_or(index_conv.default_payment_frequency);
                let float_dc = float_leg_conventions
                    .day_count
                    .unwrap_or(index_conv.day_count);

                let _settlement_days = conventions
                    .settlement_days
                    .or(self.settlement_days)
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires settlement_days to be set (quote or step)"
                                .to_string(),
                        )
                    })?;

                let calendar_id = conventions
                    .calendar_id
                    .as_deref()
                    .or(fixed_leg_conventions.calendar_id.as_deref())
                    .or(float_leg_conventions.calendar_id.as_deref())
                    .or(self.calendar_id.as_deref())
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires calendar_id to be set (quote or step)"
                                .to_string(),
                        )
                    })?;

                let bdc = conventions
                    .business_day_convention
                    .or(fixed_leg_conventions.business_day_convention)
                    .or(float_leg_conventions.business_day_convention)
                    .or(self.business_day_convention)
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires business_day_convention to be set (quote or step)"
                                .to_string(),
                        )
                    })?;

                let fixing_calendar_id = conventions
                    .fixing_calendar_id
                    .as_deref()
                    .or(float_leg_conventions.fixing_calendar_id.as_deref())
                    .or(conventions.calendar_id.as_deref())
                    .or(float_leg_conventions.calendar_id.as_deref())
                    .or(self.calendar_id.as_deref())
                    .unwrap_or(calendar_id);

                let payment_calendar_id = conventions
                    .payment_calendar_id
                    .as_deref()
                    .or(fixed_leg_conventions.payment_calendar_id.as_deref())
                    .or(float_leg_conventions.payment_calendar_id.as_deref())
                    .or(conventions.calendar_id.as_deref())
                    .or(fixed_leg_conventions.calendar_id.as_deref())
                    .or(float_leg_conventions.calendar_id.as_deref())
                    .or(self.calendar_id.as_deref())
                    .unwrap_or(calendar_id);

                let payment_delay = conventions
                    .payment_delay_days
                    .or(fixed_leg_conventions.payment_delay_days)
                    .or(float_leg_conventions.payment_delay_days)
                    .or_else(|| {
                        if index_conv.kind == RateIndexKind::OvernightRfr {
                            Some(index_conv.default_payment_delay_days)
                        } else {
                            self.default_payment_delay_days
                        }
                    })
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires payment_delay_days to be set (quote or step)"
                                .to_string(),
                        )
                    })?;

                let reset_lag = float_leg_conventions
                    .reset_lag
                    .or(conventions.reset_lag)
                    .or_else(|| {
                        if index_conv.kind == RateIndexKind::OvernightRfr {
                            Some(index_conv.default_reset_lag_days)
                        } else {
                            self.default_reset_lag_days
                        }
                    })
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires reset_lag to be set (quote or step)"
                                .to_string(),
                        )
                    })?;

                let start = if self.use_settlement_start {
                    // settlement_date_for_quote_explicit uses pricer step-level conventions.
                    self.settlement_date_for_quote_explicit(conventions, currency)?
                } else {
                    self.base_date
                };

                let payment_calendar_id = payment_calendar_id.to_string();
                let fixing_calendar_id = fixing_calendar_id.to_string();

                let fixed_spec = FixedLegSpec {
                    discount_curve_id: self.discount_curve_id.clone(),
                    rate: *rate,
                    freq: fixed_freq,
                    dc: fixed_dc,
                    bdc,
                    calendar_id: Some(payment_calendar_id.clone()),
                    stub: StubKind::None,
                    par_method: None,
                    compounding_simple: true,
                    start,
                    end: *maturity,
                    payment_delay_days: payment_delay,
                };

                let use_ois_pricing = quote.is_ois_suitable() && self.tenor_years.is_none();
                let (float_discount_id, float_forward_id, compounding) = if use_ois_pricing {
                    (
                        self.discount_curve_id.clone(),
                        self.discount_curve_id.clone(),
                        RateIndexConventions::for_index_with_currency(index, currency)
                            .ois_compounding
                            .unwrap_or(FloatingLegCompounding::sofr()),
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
                    freq: float_freq,
                    dc: float_dc,
                    bdc,
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
                let (primary_leg_conventions, reference_leg_conventions) = match quote {
                    RatesQuote::BasisSwap {
                        primary_leg_conventions,
                        reference_leg_conventions,
                        ..
                    } => (primary_leg_conventions, reference_leg_conventions),
                    _ => {
                        return Err(finstack_core::Error::Input(
                            finstack_core::error::InputError::Invalid,
                        ))
                    }
                };

                let basis_currency = conventions.currency.unwrap_or(currency);

                let primary_index = primary_leg_conventions.index.as_ref().ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "BasisSwap quote requires primary_leg_conventions.index to be set"
                            .to_string(),
                    )
                })?;
                let reference_index =
                    reference_leg_conventions.index.as_ref().ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "BasisSwap quote requires reference_leg_conventions.index to be set"
                                .to_string(),
                        )
                    })?;

                let primary_freq = primary_leg_conventions.payment_frequency.ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "Strict pricing requires primary_leg_conventions.payment_frequency to be set"
                            .to_string(),
                    )
                })?;
                let reference_freq = reference_leg_conventions
                    .payment_frequency
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires reference_leg_conventions.payment_frequency to be set"
                                .to_string(),
                        )
                    })?;

                let primary_dc = primary_leg_conventions.day_count.ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "Strict pricing requires primary_leg_conventions.day_count to be set"
                            .to_string(),
                    )
                })?;
                let reference_dc = reference_leg_conventions.day_count.ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "Strict pricing requires reference_leg_conventions.day_count to be set"
                            .to_string(),
                    )
                })?;

                let settlement_days = conventions
                    .settlement_days
                    .or(self.settlement_days)
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires settlement_days to be set (quote or step)"
                                .to_string(),
                        )
                    })?;

                let calendar_id = conventions
                    .calendar_id
                    .as_deref()
                    .or(self.calendar_id.as_deref())
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires calendar_id to be set (quote or step)"
                                .to_string(),
                        )
                    })?;

                let bdc = conventions
                    .business_day_convention
                    .or(self.business_day_convention)
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires business_day_convention to be set (quote or step)"
                                .to_string(),
                        )
                    })?;

                let payment_calendar_id = conventions
                    .payment_calendar_id
                    .as_deref()
                    .or(conventions.calendar_id.as_deref())
                    .or(self.calendar_id.as_deref())
                    .unwrap_or(calendar_id);

                let payment_delay_days = conventions
                    .payment_delay_days
                    .or(self.default_payment_delay_days)
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires payment_delay_days to be set (quote or step)"
                                .to_string(),
                        )
                    })?;

                let reset_lag_days = conventions
                    .reset_lag
                    .or(self.default_reset_lag_days)
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires reset_lag to be set (quote or step)"
                                .to_string(),
                        )
                    })?;

                let start = if self.use_settlement_start {
                    let conv_for_settlement = InstrumentConventions {
                        settlement_days: Some(settlement_days),
                        calendar_id: Some(calendar_id.to_string()),
                        business_day_convention: Some(bdc),
                        ..Default::default()
                    };
                    self.settlement_date_for_quote_explicit(&conv_for_settlement, basis_currency)?
                } else {
                    self.base_date
                };
                let basis_swap = BasisSwap::new(
                    format!("CALIB_BASIS_{}", maturity),
                    Money::new(1_000_000.0, basis_currency),
                    start,
                    *maturity,
                    BasisSwapLeg {
                        forward_curve_id: self.resolve_forward_curve_id(primary_index.as_str()),
                        frequency: primary_freq,
                        day_count: primary_dc,
                        bdc,
                        payment_lag_days: payment_delay_days,
                        reset_lag_days,
                        spread: *spread_bp / 10_000.0,
                    },
                    BasisSwapLeg {
                        forward_curve_id: self.resolve_forward_curve_id(reference_index.as_str()),
                        frequency: reference_freq,
                        day_count: reference_dc,
                        bdc,
                        payment_lag_days: payment_delay_days,
                        reset_lag_days,
                        spread: 0.0,
                    },
                    self.discount_curve_id.as_str(),
                )
                .with_allow_calendar_fallback(self.allow_calendar_fallback)
                .with_calendar(payment_calendar_id.to_string());
                let pv = basis_swap.value(context, self.base_date)?;
                Ok(pv.amount() / basis_swap.notional.amount())
            }
        }
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
}


