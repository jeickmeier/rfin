//! Quote → Instrument factory for calibration pricing.
//!
//! Centralizes how calibration quotes are mapped to concrete instruments so
//! pricing uses a single construction path before calling `Instrument::value`.

use super::convention_resolution as conv;
use super::pricer::CalibrationPricer;
use crate::calibration::api::schema::HazardCurveParams;
use crate::calibration::quotes::{CreditQuote, RatesQuote};
use crate::instruments::basis_swap::{BasisSwap, BasisSwapLeg};
use crate::instruments::cds::CreditDefaultSwapBuilder;
use crate::instruments::common::traits::Instrument;
use crate::instruments::deposit::Deposit;
use crate::instruments::fra::ForwardRateAgreement;
use crate::instruments::ir_future::InterestRateFuture;
use crate::instruments::irs::{FloatingLegCompounding, IrsLegConventions, PayReceive};
use crate::instruments::InterestRateSwap;
use finstack_core::dates::StubKind;
use finstack_core::money::Money;
use finstack_core::types::Currency;
use finstack_core::Result;
use std::sync::Arc;

/// Notional used for calibration instruments (PV is normalized by this).
pub(crate) const CALIBRATION_NOTIONAL: f64 = 1_000_000.0;

/// Build a rates instrument (deposit/FRA/future/swap/basis swap) for a quote.
///
/// Maps generic [`MarketQuote`] types to concrete pricing instruments
/// compatible with the [`Instrument`] trait.
///
/// `strict` mirrors the previous strict pricing path (requires explicit
/// conventions and fails fast when missing).
pub(crate) fn build_instrument_for_rates_quote(
    pricer: &CalibrationPricer,
    quote: &RatesQuote,
    currency: Currency,
    strict: bool,
) -> Result<Arc<dyn Instrument>> {
    match quote {
        RatesQuote::Deposit {
            maturity,
            rate,
            conventions,
        } => {
            let day_count = if strict {
                conventions.day_count.ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "Deposit quote requires conventions.day_count to be set in strict pricing"
                            .to_string(),
                    )
                })?
            } else {
                conventions.day_count.unwrap_or_else(|| {
                    crate::calibration::quotes::conventions::DepositConventions::for_currency_or_index(
                        currency,
                        conventions.index.as_ref(),
                    )
                    .day_count
                })
            };

            let start = pricer.effective_start_date(conventions, currency)?;
            let common = conv::resolve_common(pricer, conventions, currency);

            let dep = Deposit {
                id: format!("CALIB_DEP_{}", maturity).into(),
                notional: Money::new(CALIBRATION_NOTIONAL, currency),
                start,
                end: *maturity,
                day_count,
                quote_rate: Some(*rate),
                discount_curve_id: pricer.discount_curve_id.clone(),
                attributes: Default::default(),
                spot_lag_days: if pricer.conventions.use_settlement_start.unwrap_or(true)
                    && common.settlement_days != 0
                {
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

            Ok(Arc::new(dep))
        }

        RatesQuote::FRA {
            start,
            end,
            rate,
            conventions,
        } => {
            let day_count = if strict {
                conventions.day_count.ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "FRA quote requires conventions.day_count to be set in strict pricing"
                            .to_string(),
                    )
                })?
            } else {
                conventions.day_count.unwrap_or_else(|| {
                    crate::calibration::quotes::conventions::FraConventions::for_currency(currency)
                        .day_count
                })
            };

            let common = conv::resolve_common(pricer, conventions, currency);
            let reset_lag = common.reset_lag_days;
            let calendar_id = common.fixing_calendar_id;

            let (fixing_date, calendar_found) = pricer.compute_fra_fixing_date(
                *start,
                reset_lag,
                calendar_id,
                pricer.conventions.allow_calendar_fallback.unwrap_or(false),
            )?;

            let fixing_calendar_id_opt = if calendar_found {
                Some(calendar_id.to_string())
            } else {
                None
            };

            let fra = ForwardRateAgreement::builder()
                .id(format!("CALIB_FRA_{}_{}", start, end).into())
                .notional(Money::new(CALIBRATION_NOTIONAL, currency))
                .fixing_date(fixing_date)
                .start_date(*start)
                .end_date(*end)
                .fixed_rate(*rate)
                .day_count(day_count)
                .reset_lag(reset_lag)
                .fixing_calendar_id_opt(fixing_calendar_id_opt)
                .discount_curve_id(pricer.discount_curve_id.clone())
                .forward_id(pricer.forward_curve_id.clone())
                .pay_fixed(false)
                .build()
                .map_err(|e| finstack_core::Error::Calibration {
                    message: format!("FRA builder failed for {}: {}", end, e),
                    category: "yield_curve_bootstrap".to_string(),
                })?;

            Ok(Arc::new(fra))
        }

        RatesQuote::Future {
            expiry,
            period_start,
            period_end,
            fixing_date,
            price,
            specs,
            ..
        } => {
            let specs = specs
                .clone()
                .unwrap_or_else(|| crate::calibration::quotes::rates::FutureSpecs::default_for_currency(currency));
            let fixing_date = fixing_date.unwrap_or(*period_start);

            let dc_ctx = finstack_core::dates::DayCountCtx::default();
            let time_to_expiry = specs
                .day_count
                .year_fraction(pricer.base_date, *expiry, dc_ctx)
                .unwrap_or(0.0);
            let time_to_maturity = specs
                .day_count
                .year_fraction(pricer.base_date, *period_end, dc_ctx)
                .unwrap_or(0.0);

            let convexity_adj = pricer.resolve_future_convexity(
                &specs,
                currency,
                time_to_expiry,
                time_to_maturity,
            )?;

            let future = InterestRateFuture::builder()
                .id(format!("CALIB_FUT_{}", expiry).into())
                .notional(Money::new(specs.face_value * specs.multiplier, currency))
                .expiry_date(*expiry)
                .fixing_date(fixing_date)
                .period_start(*period_start)
                .period_end(*period_end)
                .quoted_price(*price)
                .day_count(specs.day_count)
                .position(crate::instruments::ir_future::Position::Long)
                .contract_specs(crate::instruments::ir_future::FutureContractSpecs {
                    face_value: specs.face_value,
                    tick_size: specs.tick_size,
                    tick_value: specs.tick_value,
                    delivery_months: specs.delivery_months,
                    convexity_adjustment: Some(convexity_adj),
                })
                .discount_curve_id(pricer.discount_curve_id.clone())
                .forward_id(pricer.forward_curve_id.clone())
                .build()
                .map_err(|e| finstack_core::Error::Calibration {
                    message: format!("IRFuture builder failed for expiry {}: {}", expiry, e),
                    category: "yield_curve_bootstrap".to_string(),
                })?;

            Ok(Arc::new(future))
        }

        RatesQuote::Swap { maturity, rate, .. } => {
            let resolved = conv::resolve_swap_conventions(pricer, quote, currency)?;
            let start = if pricer.conventions.use_settlement_start.unwrap_or(true) {
                pricer.settlement_date_from_components(
                    resolved.common.settlement_days,
                    resolved.common.calendar_id,
                    resolved.common.bdc,
                    currency,
                )?
            } else {
                pricer.base_date
            };
            let payment_delay = resolved.common.payment_delay_days;
            let reset_lag = resolved.common.reset_lag_days;
            let payment_calendar_id = resolved.common.payment_calendar_id.to_string();
            let fixing_calendar_id = resolved.common.fixing_calendar_id.to_string();

            let index_conv =
                crate::calibration::quotes::rate_index::RateIndexConventions::require_for_index(
                    resolved.index,
                )?;
            let use_compounding = index_conv.kind
                == crate::calibration::quotes::rate_index::RateIndexKind::OvernightRfr;

            let compounding = if use_compounding {
                index_conv
                    .ois_compounding
                    .clone()
                    .unwrap_or_else(FloatingLegCompounding::sofr)
            } else {
                FloatingLegCompounding::Simple
            };

            let (float_discount_id, float_forward_id) = if use_compounding {
                let proj_id = if pricer.tenor_years.is_none() {
                    pricer.discount_curve_id.clone()
                } else {
                    pricer.forward_curve_id.clone()
                };
                (pricer.discount_curve_id.clone(), proj_id)
            } else {
                (
                    pricer.discount_curve_id.clone(),
                    pricer.forward_curve_id.clone(),
                )
            };

            let conventions = IrsLegConventions {
                fixed_freq: resolved.fixed_freq,
                float_freq: resolved.float_freq,
                fixed_dc: resolved.fixed_dc,
                float_dc: resolved.float_dc,
                bdc: resolved.common.bdc,
                payment_calendar_id: Some(payment_calendar_id),
                fixing_calendar_id: Some(fixing_calendar_id),
                stub: StubKind::None,
                reset_lag_days: reset_lag,
                payment_delay_days: payment_delay,
            };

            let id = format!("CALIB_SWAP_{}", maturity).into();
            let notional = Money::new(CALIBRATION_NOTIONAL, currency);
            let side = PayReceive::ReceiveFixed;

            let swap = if use_compounding {
                InterestRateSwap::create_ois_swap_with_conventions(
                    id,
                    notional,
                    *rate,
                    start,
                    *maturity,
                    side,
                    float_discount_id,
                    float_forward_id,
                    compounding,
                    conventions,
                )?
            } else {
                InterestRateSwap::create_term_swap_with_conventions(
                    id,
                    notional,
                    *rate,
                    start,
                    *maturity,
                    side,
                    float_discount_id,
                    float_forward_id,
                    conventions,
                )?
            };

            Ok(Arc::new(swap))
        }

        RatesQuote::BasisSwap {
            maturity,
            spread_bp,
            conventions,
            ..
        } => {
            let resolved = conv::resolve_basis_swap_conventions(pricer, quote, currency)?;

            let strict = pricer.conventions.strict_pricing.unwrap_or(false);
            let primary_index_conv =
                crate::calibration::quotes::rate_index::RateIndexConventions::require_for_index(
                    resolved.primary_index,
                )?;

            let settlement_days = if strict {
                conventions
                    .settlement_days
                    .or(pricer.conventions.settlement_days)
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires settlement_days to be set (quote or step)"
                                .to_string(),
                        )
                    })?
            } else {
                conventions
                    .settlement_days
                    .or(pricer.conventions.settlement_days)
                    .unwrap_or(primary_index_conv.market_settlement_days)
            };

            let calendar_id = if strict {
                conventions
                    .calendar_id
                    .as_deref()
                    .or(pricer.conventions.calendar_id.as_deref())
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires calendar_id to be set (quote or step)"
                                .to_string(),
                        )
                    })?
            } else {
                conventions
                    .calendar_id
                    .as_deref()
                    .or(pricer.conventions.calendar_id.as_deref())
                    .unwrap_or(primary_index_conv.market_calendar_id.as_str())
            };

            let bdc = if strict {
                conventions
                    .business_day_convention
                    .or(pricer.conventions.business_day_convention)
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Strict pricing requires business_day_convention to be set (quote or step)"
                                .to_string(),
                        )
                    })?
            } else {
                conventions
                    .business_day_convention
                    .or(pricer.conventions.business_day_convention)
                    .unwrap_or(primary_index_conv.market_business_day_convention)
            };

            let payment_calendar_id = conventions
                .payment_calendar_id
                .as_deref()
                .or(conventions.calendar_id.as_deref())
                .or(pricer.conventions.calendar_id.as_deref())
                .unwrap_or(calendar_id);

            let payment_delay_days = conventions
                .payment_delay_days
                .or(pricer.conventions.default_payment_delay_days)
                .unwrap_or(0);

            let reset_lag_days = conventions
                .reset_lag
                .or(pricer.conventions.default_reset_lag_days)
                .unwrap_or(0);

            let start = if pricer.conventions.use_settlement_start.unwrap_or(true) {
                pricer.settlement_date_from_components(settlement_days, calendar_id, bdc, resolved.currency)?
            } else {
                pricer.base_date
            };

            let basis_swap = BasisSwap::new(
                format!("CALIB_BASIS_{}", maturity),
                Money::new(CALIBRATION_NOTIONAL, resolved.currency),
                start,
                *maturity,
                BasisSwapLeg {
                    forward_curve_id: pricer
                        .resolve_forward_curve_id(resolved.primary_index.as_str()),
                    frequency: resolved.primary_freq,
                    day_count: resolved.primary_dc,
                    bdc,
                    payment_lag_days: payment_delay_days,
                    reset_lag_days,
                    spread: *spread_bp / 10_000.0,
                },
                BasisSwapLeg {
                    forward_curve_id: pricer
                        .resolve_forward_curve_id(resolved.reference_index.as_str()),
                    frequency: resolved.reference_freq,
                    day_count: resolved.reference_dc,
                    bdc,
                    payment_lag_days: payment_delay_days,
                    reset_lag_days,
                    spread: 0.0,
                },
                pricer.discount_curve_id.as_str(),
            )
            .with_allow_calendar_fallback(
                pricer.conventions.allow_calendar_fallback.unwrap_or(false),
            )
            .with_calendar(payment_calendar_id.to_string());

            Ok(Arc::new(basis_swap))
        }
    }
}

