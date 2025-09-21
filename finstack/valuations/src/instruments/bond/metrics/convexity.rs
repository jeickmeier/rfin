use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::money::Money;
use finstack_core::prelude::*;
use finstack_core::F;

/// Calculates convexity for bonds.
pub struct ConvexityCalculator;

impl MetricCalculator for ConvexityCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let ytm = context
            .computed
            .get(&MetricId::Ytm)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric:Ytm".to_string(),
                })
            })?;

        // YTM dependency ensures cashflows are already built and cached
        let flows: &Vec<(Date, Money)> = context.cashflows.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "context.cashflows".to_string(),
            })
        })?;

        // Bump size: configurable via context overrides, default 1 bp
        let dy = context
            .pricing_overrides
            .as_ref()
            .and_then(|po| po.ytm_bump_bp)
            .unwrap_or(1e-4);

        // Calculate prices with yield bumps for numerical convexity
        let (p0, p_up, p_dn) = {
            let bond: &Bond = context.instrument_as()?;
            let p0 = crate::instruments::bond::pricing::helpers::price_from_ytm(
                bond,
                flows,
                context.as_of,
                ytm,
            )?;
            let p_up = crate::instruments::bond::pricing::helpers::price_from_ytm(
                bond,
                flows,
                context.as_of,
                ytm + dy,
            )?;
            let p_dn = crate::instruments::bond::pricing::helpers::price_from_ytm(
                bond,
                flows,
                context.as_of,
                ytm - dy,
            )?;
            (p0, p_up, p_dn)
        };

        if p0 == 0.0 || dy == 0.0 {
            return Ok(0.0);
        }

        // Convexity = (P+ + P- - 2*P0) / (P0 * dy^2)
        Ok((p_up + p_dn - 2.0 * p0) / (p0 * dy * dy))
    }
}


