#![deny(missing_docs)]
//! Bond-specific metric calculators.

use super::traits::{MetricCalculator, MetricContext};
use crate::instruments::bond::Bond;
use crate::pricing::quotes;
use finstack_core::prelude::*;
use finstack_core::F;

/// Calculates accrued interest for bonds.
pub struct AccruedInterestCalculator;

impl MetricCalculator for AccruedInterestCalculator {
    fn id(&self) -> &str {
        "accrued"
    }
    
    fn description(&self) -> &str {
        "Accrued interest since last coupon payment"
    }
    
    fn is_applicable(&self, instrument_type: &str) -> bool {
        instrument_type == "Bond"
    }
    
    fn calculate(&self, context: &MetricContext) -> finstack_core::Result<F> {
        let bond = context.instrument_as::<Bond>()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        
        // Get or compute schedule
        let schedule = context.get_cached::<Vec<Date>>("schedule")
            .map(|arc| (*arc).clone())
            .unwrap_or_else(|| {
                finstack_core::dates::ScheduleBuilder::new(bond.issue, bond.maturity)
                    .frequency(bond.freq)
                    .build_raw()
                    .collect()
            });
        
        // Find last and next coupon dates around as_of
        let (mut last, mut next) = (bond.issue, bond.maturity);
        for w in schedule.windows(2) {
            let (a, b) = (w[0], w[1]);
            if a <= context.as_of && context.as_of < b {
                last = a;
                next = b;
                break;
            }
        }
        
        let ai = quotes::accrued_interest(
            bond.notional,
            bond.coupon,
            last,
            context.as_of,
            next,
            bond.dc,
        );
        
        Ok(ai.amount())
    }
}

/// Calculates yield to maturity for bonds.
pub struct YtmCalculator;

impl MetricCalculator for YtmCalculator {
    fn id(&self) -> &str {
        "ytm"
    }
    
    fn description(&self) -> &str {
        "Yield to maturity"
    }
    
    fn is_applicable(&self, instrument_type: &str) -> bool {
        instrument_type == "Bond"
    }
    
    fn dependencies(&self) -> Vec<&str> {
        vec!["accrued"]
    }
    
    fn calculate(&self, context: &MetricContext) -> finstack_core::Result<F> {
        let bond = context.instrument_as::<Bond>()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        
        // YTM only makes sense if we have a quoted clean price
        let clean_px = bond.quoted_clean
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        // Get accrued from computed metrics
        let ai = context.computed.get("accrued").copied().unwrap_or(0.0);
        
        // Compute dirty price
        let dirty_amt = clean_px + ai;
        let dirty = Money::new(dirty_amt, bond.notional.currency());
        
        // Get or compute schedule
        let schedule = context.get_cached::<Vec<Date>>("schedule")
            .map(|arc| (*arc).clone())
            .unwrap_or_else(|| {
                finstack_core::dates::ScheduleBuilder::new(bond.issue, bond.maturity)
                    .frequency(bond.freq)
                    .build_raw()
                    .collect()
            });
        
        let ytm = quotes::bond_ytm_from_dirty(
            bond.notional,
            bond.coupon,
            &schedule,
            bond.dc,
            context.as_of,
            dirty,
        );
        
        Ok(ytm)
    }
}

/// Calculates Macaulay duration for bonds.
pub struct MacaulayDurationCalculator;

impl MetricCalculator for MacaulayDurationCalculator {
    fn id(&self) -> &str {
        "duration_mac"
    }
    
    fn description(&self) -> &str {
        "Macaulay duration"
    }
    
    fn is_applicable(&self, instrument_type: &str) -> bool {
        instrument_type == "Bond"
    }
    
    fn dependencies(&self) -> Vec<&str> {
        vec!["ytm"]
    }
    
    fn calculate(&self, context: &MetricContext) -> finstack_core::Result<F> {
        let bond = context.instrument_as::<Bond>()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        
        let ytm = context.computed.get("ytm").copied()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        let schedule = context.get_cached::<Vec<Date>>("schedule")
            .map(|arc| (*arc).clone())
            .unwrap_or_else(|| {
                finstack_core::dates::ScheduleBuilder::new(bond.issue, bond.maturity)
                    .frequency(bond.freq)
                    .build_raw()
                    .collect()
            });
        
        let (d_mac, _) = quotes::bond_duration_mac_mod(
            bond.notional,
            bond.coupon,
            &schedule,
            bond.dc,
            context.as_of,
            ytm,
        );
        
        Ok(d_mac)
    }
}

/// Calculates modified duration for bonds.
pub struct ModifiedDurationCalculator;

impl MetricCalculator for ModifiedDurationCalculator {
    fn id(&self) -> &str {
        "duration_mod"
    }
    
