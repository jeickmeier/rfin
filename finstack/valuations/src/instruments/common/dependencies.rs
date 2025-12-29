//! Unified market data dependency representation for instruments.

use crate::instruments::common::traits::{
    CurveDependencies, CurveIdVec, EquityDependencies, EquityInstrumentDeps, Instrument,
    InstrumentCurves,
};
use finstack_core::currency::Currency;
use finstack_core::types::CurveId;

#[cfg(feature = "serde")]
use crate::instruments::json_loader::InstrumentJson;

/// FX pair identifier using base/quote currency ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FxPair {
    /// Base currency (numerator).
    pub base: Currency,
    /// Quote currency (denominator).
    pub quote: Currency,
}

impl FxPair {
    /// Create a new FX pair identifier.
    pub fn new(base: Currency, quote: Currency) -> Self {
        Self { base, quote }
    }
}

/// Unified dependency container for instrument market data requirements.
#[derive(Clone, Debug, Default)]
pub struct InstrumentDependencies {
    /// Curve dependencies grouped by type.
    pub curves: InstrumentCurves,
    /// Spot identifiers (equity, FX spot IDs, commodity spot IDs).
    pub spot_ids: Vec<String>,
    /// Volatility surface identifiers.
    pub vol_surface_ids: Vec<String>,
    /// FX pairs required for pricing (spot matrices).
    pub fx_pairs: Vec<FxPair>,
}

impl InstrumentDependencies {
    /// Create an empty dependency set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build dependencies from an instrument implementing core introspection hooks.
    pub fn from_instrument<T: Instrument + ?Sized>(instrument: &T) -> Self {
        let mut deps = Self::new();

        for id in instrument.required_discount_curves() {
            push_unique_curve(&mut deps.curves.discount_curves, id);
        }
        for id in instrument.required_hazard_curves() {
            push_unique_curve(&mut deps.curves.credit_curves, id);
        }
        if let Some((base, quote)) = instrument.fx_exposure() {
            deps.add_fx_pair(base, quote);
        }
        if let Some(spot_id) = instrument.spot_id() {
            deps.add_spot_id(spot_id);
        }
        if let Some(vol_id) = instrument.vol_surface_id() {
            deps.add_vol_surface_id(vol_id.as_str());
        }

        deps
    }

    /// Build dependencies from an instrument implementing [`CurveDependencies`].
    pub fn from_curve_dependencies<T: CurveDependencies>(instrument: &T) -> Self {
        let mut deps = Self::new();
        deps.add_curves(instrument.curve_dependencies());
        deps
    }

    /// Build dependencies from an instrument implementing [`EquityDependencies`].
    pub fn from_equity_dependencies<T: EquityDependencies>(instrument: &T) -> Self {
        let mut deps = Self::new();
        deps.add_equity_dependencies(instrument.equity_dependencies());
        deps
    }

    /// Build dependencies from an instrument implementing both curve and equity traits.
    pub fn from_curves_and_equity<T: CurveDependencies + EquityDependencies>(instrument: &T) -> Self {
        let mut deps = Self::new();
        deps.add_curves(instrument.curve_dependencies());
        deps.add_equity_dependencies(instrument.equity_dependencies());
        deps
    }

    /// Merge curve dependencies into this set.
    pub fn add_curves(&mut self, curves: InstrumentCurves) {
        for id in curves.discount_curves {
            push_unique_curve(&mut self.curves.discount_curves, id);
        }
        for id in curves.forward_curves {
            push_unique_curve(&mut self.curves.forward_curves, id);
        }
        for id in curves.credit_curves {
            push_unique_curve(&mut self.curves.credit_curves, id);
        }
    }

    /// Merge equity dependencies into this set.
    pub fn add_equity_dependencies(&mut self, deps: EquityInstrumentDeps) {
        if let Some(spot_id) = deps.spot_id {
            self.add_spot_id(spot_id);
        }
        if let Some(vol_surface_id) = deps.vol_surface_id {
            self.add_vol_surface_id(vol_surface_id);
        }
    }

    /// Add a spot identifier.
    pub fn add_spot_id(&mut self, id: impl Into<String>) {
        push_unique_string(&mut self.spot_ids, id.into());
    }

    /// Add a volatility surface identifier.
    pub fn add_vol_surface_id(&mut self, id: impl Into<String>) {
        push_unique_string(&mut self.vol_surface_ids, id.into());
    }

