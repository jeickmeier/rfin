//! FRA and interest rate future metric calculators.
//!
//! Placeholder module to align with the `mod/metrics` layout used by other
//! fixed income instruments. Specific FRA or futures metrics can be added
//! incrementally without changing the module structure.

use crate::instruments::fixed_income::fra::ForwardRateAgreement;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::F;
use std::sync::Arc;

/// PV calculator for FRA returning base value from context
pub struct FraPvCalculator;

impl MetricCalculator for FraPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        Ok(context.base_value.amount())
    }
}

/// DV01 calculator for FRA using analytic approximation: notional * tau * DF_start * 1bp
pub struct FraDv01Calculator;

impl MetricCalculator for FraDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fra: &ForwardRateAgreement = context.instrument_as()?;

        // use finstack_core::market_data::traits::Discounting;
        let disc = context.curves.discount_ref(fra.disc_id)?;
        let base = disc.base_date();

        // Settlement at start of period
        let _t_start = fra
            .day_count
            .year_fraction(
                base,
                fra.start_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let tau = fra
            .day_count
            .year_fraction(
                fra.start_date,
                fra.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        if tau <= 0.0 {
            return Ok(0.0);
        }

        let df_start = DiscountCurve::df_on(disc, base, fra.start_date, fra.day_count);
        let dv01 = fra.notional.amount() * tau * df_start * 1e-4;

        // Sign: Receive-fixed has positive DV01; Pay-fixed negative
        Ok(if fra.pay_fixed { -dv01 } else { dv01 })
    }
}

/// Registers FRA and interest rate future metrics.
///
/// Currently no FRA-specific metrics are defined; this function exists to
/// maintain a consistent registration surface across instruments.
pub fn register_fra_metrics(registry: &mut MetricRegistry) {
    registry
        .register_metric(
            MetricId::custom("fra_pv"),
            Arc::new(FraPvCalculator),
            &["FRA"],
        )
        .register_metric(
            MetricId::custom("fra_dv01"),
            Arc::new(FraDv01Calculator),
            &["FRA"],
        );
}
