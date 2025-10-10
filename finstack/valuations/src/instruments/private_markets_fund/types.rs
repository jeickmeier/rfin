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
    pub id: InstrumentId,
    pub currency: Currency,
    pub spec: WaterfallSpec,
    pub events: Vec<FundEvent>,
    pub disc_id: Option<CurveId>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub attributes: Attributes,
}

impl PrivateMarketsFund {
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
            disc_id: None,
            attributes: Attributes::new(),
        }
    }

    pub fn with_discount_curve(mut self, disc_id: impl Into<CurveId>) -> Self {
        self.disc_id = Some(disc_id.into());
        self
    }

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
        if let Some(ref disc_id) = self.disc_id {
            use crate::instruments::common::discountable::Discountable;
            let flows = self.lp_cashflows()?;
            let disc = curves.get_discount_ref(disc_id.as_str())?;
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
        crate::instruments::build_with_metrics_dyn(self, curves, as_of, base_value, metrics)
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

