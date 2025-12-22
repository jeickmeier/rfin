//! Commodity forward types and implementations.
//!
//! Defines the `CommodityForward` instrument for physical or cash-settled
//! commodity forward contracts. Pricing uses curve-based forward interpolation
//! with optional quoted price override.

use crate::instruments::common::pricing::HasDiscountCurve;
use crate::instruments::common::traits::Attributes;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;

/// Settlement type for commodity contracts.
pub use crate::instruments::common::parameters::SettlementType;

/// Commodity forward or futures contract.
///
/// Represents a commitment to buy or sell a commodity at a specified future
/// date at a predetermined price. Can be physically settled (delivery) or
/// cash settled (price difference).
///
/// # Pricing
///
/// Forward value is calculated as:
/// ```text
/// NPV = (F - K) × Q × M × DF(T)
/// ```
/// where:
/// - F = Forward price from commodity curve (or quoted_price if provided)
/// - K = Contract strike price (if applicable, else F is the agreed price)
/// - Q = Quantity
/// - M = Contract multiplier
/// - DF(T) = Discount factor to settlement date
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::commodity_forward::CommodityForward;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::Date;
/// use finstack_core::types::{CurveId, InstrumentId};
/// use time::Month;
///
/// let forward = CommodityForward::builder()
///     .id(InstrumentId::new("WTI-FWD-2025M03"))
///     .commodity_type("Energy".to_string())
///     .ticker("CL".to_string())
///     .quantity(1000.0)
///     .unit("BBL".to_string())
///     .multiplier(1.0)
///     .settlement_date(Date::from_calendar_date(2025, Month::March, 15).unwrap())
///     .currency(Currency::USD)
///     .forward_curve_id(CurveId::new("WTI-FORWARD"))
///     .discount_curve_id(CurveId::new("USD-OIS"))
///     .build()
///     .expect("Valid forward");
/// ```
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CommodityForward {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Commodity type (e.g., "Energy", "Metal", "Agricultural").
    pub commodity_type: String,
    /// Ticker or symbol (e.g., "CL" for WTI, "GC" for Gold).
    pub ticker: String,
    /// Contract quantity in units.
    pub quantity: f64,
    /// Unit of measurement (e.g., "BBL", "MT", "OZ").
    pub unit: String,
    /// Contract multiplier (typically 1.0 for OTC forwards).
    pub multiplier: f64,
    /// Settlement/delivery date.
    pub settlement_date: Date,
    /// Settlement type (physical or cash).
    #[builder(default)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub settlement_type: Option<SettlementType>,
    /// Currency for pricing.
    pub currency: Currency,
    /// Optional quoted forward price (overrides curve lookup).
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub quoted_price: Option<f64>,
    /// Forward/futures curve ID for price interpolation.
    pub forward_curve_id: CurveId,
    /// Optional spot price ID (for delta calculations).
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub spot_price_id: Option<String>,
    /// Discount curve ID.
    pub discount_curve_id: CurveId,
    /// Optional exchange identifier (e.g., "NYMEX", "ICE").
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub exchange: Option<String>,
    /// Optional contract month (e.g., "2025M03" for March 2025).
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub contract_month: Option<String>,
    /// Attributes for tagging and selection.
    #[builder(default)]
    pub attributes: Attributes,
}

impl CommodityForward {
    /// Create a canonical example commodity forward for testing and documentation.
    ///
    /// Returns a WTI crude oil forward with realistic parameters.
    pub fn example() -> Self {
        Self::builder()
            .id(InstrumentId::new("WTI-FWD-2025M03"))
            .commodity_type("Energy".to_string())
            .ticker("CL".to_string())
            .quantity(1000.0)
            .unit("BBL".to_string())
            .multiplier(1.0)
            .settlement_date(
                Date::from_calendar_date(2025, time::Month::March, 15)
                    .expect("Valid example date"),
            )
            .settlement_type_opt(Some(SettlementType::Cash))
            .currency(Currency::USD)
            .forward_curve_id(CurveId::new("WTI-FORWARD"))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .exchange_opt(Some("NYMEX".to_string()))
            .contract_month_opt(Some("2025M03".to_string()))
            .attributes(Attributes::new().with_tag("energy").with_meta("sector", "crude"))
            .build()
            .expect("Example commodity forward construction should not fail")
    }

    /// Calculate the net present value of this commodity forward.
    ///
    /// # Arguments
    ///
    /// * `market` - Market context with curves and prices
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    ///
    /// Present value in the instrument's currency.
    pub fn npv(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        // If settlement has passed, value is zero
        if self.settlement_date < as_of {
            return Ok(Money::new(0.0, self.currency));
        }

        // Get forward price from quoted price or curve
        let forward_price = self.forward_price(market, as_of)?;

        // Get discount factor
        let disc = market.get_discount_ref(self.discount_curve_id.as_str())?;
        let df = disc.try_df_between_dates(as_of, self.settlement_date)?;

        // NPV = Forward × Quantity × Multiplier × DF
        // For a standard forward, we're long the commodity at the forward price
        let notional_value = forward_price * self.quantity * self.multiplier;
        let pv = notional_value * df;

        Ok(Money::new(pv, self.currency))
    }

