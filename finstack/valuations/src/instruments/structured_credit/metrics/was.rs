//! Weighted Average Spread calculator for CLO

use crate::metrics::MetricContext;

/// CLO WAS calculator - in basis points
///
/// Market standard: WAS should use the **spread component only**, not the all-in coupon.
/// For floating rate assets: use the spread over the index (e.g., SOFR + 450bps -> 450)
/// For fixed rate assets: fall back to the all-in rate as a proxy
pub struct CloWasCalculator;

impl crate::metrics::MetricCalculator for CloWasCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let clo = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::structured_credit::StructuredCredit>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        let mut weighted_spread = 0.0;
        let mut total_balance = 0.0;

        for asset in &clo.pool.assets {
            let balance = asset.balance.amount();

            // Use explicit spread_bps if available (correct for floating rate assets)
            // Otherwise fall back to rate * 10000 (proxy for fixed rate)
            let spread_bps = asset.spread_bps.unwrap_or(asset.rate * 10000.0);

            weighted_spread += balance * spread_bps;
            total_balance += balance;
        }

        if total_balance > 0.0 {
            Ok(weighted_spread / total_balance)
        } else {
            Ok(0.0)
        }
    }
}
