//! Bond-specific metric calculators.

use crate::instruments::{Bond, Instrument};
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use crate::traits::CashflowProvider;
use finstack_core::prelude::*;
use finstack_core::F;

/// Calculates accrued interest for bonds.
pub struct AccruedInterestCalculator;

impl MetricCalculator for AccruedInterestCalculator {
    fn id(&self) -> MetricId {
        MetricId::Accrued
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        // Extract bond data first to avoid borrowing issues
        let (flows, disc_id, dc) = {
            let bond = match context.instrument() {
                Instrument::Bond(bond) => bond,
                _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
            };
            
            // Use Bond's cashflow building instead of separate schedule
            let flows = bond.build_schedule(&context.market_data.curves, context.market_data.as_of)?;
            (flows, bond.disc_id, bond.dc)
        };
        
        // Cache flows for other metrics (including risk metrics)
        context.cache_value("cashflows", flows.clone());
        context.cache_value("discount_curve_id", disc_id);
        context.cache_value("day_count", dc);
        
        // Extract coupon dates from flows - filter for positive flows (coupons) 
        let mut coupon_dates: Vec<Date> = flows.iter()
            .filter(|(_, amount)| amount.amount() > 0.0)
            .map(|(date, _)| *date)
            .collect();
        coupon_dates.sort();
        
        // Get bond again for the calculation
        let bond = match context.instrument() {
            Instrument::Bond(bond) => bond,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        // Find last and next coupon dates around as_of
        let (mut last, mut next) = (bond.issue, bond.maturity);
        for w in coupon_dates.windows(2) {
            let (a, b) = (w[0], w[1]);
            if a <= context.market_data.as_of && context.market_data.as_of < b {
                last = a;
                next = b;
                break;
            }
        }
        
        // Calculate accrued interest directly
        if context.market_data.as_of <= last || context.market_data.as_of >= next {
            return Ok(0.0);
        }
        
        use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
        let yf = DiscountCurve::year_fraction(last, next, bond.dc);
        let elapsed = DiscountCurve::year_fraction(last, context.market_data.as_of, bond.dc);
        let period_coupon = bond.notional * (bond.coupon * yf);
        let accrued = period_coupon * (elapsed / yf);
        
        Ok(accrued.amount())
    }
}

/// Calculates yield to maturity for bonds.
pub struct YtmCalculator;

impl MetricCalculator for YtmCalculator {
    fn id(&self) -> MetricId {
        MetricId::Ytm
    }
    
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let bond = match context.instrument() {
            Instrument::Bond(bond) => bond,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        // YTM only makes sense if we have a quoted clean price
        let clean_px = bond.quoted_clean
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        // Get accrued from computed metrics
        let ai = context.cache.computed.get(&MetricId::Accrued).copied().unwrap_or(0.0);
        
        // Compute dirty price
        let dirty_amt = clean_px + ai;
        let dirty = Money::new(dirty_amt, bond.notional.currency());
        
        // Use Bond's cashflow building
        let flows = bond.build_schedule(&context.market_data.curves, context.market_data.as_of)?;
        
        // Solve for YTM using Brent's method
        let ytm = self.solve_ytm_from_flows(bond, &flows, context.market_data.as_of, dirty)?;
        
        Ok(ytm)
    }
}

impl YtmCalculator {
    fn solve_ytm_from_flows(&self, bond: &Bond, flows: &[(Date, Money)], as_of: Date, target_price: Money) -> finstack_core::Result<F> {
        use finstack_core::math::root_finding::brent;
        use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
        
        let target = target_price.amount();
        
        let f = |y: f64| -> f64 {
            let mut pv = 0.0;
            for &(date, amount) in flows {
                if date <= as_of {
                    continue;
                }
                let t = DiscountCurve::year_fraction(as_of, date, bond.dc);
                if t > 0.0 {
                    let df = (1.0 + y).powf(-t);
                    pv += amount.amount() * df;
                }
            }
            pv - target
        };
        
        // Try bracket [-0.99, 1.0] first, then widen if needed
        let mut a = -0.99;
        let mut b = 1.0;
        let mut root = brent(f, a, b, 1e-10, 128).unwrap_or(0.05);
        
        if !root.is_finite() {
            a = -0.99; 
            b = 5.0;
            root = brent(f, a, b, 1e-10, 256).unwrap_or(0.05);
        }
        
        Ok(if root.is_finite() { root } else { 0.05 })
    }
}

/// Calculates Macaulay duration for bonds.
pub struct MacaulayDurationCalculator;

impl MetricCalculator for MacaulayDurationCalculator {
    fn id(&self) -> MetricId {
        MetricId::DurationMac
    }
    
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let bond = match context.instrument() {
            Instrument::Bond(bond) => bond,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        let ytm = context.cache.computed.get(&MetricId::Ytm).copied()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        // Use Bond's cashflow building
        let flows = bond.build_schedule(&context.market_data.curves, context.market_data.as_of)?;
        
        // Calculate price from flows to ensure consistency
        let price = self.price_from_ytm(bond, &flows, context.market_data.as_of, ytm)?;
        if price == 0.0 {
            return Ok(0.0);
        }
        
        // Calculate Macaulay duration
        use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
        let mut weighted_time = 0.0;
        
        for &(date, amount) in &flows {
            if date <= context.market_data.as_of {
                continue;
            }
            let t = DiscountCurve::year_fraction(context.market_data.as_of, date, bond.dc).max(0.0);
            let df = (1.0 + ytm).powf(-t);
            weighted_time += t * amount.amount() * df;
        }
        
        Ok(weighted_time / price)
    }
}

impl MacaulayDurationCalculator {
    fn price_from_ytm(&self, bond: &Bond, flows: &[(Date, Money)], as_of: Date, ytm: F) -> finstack_core::Result<F> {
        use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
        
        let mut pv = 0.0;
        for &(date, amount) in flows {
            if date <= as_of {
                continue;
            }
            let t = DiscountCurve::year_fraction(as_of, date, bond.dc);
            if t > 0.0 {
                let df = (1.0 + ytm).powf(-t);
                pv += amount.amount() * df;
            }
        }
        Ok(pv)
    }
}

/// Calculates modified duration for bonds.
pub struct ModifiedDurationCalculator;

impl MetricCalculator for ModifiedDurationCalculator {
    fn id(&self) -> MetricId {
        MetricId::DurationMod
    }
    
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::DurationMac]
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let _bond = match context.instrument() {
            Instrument::Bond(bond) => bond,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        let ytm = context.cache.computed.get(&MetricId::Ytm).copied()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        let d_mac = context.cache.computed.get(&MetricId::DurationMac).copied()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        // Modified duration = Macaulay duration / (1 + ytm)
        Ok(d_mac / (1.0 + ytm))
    }
}

