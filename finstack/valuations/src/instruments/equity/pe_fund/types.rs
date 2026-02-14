//! Private markets fund investment instrument type and implementations.

use crate::cashflow::traits::CashflowProvider;
use crate::impl_instrument_base;
use crate::instruments::common_impl::traits::{Attributes, Instrument};
use crate::instruments::equity::pe_fund::waterfall::{
    AllocationLedger, EquityWaterfallEngine, FundEvent, WaterfallSpec,
};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use time::macros::date;

/// Private markets fund investment instrument.
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
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
    #[serde(default)]
    #[builder(default)]
    pub pricing_overrides: crate::instruments::PricingOverrides,
    /// Attributes for scenario selection and tagging
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
            pricing_overrides: crate::instruments::PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }

    /// Create a canonical example private markets fund with a simple waterfall and events.
    pub fn example() -> Self {
        use super::waterfall::{WaterfallSpec, WaterfallStyle};
        use finstack_core::currency::Currency;
        // Build a simple European-style waterfall: Return of capital -> 8% pref -> 50% catchup -> 80/20 promote
        let spec = WaterfallSpec::builder()
            .style(WaterfallStyle::European)
            .return_of_capital()
            .preferred_irr(0.08)
            .catchup(0.5)
            .promote_tier(0.12, 0.8, 0.2)
            .build()
            .unwrap_or_else(|_| {
                unreachable!("WaterfallSpec with valid constants should never fail")
            });
        // Define a few cashflow events: contributions in year 1, proceeds in year 3, distribution in year 4
        let events = vec![
            super::waterfall::FundEvent::contribution(
                date!(2024 - 01 - 15),
                Money::new(5_000_000.0, Currency::USD),
            ),
            super::waterfall::FundEvent::contribution(
                date!(2024 - 06 - 15),
                Money::new(2_000_000.0, Currency::USD),
            ),
            super::waterfall::FundEvent::proceeds(
                date!(2026 - 03 - 01),
                Money::new(4_000_000.0, Currency::USD),
                "DEAL-1",
            ),
            super::waterfall::FundEvent::distribution(
                date!(2027 - 01 - 01),
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
    impl_instrument_base!(crate::pricer::InstrumentType::PrivateMarketsFund);

    // === Pricing Methods ===

    fn value(&self, curves: &MarketContext, _as_of: Date) -> finstack_core::Result<Money> {
        if let Some(ref discount_curve_id) = self.discount_curve_id {
            use crate::instruments::common_impl::discountable::Discountable;
            let flows = self.lp_cashflows()?;
            let disc = curves.get_discount(discount_curve_id.as_str())?;
            flows.npv(disc.as_ref(), disc.base_date(), Some(self.spec.irr_basis))
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

    fn effective_start_date(&self) -> Option<Date> {
        None
    }

    fn as_cashflow_provider(&self) -> Option<&dyn CashflowProvider> {
        Some(self)
    }

    fn scenario_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&mut self.pricing_overrides)
    }

    fn scenario_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&self.pricing_overrides)
    }
}

impl CashflowProvider for PrivateMarketsFund {
    // Private markets funds don't have a simple notional concept
    // (commitment varies with capital calls/distributions)

    fn build_full_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        let flows = self.lp_cashflows()?;
        Ok(crate::cashflow::traits::schedule_from_dated_flows(
            flows,
            None,
            finstack_core::dates::DayCount::Act365F, // Standard for PE fund cashflows
        ))
    }
}
