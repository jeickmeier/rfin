//! Commodity swap types and implementations.
//!
//! Defines the `CommoditySwap` instrument for fixed-for-floating commodity
//! price exchange contracts. One party pays a fixed price per unit while
//! the other pays a floating price based on an index.

use crate::instruments::common::pricing::HasDiscountCurve;
use crate::instruments::common::traits::{Attributes, CurveIdVec};
use finstack_core::currency::Currency;
use finstack_core::dates::{
    BusinessDayConvention, CalendarRegistry, Date, DayCount, DayCountCtx, ScheduleBuilder, Tenor,
};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;
use smallvec::smallvec;

/// Commodity swap (fixed-for-floating commodity price exchange).
///
/// One party pays a fixed price per unit, the other pays a floating price
/// determined by an index or average of spot prices over the period.
///
/// # Pricing
///
/// Fixed leg: ∑ Q × P_fixed × DF(t_i)
/// Floating leg: ∑ Q × E[P_float(t_i)] × DF(t_i)
///
/// For a payer of fixed:
/// NPV = Floating leg PV - Fixed leg PV
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::commodity_swap::CommoditySwap;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::{Date, BusinessDayConvention, Tenor, TenorUnit};
/// use finstack_core::types::{CurveId, InstrumentId};
/// use time::Month;
///
/// let swap = CommoditySwap::builder()
///     .id(InstrumentId::new("NG-SWAP-2025"))
///     .commodity_type("Energy".to_string())
///     .ticker("NG".to_string())
///     .unit("MMBTU".to_string())
///     .currency(Currency::USD)
///     .notional_quantity(10000.0)
///     .fixed_price(3.50)
///     .floating_index_id(CurveId::new("NG-SPOT-AVG"))
///     .pay_fixed(true)
///     .start_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
///     .end_date(Date::from_calendar_date(2025, Month::December, 31).unwrap())
///     .payment_frequency(Tenor::new(1, TenorUnit::Months))
///     .discount_curve_id(CurveId::new("USD-OIS"))
///     .build()
///     .expect("Valid swap");
/// ```
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CommoditySwap {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Commodity type (e.g., "Energy", "Metal", "Agricultural").
    pub commodity_type: String,
    /// Ticker or symbol (e.g., "CL" for WTI, "NG" for Natural Gas).
    pub ticker: String,
    /// Unit of measurement (e.g., "BBL", "MMBTU", "MT").
    pub unit: String,
    /// Currency for pricing and settlement.
    pub currency: Currency,
    /// Notional quantity per period.
    pub notional_quantity: f64,
    /// Fixed price per unit.
    pub fixed_price: f64,
    /// Floating index ID for price lookups.
    pub floating_index_id: CurveId,
    /// True if paying fixed (receiving floating), false if receiving fixed.
    pub pay_fixed: bool,
    /// Start date of the swap.
    pub start_date: Date,
    /// End date of the swap.
    pub end_date: Date,
    /// Payment frequency as a Tenor.
    pub payment_frequency: Tenor,
    /// Optional calendar ID for date adjustments.
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub calendar_id: Option<String>,
    /// Business day convention for date adjustments.
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub bdc: Option<BusinessDayConvention>,
    /// Discount curve ID.
    pub discount_curve_id: CurveId,
    /// Optional index lag in days (for averaging period).
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub index_lag_days: Option<i32>,
    /// Attributes for tagging and selection.
    #[builder(default)]
    pub attributes: Attributes,
}

