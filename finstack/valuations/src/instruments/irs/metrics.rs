#![deny(missing_docs)]
//! Interest rate swap specific metric calculators.

use crate::instruments::Instrument;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use super::PayReceive;
use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;


/// Calculates annuity (sum of discounted year fractions) for IRS.
pub struct AnnuityCalculator;

impl MetricCalculator for AnnuityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let irs = match &*context.instrument {
            Instrument::IRS(irs) => irs,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        let disc = context.curves.discount(irs.fixed.disc_id)?;
        let base = disc.base_date();
        
        // Build fixed leg schedule
        let builder = finstack_core::dates::ScheduleBuilder::new(irs.fixed.start, irs.fixed.end)
            .frequency(irs.fixed.freq)
            .stub_rule(irs.fixed.stub);
        
        let schedule: Vec<Date> = if let Some(id) = irs.fixed.calendar_id {
            if let Some(cal) = finstack_core::dates::holiday::calendars::calendar_by_id(id) {
                builder.adjust_with(irs.fixed.bdc, cal).build().collect()
            } else {
                builder.build_raw().collect()
            }
        } else {
            builder.build_raw().collect()
        };
        
        if schedule.len() < 2 {
            return Ok(0.0);
        }
        
        // Compute annuity as sum(yf * df)
        let mut annuity = 0.0;
        let mut prev = schedule[0];
        for &d in &schedule[1..] {
            let yf = DiscountCurve::year_fraction(prev, d, irs.fixed.dc);
            let df = DiscountCurve::df_on(&*disc, base, d, irs.fixed.dc);
            annuity += yf * df;
            prev = d;
        }
        
        Ok(annuity)
    }
}

/// Calculates par rate for IRS.
pub struct ParRateCalculator;

impl MetricCalculator for ParRateCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Annuity]
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let irs = match &*context.instrument {
            Instrument::IRS(irs) => irs,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        let disc = context.curves.discount(irs.fixed.disc_id)?;
        let fwd = context.curves.forecast(irs.float.fwd_id)?;
        let base_d = disc.base_date();
        
        // Get annuity from computed metrics
        let annuity = context.computed.get(&MetricId::Annuity).copied().unwrap_or(0.0);
        if annuity == 0.0 {
            return Ok(0.0);
        }
        
        // Compute PV of float leg
        let builder = finstack_core::dates::ScheduleBuilder::new(irs.float.start, irs.float.end)
            .frequency(irs.float.freq)
            .stub_rule(irs.float.stub);
        
        let float_schedule: Vec<Date> = if let Some(id) = irs.float.calendar_id {
            if let Some(cal) = finstack_core::dates::holiday::calendars::calendar_by_id(id) {
                builder.adjust_with(irs.float.bdc, cal).build().collect()
            } else {
                builder.build_raw().collect()
            }
        } else {
            builder.build_raw().collect()
        };
        
        if float_schedule.len() < 2 {
            return Ok(0.0);
        }
        
        let mut float_pv = 0.0;
        let mut prev = float_schedule[0];
        for &d in &float_schedule[1..] {
            let t1 = DiscountCurve::year_fraction(base_d, prev, irs.float.dc);
            let t2 = DiscountCurve::year_fraction(base_d, d, irs.float.dc);
            let yf = DiscountCurve::year_fraction(prev, d, irs.float.dc);
            let f = fwd.rate_period(t1, t2);
            let rate = f + (irs.float.spread_bp * 1e-4);
            let coupon = irs.notional.amount() * rate * yf;
            let df = DiscountCurve::df_on(&*disc, base_d, d, irs.float.dc);
            float_pv += coupon * df;
            prev = d;
        }
        
        // Par rate = float_pv / (notional * annuity)
        Ok(float_pv / irs.notional.amount() / annuity)
    }
}

/// Calculates DV01 (dollar value of 1 basis point) for IRS.
pub struct Dv01Calculator;

impl MetricCalculator for Dv01Calculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Annuity]
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let irs = match &*context.instrument {
            Instrument::IRS(irs) => irs,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        // Get annuity from computed metrics
        let annuity = context.computed.get(&MetricId::Annuity).copied().unwrap_or(0.0);
        
        // DV01 = annuity * notional * 1bp, with sign based on pay/receive
        let dv01_magnitude = annuity * irs.notional.amount() * 1e-4;
        
        let dv01 = match irs.side {
            PayReceive::ReceiveFixed => dv01_magnitude,
            PayReceive::PayFixed => -dv01_magnitude,
        };
        
        Ok(dv01)
    }
}