/// Calculates convexity for bonds.
pub struct ConvexityCalculator;

impl MetricCalculator for ConvexityCalculator {
    fn id(&self) -> MetricId {
        MetricId::Convexity
    }
    
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let bond = match context.instrument() {
            Instrument::Bond(bond) => bond,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        let ytm = context.cache.computed.get(&MetricId::Ytm).copied()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        // Use Bond's cashflow building
        let flows = bond.build_schedule(&context.market_data.curves, context.market_data.as_of)?;
        
        let dy = 1e-4; // 1 basis point for numerical differentiation
        
        // Calculate prices with yield bumps for numerical convexity
        let p0 = self.price_from_ytm(bond, &flows, context.market_data.as_of, ytm)?;
        let p_up = self.price_from_ytm(bond, &flows, context.market_data.as_of, ytm + dy)?;
        let p_dn = self.price_from_ytm(bond, &flows, context.market_data.as_of, ytm - dy)?;
        
        if p0 == 0.0 || dy == 0.0 {
            return Ok(0.0);
        }
        
        // Convexity = (P+ + P- - 2*P0) / (P0 * dy^2)
        Ok((p_up + p_dn - 2.0 * p0) / (p0 * dy * dy))
    }
}

impl ConvexityCalculator {
    fn price_from_ytm(&self, bond: &Bond, flows: &[(Date, Money)], as_of: Date, ytm: F) -> finstack_core::Result<F> {
        use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
        
        let mut pv = 0.0;
        for &(date, amount) in flows {
            if date <= as_of {
                continue;
            }
            let t = DiscountCurve::year_fraction(as_of, date, bond.dc);
            if t > 0.0 {
                let df = (1.0 + ytm).powf(-t);
                pv += amount.amount() * df;
            }
        }
        Ok(pv)
    }
}