impl CommoditySwap {
    /// Create a canonical example commodity swap for testing and documentation.
    ///
    /// Returns a natural gas swap with monthly settlements.
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        Self::builder()
            .id(InstrumentId::new("NG-SWAP-2025"))
            .commodity_type("Energy".to_string())
            .ticker("NG".to_string())
            .unit("MMBTU".to_string())
            .currency(Currency::USD)
            .notional_quantity(10000.0)
            .fixed_price(3.50)
            .floating_index_id(CurveId::new("NG-SPOT-AVG"))
            .pay_fixed(true)
            .start_date(
                Date::from_calendar_date(2025, time::Month::January, 1)
                    .expect("Valid example date"),
            )
            .end_date(
                Date::from_calendar_date(2025, time::Month::December, 31)
                    .expect("Valid example date"),
            )
            .payment_frequency(Tenor::new(1, finstack_core::dates::TenorUnit::Months))
            .bdc_opt(Some(BusinessDayConvention::ModifiedFollowing))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .attributes(
                Attributes::new()
                    .with_tag("energy")
                    .with_meta("sector", "natural-gas"),
            )
            .build()
            .expect("Example commodity swap construction should not fail")
    }

    /// Calculate the net present value of this commodity swap.
    pub fn npv(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        let fixed_leg_pv = self.fixed_leg_pv(market, as_of)?;
        let floating_leg_pv = self.floating_leg_pv(market, as_of)?;

        let npv = if self.pay_fixed {
            // Pay fixed, receive floating
            floating_leg_pv - fixed_leg_pv
        } else {
            // Receive fixed, pay floating
            fixed_leg_pv - floating_leg_pv
        };

        Ok(Money::new(npv, self.currency))
    }

    /// Calculate the present value of the fixed leg.
    pub fn fixed_leg_pv(&self, market: &MarketContext, as_of: Date) -> Result<f64> {
        let disc = market.get_discount_ref(self.discount_curve_id.as_str())?;
        let schedule = self.payment_schedule(as_of)?;

        let mut pv = 0.0;
        for payment_date in schedule {
            if payment_date < as_of {
                continue; // Skip past payments
            }
            let df = disc.df_between_dates(as_of, payment_date)?;
            let period_value = self.notional_quantity * self.fixed_price;
            pv += period_value * df;
        }

        Ok(pv)
    }

    /// Calculate the present value of the floating leg.
    pub fn floating_leg_pv(&self, market: &MarketContext, as_of: Date) -> Result<f64> {
        let disc = market.get_discount_ref(self.discount_curve_id.as_str())?;
        let schedule = self.payment_schedule(as_of)?;

        // Try to get floating index curve
        let float_curve = market.get_discount_ref(self.floating_index_id.as_str())?;

        let mut pv = 0.0;
        for payment_date in schedule {
            if payment_date < as_of {
                continue; // Skip past payments
            }

            // Calculate time to payment for forward price interpolation
            let t = DayCount::Act365F
                .year_fraction(as_of, payment_date, DayCountCtx::default())
                .unwrap_or(0.0);

            // Get forward price from curve (using rate as proxy)
            // In practice, this would use actual commodity forward prices
            let rate = float_curve.zero(t);
            let forward_price = if t > 0.0 {
                // Use cost-of-carry approximation
                self.fixed_price * (rate * t).exp()
            } else {
                self.fixed_price
            };

            let df = disc.df_between_dates(as_of, payment_date)?;
            let period_value = self.notional_quantity * forward_price;
            pv += period_value * df;
        }

        Ok(pv)
    }

    /// Generate the payment schedule for this swap.
    pub fn payment_schedule(&self, _as_of: Date) -> Result<Vec<Date>> {
        let bdc = self.bdc.unwrap_or(BusinessDayConvention::Following);

        let mut builder =
            ScheduleBuilder::new(self.start_date, self.end_date).frequency(self.payment_frequency);

        // Apply calendar adjustment if calendar_id is specified
        if let Some(ref cal_id) = self.calendar_id {
            if let Some(cal) = CalendarRegistry::global().resolve_str(cal_id) {
                builder = builder.adjust_with(bdc, cal);
            }
        }

        let schedule = builder.build()?;

        // Filter to payment dates only (skip start date if it's in the schedule)
        let dates: Vec<Date> = schedule
            .into_iter()
            .filter(|&d| d > self.start_date && d <= self.end_date)
            .collect();

        Ok(dates)
    }

    /// Get all projected cashflows for this swap.
    pub fn cashflows(&self, market: &MarketContext, as_of: Date) -> Result<Vec<(Date, Money)>> {
        let schedule = self.payment_schedule(as_of)?;
        let float_curve = market.get_discount_ref(self.floating_index_id.as_str())?;

        let mut flows = Vec::new();
        for payment_date in schedule {
            if payment_date < as_of {
                continue;
            }

            let t = DayCount::Act365F
                .year_fraction(as_of, payment_date, DayCountCtx::default())
                .unwrap_or(0.0);

            let rate = float_curve.zero(t);
            let forward_price = if t > 0.0 {
                self.fixed_price * (rate * t).exp()
            } else {
                self.fixed_price
            };

            let net_cashflow = if self.pay_fixed {
                self.notional_quantity * (forward_price - self.fixed_price)
            } else {
                self.notional_quantity * (self.fixed_price - forward_price)
            };

            flows.push((payment_date, Money::new(net_cashflow, self.currency)));
        }

        Ok(flows)
    }
}

