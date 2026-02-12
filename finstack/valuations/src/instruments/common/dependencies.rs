//! Unified market data dependency representation for instruments.

use crate::instruments::common_impl::traits::{
    CurveDependencies, CurveIdVec, EquityDependencies, EquityInstrumentDeps, Instrument,
    InstrumentCurves,
};
use finstack_core::currency::Currency;
use finstack_core::types::CurveId;

use crate::instruments::json_loader::InstrumentJson;

/// FX pair identifier using base/quote currency ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Default)]
pub struct MarketDependencies {
    /// Curve dependencies grouped by type.
    pub curves: InstrumentCurves,
    /// Spot identifiers (equity, FX spot IDs, commodity spot IDs).
    pub spot_ids: Vec<String>,
    /// Volatility surface identifiers.
    pub vol_surface_ids: Vec<String>,
    /// FX pairs required for pricing (spot matrices).
    pub fx_pairs: Vec<FxPair>,
}

impl MarketDependencies {
    /// Create an empty dependency set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the curve dependencies view for this market dependency set.
    pub fn curve_dependencies(&self) -> &InstrumentCurves {
        &self.curves
    }

    /// Return the primary equity dependencies view for this market dependency set.
    ///
    /// This returns the first spot/vol IDs when multiple are present (e.g., baskets).
    pub fn equity_dependencies(&self) -> EquityInstrumentDeps {
        EquityInstrumentDeps {
            spot_id: self.spot_ids.first().cloned(),
            vol_surface_id: self.vol_surface_ids.first().cloned(),
        }
    }

    /// Build dependencies from an instrument implementing [`CurveDependencies`].
    pub fn from_curve_dependencies<T: CurveDependencies>(
        instrument: &T,
    ) -> finstack_core::Result<Self> {
        let mut deps = Self::new();
        deps.add_curves(instrument.curve_dependencies()?);
        Ok(deps)
    }

    /// Build dependencies from an instrument implementing [`EquityDependencies`].
    pub fn from_equity_dependencies<T: EquityDependencies>(
        instrument: &T,
    ) -> finstack_core::Result<Self> {
        let mut deps = Self::new();
        deps.add_equity_dependencies(instrument.equity_dependencies()?);
        Ok(deps)
    }

