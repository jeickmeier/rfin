//! FX digital (binary) option instrument definition.

use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::OptionType;
use crate::instruments::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Payout type for digital (binary) options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DigitalPayoutType {
    /// Pays a fixed cash amount in the payout currency if ITM at expiry.
    CashOrNothing,
    /// Pays one unit of the foreign (base) currency if ITM at expiry.
    AssetOrNothing,
}

/// FX digital (binary) option instrument.
///
/// Pays a fixed cash amount if the option expires in-the-money.
/// Two payout types:
/// - Cash-or-nothing: pays a fixed amount in the payout currency
/// - Asset-or-nothing: pays the spot rate (one unit of foreign currency)
///
/// # Pricing
///
/// Uses Garman-Kohlhagen adapted formulas:
///
/// **Cash-or-nothing call**: `PV = e^{-r_d T} × N(d2) × payout_amount`
/// **Cash-or-nothing put**: `PV = e^{-r_d T} × N(-d2) × payout_amount`
/// **Asset-or-nothing call**: `PV = S × e^{-r_f T} × N(d1) × notional`
/// **Asset-or-nothing put**: `PV = S × e^{-r_f T} × N(-d1) × notional`
///
/// # References
///
/// - Reiner, E., & Rubinstein, M. (1991). "Unscrambling the Binary Code."
///   *Risk Magazine*, 4(9), 75-83.
/// - Wystup, U. (2006). *FX Options and Structured Products*. Wiley.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FxDigitalOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Base currency (foreign currency)
    pub base_currency: Currency,
    /// Quote currency (domestic currency)
    pub quote_currency: Currency,
    /// Strike exchange rate (quote per base)
    pub strike: f64,
    /// Option type (call or put on base currency)
    pub option_type: OptionType,
    /// Payout type (cash-or-nothing or asset-or-nothing)
    pub payout_type: DigitalPayoutType,
    /// Fixed payout amount (used for cash-or-nothing; for asset-or-nothing this
    /// is the notional of foreign currency delivered)
    pub payout_amount: Money,
    /// Option expiry date
    pub expiry: Date,
    /// Day count convention
    pub day_count: DayCount,
    /// Notional amount in base currency
    pub notional: Money,
    /// Domestic currency discount curve ID
    pub domestic_discount_curve_id: CurveId,
    /// Foreign currency discount curve ID
    pub foreign_discount_curve_id: CurveId,
    /// FX volatility surface ID
    pub vol_surface_id: CurveId,
    /// Pricing overrides (manual price, yield, spread)
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and grouping
    pub attributes: Attributes,
}

impl crate::instruments::common_impl::traits::CurveDependencies for FxDigitalOption {
    fn curve_dependencies(&self) -> crate::instruments::common_impl::traits::InstrumentCurves {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.domestic_discount_curve_id.clone())
            .discount(self.foreign_discount_curve_id.clone())
            .build()
    }
}

impl FxDigitalOption {
    /// Create a canonical example FX digital option for testing and documentation.
    ///
    /// Returns a 6-month EUR/USD cash-or-nothing digital call.
    pub fn example() -> Self {
        use time::macros::date;
        Self::builder()
            .id(InstrumentId::new("FXDIG-EURUSD-CALL"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .strike(1.12)
            .option_type(OptionType::Call)
            .payout_type(DigitalPayoutType::CashOrNothing)
            .payout_amount(Money::new(1_000_000.0, Currency::USD))
            .expiry(date!(2024 - 06 - 21))
            .day_count(DayCount::Act365F)
            .notional(Money::new(1_000_000.0, Currency::EUR))
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .vol_surface_id(CurveId::new("EURUSD-VOL"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .unwrap_or_else(|_| {
                unreachable!("Example FX digital option with valid constants should never fail")
            })
    }

    fn price_internal(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let calculator = super::calculator::FxDigitalOptionCalculator::default();
        calculator.npv(self, market, as_of)
    }

    fn greeks_internal(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<super::calculator::FxDigitalOptionGreeks> {
        let calculator = super::calculator::FxDigitalOptionCalculator::default();
        calculator.compute_greeks(self, market, as_of)
    }
}

impl crate::instruments::common_impl::traits::Instrument for FxDigitalOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::FxDigitalOption
    }

    fn as_any(&self) -> &dyn std::any::Any {
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
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.price_internal(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common_impl::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
            None,
            None,
        )
    }

    fn expiry(&self) -> Option<finstack_core::dates::Date> {
        Some(self.expiry)
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        None
    }
}

impl crate::instruments::common_impl::traits::OptionDeltaProvider for FxDigitalOption {
    fn option_delta(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        Ok(self.greeks_internal(market, as_of)?.delta)
    }
}

impl crate::instruments::common_impl::traits::OptionGammaProvider for FxDigitalOption {
    fn option_gamma(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        Ok(self.greeks_internal(market, as_of)?.gamma)
    }
}

impl crate::instruments::common_impl::traits::OptionVegaProvider for FxDigitalOption {
    fn option_vega(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        Ok(self.greeks_internal(market, as_of)?.vega)
    }
}

impl crate::instruments::common_impl::traits::OptionThetaProvider for FxDigitalOption {
    fn option_theta(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        Ok(self.greeks_internal(market, as_of)?.theta)
    }
}

impl crate::instruments::common_impl::traits::OptionRhoProvider for FxDigitalOption {
    fn option_rho_bp(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        // Rho domestic per 1bp
        Ok(self.greeks_internal(market, as_of)?.rho_domestic / 100.0)
    }
}
