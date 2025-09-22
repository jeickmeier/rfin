//! Unified pricer trait and registry for instruments.
//!
//! This enables swapping pricing models on the fly without changing instrument code.
//!
//! Quick start:
//! ```rust no_run
//! use finstack_valuations::instruments::{Bond};
//! use finstack_valuations::instruments::common::{Pricer, register_pricer_for_key};
//! use finstack_valuations::install_pricers;
//! use finstack_core::dates::Date;
//! use time::Month;
//! use finstack_core::prelude::*;
//! use std::sync::Arc;
//! use finstack_valuations::cashflow::builder::ScheduleParams;
//!
//! // 1) Register built-in defaults (discounting for Bond/IRS)
//! install_pricers();
//!
//! // 2) Optionally register a custom model under a key
//! #[derive(Clone)]
//! struct MyBondPricer;
//! impl Pricer<Bond> for MyBondPricer {
//!     fn price(
//!         &self,
//!         bond: &Bond,
//!         ctx: &finstack_core::market_data::MarketContext,
//!         as_of: Date,
//!     ) -> finstack_core::Result<finstack_core::money::Money> {
//!         // Delegate to discounting by default (customize as desired)
//!         finstack_valuations::instruments::bond::pricing::engine::BondEngine::price(bond, ctx, as_of)
//!     }
//! }
//! register_pricer_for_key::<Bond, _>("my_model", MyBondPricer);
//!
//! // 3) Toggle model at the instrument-instance level via attributes:
//! let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
//! // Create an instrument
//! let maturity = Date::from_calendar_date(2028, Month::January, 1).unwrap();
//! let schedule = ScheduleParams {
//!     freq: finstack_core::dates::Frequency::semi_annual(),
//!     dc: finstack_core::dates::DayCount::Thirty360,
//!     bdc: finstack_core::dates::BusinessDayConvention::Following,
//!     calendar_id: None,
//!     stub: finstack_core::dates::StubKind::None,
//! };
//! let mut bond = Bond::builder()
//!     .id("TEST_BOND".into())
//!     .notional(Money::new(1_000_000.0, Currency::USD))
//!     .coupon(0.05)
//!     .issue(Date::from_calendar_date(2023, Month::January, 1).unwrap())
//!     .maturity(maturity)
//!     .schedule(schedule)
//!     .disc_id("USD-OIS".into())
//!     .build()
//!     .unwrap();
//! // let pv = bond.value(&ctx, as_of)?;
//! ```
//!
use crate::instruments::common::traits::Instrument;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use hashbrown::HashMap;
use once_cell::sync::Lazy;
use std::any::TypeId;
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};

/// Trait implemented by pricing engines for a specific instrument type `I`.
pub trait Pricer<I: Instrument>: Send + Sync + 'static {
    fn price(&self, instrument: &I, context: &MarketContext, as_of: Date) -> Result<Money>;
}

// -----------------------------------------------------------------------------
// Erased pricer registry
// -----------------------------------------------------------------------------

trait AnyPricer: Send + Sync {
    fn price_erased(
        &self,
        instrument: &dyn Instrument,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<Money>;
}

struct TypedPricer<I: Instrument, P: Pricer<I>> {
    inner: P,
    _marker: PhantomData<I>,
}

impl<I: Instrument + 'static, P: Pricer<I>> AnyPricer for TypedPricer<I, P> {
    fn price_erased(
        &self,
        instrument: &dyn Instrument,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        // Downcast instrument to expected concrete type; this should always succeed
        // because we look up by the concrete TypeId of I.
        let typed = instrument
            .as_any()
            .downcast_ref::<I>()
            .expect("instrument type mismatch for pricer");
        self.inner.price(typed, context, as_of)
    }
}

/// Holds default and keyed pricers for a concrete instrument type.
pub(crate) struct ModelRegistry {
    default: Option<Arc<dyn AnyPricer>>,                     // default model
    keyed: HashMap<String, Arc<dyn AnyPricer>>,              // named models
}

impl ModelRegistry {
    fn new() -> Self {
        Self {
            default: None,
            keyed: HashMap::new(),
        }
    }
}

/// Global pricer registry keyed by the concrete instrument TypeId.
pub(super) static PRICER_REGISTRY: Lazy<RwLock<HashMap<TypeId, ModelRegistry>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Register a pricer for the concrete type `I`.
/// Register a default pricer for the concrete type `I`.
pub fn register_pricer<I: Instrument + 'static, P: Pricer<I> + 'static>(pricer: P) {
    set_default_pricer::<I, P>(pricer);
}

/// Explicitly set default pricer for `I`.
pub fn set_default_pricer<I: Instrument + 'static, P: Pricer<I> + 'static>(pricer: P) {
    let mut map = PRICER_REGISTRY
        .write()
        .expect("Pricer registry poisoned");
    let entry = map.entry(TypeId::of::<I>()).or_insert_with(ModelRegistry::new);
    let wrapper = TypedPricer::<I, P> { inner: pricer, _marker: PhantomData };
    entry.default = Some(Arc::new(wrapper));
}

/// Register a pricer for `I` under a specific model key.
pub fn register_pricer_for_key<I: Instrument + 'static, P: Pricer<I> + 'static>(
    key: impl Into<String>,
    pricer: P,
) {
    let key = key.into();
    let mut map = PRICER_REGISTRY
        .write()
        .expect("Pricer registry poisoned");
    let entry = map.entry(TypeId::of::<I>()).or_insert_with(ModelRegistry::new);
    let wrapper = TypedPricer::<I, P> { inner: pricer, _marker: PhantomData };
    entry.keyed.insert(key, Arc::new(wrapper));
}

