use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};

/// Calculates dirty price for bonds (clean price + accrued interest).
///
/// Dirty price is the full price paid by the buyer, including accrued interest
/// since the last coupon payment. It is computed as:
/// ```text
/// Dirty Price = Clean Price + Accrued Interest
/// ```
///
/// # Dependencies
///
/// Requires `Accrued` metric to be computed first.
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
/// // Dirty price is computed automatically when requesting bond metrics
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct DirtyPriceCalculator;

impl MetricCalculator for DirtyPriceCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;

        // Dirty price only makes sense if we have a quoted clean price
        let clean_px = bond.pricing_overrides.quoted_clean_price.ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "bond.pricing_overrides.quoted_clean_price".to_string(),
            })
        })?;

        // Get accrued from computed metrics
        let accrued = context
            .computed
            .get(&MetricId::Accrued)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric:Accrued".to_string(),
                })
            })?;

        // Dirty price in currency = (clean % of par) * notional + accrued (currency)
        Ok(clean_px * bond.notional.amount() / 100.0 + accrued)
    }
}

/// Calculates clean price for bonds (dirty price - accrued interest).
///
/// Clean price is the quoted price excluding accrued interest. It can be:
/// - Retrieved directly from `bond.pricing_overrides.quoted_clean_price` if set
/// - Computed from the base value (dirty price) minus accrued interest
///
/// # Dependencies
///
/// Requires `Accrued` metric to be computed first.
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
/// // Clean price is computed automatically when requesting bond metrics
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct CleanPriceCalculator;

impl MetricCalculator for CleanPriceCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;

        // If we have quoted clean price, return currency value
        if let Some(clean_px) = bond.pricing_overrides.quoted_clean_price {
            return Ok(clean_px * bond.notional.amount() / 100.0);
        }

        // Otherwise calculate from base value (which should be dirty price in currency)
        let dirty_px = context.base_value.amount();
        let accrued = context
            .computed
            .get(&MetricId::Accrued)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric:Accrued".to_string(),
                })
            })?;

        // Clean price in currency
        Ok(dirty_px - accrued)
    }
}
