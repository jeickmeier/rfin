//! Futures pricing helpers for `CalibrationPricer`.

use super::CalibrationPricer;
use crate::calibration::quotes::{FutureSpecs, InstrumentConventions};
use crate::instruments::common::traits::Instrument;
use crate::instruments::ir_future::InterestRateFuture;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::Currency;
use finstack_core::Result;

impl CalibrationPricer {
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
            self.resolve_future_convexity(specs, currency, time_to_expiry, time_to_maturity)?;

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
                convexity_adjustment: Some(convexity_adj),
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

    pub(in crate::calibration::pricing::pricer) fn resolve_future_convexity(
        &self,
        specs: &FutureSpecs,
        currency: Currency,
        time_to_expiry: f64,
        time_to_maturity: f64,
    ) -> Result<f64> {
        if let Some(adj) = specs.convexity_adjustment {
            return Ok(adj);
        }

        let params = match &self.conventions.convexity_params {
            Some(p) => p.clone(),
            None => {
                if self.conventions.strict_pricing.unwrap_or(false) {
                    return Err(finstack_core::Error::Validation(
                        "Strict pricing requires futures convexity_params (step-level) or specs.convexity_adjustment (quote-level)".to_string(),
                    ));
                }
                super::super::convexity::ConvexityParameters::for_currency(currency)
            }
        };

        // In strict mode, prohibit "market implied" fallback to internal defaults.
        // If the caller requests MarketImplied but does not supply an explicit vol,
        // fail fast rather than silently using base_volatility heuristics.
        if self.conventions.strict_pricing.unwrap_or(false)
            && matches!(
                params.vol_source,
                super::super::convexity::VolatilitySource::MarketImplied { .. }
            )
            && specs.market_implied_vol.is_none()
        {
            return Err(finstack_core::Error::Validation(
                "Strict pricing requires FutureSpecs.market_implied_vol when convexity_params.vol_source=MarketImplied".to_string(),
            ));
        }

        Ok(params.calculate_adjustment_with_market_vol(
            time_to_expiry,
            time_to_maturity,
            specs.market_implied_vol,
        ))
    }

    pub(in crate::calibration::pricing::pricer) fn future_notional(
        &self,
        specs: &FutureSpecs,
        currency: Currency,
    ) -> Money {
        Money::new(specs.face_value * specs.multiplier, currency)
    }
}
