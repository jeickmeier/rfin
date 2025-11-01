//! Barrier option instrument definition.

use crate::instruments::common::traits::Attributes;
use crate::instruments::OptionType;
use crate::instruments::PricingOverrides;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Barrier type for barrier options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BarrierType {
    /// Up-and-out: option knocked out if S >= B
    UpAndOut,
    /// Up-and-in: option activated if S >= B
    UpAndIn,
    /// Down-and-out: option knocked out if S <= B
    DownAndOut,
    /// Down-and-in: option activated if S <= B
    DownAndIn,
}

/// Barrier option instrument.
///
/// Barrier options are options with a barrier level that can knock in or out.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BarrierOption {
    pub id: InstrumentId,
    pub underlying_ticker: String,
    pub strike: Money,
    pub barrier: Money,
    pub option_type: OptionType,
    pub barrier_type: BarrierType,
    pub expiry: Date,
    pub notional: f64,
    pub day_count: finstack_core::dates::DayCount,
    pub use_gobet_miri: bool,
    pub disc_id: CurveId,
    pub spot_id: String,
    pub vol_id: CurveId,
    pub div_yield_id: Option<String>,
    pub pricing_overrides: PricingOverrides,
    pub attributes: Attributes,
}

impl BarrierOption {
    /// Calculate the net present value of this barrier option.
    #[cfg(feature = "mc")]
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::barrier_option::pricer;
        pricer::npv(self, curves, as_of)
    }
}

impl crate::instruments::common::traits::Instrument for BarrierOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::BarrierOption
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
                "MC feature required for BarrierOption pricing".to_string(),
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
