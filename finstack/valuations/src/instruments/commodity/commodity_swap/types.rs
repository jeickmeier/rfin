//! Commodity swap types and implementations.
//!
//! Defines the `CommoditySwap` instrument for fixed-for-floating commodity
//! price exchange contracts. One party pays a fixed price per unit while
//! the other pays a floating price based on an index.

use crate::instruments::common_impl::traits::Attributes;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, CalendarRegistry, Date, ScheduleBuilder, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;

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
/// use finstack_valuations::instruments::commodity::commodity_swap::CommoditySwap;
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

    /// Calculate the present value of the fixed leg.
    pub fn fixed_leg_pv(&self, market: &MarketContext, as_of: Date) -> Result<f64> {
        let disc = market.get_discount(self.discount_curve_id.as_str())?;
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
    ///
    /// Projects floating prices from the `PriceCurve` referenced by `floating_index_id`,
    /// with optional index lag and period averaging.
    pub fn floating_leg_pv(&self, market: &MarketContext, as_of: Date) -> Result<f64> {
        let disc = market.get_discount(self.discount_curve_id.as_str())?;
        let schedule = self.payment_schedule(as_of)?;

        // Try to get PriceCurve for floating index
        let price_curve = market.get_price_curve(self.floating_index_id.as_str())?;

        let mut pv = 0.0;
        let mut prev_period_end = self.start_date;

        for payment_date in schedule {
            if payment_date < as_of {
                prev_period_end = payment_date;
                continue; // Skip past payments
            }

            // Period start is previous period end (or swap start for first period)
            let period_start = prev_period_end;
            let period_end = payment_date;

            // Get expected average price for this period
            let forward_price =
                self.expected_period_price(&price_curve, as_of, period_start, period_end)?;

            let df = disc.df_between_dates(as_of, payment_date)?;
            let period_value = self.notional_quantity * forward_price;
            pv += period_value * df;

            prev_period_end = payment_date;
        }

        Ok(pv)
    }

    /// Calculate expected average price for a period.
    ///
    /// Uses business day weighted averaging for the observation period, which is
    /// the market standard for commodity swaps. Weekends are excluded from the
    /// average (no calendar applied yet - just weekday filtering).
    ///
    /// # Arguments
    /// * `price_curve` - The commodity price curve
    /// * `as_of` - Valuation date
    /// * `period_start` - Start of the averaging period
    /// * `period_end` - End of the averaging period
    ///
    /// # Averaging Method
    ///
    /// Uses daily business day sampling for all periods (market standard for
    /// commodity swaps). When a `calendar_id` is provided and resolves to a
    /// valid holiday calendar, exchange holidays are also excluded from the
    /// average. Otherwise, only weekends are filtered.
    ///
    /// # Note on PriceCurve Evaluation
    ///
    /// This method uses `price_on_date(date)` to respect the curve's day count convention
    /// rather than hard-coding Act365F. If a date falls before the curve's base date,
    /// it falls back to the curve's spot price.
    fn expected_period_price(
        &self,
        price_curve: &finstack_core::market_data::term_structures::PriceCurve,
        _as_of: Date,
        period_start: Date,
        period_end: Date,
    ) -> Result<f64> {
        // Apply index lag if specified (shift observation window backwards)
        let lag_days = self.index_lag_days.unwrap_or(0);
        let obs_start = period_start - time::Duration::days(lag_days as i64);
        let obs_end = period_end - time::Duration::days(lag_days as i64);

        // Helper to get price, falling back to spot if date is before curve base
        let get_price = |date: Date| -> f64 {
            price_curve
                .price_on_date(date)
                .unwrap_or_else(|_| price_curve.spot_price())
        };

        // Resolve holiday calendar if available (Item 8: integrate holiday calendars)
        let calendar = self
            .calendar_id
            .as_deref()
            .and_then(|id| CalendarRegistry::global().resolve_str(id));

        // Business day filter: exclude weekends and exchange holidays
        let is_business_day = |date: Date| -> bool {
            let wd = date.weekday();
            if wd == time::Weekday::Saturday || wd == time::Weekday::Sunday {
                return false;
            }
            // If we have a holiday calendar, check it
            if let Some(cal) = &calendar {
                return cal.is_business_day(date);
            }
            true
        };

        // For past periods (both obs dates <= curve base), use spot price
        let curve_base = price_curve.base_date();
        if obs_end <= curve_base {
            return Ok(price_curve.spot_price());
        }

        // Market standard: daily business day sampling for all periods
        let mut sum = 0.0;
        let mut count = 0;
        let mut current = obs_start;

        while current <= obs_end {
            if is_business_day(current) {
                sum += get_price(current);
                count += 1;
            }
            current += time::Duration::days(1);
        }

        // Ensure we have at least one observation
        if count == 0 {
            // Fallback: use midpoint if no business days found (shouldn't happen in practice)
            let mid = obs_start + (obs_end - obs_start) / 2;
            return Ok(get_price(mid));
        }

        Ok(sum / count as f64)
    }

    /// Generate the payment schedule for this swap.
    pub fn payment_schedule(&self, _as_of: Date) -> Result<Vec<Date>> {
        // Market standard: Modified Following for commodity swaps (matches QuantLib/Bloomberg)
        let bdc = self.bdc.unwrap_or(BusinessDayConvention::ModifiedFollowing);

        let mut builder =
            ScheduleBuilder::new(self.start_date, self.end_date)?.frequency(self.payment_frequency);

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
    ///
    /// Returns net cashflows (floating - fixed for pay-fixed, fixed - floating otherwise)
    /// at each payment date.
    pub fn cashflows(&self, market: &MarketContext, as_of: Date) -> Result<Vec<(Date, Money)>> {
        let schedule = self.payment_schedule(as_of)?;
        let price_curve = market.get_price_curve(self.floating_index_id.as_str())?;

        let mut flows = Vec::new();
        let mut prev_period_end = self.start_date;

        for payment_date in schedule {
            if payment_date < as_of {
                prev_period_end = payment_date;
                continue;
            }

            let period_start = prev_period_end;
            let period_end = payment_date;

            let forward_price =
                self.expected_period_price(&price_curve, as_of, period_start, period_end)?;

            let net_cashflow = if self.pay_fixed {
                self.notional_quantity * (forward_price - self.fixed_price)
            } else {
                self.notional_quantity * (self.fixed_price - forward_price)
            };

            flows.push((payment_date, Money::new(net_cashflow, self.currency)));
            prev_period_end = payment_date;
        }

        Ok(flows)
    }
}

impl crate::instruments::common_impl::traits::CurveDependencies for CommoditySwap {
    fn curve_dependencies(&self) -> crate::instruments::common_impl::traits::InstrumentCurves {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.floating_index_id.clone())
            .build()
    }
}

