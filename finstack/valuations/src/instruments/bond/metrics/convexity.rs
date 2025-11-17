use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::money::Money;
use finstack_core::prelude::*;

/// Calculates convexity for bonds.
///
/// Convexity measures the curvature of the price/yield relationship and is
/// computed using a numerical second derivative approximation:
/// ```text
/// Convexity = (P+ + P- - 2*P0) / (P0 * dy²)
/// ```
/// where `P+` and `P-` are prices computed with yield bumped up and down by `dy`.
///
/// # Dependencies
///
/// Requires `Ytm` metric to be computed first.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::bond::Bond;
/// use finstack_valuations::metrics::{MetricRegistry, MetricId, MetricContext};
/// use finstack_core::market_data::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example();
/// # let market = MarketContext::new();
/// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
/// // Convexity is computed automatically when requesting bond metrics
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct ConvexityCalculator;

impl MetricCalculator for ConvexityCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
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
            .and_then(|po| po.ytm_bump_decimal)
            .unwrap_or(1e-4);

        // Calculate prices with yield bumps for numerical convexity
        let (p0, p_up, p_dn) = {
            let bond: &Bond = context.instrument_as()?;
            let p0 = crate::instruments::bond::pricing::quote_engine::price_from_ytm(
                bond,
                flows,
                context.as_of,
                ytm,
            )?;
            let p_up = crate::instruments::bond::pricing::quote_engine::price_from_ytm(
                bond,
                flows,
                context.as_of,
                ytm + dy,
            )?;
            let p_dn = crate::instruments::bond::pricing::quote_engine::price_from_ytm(
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
        let convexity = (p_up + p_dn - 2.0 * p0) / (p0 * dy * dy);

        Ok(convexity)
    }
}
