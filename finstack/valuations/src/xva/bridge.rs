//! Bridge between valuations instruments and `finstack-margin` XVA traits.

use std::sync::Arc;

use crate::instruments::DynInstrument;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use finstack_margin::xva::Valuable;

impl Valuable for DynInstrument {
    fn id(&self) -> &str {
        crate::instruments::Instrument::id(self)
    }

    fn value(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        crate::instruments::Instrument::value(self, market, as_of)
    }
}

#[derive(Clone)]
struct InstrumentValuable {
    inner: Arc<DynInstrument>,
}

impl Valuable for InstrumentValuable {
    fn id(&self) -> &str {
        self.inner.id()
    }

    fn value(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        self.inner.value(market, as_of)
    }
}

pub(crate) fn wrap_instruments(instruments: &[Arc<DynInstrument>]) -> Vec<Arc<dyn Valuable>> {
    instruments
        .iter()
        .cloned()
        .map(|instrument| Arc::new(InstrumentValuable { inner: instrument }) as Arc<dyn Valuable>)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::instruments::{DynInstrument, Instrument, Repo};
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::market_data::context::MarketContext;
    use finstack_margin::xva::Valuable;

    use crate::xva::{XvaConfig, XvaNettingSet};

    fn valuable_id<T: Valuable + ?Sized>(instrument: &T) -> &str {
        instrument.id()
    }

    #[test]
    fn valuations_xva_root_reexports_support_instrument_bridge() {
        let repo = Repo::example();
        let instrument: &DynInstrument = &repo;

        let _config = XvaConfig::default();
        let _netting_set = XvaNettingSet {
            id: "NS-VALUATIONS".into(),
            counterparty_id: "CP".into(),
            csa: None,
            reporting_currency: None,
        };

        assert_eq!(valuable_id(instrument), repo.id());
    }

    #[test]
    fn valuations_xva_wrapper_accepts_instrument_portfolios() {
        let repo: Arc<DynInstrument> = Arc::new(Repo::example());
        let instruments = vec![repo];
        let market = MarketContext::new();
        let as_of = Date::from_calendar_date(2024, time::Month::January, 1).expect("valid date");
        let config = XvaConfig::default();
        let netting_set = XvaNettingSet {
            id: "NS-VALUATIONS".into(),
            counterparty_id: "CP".into(),
            csa: None,
            reporting_currency: Some(Currency::USD),
        };

        let profile = crate::xva::exposure::compute_exposure_profile(
            &instruments,
            &market,
            as_of,
            &config,
            &netting_set,
        )
        .expect("valuations wrapper should accept instrument portfolios");

        assert_eq!(profile.times, config.time_grid);
        assert_eq!(profile.epe.len(), config.time_grid.len());
    }
}
