//! Private markets fund investment instrument type and implementations.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::traits::{Attributes, Instrument};
use crate::instruments::private_markets_fund::waterfall::{
    AllocationLedger, EquityWaterfallEngine, FundEvent, WaterfallSpec,
};
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;
use finstack_core::types::{CurveId, InstrumentId};

/// Private markets fund investment instrument.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct PrivateMarketsFund {
    /// id.
    pub id: InstrumentId,
    /// currency.
    pub currency: Currency,
    /// spec.
    pub spec: WaterfallSpec,
    /// events.
    pub events: Vec<FundEvent>,
    /// disc id.
    pub discount_curve_id: Option<CurveId>,
    /// Attributes.
    #[cfg_attr(feature = "serde", serde(default))]
    pub attributes: Attributes,
}

impl PrivateMarketsFund {
    /// new.
    pub fn new(
        id: impl Into<InstrumentId>,
        currency: Currency,
        spec: WaterfallSpec,
        events: Vec<FundEvent>,
    ) -> Self {
        Self {
            id: id.into(),
            currency,
            spec,
            events,
            discount_curve_id: None,
            attributes: Attributes::new(),
        }
    }

    /// Create a canonical example private markets fund with a simple waterfall and events.
    pub fn example() -> Self {
        use super::waterfall::{WaterfallSpec, WaterfallStyle};
        use finstack_core::currency::Currency;
        use finstack_core::dates::Date;
        use time::Month;
        // Build a simple European-style waterfall: Return of capital -> 8% pref -> 50% catchup -> 80/20 promote
        let spec = WaterfallSpec::builder()
            .style(WaterfallStyle::European)
            .return_of_capital()
            .preferred_irr(0.08)
            .catchup(0.5)
            .promote_tier(0.12, 0.8, 0.2)
            .build()
            .expect("WaterfallSpec build should not fail");
        // Define a few cashflow events: contributions in year 1, proceeds in year 3, distribution in year 4
        let events = vec![
            super::waterfall::FundEvent::contribution(
                Date::from_calendar_date(2024, Month::January, 15).unwrap(),
                Money::new(5_000_000.0, Currency::USD),
            ),
            super::waterfall::FundEvent::contribution(
                Date::from_calendar_date(2024, Month::June, 15).unwrap(),
                Money::new(2_000_000.0, Currency::USD),
            ),
            super::waterfall::FundEvent::proceeds(
                Date::from_calendar_date(2026, Month::March, 1).unwrap(),
                Money::new(4_000_000.0, Currency::USD),
                "DEAL-1",
            ),
            super::waterfall::FundEvent::distribution(
                Date::from_calendar_date(2027, Month::January, 1).unwrap(),
                Money::new(4_000_000.0, Currency::USD),
            ),
        ];
        PrivateMarketsFund::new(
            InstrumentId::new("PMF-EXAMPLE"),
            Currency::USD,
            spec,
            events,
        )
        .with_discount_curve("USD-OIS")
    }
    /// with discount curve.
    pub fn with_discount_curve(mut self, discount_curve_id: impl Into<CurveId>) -> Self {
        self.discount_curve_id = Some(discount_curve_id.into());
        self
    }

    /// run waterfall.
    pub fn run_waterfall(&self) -> finstack_core::Result<AllocationLedger> {
        for event in &self.events {
            if event.amount.currency() != self.currency {
                return Err(finstack_core::Error::CurrencyMismatch {
                    expected: self.currency,
                    actual: event.amount.currency(),
                });
            }
        }
        let engine = EquityWaterfallEngine::new(&self.spec);
        engine.run(&self.events)
    }

    /// lp cashflows.
    pub fn lp_cashflows(&self) -> finstack_core::Result<Vec<(Date, Money)>> {
        let ledger = self.run_waterfall()?;
        Ok(ledger.lp_cashflows())
    }
}

// Attributable is provided via blanket impl for all Instrument types

impl Instrument for PrivateMarketsFund {
    fn id(&self) -> &str {
        self.id.as_str()
    }
    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::PrivateMarketsFund
    }
    fn as_any(&self) -> &dyn std::any::Any {
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

    // === Pricing Methods ===

    fn value(&self, curves: &MarketContext, _as_of: Date) -> finstack_core::Result<Money> {
        if let Some(ref discount_curve_id) = self.discount_curve_id {
            use crate::instruments::common::discountable::Discountable;
            let flows = self.lp_cashflows()?;
            let disc = curves.get_discount_ref(discount_curve_id.as_str())?;
            flows.npv(disc, disc.base_date(), self.spec.irr_basis)
        } else {
            let ledger = self.run_waterfall()?;
            let residual_value = ledger
                .rows
                .last()
                .map(|r| r.lp_unreturned)
                .unwrap_or_else(|| Money::new(0.0, self.currency));
            Ok(residual_value)
        }
    }

    fn price_with_metrics(
        &self,
        curves: &MarketContext,
        as_of: Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
        )
    }
}

impl CashflowProvider for PrivateMarketsFund {
    fn build_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        self.lp_cashflows()
    }
}