    /// Add an FX pair dependency.
    pub fn add_fx_pair(&mut self, base: Currency, quote: Currency) {
        push_unique_fx_pair(&mut self.fx_pairs, FxPair::new(base, quote));
    }

    /// Merge another dependency set into this one.
    pub fn merge(&mut self, other: InstrumentDependencies) {
        self.add_curves(other.curves);
        for id in other.spot_ids {
            self.add_spot_id(id);
        }
        for id in other.vol_surface_ids {
            self.add_vol_surface_id(id);
        }
        for pair in other.fx_pairs {
            self.add_fx_pair(pair.base, pair.quote);
        }
    }

    /// Build dependencies from a JSON-tagged instrument representation.
    #[cfg(feature = "serde")]
    pub fn from_instrument_json(instrument: &InstrumentJson) -> Self {
        match instrument {
            InstrumentJson::CommodityOption(i) => Self::from_curves_and_equity(i),
            InstrumentJson::BarrierOption(i) => Self::from_curves_and_equity(i),
            InstrumentJson::AsianOption(i) => Self::from_curves_and_equity(i),
            InstrumentJson::Autocallable(i) => Self::from_curves_and_equity(i),
            InstrumentJson::CliquetOption(i) => Self::from_equity_dependencies(i),
            InstrumentJson::LookbackOption(i) => Self::from_equity_dependencies(i),
            InstrumentJson::RangeAccrual(i) => Self::from_equity_dependencies(i),
            InstrumentJson::FxForward(i) => {
                let mut deps = Self::from_curve_dependencies(i);
                deps.add_fx_pair(i.base_currency, i.quote_currency);
                deps
            }
            InstrumentJson::FxSwap(i) => {
                let mut deps = Self::from_curve_dependencies(i);
                deps.add_fx_pair(i.base_currency, i.quote_currency);
                deps
            }
            InstrumentJson::FxSpot(i) => {
                let mut deps = Self::from_curve_dependencies(i);
                deps.add_fx_pair(i.base, i.quote);
                deps
            }
            InstrumentJson::FxOption(i) => {
                let mut deps = Self::new();
                deps.add_curves(
                    InstrumentCurves::builder()
                        .discount(i.domestic_discount_curve_id.clone())
                        .discount(i.foreign_discount_curve_id.clone())
                        .build(),
                );
                deps.add_vol_surface_id(i.vol_surface_id.as_str());
                deps.add_fx_pair(i.base_currency, i.quote_currency);
                deps
            }
            InstrumentJson::FxBarrierOption(i) => {
                let mut deps = Self::new();
                deps.add_curves(
                    InstrumentCurves::builder()
                        .discount(i.domestic_discount_curve_id.clone())
                        .discount(i.foreign_discount_curve_id.clone())
                        .build(),
                );
                deps.add_spot_id(i.fx_spot_id.as_str());
                deps.add_vol_surface_id(i.fx_vol_id.as_str());
                deps.add_fx_pair(i.foreign_currency, i.domestic_currency);
                deps
            }
            InstrumentJson::FxVarianceSwap(i) => {
                let mut deps = Self::new();
                deps.add_curves(
                    InstrumentCurves::builder()
                        .discount(i.domestic_discount_curve_id.clone())
                        .discount(i.foreign_discount_curve_id.clone())
                        .build(),
                );
                if let Some(spot_id) = i.spot_id.as_deref() {
                    deps.add_spot_id(spot_id);
                }
                deps.add_vol_surface_id(i.vol_surface_id.as_str());
                deps.add_fx_pair(i.base_currency, i.quote_currency);
                deps
            }
            _ => {
                let Ok(boxed) = instrument.clone().into_boxed() else {
                    return Self::new();
                };
                Self::from_instrument(boxed.as_ref())
            }
        }
    }
}

fn push_unique_curve(target: &mut CurveIdVec, id: CurveId) {
    if target.contains(&id) {
        return;
    }
    target.push(id);
}

fn push_unique_string(target: &mut Vec<String>, value: String) {
    if target.contains(&value) {
        return;
    }
    target.push(value);
}

fn push_unique_fx_pair(target: &mut Vec<FxPair>, pair: FxPair) {
    if target.contains(&pair) {
        return;
    }
    target.push(pair);
}