    fn description(&self) -> &str {
        "Modified duration"
    }
    
    fn is_applicable(&self, instrument_type: &str) -> bool {
        instrument_type == "Bond"
    }
    
    fn dependencies(&self) -> Vec<&str> {
        vec!["ytm"]
    }
    
    fn calculate(&self, context: &MetricContext) -> finstack_core::Result<F> {
        let bond = context.instrument_as::<Bond>()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        
        let ytm = context.computed.get("ytm").copied()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        let schedule = context.get_cached::<Vec<Date>>("schedule")
            .map(|arc| (*arc).clone())
            .unwrap_or_else(|| {
                finstack_core::dates::ScheduleBuilder::new(bond.issue, bond.maturity)
                    .frequency(bond.freq)
                    .build_raw()
                    .collect()
            });
        
        let (_, d_mod) = quotes::bond_duration_mac_mod(
            bond.notional,
            bond.coupon,
            &schedule,
            bond.dc,
            context.as_of,
            ytm,
        );
        
        Ok(d_mod)
    }
}

/// Calculates convexity for bonds.
pub struct ConvexityCalculator;

impl MetricCalculator for ConvexityCalculator {
    fn id(&self) -> &str {
        "convexity"
    }
    
    fn description(&self) -> &str {
        "Bond convexity"
    }
    
    fn is_applicable(&self, instrument_type: &str) -> bool {
        instrument_type == "Bond"
    }
    
    fn dependencies(&self) -> Vec<&str> {
        vec!["ytm"]
    }
    
    fn calculate(&self, context: &MetricContext) -> finstack_core::Result<F> {
        let bond = context.instrument_as::<Bond>()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        
        let ytm = context.computed.get("ytm").copied()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        let schedule = context.get_cached::<Vec<Date>>("schedule")
            .map(|arc| (*arc).clone())
            .unwrap_or_else(|| {
                finstack_core::dates::ScheduleBuilder::new(bond.issue, bond.maturity)
                    .frequency(bond.freq)
                    .build_raw()
                    .collect()
            });
        
        let convex = quotes::bond_convexity_numeric(
            bond.notional,
            bond.coupon,
            &schedule,
            bond.dc,
            context.as_of,
            ytm,
            1e-4,
        );
        
        Ok(convex)
    }
}

/// Calculates yield-to-worst for bonds with call/put schedules.
pub struct YtwCalculator;

impl MetricCalculator for YtwCalculator {
    fn id(&self) -> &str {
        "ytw"
    }
    
    fn description(&self) -> &str {
        "Yield to worst (minimum yield considering call/put options)"
    }
    
    fn is_applicable(&self, instrument_type: &str) -> bool {
        instrument_type == "Bond"
    }
    
    fn calculate(&self, context: &MetricContext) -> finstack_core::Result<F> {
        let bond = context.instrument_as::<Bond>()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        
        let cp = bond.call_put.as_ref()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        let schedule: Vec<Date> = finstack_core::dates::ScheduleBuilder::new(bond.issue, bond.maturity)
            .frequency(bond.freq)
            .build_raw()
            .collect();
        
        // Build candidate exercise dates
        let mut candidates: Vec<(Date, Money)> = Vec::new();
        for c in &cp.calls {
            if c.date >= context.as_of && c.date <= bond.maturity {
                let redemption = bond.notional * (c.price_pct_of_par / 100.0);
                candidates.push((c.date, redemption));
            }
        }
        for p in &cp.puts {
            if p.date >= context.as_of && p.date <= bond.maturity {
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
        for (exercise, red) in candidates {
            // Truncate schedule to exercise date
            let mut trunc: Vec<Date> = schedule.iter().cloned()
                .filter(|d| *d <= exercise)
                .collect();
            if *trunc.last().unwrap() != exercise {
                trunc.push(exercise);
            }
            
            let y = quotes::bond_ytm_from_dirty_with_redemption(
                bond.notional,
                bond.coupon,
                &trunc,
                bond.dc,
                context.as_of,
                dirty_now,
                red,
            );
            
            if y < best_ytm {
                best_ytm = y;
            }
        }
        
        Ok(best_ytm)
    }
}

/// Register all bond metrics to a registry.
pub fn register_bond_metrics(registry: &mut super::registry::MetricRegistry) {
    use std::sync::Arc;
    
    registry
        .register(Arc::new(AccruedInterestCalculator))
        .register(Arc::new(YtmCalculator))
        .register(Arc::new(MacaulayDurationCalculator))
        .register(Arc::new(ModifiedDurationCalculator))
        .register(Arc::new(ConvexityCalculator))
        .register(Arc::new(YtwCalculator));
}