    /// Build dependencies from an instrument implementing both curve and equity traits.
    pub fn from_curves_and_equity<T: CurveDependencies + EquityDependencies>(
        instrument: &T,
    ) -> finstack_core::Result<Self> {
        let mut deps = Self::new();
        deps.add_curves(instrument.curve_dependencies()?);
        deps.add_equity_dependencies(instrument.equity_dependencies()?);
        Ok(deps)
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
    pub fn merge(&mut self, other: MarketDependencies) {
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
    pub fn from_instrument_json(instrument: &InstrumentJson) -> finstack_core::Result<Self> {
        match instrument {
            // Fixed Income
            InstrumentJson::Bond(i) => Self::from_curve_dependencies(i),
            InstrumentJson::ConvertibleBond(i) => Self::from_curve_dependencies(i),
            InstrumentJson::InflationLinkedBond(i) => Self::from_curve_dependencies(i),
            InstrumentJson::TermLoan(i) => Self::from_curve_dependencies(i),
            InstrumentJson::RevolvingCredit(i) => Self::from_curve_dependencies(i),
            InstrumentJson::BondFuture(i) => Self::from_curve_dependencies(i),
            InstrumentJson::AgencyMbsPassthrough(i) => Self::from_curve_dependencies(i),
            InstrumentJson::AgencyTba(i) => Self::from_curve_dependencies(i),
            InstrumentJson::AgencyCmo(i) => Self::from_curve_dependencies(i),
            InstrumentJson::DollarRoll(i) => Self::from_curve_dependencies(i),

            // Rates
            InstrumentJson::InterestRateSwap(i) => Self::from_curve_dependencies(i),
            InstrumentJson::BasisSwap(i) => Self::from_curve_dependencies(i),
            InstrumentJson::XccySwap(i) => Self::from_curve_dependencies(i),
            InstrumentJson::InflationSwap(i) => Self::from_curve_dependencies(i),
            InstrumentJson::YoYInflationSwap(i) => Self::from_curve_dependencies(i),
            InstrumentJson::InflationCapFloor(i) => Self::from_curve_dependencies(i),
            InstrumentJson::ForwardRateAgreement(i) => Self::from_curve_dependencies(i),
            InstrumentJson::Swaption(i) => Self::from_curve_dependencies(i),
            InstrumentJson::InterestRateFuture(i) => Self::from_curve_dependencies(i),
            InstrumentJson::InterestRateOption(i) => Self::from_curve_dependencies(i),
            InstrumentJson::CmsOption(i) => Self::from_curve_dependencies(i),
            InstrumentJson::Deposit(i) => Self::from_curve_dependencies(i),
            InstrumentJson::Repo(i) => Self::from_curve_dependencies(i),

            // Credit
            InstrumentJson::CreditDefaultSwap(i) => Self::from_curve_dependencies(i),
            InstrumentJson::CDSIndex(i) => Self::from_curve_dependencies(i),
            InstrumentJson::CDSTranche(i) => Self::from_curve_dependencies(i),
            InstrumentJson::CDSOption(i) => Self::from_curve_dependencies(i),

            // Equity
            InstrumentJson::Equity(i) => Self::from_curve_dependencies(i),
            InstrumentJson::EquityOption(i) => Self::from_curves_and_equity(i),
            InstrumentJson::AsianOption(i) => Self::from_curves_and_equity(i),
            InstrumentJson::BarrierOption(i) => Self::from_curves_and_equity(i),
            InstrumentJson::LookbackOption(i) => Self::from_curves_and_equity(i),
            InstrumentJson::VarianceSwap(i) => Self::from_curve_dependencies(i),
            InstrumentJson::EquityIndexFuture(i) => Self::from_curves_and_equity(i),
            InstrumentJson::VolatilityIndexFuture(i) => Self::from_curve_dependencies(i),
            InstrumentJson::VolatilityIndexOption(i) => Self::from_curve_dependencies(i),

            // FX
            InstrumentJson::FxSpot(i) => {
                let mut deps = Self::from_curve_dependencies(i)?;
                deps.add_fx_pair(i.base, i.quote);
                Ok(deps)
            }
            InstrumentJson::FxSwap(i) => {
                let mut deps = Self::from_curve_dependencies(i)?;
                deps.add_fx_pair(i.base_currency, i.quote_currency);
                Ok(deps)
            }
            InstrumentJson::FxForward(i) => {
                let mut deps = Self::from_curve_dependencies(i)?;
                deps.add_fx_pair(i.base_currency, i.quote_currency);
                Ok(deps)
            }
            InstrumentJson::Ndf(i) => {
                let mut deps = Self::from_curve_dependencies(i)?;
                deps.add_fx_pair(i.base_currency, i.settlement_currency);
                Ok(deps)
            }
            InstrumentJson::FxOption(i) => {
                let mut deps = Self::from_curve_dependencies(i)?;
                deps.add_vol_surface_id(i.vol_surface_id.as_str());
                deps.add_fx_pair(i.base_currency, i.quote_currency);
                Ok(deps)
            }
            InstrumentJson::FxBarrierOption(i) => {
                let mut deps = Self::from_curve_dependencies(i)?;
                deps.add_spot_id(i.fx_spot_id.as_str());
                deps.add_vol_surface_id(i.fx_vol_id.as_str());
                deps.add_fx_pair(i.foreign_currency, i.domestic_currency);
                Ok(deps)
            }
            InstrumentJson::FxVarianceSwap(i) => {
                let mut deps = Self::from_curve_dependencies(i)?;
                if let Some(spot_id) = i.spot_id.as_deref() {
                    deps.add_spot_id(spot_id);
                }
                deps.add_vol_surface_id(i.vol_surface_id.as_str());
                deps.add_fx_pair(i.base_currency, i.quote_currency);
                Ok(deps)
            }
            InstrumentJson::QuantoOption(i) => {
                let mut deps = Self::from_curve_dependencies(i)?;
                deps.add_spot_id(i.spot_id.as_str());
                deps.add_vol_surface_id(i.vol_surface_id.as_str());
                Ok(deps)
            }

            // Commodity
            InstrumentJson::CommodityOption(i) => Self::from_curves_and_equity(i),
            InstrumentJson::CommodityForward(i) => Self::from_curves_and_equity(i),
            InstrumentJson::CommoditySwap(i) => Self::from_curve_dependencies(i),

            // Exotic Options
            InstrumentJson::Autocallable(i) => Self::from_curves_and_equity(i),
            InstrumentJson::CliquetOption(i) => Self::from_curves_and_equity(i),
            InstrumentJson::RangeAccrual(i) => Self::from_curves_and_equity(i),

            // Total Return Swaps
            InstrumentJson::TrsEquity(i) => Self::from_curve_dependencies(i),
            InstrumentJson::TrsFixedIncomeIndex(i) => Self::from_curve_dependencies(i),

            // Structured Credit
            InstrumentJson::StructuredCredit(i) => Self::from_curve_dependencies(i.as_ref()),

            // Other
            InstrumentJson::Basket(i) => Self::from_curve_dependencies(i),
            InstrumentJson::PrivateMarketsFund(i) => i.market_dependencies(),
            InstrumentJson::RealEstateAsset(i) => Self::from_curve_dependencies(i),
            InstrumentJson::LeveredRealEstateEquity(i) => Self::from_curve_dependencies(i.as_ref()),
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
