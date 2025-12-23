//! Autocallable structured product instrument definition.

use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Final payoff type for autocallable products.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FinalPayoffType {
    /// Capital protection: max(floor, participation * min(S_T/S_0, cap))
    CapitalProtection {
        /// Minimum return floor (e.g., 1.0 for 100% protection)
        floor: f64,
    },
    /// Participation: 1 + participation_rate * max(0, S_T/S_0 - 1)
    Participation {
        /// Participation rate in upside (e.g., 1.0 for 100% participation)
        rate: f64,
    },
    /// Knock-in put: Put option if barrier breached, otherwise return principal
    KnockInPut {
        /// Strike price for knock-in put option
        strike: f64,
    },
}

/// Autocallable structured product instrument.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Autocallable {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Underlying asset ticker symbol
    pub underlying_ticker: String,
    /// Observation dates for autocall and coupon checks
    pub observation_dates: Vec<Date>,
    /// Autocall barrier levels (as ratios of initial spot, e.g., 1.0 = 100%)
    pub autocall_barriers: Vec<f64>, // Ratios relative to initial spot
    /// Coupon amounts paid if observation barrier is met
    pub coupons: Vec<f64>,
    /// Final barrier level for final payoff determination
    pub final_barrier: f64,
    /// Type of final payoff (capital protection, participation, knock-in put)
    pub final_payoff_type: FinalPayoffType,
    /// Participation rate in underlying performance
    pub participation_rate: f64,
    /// Cap level for final payoff (maximum return)
    pub cap_level: f64,
    /// Notional amount
    pub notional: Money,
    /// Day count convention for interest calculations
    pub day_count: finstack_core::dates::DayCount,
    /// Discount curve ID for present value calculations
    pub discount_curve_id: CurveId,
    /// Spot price identifier for underlying asset
    pub spot_id: String,
    /// Volatility surface ID for option pricing
    pub vol_surface_id: CurveId,
    /// Optional dividend yield curve ID
    pub div_yield_id: Option<CurveId>,
    /// Pricing overrides (manual price, yield, spread)
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and grouping
    pub attributes: Attributes,
}

impl Autocallable {
    /// Create a canonical example autocallable (quarterly observations, simple barriers/coupons).
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use finstack_core::dates::DayCount;
        use time::Month;
        let observation_dates = vec![
            Date::from_calendar_date(2024, Month::March, 29).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::June, 28).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::September, 30).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::December, 31).expect("Valid example date"),
        ];
        let autocall_barriers = vec![1.0, 1.0, 1.0, 1.0]; // 100% of initial
        let coupons = vec![0.02, 0.02, 0.02, 0.02]; // 2% per observation if called
        AutocallableBuilder::new()
            .id(InstrumentId::new("AUTO-SPX-QTR"))
            .underlying_ticker("SPX".to_string())
            .observation_dates(observation_dates)
            .autocall_barriers(autocall_barriers)
            .coupons(coupons)
            .final_barrier(0.6) // 60% final KI barrier
            .final_payoff_type(FinalPayoffType::Participation { rate: 1.0 })
            .participation_rate(1.0)
            .cap_level(1.5) // 150% cap
            .notional(Money::new(100_000.0, Currency::USD))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".to_string())
            .vol_surface_id(CurveId::new("SPX-VOL"))
            .div_yield_id_opt(Some(CurveId::new("SPX-DIV")))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Example Autocallable construction should not fail")
    }
    /// Calculate the net present value of this autocallable.
    #[cfg(feature = "mc")]
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::autocallable::pricer;
        pricer::npv(self, curves, as_of)
    }
}

impl crate::instruments::common::traits::Instrument for Autocallable {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::Autocallable
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
        #[cfg(feature = "mc")]
        {
            self.npv(market, as_of)
        }
        #[cfg(not(feature = "mc"))]
        {
            let _ = (market, as_of);
            Err(finstack_core::Error::Validation(
                "MC feature required for Autocallable pricing".to_string(),
            ))
        }
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
}
