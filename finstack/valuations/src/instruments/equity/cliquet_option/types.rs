//! Cliquet option instrument definition.

use crate::impl_instrument_base;
use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId, PriceId};
use time::macros::date;

/// Cliquet option instrument.
#[derive(
    Clone,
    Debug,
    finstack_valuations_macros::FinancialBuilder,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct CliquetOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Underlying asset ticker symbol
    pub underlying_ticker: crate::instruments::equity::spot::Ticker,
    /// Reset dates for periodic return locking
    #[schemars(with = "Vec<String>")]
    pub reset_dates: Vec<Date>,
    /// Explicit terminal expiry date for the structure.
    #[schemars(with = "String")]
    pub expiry: Date,
    /// Local cap on individual period returns
    pub local_cap: f64,
    /// Local floor on individual period returns (default 0.0)
    pub local_floor: f64,
    /// Global cap on sum of all period returns
    pub global_cap: f64,
    /// Global floor on sum of all period returns (default 0.0)
    pub global_floor: f64,
    /// Notional amount
    pub notional: Money,
    /// Day count convention
    pub day_count: finstack_core::dates::DayCount,
    /// Discount curve ID for present value calculations
    pub discount_curve_id: CurveId,
    /// Spot price identifier
    pub spot_id: PriceId,
    /// Volatility surface ID
    pub vol_surface_id: CurveId,
    /// Optional dividend yield curve ID
    pub div_yield_id: Option<CurveId>,
    /// Pricing overrides (manual price, yield, spread)
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and grouping
    pub attributes: Attributes,
    /// Payoff aggregation type (default: Additive)
    #[builder(default)]
    #[serde(default)]
    pub payoff_type: CliquetPayoffType,
}

/// Cliquet payoff aggregation type.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
pub enum CliquetPayoffType {
    /// Additive: Sum of period returns
    #[default]
    Additive,
    /// Multiplicative: Product of (1 + period returns) - 1
    Multiplicative,
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common_impl::traits::CurveDependencies for CliquetOption {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl CliquetOption {
    /// Create a canonical example cliquet option (quarterly resets with local/global caps).
    pub fn example() -> finstack_core::Result<Self> {
        use finstack_core::currency::Currency;
        use finstack_core::dates::DayCount;
        let reset_dates = vec![
            date!(2024 - 03 - 29),
            date!(2024 - 06 - 28),
            date!(2024 - 09 - 30),
            date!(2024 - 12 - 31),
        ];
        CliquetOption::builder()
            .id(InstrumentId::new("CLIQ-SPX-QTR"))
            .underlying_ticker("SPX".to_string())
            .reset_dates(reset_dates)
            .expiry(date!(2024 - 12 - 31))
            .local_cap(0.05) // 5% per period
            .local_floor(0.0) // 0% per period (min)
            .global_cap(0.20) // 20% max cumulative
            .global_floor(0.0) // 0% min cumulative
            .notional(Money::new(100_000.0, Currency::USD))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".into())
            .vol_surface_id(CurveId::new("SPX-VOL"))
            .div_yield_id_opt(Some(CurveId::new("SPX-DIV")))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
    }
}

impl crate::instruments::common_impl::traits::Instrument for CliquetOption {
    impl_instrument_base!(crate::pricer::InstrumentType::CliquetOption);

    fn default_model(&self) -> crate::pricer::ModelKey {
        crate::pricer::ModelKey::MonteCarloGBM
    }

    fn market_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::dependencies::MarketDependencies>
    {
        crate::instruments::common_impl::dependencies::MarketDependencies::from_curves_and_equity(
            self,
        )
    }

    fn base_value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::equity::cliquet_option::pricer;
        pricer::compute_pv(self, market, as_of)
    }

    fn effective_start_date(&self) -> Option<Date> {
        self.reset_dates.first().copied()
    }

    fn expiry(&self) -> Option<Date> {
        Some(self.expiry)
    }

    fn pricing_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&mut self.pricing_overrides)
    }

    fn pricing_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&self.pricing_overrides)
    }
}

crate::impl_empty_cashflow_provider!(
    CliquetOption,
    crate::cashflow::builder::CashflowRepresentation::Placeholder
);