impl crate::instruments::common_impl::traits::Instrument for CommoditySwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::CommoditySwap
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common_impl::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common_impl::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common_impl::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        let fixed_leg_pv = self.fixed_leg_pv(market, as_of)?;
        let floating_leg_pv = self.floating_leg_pv(market, as_of)?;

        let npv = if self.pay_fixed {
            // Pay fixed, receive floating
            floating_leg_pv - fixed_leg_pv
        } else {
            // Receive fixed, pay floating
            fixed_leg_pv - floating_leg_pv
        };

        Ok(finstack_core::money::Money::new(npv, self.currency))
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;
        crate::instruments::common_impl::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
            None,
            None,
        )
    }

    fn effective_start_date(&self) -> Option<Date> {
        Some(self.start_date)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::traits::Instrument;
    use finstack_core::market_data::term_structures::{DiscountCurve, PriceCurve};
    use time::Month;

    fn test_market(as_of: Date) -> MarketContext {
        // Create discount curve
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots([(0.0, 1.0), (0.5, 0.975), (1.0, 0.95), (2.0, 0.90)])
            .build()
            .expect("Valid discount curve");

        // Create price curve for NG forward prices (slight contango)
        let price_curve = PriceCurve::builder("NG-SPOT-AVG")
            .base_date(as_of)
            .spot_price(3.50)
            .knots([
                (0.0, 3.50),
                (0.25, 3.55),
                (0.5, 3.60),
                (0.75, 3.65),
                (1.0, 3.70),
            ])
            .build()
            .expect("Valid price curve");

        MarketContext::new()
            .insert_discount(disc)
            .insert_price_curve(price_curve)
    }

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
    fn test_commodity_swap_npv_at_market() {
        // When fixed price equals expected floating average, NPV should be ~0
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let market = test_market(as_of);

        let swap = CommoditySwap::builder()
            .id(InstrumentId::new("AT-MARKET-SWAP"))
            .commodity_type("Energy".to_string())
            .ticker("NG".to_string())
            .unit("MMBTU".to_string())
            .currency(Currency::USD)
            .notional_quantity(10000.0)
            .fixed_price(3.50) // Same as spot
            .floating_index_id(CurveId::new("NG-SPOT-AVG"))
            .pay_fixed(true)
            .start_date(as_of)
            .end_date(Date::from_calendar_date(2025, Month::June, 30).expect("valid date"))
            .payment_frequency(Tenor::new(1, finstack_core::dates::TenorUnit::Months))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .build()
            .expect("should build");

        let npv = swap.value(&market, as_of).expect("should price");

        // In contango (forward > spot), pay-fixed should receive more on floating leg
        // So NPV should be slightly positive
        assert!(
            npv.amount() > 0.0,
            "Pay-fixed swap in contango should have positive NPV, got {}",
            npv.amount()
        );
    }

    #[test]
    fn test_commodity_swap_pay_receive_symmetry() {
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let market = test_market(as_of);

        let pay_fixed = CommoditySwap::builder()
            .id(InstrumentId::new("PAY-FIXED"))
            .commodity_type("Energy".to_string())
            .ticker("NG".to_string())
            .unit("MMBTU".to_string())
            .currency(Currency::USD)
            .notional_quantity(10000.0)
            .fixed_price(3.55)
            .floating_index_id(CurveId::new("NG-SPOT-AVG"))
            .pay_fixed(true)
            .start_date(as_of)
            .end_date(Date::from_calendar_date(2025, Month::June, 30).expect("valid date"))
            .payment_frequency(Tenor::new(1, finstack_core::dates::TenorUnit::Months))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .build()
            .expect("should build");

        let receive_fixed = CommoditySwap::builder()
            .id(InstrumentId::new("RECEIVE-FIXED"))
            .commodity_type("Energy".to_string())
            .ticker("NG".to_string())
            .unit("MMBTU".to_string())
            .currency(Currency::USD)
            .notional_quantity(10000.0)
            .fixed_price(3.55)
            .floating_index_id(CurveId::new("NG-SPOT-AVG"))
            .pay_fixed(false) // Receiving fixed
            .start_date(as_of)
            .end_date(Date::from_calendar_date(2025, Month::June, 30).expect("valid date"))
            .payment_frequency(Tenor::new(1, finstack_core::dates::TenorUnit::Months))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .build()
            .expect("should build");

        let pay_npv = pay_fixed.value(&market, as_of).expect("should price");
        let recv_npv = receive_fixed.value(&market, as_of).expect("should price");

        // Offsetting swaps should net to zero
        let net = pay_npv.amount() + recv_npv.amount();
        assert!(
            net.abs() < 1e-10,
            "Pay + Receive NPV should sum to 0, got {}",
            net
        );
    }

    #[test]
    fn test_commodity_swap_cashflows() {
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let market = test_market(as_of);

        let swap = CommoditySwap::builder()
            .id(InstrumentId::new("CASHFLOW-TEST"))
            .commodity_type("Energy".to_string())
            .ticker("NG".to_string())
            .unit("MMBTU".to_string())
            .currency(Currency::USD)
            .notional_quantity(10000.0)
            .fixed_price(3.50)
            .floating_index_id(CurveId::new("NG-SPOT-AVG"))
            .pay_fixed(true)
            .start_date(as_of)
            .end_date(Date::from_calendar_date(2025, Month::March, 31).expect("valid date"))
            .payment_frequency(Tenor::new(1, finstack_core::dates::TenorUnit::Months))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .build()
            .expect("should build");

        let flows = swap.cashflows(&market, as_of).expect("should get flows");

        // Should have 3 payments (Jan, Feb, Mar end of month)
        assert_eq!(flows.len(), 3, "Expected 3 monthly payments");

        // All cashflows should be positive in contango (floating > fixed)
        for (date, cf) in &flows {
            assert!(
                cf.amount() > 0.0,
                "Cashflow on {} should be positive in contango, got {}",
                date,
                cf.amount()
            );
        }
    }

    #[test]
    fn test_commodity_swap_instrument_trait() {
        use crate::instruments::common_impl::traits::Instrument;

        let swap = CommoditySwap::example();

        assert_eq!(swap.id(), "NG-SWAP-2025");
        assert_eq!(swap.key(), crate::pricer::InstrumentType::CommoditySwap);
    }

    #[test]
    fn test_commodity_swap_curve_dependencies() {
        use crate::instruments::common_impl::traits::CurveDependencies;

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