/// Build a CDS instrument for hazard calibration.
///
/// Returns the instrument plus an optional upfront [`Money`] to subtract from PV
/// when handling `CDSUpfront` residuals.
pub(crate) fn build_instrument_for_credit_quote(
    quote: &CreditQuote,
    params: &HazardCurveParams,
    cds_conventions: &crate::instruments::cds::CdsConventionResolved,
) -> Result<(Arc<dyn Instrument>, Option<Money>)> {
    let (maturity, spread_bp, upfront_pct_opt, conventions) = match quote {
        CreditQuote::CDS {
            maturity,
            spread_bp,
            conventions,
            ..
        } => (*maturity, *spread_bp, None, conventions),
        CreditQuote::CDSUpfront {
            maturity,
            running_spread_bp,
            upfront_pct,
            conventions,
            ..
        } => (
            *maturity,
            *running_spread_bp,
            Some(*upfront_pct),
            conventions,
        ),
        _ => {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            ))
        }
    };

    let premium_spec = crate::instruments::cds::PremiumLegSpec {
        start: params.base_date,
        end: maturity,
        freq: conventions
            .payment_frequency
            .unwrap_or(cds_conventions.frequency),
        stub: cds_conventions.stub_convention,
        bdc: conventions
            .business_day_convention
            .unwrap_or(cds_conventions.business_day_convention),
        calendar_id: Some(
            conventions
                .effective_payment_calendar_id()
                .unwrap_or(cds_conventions.default_calendar_id.as_str())
                .to_string(),
        ),
        dc: conventions.day_count.unwrap_or(cds_conventions.day_count),
        spread_bp,
        discount_curve_id: params.discount_curve_id.clone(),
    };

    let protection_spec = crate::instruments::cds::ProtectionLegSpec {
        credit_curve_id: params.curve_id.clone(),
        recovery_rate: params.recovery_rate,
        settlement_delay: conventions
            .settlement_days
            .unwrap_or(cds_conventions.settlement_delay_days as i32) as u16,
    };

    let cds = CreditDefaultSwapBuilder::new()
        .id("CALIB_CDS".into())
        .notional(Money::new(params.notional, params.currency))
        .side(crate::instruments::cds::PayReceive::PayFixed)
        .convention(cds_conventions.doc_clause)
        .premium(premium_spec)
        .protection(protection_spec)
        .pricing_overrides(crate::instruments::PricingOverrides::default())
        .attributes(crate::instruments::common::traits::Attributes::new())
        .build()
        .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;

    let upfront_cash =
        upfront_pct_opt.map(|pct| Money::new(params.notional * pct / 100.0, params.currency));

    Ok((Arc::new(cds), upfront_cash))
}
