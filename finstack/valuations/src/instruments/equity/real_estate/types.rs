//! Real estate asset valuation types and logic.

use crate::instruments::common_impl::traits::{
    Attributes, CurveDependencies, Instrument, InstrumentCurves,
};
use crate::pricer::InstrumentType;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Error as CoreError;
use std::any::Any;

/// Valuation method for a real estate asset.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealEstateValuationMethod {
    /// Discounted cashflow using an explicit NOI schedule and discount rate.
    Dcf,
    /// Direct capitalization using a stabilized NOI and cap rate.
    DirectCap,
}

/// Real estate asset valuation instrument.
///
/// Supports DCF (explicit NOI schedule) and direct capitalization valuation.
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct RealEstateAsset {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Currency for valuation.
    pub currency: Currency,
    /// Valuation date (base date for discounting).
    pub valuation_date: Date,
    /// Valuation method (DCF or DirectCap).
    pub valuation_method: RealEstateValuationMethod,
    /// Net operating income schedule (date, amount).
    pub noi_schedule: Vec<(Date, f64)>,
    /// Discount rate for DCF (annualized).
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub discount_rate: Option<f64>,
    /// Capitalization rate for direct cap (annualized).
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub cap_rate: Option<f64>,
    /// Optional stabilized NOI override for direct cap.
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub stabilized_noi: Option<f64>,
    /// Optional terminal cap rate for DCF (uses last NOI).
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub terminal_cap_rate: Option<f64>,
    /// Optional appraisal override value.
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub appraisal_value: Option<Money>,
    /// Day count convention for year fractions.
    pub day_count: DayCount,
    /// Discount curve identifier (for risk attribution).
    pub discount_curve_id: CurveId,
    /// Attributes for tagging and scenarios.
    #[builder(default)]
    pub attributes: Attributes,
}

impl RealEstateAsset {
    /// DCF valuation using **annual compounding** per real estate appraisal
    /// standards (RICS Red Book / USPAP).
    ///
    /// Real estate industry convention uses discrete annual discounting:
    /// ```text
    /// PV = NOI / (1 + r)^t
    /// ```
    /// rather than the continuous compounding (`exp(-r*t)`) used by capital
    /// markets instruments elsewhere in this library.  This is deliberate
    /// and aligns with how discount rates are quoted in property appraisals.
    fn npv_dcf(&self) -> finstack_core::Result<f64> {
        let discount_rate = self
            .discount_rate
            .ok_or_else(|| CoreError::Validation("Missing discount_rate for DCF".into()))?;
        if discount_rate <= -1.0 {
            return Err(CoreError::Validation(
                "discount_rate must be greater than -100%".into(),
            ));
        }

        let flows = self.future_noi_flows()?;
        let pv_flows: f64 = flows
            .iter()
            .map(|(date, amount)| {
                let t = self.year_fraction(self.valuation_date, *date);
                // Annual compounding per real estate appraisal convention
                amount / (1.0 + discount_rate).powf(t)
            })
            .sum();

        let pv_terminal = if let Some(cap_rate) = self.terminal_cap_rate {
            if cap_rate <= 0.0 {
                return Err(CoreError::Validation(
                    "terminal_cap_rate must be positive".into(),
                ));
            }
            let (terminal_date, terminal_noi) = flows
                .last()
                .ok_or_else(|| CoreError::Validation("NOI schedule is empty".into()))?;
            let terminal_value = terminal_noi / cap_rate;
            let t = self.year_fraction(self.valuation_date, *terminal_date);
            terminal_value / (1.0 + discount_rate).powf(t)
        } else {
            0.0
        };

        Ok(pv_flows + pv_terminal)
    }

    fn npv_direct_cap(&self) -> finstack_core::Result<f64> {
        let cap_rate = self
            .cap_rate
            .ok_or_else(|| CoreError::Validation("Missing cap_rate for direct cap".into()))?;
        if cap_rate <= 0.0 {
            return Err(CoreError::Validation("cap_rate must be positive".into()));
        }

        let noi = if let Some(noi) = self.stabilized_noi {
            noi
        } else {
            let flows = self.future_noi_flows()?;
            flows
                .last()
                .map(|(_, amount)| *amount)
                .ok_or_else(|| CoreError::Validation("NOI schedule is empty".into()))?
        };

        Ok(noi / cap_rate)
    }

    fn future_noi_flows(&self) -> finstack_core::Result<Vec<(Date, f64)>> {
        let mut flows: Vec<(Date, f64)> = self
            .noi_schedule
            .iter()
            .copied()
            .filter(|(date, _)| *date >= self.valuation_date)
            .collect();
        if flows.is_empty() {
            return Err(CoreError::Validation(
                "NOI schedule must include at least one flow on/after valuation_date".into(),
            ));
        }
        flows.sort_by_key(|(date, _)| *date);
        Ok(flows)
    }

    fn year_fraction(&self, start: Date, end: Date) -> f64 {
        self.day_count
            .year_fraction(start, end, DayCountCtx::default())
            .unwrap_or(0.0)
    }
}

impl Instrument for RealEstateAsset {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> InstrumentType {
        InstrumentType::RealEstateAsset
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn Instrument> {
        Box::new(self.clone())
    }

    fn value(&self, _market: &MarketContext, _as_of: Date) -> finstack_core::Result<Money> {
        if let Some(appraisal) = &self.appraisal_value {
            if appraisal.currency() != self.currency {
                return Err(CoreError::Validation(format!(
                    "Appraisal currency {} does not match instrument currency {}",
                    appraisal.currency(),
                    self.currency
                )));
            }
            return Ok(*appraisal);
        }

        let value = match self.valuation_method {
            RealEstateValuationMethod::Dcf => self.npv_dcf()?,
            RealEstateValuationMethod::DirectCap => self.npv_direct_cap()?,
        };

        Ok(finstack_core::money::Money::new(value, self.currency))
    }

    fn price_with_metrics(
        &self,
        market: &MarketContext,
        as_of: Date,
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
        None
    }
}
impl CurveDependencies for RealEstateAsset {
    fn curve_dependencies(&self) -> finstack_core::Result<InstrumentCurves> {
        InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}