/// Calculates PV of fixed leg for IRS.
pub struct FixedLegPvCalculator;

impl MetricCalculator for FixedLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let irs = match &*context.instrument {
            Instrument::IRS(irs) => irs,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        let disc = context.curves.discount(irs.fixed.disc_id)?;
        let base = disc.base_date();
        
        // Build fixed leg schedule and compute PV
        let builder = finstack_core::dates::ScheduleBuilder::new(irs.fixed.start, irs.fixed.end)
            .frequency(irs.fixed.freq)
            .stub_rule(irs.fixed.stub);
        
        let schedule: Vec<Date> = if let Some(id) = irs.fixed.calendar_id {
            if let Some(cal) = finstack_core::dates::holiday::calendars::calendar_by_id(id) {
                builder.adjust_with(irs.fixed.bdc, cal).build().collect()
            } else {
                builder.build_raw().collect()
            }
        } else {
            builder.build_raw().collect()
        };
        
        if schedule.len() < 2 {
            return Ok(0.0);
        }
        
        let mut pv = 0.0;
        let mut prev = schedule[0];
        for &d in &schedule[1..] {
            let yf = DiscountCurve::year_fraction(prev, d, irs.fixed.dc);
            let coupon = irs.notional.amount() * irs.fixed.rate * yf;
            let df = DiscountCurve::df_on(&*disc, base, d, irs.fixed.dc);
            pv += coupon * df;
            prev = d;
        }
        
        Ok(pv)
    }
}

/// Calculates PV of floating leg for IRS.
pub struct FloatLegPvCalculator;

impl MetricCalculator for FloatLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let irs = match &*context.instrument {
            Instrument::IRS(irs) => irs,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        let disc = context.curves.discount(irs.float.disc_id)?;
        let fwd = context.curves.forecast(irs.float.fwd_id)?;
        let base = disc.base_date();
        
        // Build float leg schedule and compute PV
        let builder = finstack_core::dates::ScheduleBuilder::new(irs.float.start, irs.float.end)
            .frequency(irs.float.freq)
            .stub_rule(irs.float.stub);
        
        let schedule: Vec<Date> = if let Some(id) = irs.float.calendar_id {
            if let Some(cal) = finstack_core::dates::holiday::calendars::calendar_by_id(id) {
                builder.adjust_with(irs.float.bdc, cal).build().collect()
            } else {
                builder.build_raw().collect()
            }
        } else {
            builder.build_raw().collect()
        };
        
        if schedule.len() < 2 {
            return Ok(0.0);
        }
        
        let mut pv = 0.0;
        let mut prev = schedule[0];
        for &d in &schedule[1..] {
            let t1 = DiscountCurve::year_fraction(base, prev, irs.float.dc);
            let t2 = DiscountCurve::year_fraction(base, d, irs.float.dc);
            let yf = DiscountCurve::year_fraction(prev, d, irs.float.dc);
            let f = fwd.rate_period(t1, t2);
            let rate = f + (irs.float.spread_bp * 1e-4);
            let coupon = irs.notional.amount() * rate * yf;
            let df = DiscountCurve::df_on(&*disc, base, d, irs.float.dc);
            pv += coupon * df;
            prev = d;
        }
        
        Ok(pv)
    }
}

/// Register all IRS metrics to a registry.
pub fn register_irs_metrics(registry: &mut crate::metrics::MetricRegistry) {
    use std::sync::Arc;
    use crate::metrics::MetricId;
    
    registry
        .register_metric(MetricId::Annuity, Arc::new(AnnuityCalculator), &["IRS"])
        .register_metric(MetricId::ParRate, Arc::new(ParRateCalculator), &["IRS"])
        .register_metric(MetricId::Dv01, Arc::new(Dv01Calculator), &["IRS"])
        .register_metric(MetricId::PvFixed, Arc::new(FixedLegPvCalculator), &["IRS"])
        .register_metric(MetricId::PvFloat, Arc::new(FloatLegPvCalculator), &["IRS"]);
}
