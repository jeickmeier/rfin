//! Quanto option instrument definition.

use crate::instruments::common::traits::Attributes;
use crate::instruments::OptionType;
use crate::instruments::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Quanto option instrument.
///
/// Quanto options have payoffs that depend on an underlying asset in one currency
/// but are settled in another currency, creating FX exposure.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QuantoOption {
    pub id: InstrumentId,
    pub underlying_ticker: String,
    pub equity_strike: Money,
    pub option_type: OptionType,
    pub expiry: Date,
    pub notional: f64,
    pub domestic_currency: Currency,
    pub foreign_currency: Currency,
    pub correlation: f64, // Correlation between equity and FX
    pub day_count: finstack_core::dates::DayCount,
    pub disc_id: CurveId,
    pub spot_id: String,
    pub vol_id: CurveId,
    pub div_yield_id: Option<String>,
    pub fx_rate_id: Option<String>,
    pub fx_vol_id: Option<CurveId>,
    pub pricing_overrides: PricingOverrides,
    pub attributes: Attributes,
}

impl QuantoOption {
    /// Calculate the net present value of this quanto option.
    #[cfg(feature = "mc")]
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::quanto_option::pricer;
        pricer::npv(self, curves, as_of)
    }
}

impl crate::instruments::common::traits::Instrument for QuantoOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::QuantoOption
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
        market: &finstack_core::market_data::MarketContext,
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
                "MC feature required for QuantoOption pricing".to_string(),
            ))
        }
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            self, market, as_of, base_value, metrics,
        )
    }
}
