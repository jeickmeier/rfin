//! CDS DV01 metric calculator.
//!
//! Provides DV01 calculation for CDS instruments using risky PV01.
//!
//! # Market Standard Formula
//!
//! For CDS, DV01 = Risky PV01 = Risky Annuity × Notional / 10,000
//!
//! Where:
//! - Risky Annuity = Sum of survival-weighted discount factors
//! - This represents the present value of a 1bp premium stream
//!
//! # Sign Convention
//!
//! Positive for protection buyer (long protection): when spreads widen,
//! the mark-to-market value increases for the protection buyer.

use crate::instruments::cds::pricer::CDSPricer;
use crate::instruments::cds::CreditDefaultSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// DV01 calculator for CDS instruments using market-standard risky PV01.
pub struct CdsDv01Calculator;

impl MetricCalculator for CdsDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let as_of = context.as_of;

        // Get maturity from premium leg end date
        let maturity = cds.premium.end;
        if as_of >= maturity {
            return Ok(0.0);
        }

        // Market standard: Use risky PV01 from the CDS pricer
        let pricer = CDSPricer::new();
        let disc = context.curves.get_discount_ref(&cds.premium.disc_id)?;
        let surv = context.curves.get_hazard_ref(&cds.protection.credit_curve_id)?;

        pricer.risky_pv01(cds, disc, surv, as_of)
    }
}