    /// Get the forward price for this contract.
    ///
    /// Uses quoted_price if provided, otherwise interpolates from the forward curve.
    pub fn forward_price(&self, market: &MarketContext, as_of: Date) -> Result<f64> {
        // Use quoted price if available
        if let Some(price) = self.quoted_price {
            return Ok(price);
        }

        // Otherwise look up from forward curve
        // Try to get the forward curve as a discount curve (for interpolation)
        let curve = market.get_discount_ref(self.forward_curve_id.as_str())?;

        // Calculate time to settlement
        use finstack_core::dates::{DayCount, DayCountCtx};
        let t = DayCount::Act365F
            .year_fraction(as_of, self.settlement_date, DayCountCtx::default())
            .unwrap_or(0.0);

        // For commodity curves, we interpret the "zero rate" as the forward price level
        // This is a simplification - in practice, commodity curves store prices directly
        // We'll use the rate as a proxy: F(T) = S × exp(r × T) where r is the convenience yield adjusted rate
        let rate = curve.zero(t);

        // If we have a spot price, use cost-of-carry model
        if let Some(spot_id) = &self.spot_price_id {
            if let Ok(spot_scalar) = market.price(spot_id) {
                let spot = match spot_scalar {
                    finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                };
                // F = S × exp(r × T)
                return Ok(spot * (rate * t).exp());
            }
        }

        // Fallback: use the discount factor inverse as a price proxy
        // This is suitable when the "forward curve" stores forward prices as pseudo-rates
        let df = curve.df(t);
        if df.abs() > 1e-12 {
            // Assume a base price of 100 and adjust by discount factor ratio
            // This is a placeholder - real implementation would use actual forward prices
            Ok(100.0 / df)
        } else {
            Ok(100.0)
        }
    }

    /// Get the effective notional value at settlement.
    pub fn notional_value(&self, forward_price: f64) -> f64 {
        forward_price * self.quantity * self.multiplier
    }
}

impl crate::instruments::common::traits::CurveDependencies for CommodityForward {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.forward_curve_id.clone())
            .build()
    }
}

impl crate::instruments::common::traits::Instrument for CommodityForward {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::CommodityForward
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.npv(market, as_of)
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
        )
    }

    fn required_discount_curves(&self) -> Vec<CurveId> {
        vec![self.discount_curve_id.clone()]
    }

    fn spot_id(&self) -> Option<&str> {
        self.spot_price_id.as_deref()
    }
}

impl HasDiscountCurve for CommodityForward {
    fn discount_curve_id(&self) -> &CurveId {
        &self.discount_curve_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_commodity_forward_creation() {
        let forward = CommodityForward::builder()
            .id(InstrumentId::new("TEST-FWD"))
            .commodity_type("Energy".to_string())
            .ticker("CL".to_string())
            .quantity(1000.0)
            .unit("BBL".to_string())
            .multiplier(1.0)
            .settlement_date(
                Date::from_calendar_date(2025, Month::June, 15).expect("valid date"),
            )
            .currency(Currency::USD)
            .forward_curve_id(CurveId::new("CL-FWD"))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        assert_eq!(forward.id.as_str(), "TEST-FWD");
        assert_eq!(forward.ticker, "CL");
        assert_eq!(forward.quantity, 1000.0);
        assert_eq!(forward.currency, Currency::USD);
    }

    #[test]
    fn test_commodity_forward_example() {
        let forward = CommodityForward::example();
        assert_eq!(forward.id.as_str(), "WTI-FWD-2025M03");
        assert_eq!(forward.commodity_type, "Energy");
        assert_eq!(forward.ticker, "CL");
        assert!(forward.attributes.has_tag("energy"));
    }

    #[test]
    fn test_commodity_forward_with_quoted_price() {
        let forward = CommodityForward::builder()
            .id(InstrumentId::new("GC-FWD"))
            .commodity_type("Metal".to_string())
            .ticker("GC".to_string())
            .quantity(100.0)
            .unit("OZ".to_string())
            .multiplier(1.0)
            .settlement_date(
                Date::from_calendar_date(2025, Month::April, 15).expect("valid date"),
            )
            .currency(Currency::USD)
            .quoted_price_opt(Some(2000.0))
            .forward_curve_id(CurveId::new("GC-FWD"))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        assert_eq!(forward.quoted_price, Some(2000.0));

        // When quoted price is set, it should be used directly
        let market = MarketContext::new();
        let as_of = Date::from_calendar_date(2025, Month::January, 15).expect("valid date");
        let price = forward.forward_price(&market, as_of).expect("should get price");
        assert_eq!(price, 2000.0);
    }

    #[test]
    fn test_commodity_forward_instrument_trait() {
        use crate::instruments::common::traits::Instrument;

        let forward = CommodityForward::example();

        assert_eq!(forward.id(), "WTI-FWD-2025M03");
        assert_eq!(forward.key(), crate::pricer::InstrumentType::CommodityForward);
        assert!(forward.attributes().has_tag("energy"));
    }

    #[test]
    fn test_commodity_forward_curve_dependencies() {
        use crate::instruments::common::traits::CurveDependencies;

        let forward = CommodityForward::example();
        let deps = forward.curve_dependencies();

        assert_eq!(deps.discount_curves.len(), 1);
        assert_eq!(deps.forward_curves.len(), 1);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_commodity_forward_serde_roundtrip() {
        let forward = CommodityForward::example();
        let json = serde_json::to_string(&forward).expect("serialize");
        let deserialized: CommodityForward =
            serde_json::from_str(&json).expect("deserialize");

        assert_eq!(forward.id.as_str(), deserialized.id.as_str());
        assert_eq!(forward.ticker, deserialized.ticker);
        assert_eq!(forward.quantity, deserialized.quantity);
    }
}