/// Calculates yield-to-worst for bonds with call/put schedules.
pub struct YtwCalculator;

impl MetricCalculator for YtwCalculator {
    fn id(&self) -> MetricId {
        MetricId::Ytw
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let bond = match context.instrument() {
            Instrument::Bond(bond) => bond,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        let cp = bond.call_put.as_ref()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        // Use Bond's cashflow building
        let flows = bond.build_schedule(&context.market_data.curves, context.market_data.as_of)?;
        
        // Build candidate exercise dates
        let mut candidates: Vec<(Date, Money)> = Vec::new();
        for c in &cp.calls {
            if c.date >= context.market_data.as_of && c.date <= bond.maturity {
                let redemption = bond.notional * (c.price_pct_of_par / 100.0);
                candidates.push((c.date, redemption));
            }
        }
        for p in &cp.puts {
            if p.date >= context.market_data.as_of && p.date <= bond.maturity {
                let redemption = bond.notional * (p.price_pct_of_par / 100.0);
                candidates.push((p.date, redemption));
            }
        }
        // Always include maturity
        candidates.push((bond.maturity, bond.notional));
        
        // Get current dirty price from PV
        let dirty_now = context.base_value;
        
        // Find worst yield
        let mut best_ytm = f64::INFINITY;
        for (exercise_date, redemption) in candidates {
            let y = self.solve_ytm_with_exercise(bond, &flows, context.market_data.as_of, dirty_now, exercise_date, redemption)?;
            
            if y < best_ytm {
                best_ytm = y;
            }
        }
        
        Ok(best_ytm)
    }
}

impl YtwCalculator {
    fn solve_ytm_with_exercise(&self, bond: &Bond, flows: &[(Date, Money)], as_of: Date, target_price: Money, exercise_date: Date, redemption: Money) -> finstack_core::Result<F> {
        use finstack_core::math::root_finding::brent;
        use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
        
        let target = target_price.amount();
        
        let f = |y: f64| -> f64 {
            let mut pv = 0.0;
            
            // Include cashflows up to exercise date
            for &(date, amount) in flows {
                if date <= as_of || date > exercise_date {
                    continue;
                }
                let t = DiscountCurve::year_fraction(as_of, date, bond.dc);
                if t > 0.0 {
                    let df = (1.0 + y).powf(-t);
                    pv += amount.amount() * df;
                }
            }
            
            // Add redemption at exercise date
            let t_exercise = DiscountCurve::year_fraction(as_of, exercise_date, bond.dc);
            if t_exercise > 0.0 {
                let df_exercise = (1.0 + y).powf(-t_exercise);
                pv += redemption.amount() * df_exercise;
            }
            
            pv - target
        };
        
        // Try bracket [-0.99, 1.0] first, then widen if needed
        let mut a = -0.99;
        let mut b = 1.0;
        let mut root = brent(f, a, b, 1e-10, 128).unwrap_or(0.05);
        
        if !root.is_finite() {
            a = -0.99; 
            b = 5.0;
            root = brent(f, a, b, 1e-10, 256).unwrap_or(0.05);
        }
        
        Ok(if root.is_finite() { root } else { 0.05 })
    }
}

/// Calculates dirty price for bonds (clean price + accrued interest).
pub struct DirtyPriceCalculator;

impl MetricCalculator for DirtyPriceCalculator {
    fn id(&self) -> MetricId {
        MetricId::DirtyPrice
    }
    
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let bond = match context.instrument() {
            Instrument::Bond(bond) => bond,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        // Dirty price only makes sense if we have a quoted clean price
        let clean_px = bond.quoted_clean
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        // Get accrued from computed metrics
        let accrued = context.cache.computed.get(&MetricId::Accrued).copied().unwrap_or(0.0);
        
        // Dirty price = clean price + accrued interest
        Ok(clean_px + accrued)
    }
}

/// Calculates clean price for bonds (dirty price - accrued interest).
pub struct CleanPriceCalculator;

impl MetricCalculator for CleanPriceCalculator {
    fn id(&self) -> MetricId {
        MetricId::CleanPrice
    }
    
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let bond = match context.instrument() {
            Instrument::Bond(bond) => bond,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        // If we have quoted clean price, just return it
        if let Some(clean_px) = bond.quoted_clean {
            return Ok(clean_px);
        }
        
        // Otherwise calculate from base value (which should be dirty price)
        let dirty_px = context.base_value.amount();
        let accrued = context.cache.computed.get(&MetricId::Accrued).copied().unwrap_or(0.0);
        
        // Clean price = dirty price - accrued interest
        Ok(dirty_px - accrued)
    }
}

/// Calculates CS01 (credit spread sensitivity) for bonds.
pub struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn id(&self) -> MetricId {
        MetricId::Cs01
    }
    
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let bond = match context.instrument() {
            Instrument::Bond(bond) => bond,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        let ytm = context.cache.computed.get(&MetricId::Ytm).copied()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        // Build cashflow schedule from Bond
        let flows = bond.build_schedule(&context.market_data.curves, context.market_data.as_of)?;
        
        let bp = 1e-4; // 1 basis point
        
        // Calculate prices with yield bump up and down
        let p_up = self.price_from_ytm(bond, &flows, context.market_data.as_of, ytm + bp)?;
        let p_dn = self.price_from_ytm(bond, &flows, context.market_data.as_of, ytm - bp)?;
        
        // CS01 = (price_down - price_up) / (2 * bump)
        Ok((p_dn - p_up) / (2.0 * bp))
    }
}

