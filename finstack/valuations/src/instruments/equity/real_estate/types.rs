//! Real estate asset valuation types.

use super::pricer;
use crate::impl_instrument_base;
use crate::instruments::common_impl::traits::{
    Attributes, CurveDependencies, Instrument, InstrumentCurves,
};
use crate::pricer::InstrumentType;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Broad property classification for reporting / tagging.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RealEstatePropertyType {
    /// Office (CBD/suburban, single-tenant or multi-tenant).
    Office,
    /// Multifamily / residential rental.
    Multifamily,
    /// Retail (strip, mall, big box, high street).
    Retail,
    /// Industrial (warehouse, logistics, manufacturing).
    Industrial,
    /// Hospitality (hotel, resort).
    Hospitality,
    /// Mixed-use (multiple property types).
    MixedUse,
    /// Other / uncategorized.
    Other,
}

/// Valuation method for a real estate asset.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
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
    Clone,
    Debug,
    finstack_valuations_macros::FinancialBuilder,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct RealEstateAsset {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Currency for valuation.
    pub currency: Currency,
    /// Valuation date (base date for discounting).
    #[schemars(with = "String")]
    pub valuation_date: Date,
    /// Valuation method (DCF or DirectCap).
    pub valuation_method: RealEstateValuationMethod,
    /// Optional property type classification (for reporting).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property_type: Option<RealEstatePropertyType>,
    /// Net operating income schedule (date, amount).
    #[schemars(with = "Vec<(String, f64)>")]
    pub noi_schedule: Vec<(Date, f64)>,
    /// Capital expenditure schedule (date, amount). Values are treated as **positive outflows**.
    ///
    /// When present, cashflows are valued as `NOI - CapEx` (unlevered net cash flow).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<Vec<(String, f64)>>")]
    pub capex_schedule: Option<Vec<(Date, f64)>>,
    /// Discount rate for DCF (annualized).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discount_rate: Option<f64>,
    /// Capitalization rate for direct cap (annualized).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap_rate: Option<f64>,
    /// Optional stabilized NOI override for direct cap.
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stabilized_noi: Option<f64>,
    /// Optional terminal cap rate for DCF (uses last NOI).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_cap_rate: Option<f64>,
    /// Optional terminal growth rate used to project `NOI_{N+1}` for exit valuation.
    ///
    /// Market convention for exit-cap terminal value is \(TV = NOI_{N+1} / cap\_rate\_exit\).
    /// When not provided, defaults to 0 (uses last NOI as-is).
    /// Validation range is \([-100\%, 20\%]\) to guard against configuration errors.
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_growth_rate: Option<f64>,
    /// Optional sale/exit date that truncates the DCF horizon.
    ///
    /// When set, DCF only values unlevered flows up to and including `sale_date`.
    /// Terminal proceeds (if configured) are realized on `sale_date`.
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub sale_date: Option<Date>,
    /// Optional explicit gross sale price (terminal proceeds), before disposition costs.
    ///
    /// When set, this takes precedence over `terminal_cap_rate` for terminal proceeds.
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sale_price: Option<Money>,
    /// Optional purchase price (useful for IRR / cap rate metrics).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purchase_price: Option<Money>,
    /// Optional one-time acquisition cost deducted at `as_of` in DCF valuation.
    ///
    /// This is intended for closing costs, fees, and other transaction costs.
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acquisition_cost: Option<f64>,
    /// Optional detailed acquisition cost line items (positive outflows) deducted at `as_of`.
    #[builder(default)]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acquisition_costs: Vec<Money>,
    /// Optional disposition cost percentage applied to terminal value.
    ///
    /// A value of `0.02` represents 2% selling costs. Must be in \([0, 1)\).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disposition_cost_pct: Option<f64>,
    /// Optional detailed disposition cost line items (positive outflows) deducted from terminal proceeds.
    #[builder(default)]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disposition_costs: Vec<Money>,
    /// Optional appraisal override value.
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appraisal_value: Option<Money>,
    /// Day count convention for year fractions.
    pub day_count: DayCount,
    /// Discount curve identifier (for risk attribution).
    pub discount_curve_id: CurveId,
    /// Attributes for tagging and scenarios.
    #[builder(default)]
    #[serde(default)]
    pub pricing_overrides: crate::instruments::PricingOverrides,
    /// Attributes for scenario selection and tagging
    #[serde(default)]
    #[builder(default)]
    pub attributes: Attributes,
}

impl RealEstateAsset {
    /// Create a representative office building DCF valuation example.
    ///
    /// 5-year NOI schedule at $100K/year, 8% discount rate, 5.5% terminal
    /// cap rate, Act/365F day count convention.
    pub fn example() -> finstack_core::Result<Self> {
        use finstack_core::dates::DayCount;

        let valuation_date = time::macros::date!(2025 - 01 - 01);
        let noi_schedule: Vec<(Date, f64)> = (1..=5)
            .map(|y| {
                Date::from_calendar_date(2025 + y, time::Month::January, 1)
                    .map(|date| (date, 100_000.0))
                    .map_err(|error| finstack_core::Error::Validation(error.to_string()))
            })
            .collect::<finstack_core::Result<Vec<_>>>()?;

        Self::builder()
            .id(InstrumentId::new("RE-OFFICE-DCF"))
            .currency(Currency::USD)
            .valuation_date(valuation_date)
            .valuation_method(RealEstateValuationMethod::Dcf)
            .property_type_opt(Some(RealEstatePropertyType::Office))
            .noi_schedule(noi_schedule)
            .discount_rate_opt(Some(0.08))
            .terminal_cap_rate_opt(Some(0.055))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .attributes(Attributes::default())
            .build()
    }

