//! Dollar roll types.
//!
//! A dollar roll is a simultaneous sale and purchase of agency MBS TBAs
//! for different settlement months, used for financing and carry trades.

use crate::instruments::agency_mbs_passthrough::AgencyProgram;
use crate::instruments::agency_tba::{AgencyTba, TbaTerm};
use crate::instruments::common::traits::{Attributes, CurveIdVec};
use crate::instruments::PricingOverrides;
use smallvec::smallvec;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Dollar roll - simultaneous sale and purchase of TBAs for different months.
///
/// A dollar roll involves:
/// 1. Selling TBA for near-month settlement
/// 2. Buying TBA for far-month settlement
///
/// The price difference between the two legs represents the "drop" and
/// implies a financing rate.
///
/// # Financing and Carry
///
/// Dollar rolls are used for:
/// - **Financing**: Implied repo rate is often cheaper than repo
/// - **Carry trades**: Profit from drop vs. expected prepayment
/// - **Roll specialness**: When roll drops exceed fair value
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::dollar_roll::DollarRoll;
/// use finstack_valuations::instruments::agency_tba::TbaTerm;
/// use finstack_valuations::instruments::agency_mbs_passthrough::AgencyProgram;
/// use finstack_core::currency::Currency;
/// use finstack_core::money::Money;
/// use finstack_core::types::{CurveId, InstrumentId};
///
/// let roll = DollarRoll::builder()
///     .id(InstrumentId::new("FN30-4.0-ROLL-0324-0424"))
///     .agency(AgencyProgram::Fnma)
///     .coupon(0.04)
///     .term(TbaTerm::ThirtyYear)
///     .notional(Money::new(10_000_000.0, Currency::USD))
///     .front_settlement_year(2024)
///     .front_settlement_month(3)
///     .back_settlement_year(2024)
///     .back_settlement_month(4)
///     .front_price(98.5)
///     .back_price(98.0)
///     .discount_curve_id(CurveId::new("USD-OIS"))
///     .build()
///     .expect("Valid dollar roll");
/// ```
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DollarRoll {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Agency program.
    pub agency: AgencyProgram,
    /// Pass-through coupon rate.
    pub coupon: f64,
    /// Original loan term.
    pub term: TbaTerm,
    /// Trade notional (par amount).
    pub notional: Money,
    /// Front-month settlement year.
    pub front_settlement_year: i32,
    /// Front-month settlement month (1-12).
    pub front_settlement_month: u8,
    /// Back-month settlement year.
    pub back_settlement_year: i32,
    /// Back-month settlement month (1-12).
    pub back_settlement_month: u8,
    /// Front-month price (sell price).
    pub front_price: f64,
    /// Back-month price (buy price).
    pub back_price: f64,
    /// Trade date.
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub trade_date: Option<Date>,
    /// Discount curve identifier.
    pub discount_curve_id: CurveId,
    /// Pricing overrides.
    #[builder(default)]
    #[cfg_attr(feature = "serde", serde(default))]
    pub pricing_overrides: PricingOverrides,
    /// Attributes for tagging and selection.
    #[builder(default)]
    pub attributes: Attributes,
}

impl DollarRoll {
    /// Create a canonical example dollar roll for testing.
    pub fn example() -> Self {
        Self::builder()
            .id(InstrumentId::new("FN30-4.0-ROLL-0324-0424"))
            .agency(AgencyProgram::Fnma)
            .coupon(0.04)
            .term(TbaTerm::ThirtyYear)
            .notional(Money::new(10_000_000.0, Currency::USD))
            .front_settlement_year(2024)
            .front_settlement_month(3)
            .back_settlement_year(2024)
            .back_settlement_month(4)
            .front_price(98.5)
            .back_price(98.0)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .attributes(
                Attributes::new()
                    .with_tag("dollar_roll")
                    .with_tag("agency")
                    .with_meta("program", "fnma"),
            )
            .build()
            .expect("Example dollar roll construction should not fail")
    }

    /// Get the drop (price difference between front and back month).
    ///
    /// Positive drop means front month trades at premium to back month.
    pub fn drop(&self) -> f64 {
        self.front_price - self.back_price
    }

    /// Get the drop in 32nds (common market convention).
    pub fn drop_32nds(&self) -> f64 {
        self.drop() * 32.0
    }

    /// Create the front-month TBA leg.
    pub fn front_leg(&self) -> AgencyTba {
        AgencyTba::builder()
            .id(InstrumentId::new(format!("{}-FRONT", self.id.as_str())))
            .agency(self.agency)
            .coupon(self.coupon)
            .term(self.term)
            .settlement_year(self.front_settlement_year)
            .settlement_month(self.front_settlement_month)
            .notional(self.notional)
            .trade_price(self.front_price)
            .discount_curve_id(self.discount_curve_id.clone())
            .build()
            .expect("Front leg construction")
    }

    /// Create the back-month TBA leg.
    pub fn back_leg(&self) -> AgencyTba {
        AgencyTba::builder()
            .id(InstrumentId::new(format!("{}-BACK", self.id.as_str())))
            .agency(self.agency)
            .coupon(self.coupon)
            .term(self.term)
            .settlement_year(self.back_settlement_year)
            .settlement_month(self.back_settlement_month)
            .notional(self.notional)
            .trade_price(self.back_price)
            .discount_curve_id(self.discount_curve_id.clone())
            .build()
            .expect("Back leg construction")
    }

    /// Calculate days between settlement dates.
    pub fn settlement_days(&self) -> finstack_core::Result<i64> {
        let front = self.front_leg().get_settlement_date()?;
        let back = self.back_leg().get_settlement_date()?;
        Ok((back - front).whole_days())
    }
}

impl crate::instruments::common::traits::CurveDependencies for DollarRoll {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl crate::instruments::common::traits::Instrument for DollarRoll {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::DollarRoll
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
        crate::instruments::dollar_roll::pricer::price_dollar_roll(self, market, as_of)
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

    fn scenario_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&mut self.pricing_overrides)
    }

    fn scenario_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&self.pricing_overrides)
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for DollarRoll {
    fn discount_curve_id(&self) -> &CurveId {
        &self.discount_curve_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dollar_roll_example() {
        let roll = DollarRoll::example();
        assert_eq!(roll.agency, AgencyProgram::Fnma);
        assert!((roll.coupon - 0.04).abs() < 1e-10);
    }

    #[test]
    fn test_drop_calculation() {
        let roll = DollarRoll::example();
        let drop = roll.drop();

        // Front price 98.5 - back price 98.0 = 0.5
        assert!((drop - 0.5).abs() < 1e-10);

        // 0.5 points = 16/32nds
        let drop_32 = roll.drop_32nds();
        assert!((drop_32 - 16.0).abs() < 1e-10);
    }

    #[test]
    fn test_leg_creation() {
        let roll = DollarRoll::example();

        let front = roll.front_leg();
        let back = roll.back_leg();

        assert_eq!(front.agency, roll.agency);
        assert_eq!(back.agency, roll.agency);
        assert!((front.trade_price - roll.front_price).abs() < 1e-10);
        assert!((back.trade_price - roll.back_price).abs() < 1e-10);
    }

    #[test]
    fn test_settlement_days() {
        let roll = DollarRoll::example();
        let days = roll.settlement_days().expect("valid dates");

        // One month apart should be roughly 28-31 days
        assert!((25..=35).contains(&days));
    }
}