impl crate::instruments::common::traits::CurveDependencies for CommoditySwap {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.floating_index_id.clone())
            .build()
    }
}

impl crate::instruments::common::traits::Instrument for CommoditySwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::CommoditySwap
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
            None,
        )
    }

    fn required_discount_curves(&self) -> CurveIdVec {
        smallvec![self.discount_curve_id.clone()]
    }
}

impl HasDiscountCurve for CommoditySwap {
    fn discount_curve_id(&self) -> &CurveId {
        &self.discount_curve_id
    }
}

impl crate::instruments::common::pricing::HasForwardCurves for CommoditySwap {
    fn forward_curve_ids(&self) -> Vec<CurveId> {
        vec![self.floating_index_id.clone()]
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_commodity_swap_creation() {
        let swap = CommoditySwap::builder()
            .id(InstrumentId::new("TEST-SWAP"))
            .commodity_type("Energy".to_string())
            .ticker("CL".to_string())
            .unit("BBL".to_string())
            .currency(Currency::USD)
            .notional_quantity(1000.0)
            .fixed_price(70.0)
            .floating_index_id(CurveId::new("CL-AVG"))
            .pay_fixed(true)
            .start_date(Date::from_calendar_date(2025, Month::January, 1).expect("valid date"))
            .end_date(Date::from_calendar_date(2025, Month::June, 30).expect("valid date"))
            .payment_frequency(Tenor::new(1, finstack_core::dates::TenorUnit::Months))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        assert_eq!(swap.id.as_str(), "TEST-SWAP");
        assert_eq!(swap.ticker, "CL");
        assert_eq!(swap.notional_quantity, 1000.0);
        assert_eq!(swap.fixed_price, 70.0);
        assert!(swap.pay_fixed);
    }

    #[test]
    fn test_commodity_swap_example() {
        let swap = CommoditySwap::example();
        assert_eq!(swap.id.as_str(), "NG-SWAP-2025");
        assert_eq!(swap.commodity_type, "Energy");
        assert_eq!(swap.ticker, "NG");
        assert!(swap.attributes.has_tag("energy"));
    }

    #[test]
    fn test_commodity_swap_instrument_trait() {
        use crate::instruments::common::traits::Instrument;

        let swap = CommoditySwap::example();

        assert_eq!(swap.id(), "NG-SWAP-2025");
        assert_eq!(swap.key(), crate::pricer::InstrumentType::CommoditySwap);
    }

    #[test]
    fn test_commodity_swap_curve_dependencies() {
        use crate::instruments::common::traits::CurveDependencies;

        let swap = CommoditySwap::example();
        let deps = swap.curve_dependencies();

        assert_eq!(deps.discount_curves.len(), 1);
        assert_eq!(deps.forward_curves.len(), 1);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_commodity_swap_serde_roundtrip() {
        let swap = CommoditySwap::example();
        let json = serde_json::to_string(&swap).expect("serialize");
        let deserialized: CommoditySwap = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(swap.id.as_str(), deserialized.id.as_str());
        assert_eq!(swap.ticker, deserialized.ticker);
        assert_eq!(swap.fixed_price, deserialized.fixed_price);
    }
}