fn lookup_default_erased_pricer<I: Instrument + 'static>() -> Option<Arc<dyn AnyPricer>> {
    PRICER_REGISTRY
        .read()
        .expect("Pricer registry poisoned")
        .get(&TypeId::of::<I>())
        .and_then(|mr| mr.default.as_ref().cloned())
}

/// Price using a registered pricer if present; otherwise use the fallback.
pub fn price_with_pricer_or<I: Instrument + 'static, Fallback>(
    instrument: &I,
    context: &MarketContext,
    as_of: Date,
    fallback: Fallback,
) -> Result<Money>
where
    Fallback: FnOnce() -> Result<Money>,
{
    // Try keyed model based on instrument attributes first.
    if let Some(p) = lookup_pricer_for_instrument::<I>(instrument) {
        return p.price_erased(instrument, context, as_of);
    }
    // Fall back to default model for the instrument type.
    if let Some(p) = lookup_default_erased_pricer::<I>() {
        return p.price_erased(instrument, context, as_of);
    }
    fallback()
}

/// Returns true if a pricer is registered for `I`.
pub fn has_pricer<I: Instrument + 'static>() -> bool {
    PRICER_REGISTRY
        .read()
        .expect("Pricer registry poisoned")
        .get(&TypeId::of::<I>())
        .map(|mr| mr.default.is_some() || !mr.keyed.is_empty())
        .unwrap_or(false)
}

/// RAII guard that restores the previous pricer mapping on drop.
pub struct PricerGuard {
    type_id: TypeId,
    key: Option<String>,
    previous_default: Option<Arc<dyn AnyPricer>>,
    previous_keyed: Option<Arc<dyn AnyPricer>>,
}

impl Drop for PricerGuard {
    fn drop(&mut self) {
        let mut map = PRICER_REGISTRY
            .write()
            .expect("Pricer registry poisoned");
        if let Some(entry) = map.get_mut(&self.type_id) {
            if let Some(key) = &self.key {
                if let Some(prev) = self.previous_keyed.take() {
                    entry.keyed.insert(key.clone(), prev);
                } else {
                    entry.keyed.remove(key);
                }
            } else if let Some(prev) = self.previous_default.take() {
                entry.default = Some(prev);
            } else {
                entry.default = None;
            }
        }
    }
}

/// Temporarily install a pricer for instrument type `I`, returning a guard
/// that restores the previous pricer when dropped.
pub fn push_pricer<I: Instrument + 'static, P: Pricer<I> + 'static>(pricer: P) -> PricerGuard {
    let mut map = PRICER_REGISTRY
        .write()
        .expect("Pricer registry poisoned");
    let type_id = TypeId::of::<I>();
    let entry = map.entry(type_id).or_insert_with(ModelRegistry::new);
    let previous_default = entry.default.take();
    let wrapper = TypedPricer::<I, P> { inner: pricer, _marker: PhantomData };
    entry.default = Some(Arc::new(wrapper));
    PricerGuard {
        type_id,
        key: None,
        previous_default,
        previous_keyed: None,
    }
}

/// Run `func` with `pricer` temporarily installed for type `I`.
pub fn with_pricer<I, P, R, F>(pricer: P, func: F) -> R
where
    I: Instrument + 'static,
    P: Pricer<I> + 'static,
    F: FnOnce() -> R,
{
    let _guard = push_pricer::<I, P>(pricer);
    func()
}

/// Temporarily install a keyed pricer for instrument type `I`, returning a guard
/// that restores the previous keyed pricer when dropped.
pub fn push_pricer_for_key<I: Instrument + 'static, P: Pricer<I> + 'static>(
    key: impl Into<String>,
    pricer: P,
) -> PricerGuard {
    let key = key.into();
    let mut map = PRICER_REGISTRY
        .write()
        .expect("Pricer registry poisoned");
    let type_id = TypeId::of::<I>();
    let entry = map.entry(type_id).or_insert_with(ModelRegistry::new);
    let previous_keyed = entry.keyed.remove(&key);
    let wrapper = TypedPricer::<I, P> { inner: pricer, _marker: PhantomData };
    entry.keyed.insert(key.clone(), Arc::new(wrapper));
    PricerGuard {
        type_id,
        key: Some(key),
        previous_default: None,
        previous_keyed,
    }
}

/// Run `func` with `pricer` temporarily installed for type `I` under `key`.
pub fn with_pricer_for_key<I, P, R, F>(key: impl Into<String>, pricer: P, func: F) -> R
where
    I: Instrument + 'static,
    P: Pricer<I> + 'static,
    F: FnOnce() -> R,
{
    let _guard = push_pricer_for_key::<I, P>(key, pricer);
    func()
}

/// Returns true if a pricer is registered for `I` under `key`.
pub fn has_pricer_for_key<I: Instrument + 'static>(key: &str) -> bool {
    PRICER_REGISTRY
        .read()
        .expect("Pricer registry poisoned")
        .get(&TypeId::of::<I>())
        .map(|mr| mr.keyed.contains_key(key))
        .unwrap_or(false)
}

/// Lookup a pricer for an instrument using its attributes.
fn lookup_pricer_for_instrument<I: Instrument + 'static>(instrument: &I) -> Option<Arc<dyn AnyPricer>> {
    // Check for a model key in attributes metadata.
    // Accepted keys: "pricer", "model", "pricing_model".
    let attrs = instrument.attributes();
    let model_key = attrs
        .get_meta("pricer")
        .or_else(|| attrs.get_meta("model"))
        .or_else(|| attrs.get_meta("pricing_model"));

    let key = match model_key {
        Some(k) if !k.is_empty() => k,
        _ => return None,
    };

    PRICER_REGISTRY
        .read()
        .expect("Pricer registry poisoned")
        .get(&TypeId::of::<I>())
        .and_then(|mr| mr.keyed.get(key).cloned())
}


