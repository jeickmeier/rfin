//! FX digital (binary) option instrument definition.

use super::pricer::{self, FxDigitalOptionGreeks};
use crate::impl_instrument_base;
use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::OptionType;
use crate::instruments::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Payout type for digital (binary) options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
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
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
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
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
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
    pub fn example() -> finstack_core::Result<Self> {
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
    }

    fn price_internal(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        pricer::compute_pv(self, market, as_of)
    }

    fn greeks_internal(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<FxDigitalOptionGreeks> {
        pricer::compute_greeks(self, market, as_of)
    }
}

impl crate::instruments::common_impl::traits::Instrument for FxDigitalOption {
    impl_instrument_base!(crate::pricer::InstrumentType::FxDigitalOption);

    fn value(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.price_internal(curves, as_of)
    }

    fn expiry(&self) -> Option<finstack_core::dates::Date> {
        Some(self.expiry)
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        None
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

impl crate::instruments::common_impl::traits::OptionGreeksProvider for FxDigitalOption {
    fn option_greeks(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        request: &crate::instruments::common_impl::traits::OptionGreeksRequest,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::OptionGreeks> {
        use crate::instruments::common_impl::traits::{
            OptionDeltaProvider, OptionGammaProvider, OptionGreekKind, OptionGreeks,
            OptionRhoProvider, OptionThetaProvider, OptionVegaProvider,
        };

        match request.greek {
            OptionGreekKind::Delta => Ok(OptionGreeks {
                delta: Some(self.option_delta(market, as_of)?),
                ..OptionGreeks::default()
            }),
            OptionGreekKind::Gamma => Ok(OptionGreeks {
                gamma: Some(self.option_gamma(market, as_of)?),
                ..OptionGreeks::default()
            }),
            OptionGreekKind::Vega => Ok(OptionGreeks {
                vega: Some(self.option_vega(market, as_of)?),
                ..OptionGreeks::default()
            }),
            OptionGreekKind::Theta => Ok(OptionGreeks {
                theta: Some(self.option_theta(market, as_of)?),
                ..OptionGreeks::default()
            }),
            OptionGreekKind::Rho => Ok(OptionGreeks {
                rho_bp: Some(self.option_rho_bp(market, as_of)?),
                ..OptionGreeks::default()
            }),
            _ => Ok(OptionGreeks::default()),
        }
    }
}
