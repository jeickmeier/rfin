//! Asian option instrument definition.

use crate::instruments::common::traits::Attributes;
use crate::instruments::OptionType;
use crate::instruments::PricingOverrides;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Averaging method for Asian options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AveragingMethod {
    /// Arithmetic average: (1/n) Σ S_i
    Arithmetic,
    /// Geometric average: (Π S_i)^(1/n)
    Geometric,
}

/// Asian option instrument.
///
/// Asian options depend on the average price over a period rather than
/// just the terminal price. Supports both call and put options with
/// arithmetic or geometric averaging.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AsianOption {
    pub id: InstrumentId,
    pub underlying_ticker: String,
    pub strike: Money,
    pub option_type: OptionType,
    pub averaging_method: AveragingMethod,
    pub expiry: Date,
    pub fixing_dates: Vec<Date>,
    pub notional: f64,
    pub day_count: finstack_core::dates::DayCount,
    pub disc_id: CurveId,
    pub spot_id: String,
    pub vol_id: CurveId,
    pub div_yield_id: Option<String>,
    pub pricing_overrides: PricingOverrides,
    pub attributes: Attributes,
}

impl AsianOption {
    /// Calculate the net present value of this Asian option using Monte Carlo.
    #[cfg(feature = "mc")]
    pub fn npv_mc(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::asian_option::pricer;
        pricer::npv(self, curves, as_of)
    }
    
    /// Calculate the net present value using analytical method (default).
    /// Uses geometric closed-form for geometric averaging, Turnbull-Wakeman for arithmetic.
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::asian_option::pricer::{
            AsianOptionAnalyticalGeometricPricer, AsianOptionSemiAnalyticalTwPricer,
        };
        use crate::pricer::Pricer;
        
        match self.averaging_method {
            AveragingMethod::Geometric => {
                let pricer = AsianOptionAnalyticalGeometricPricer::new();
                let result = pricer.price_dyn(self, curves, as_of)
                    .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;
                Ok(result.value)
            }
            AveragingMethod::Arithmetic => {
                let pricer = AsianOptionSemiAnalyticalTwPricer::new();
                let result = pricer.price_dyn(self, curves, as_of)
                    .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;
                Ok(result.value)
            }
        }
    }
}

impl crate::instruments::common::traits::Instrument for AsianOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::AsianOption
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
        // Default to analytical pricing
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
            self, market, as_of, base_value, metrics,
        )
    }
}