    pub(crate) fn acquisition_cost_total(&self) -> finstack_core::Result<f64> {
        pricer::acquisition_cost_total(self)
    }

    /// Compute net sale proceeds (undiscounted) realized at `exit_date`, if configured.
    ///
    /// Precedence:
    /// - If `sale_price` is set: use it as the gross proceeds.
    /// - Else if `terminal_cap_rate` is set: use exit-cap convention `TV = NOI_{N+1} / cap_rate_exit`
    ///   with NOI taken as the last schedule entry on/before `exit_date`.
    ///
    /// Then apply:
    /// - `disposition_cost_pct` (pct of gross proceeds), and
    /// - `disposition_costs` (dollar line items).
    pub(crate) fn sale_proceeds_at(
        &self,
        as_of: Date,
        exit_date: Date,
    ) -> finstack_core::Result<Option<(Date, f64)>> {
        pricer::sale_proceeds_at(self, as_of, exit_date)
    }

    /// First future NOI amount on/after `as_of`.
    pub(crate) fn first_noi(&self, as_of: Date) -> finstack_core::Result<f64> {
        pricer::first_noi(self, as_of)
    }

    /// Unlevered net cash flows (NOI - CapEx) on/after `as_of`.
    pub(crate) fn unlevered_flows(&self, as_of: Date) -> finstack_core::Result<Vec<(Date, f64)>> {
        pricer::unlevered_flows(self, as_of)
    }

    /// NOI cash flows on/after `as_of`.
    pub(crate) fn noi_flows(&self, as_of: Date) -> finstack_core::Result<Vec<(Date, f64)>> {
        pricer::noi_flows(self, as_of)
    }

    /// Compute net sale proceeds at the terminal date (undiscounted), if configured.
    ///
    /// Uses the exit-cap convention `TV = NOI_{N+1} / cap_rate_exit`, optionally applying
    /// `disposition_cost_pct`.
    pub(crate) fn terminal_sale_proceeds(
        &self,
        as_of: Date,
    ) -> finstack_core::Result<Option<(Date, f64)>> {
        pricer::terminal_sale_proceeds(self, as_of)
    }
}

impl Instrument for RealEstateAsset {
    impl_instrument_base!(InstrumentType::RealEstateAsset);

    fn value(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        pricer::compute_pv(self, market, as_of)
    }

    fn effective_start_date(&self) -> Option<Date> {
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
impl CurveDependencies for RealEstateAsset {
    fn curve_dependencies(&self) -> finstack_core::Result<InstrumentCurves> {
        InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl crate::cashflow::traits::CashflowProvider for RealEstateAsset {
    fn cashflow_schedule(
        &self,
        _curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        let mut flows: Vec<(Date, Money)> = self
            .unlevered_flows(as_of)?
            .into_iter()
            .map(|(date, amount)| (date, Money::new(amount, self.currency)))
            .collect();

        if let Some((date, amount)) = self.terminal_sale_proceeds(as_of)? {
            flows.push((date, Money::new(amount, self.currency)));
        }
        flows.sort_by_key(|(date, _)| *date);

        Ok(crate::cashflow::traits::schedule_from_dated_flows(
            flows,
            self.day_count,
            crate::cashflow::traits::ScheduleBuildOpts {
                representation: crate::cashflow::builder::CashflowRepresentation::Projected,
                ..Default::default()
            },
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::cashflow::CashflowProvider;
    use finstack_core::dates::DayCount;

    #[test]
    fn real_estate_cashflow_schedule_emits_projected_unlevered_flows() {
        let valuation_date =
            Date::from_calendar_date(2025, time::Month::January, 1).expect("valid valuation date");
        let noi1 = Date::from_calendar_date(2026, time::Month::January, 1).expect("valid noi date");
        let noi2 = Date::from_calendar_date(2027, time::Month::January, 1).expect("valid noi date");

        let asset = RealEstateAsset::builder()
            .id(InstrumentId::new("RE-SCHEDULE"))
            .currency(Currency::USD)
            .valuation_date(valuation_date)
            .valuation_method(RealEstateValuationMethod::Dcf)
            .noi_schedule(vec![(noi1, 100.0), (noi2, 100.0)])
            .discount_rate_opt(Some(0.10))
            .terminal_cap_rate_opt(Some(0.08))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .attributes(Default::default())
            .build()
            .expect("asset should build");

        let schedule = asset
            .cashflow_schedule(&MarketContext::new(), valuation_date)
            .expect("real estate schedule");

        assert_eq!(
            schedule.meta.representation,
            crate::cashflow::builder::CashflowRepresentation::Projected
        );
        assert_eq!(schedule.flows.len(), 3);
        assert_eq!(schedule.flows[0].date, noi1);
        assert_eq!(schedule.flows[0].amount.amount(), 100.0);
        assert_eq!(schedule.flows[1].date, noi2);
        assert!(schedule.flows[2].amount.amount() > 100.0);
    }
}
