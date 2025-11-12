//! CMS option instrument definition.

use crate::instruments::common::traits::Attributes;
use crate::instruments::OptionType;
use crate::instruments::PricingOverrides;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// CMS option instrument (cap/floor on CMS rates).
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CmsOption {
    pub id: InstrumentId,
    pub strike_rate: f64,
    pub cms_tenor: f64, // Tenor of the CMS swap (e.g., 10.0 for 10Y)
    pub fixing_dates: Vec<Date>,
    pub accrual_fractions: Vec<f64>,
    pub option_type: OptionType,
    pub notional: Money,
    pub day_count: finstack_core::dates::DayCount,
    pub discount_curve_id: CurveId,
    pub vol_surface_id: Option<CurveId>, // Optional volatility surface for CMS rates
    pub pricing_overrides: PricingOverrides,
    pub attributes: Attributes,
}

impl CmsOption {
    /// Calculate the net present value of this CMS option.
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::cms_option::pricer;
        pricer::npv(self, curves, as_of)
    }
}

impl crate::instruments::common::traits::Instrument for CmsOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::CmsOption
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
        self.npv(market, as_of)
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::MarketContext,
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
}

impl crate::instruments::common::pricing::HasDiscountCurve for CmsOption {
    fn discount_curve_id(&self) -> &CurveId {
        &self.discount_curve_id
    }
}
