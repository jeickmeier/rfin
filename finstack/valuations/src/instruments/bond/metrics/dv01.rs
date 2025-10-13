//! Bond DV01 metric calculator.
//!
//! Provides DV01 calculation for bond instruments using modified duration.
//!
//! # Market Standard Formula
//!
//! DV01 = Price × Modified Duration × 0.0001
//!
//! Where:
//! - Price = Current market value of the bond (dirty price)
//! - Modified Duration = Macaulay Duration / (1 + YTM/m)
//! - 0.0001 = One basis point (1bp = 0.01%)
//!
//! # Sign Convention
//!
//! Positive for long positions: when rates rise by 1bp, bond prices fall,
//! resulting in a negative P&L for a long position. The DV01 magnitude
//! indicates the dollar loss per 1bp rate increase.

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::bond::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// DV01 calculator for bonds using market-standard modified duration approach.
pub struct BondDv01Calculator;

impl MetricCalculator for BondDv01Calculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::DurationMod]
    }

    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let bond: &Bond = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= bond.maturity {
            return Ok(0.0);
        }

        // Get modified duration from computed metrics
        let modified_duration = context
            .computed
            .get(&MetricId::DurationMod)
            .copied()
            .unwrap_or(0.0);

        // Market standard: DV01 = Price × Modified Duration × 1bp
        // Use base_value (dirty price) from context
        let price = context.base_value.amount();
        let dv01 = price * modified_duration * ONE_BASIS_POINT;

        Ok(dv01)
    }
}
