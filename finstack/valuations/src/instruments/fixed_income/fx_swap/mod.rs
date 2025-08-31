//! FX Swap instrument (boilerplate implementation).
//!
//! An FX swap exchanges notional amounts in two currencies on the near date
//! and reverses the exchange on the far date at a pre-agreed forward rate.
//! This module provides a minimal scaffold of the instrument type and wiring
//! to the pricing and metrics framework. Valuation logic is intentionally
//! minimal and returns zero PV in the quote currency until completed.

pub mod metrics;

use crate::impl_attributable;
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use crate::traits::{Attributes, Priceable};
use finstack_core::prelude::*;
use finstack_core::F;

/// FX Swap instrument definition (boilerplate)
#[derive(Clone, Debug)]
pub struct FxSwap {
    /// Unique instrument identifier
    pub id: String,
    /// Base currency (foreign)
    pub base_currency: Currency,
    /// Quote currency (domestic)
    pub quote_currency: Currency,
    /// Near leg settlement date (spot leg)
    pub near_date: Date,
    /// Far leg settlement date (forward leg)
    pub far_date: Date,
    /// Notional amount in base currency (exchanged on near, reversed on far)
    pub base_notional: Money,
    /// Domestic discount curve id (quote currency)
    pub domestic_disc_id: &'static str,
    /// Foreign discount curve id (base currency)
    pub foreign_disc_id: &'static str,
    /// Optional near leg FX rate (quote per base). If None, source from market.
    pub near_rate: Option<F>,
    /// Optional far leg FX rate (quote per base). If None, source from forwards.
    pub far_rate: Option<F>,
    /// Attributes for tagging and selection
    pub attributes: Attributes,
}

impl FxSwap {
    /// Create a new FX swap with required fields
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        base_currency: Currency,
        quote_currency: Currency,
        near_date: Date,
        far_date: Date,
        base_notional: Money,
        domestic_disc_id: &'static str,
        foreign_disc_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            base_currency,
            quote_currency,
            near_date,
            far_date,
            base_notional,
            domestic_disc_id,
            foreign_disc_id,
            near_rate: None,
            far_rate: None,
            attributes: Attributes::new(),
        }
    }

    /// Builder entrypoint
    pub fn builder() -> FxSwapBuilder {
        FxSwapBuilder::new()
    }
}

impl Priceable for FxSwap {
    /// Minimal PV implementation: returns 0 in quote currency as placeholder
    fn value(&self, _curves: &finstack_core::market_data::multicurve::CurveSet, _as_of: Date) -> finstack_core::Result<Money> {
        Ok(Money::new(0.0, self.quote_currency))
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::multicurve::CurveSet,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        let instrument: crate::instruments::Instrument = crate::instruments::Instrument::FxSwap(self.clone());
        crate::instruments::build_with_metrics(instrument, curves, as_of, base_value, metrics)
    }

    fn price(
        &self,
        curves: &finstack_core::market_data::multicurve::CurveSet,
        as_of: Date,
    ) -> finstack_core::Result<ValuationResult> {
        // No standard metrics yet for boilerplate; compute just PV
        self.price_with_metrics(curves, as_of, &[])
    }
}

// Generate standard Attributable implementation using macro
impl_attributable!(FxSwap);

// Builder pattern using simple struct for clarity (avoids too_many_arguments for new)
#[derive(Default)]
pub struct FxSwapBuilder {
    id: Option<String>,
    base_currency: Option<Currency>,
    quote_currency: Option<Currency>,
    near_date: Option<Date>,
    far_date: Option<Date>,
    base_notional: Option<Money>,
    domestic_disc_id: Option<&'static str>,
    foreign_disc_id: Option<&'static str>,
    near_rate: Option<F>,
    far_rate: Option<F>,
}

impl FxSwapBuilder {
    pub fn new() -> Self { Self::default() }

    pub fn id(mut self, value: impl Into<String>) -> Self { self.id = Some(value.into()); self }
    pub fn base_currency(mut self, value: Currency) -> Self { self.base_currency = Some(value); self }
    pub fn quote_currency(mut self, value: Currency) -> Self { self.quote_currency = Some(value); self }
    pub fn near_date(mut self, value: Date) -> Self { self.near_date = Some(value); self }
    pub fn far_date(mut self, value: Date) -> Self { self.far_date = Some(value); self }
    pub fn base_notional(mut self, value: Money) -> Self { self.base_notional = Some(value); self }
    pub fn domestic_disc_id(mut self, value: &'static str) -> Self { self.domestic_disc_id = Some(value); self }
    pub fn foreign_disc_id(mut self, value: &'static str) -> Self { self.foreign_disc_id = Some(value); self }
    pub fn near_rate(mut self, value: F) -> Self { self.near_rate = Some(value); self }
    pub fn far_rate(mut self, value: F) -> Self { self.far_rate = Some(value); self }

    pub fn build(self) -> finstack_core::Result<FxSwap> {
        Ok(FxSwap {
            id: self.id.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            base_currency: self.base_currency.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            quote_currency: self.quote_currency.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            near_date: self.near_date.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            far_date: self.far_date.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            base_notional: self.base_notional.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            domestic_disc_id: self.domestic_disc_id.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            foreign_disc_id: self.foreign_disc_id.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            near_rate: self.near_rate,
            far_rate: self.far_rate,
            attributes: Attributes::new(),
        })
    }
}

impl From<FxSwap> for crate::instruments::Instrument {
    fn from(value: FxSwap) -> Self {
        crate::instruments::Instrument::FxSwap(value)
    }
}

impl std::convert::TryFrom<crate::instruments::Instrument> for FxSwap {
    type Error = finstack_core::Error;

    fn try_from(value: crate::instruments::Instrument) -> finstack_core::Result<Self> {
        match value {
            crate::instruments::Instrument::FxSwap(v) => Ok(v),
            _ => Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            )),
        }
    }
}