impl Cs01Calculator {
    fn price_from_ytm(&self, bond: &Bond, flows: &[(Date, Money)], as_of: Date, ytm: F) -> finstack_core::Result<F> {
        use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
        
        let mut pv = 0.0;
        for &(date, amount) in flows {
            if date <= as_of {
                continue;
            }
            let t = DiscountCurve::year_fraction(as_of, date, bond.dc);
            if t > 0.0 {
                let df = (1.0 + ytm).powf(-t);
                pv += amount.amount() * df;
            }
        }
        Ok(pv)
    }
}

/// Register all bond metrics to a registry.
pub fn register_bond_metrics(registry: &mut crate::metrics::MetricRegistry) {
    use std::sync::Arc;
    
    let bond_types = &["Bond"];
    
    registry
        .register_for_types(Arc::new(AccruedInterestCalculator), bond_types)
        .register_for_types(Arc::new(DirtyPriceCalculator), bond_types)
        .register_for_types(Arc::new(CleanPriceCalculator), bond_types)
        .register_for_types(Arc::new(YtmCalculator), bond_types)
        .register_for_types(Arc::new(MacaulayDurationCalculator), bond_types)
        .register_for_types(Arc::new(ModifiedDurationCalculator), bond_types)
        .register_for_types(Arc::new(ConvexityCalculator), bond_types)
        .register_for_types(Arc::new(YtwCalculator), bond_types)
        .register_for_types(Arc::new(Cs01Calculator), bond_types);
}
