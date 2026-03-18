//! Cross-factor gamma calculators.
//!
//! Provides reusable `MetricCalculator` implementations for mixed second
//! derivatives between pairs of risk factors. The calculator resolves the
//! concrete market dependencies at runtime from `MetricContext`, then applies
//! a four-corner central mixed difference.

use crate::instruments::common_impl::dependencies::FxPair;
use crate::instruments::common_impl::traits::Instrument;
use crate::metrics::core::finite_difference::{
    bump_scalar_price, bump_surface_vol_absolute, central_mixed,
};
use crate::metrics::sensitivities::config as sens_config;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::Date;
use finstack_core::market_data::bumps::{BumpSpec, MarketBump};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::fx::FxQuery;
use finstack_core::types::CurveId;
use finstack_core::Result;
use std::sync::Arc;

const NO_DEPS: &[MetricId] = &[];
const SPOT_VOL_DEPS: &[MetricId] = &[MetricId::Vanna];

/// Identifies which two risk factors are crossed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrossFactorPair {
    /// Parallel rates against parallel credit spreads.
    RatesCredit,
    /// Parallel rates against parallel volatility shifts.
    RatesVol,
    /// Spot/underlying against parallel volatility shifts.
    SpotVol,
    /// Spot/underlying against parallel credit spreads.
    SpotCredit,
    /// FX spot against parallel volatility shifts.
    FxVol,
    /// FX spot against parallel interest-rate shifts.
    FxRates,
}

impl CrossFactorPair {
    /// All currently supported cross-factor pairs.
    pub const ALL: &'static [Self] = &[
        Self::RatesCredit,
        Self::RatesVol,
        Self::SpotVol,
        Self::SpotCredit,
        Self::FxVol,
        Self::FxRates,
    ];

    /// Returns the standard `MetricId` associated with this pair.
    pub fn metric_id(self) -> MetricId {
        match self {
            Self::RatesCredit => MetricId::CrossGammaRatesCredit,
            Self::RatesVol => MetricId::CrossGammaRatesVol,
            Self::SpotVol => MetricId::CrossGammaSpotVol,
            Self::SpotCredit => MetricId::CrossGammaSpotCredit,
            Self::FxVol => MetricId::CrossGammaFxVol,
            Self::FxRates => MetricId::CrossGammaFxRates,
        }
    }

    /// Human-readable label for explanations and attribution detail keys.
    pub fn label(self) -> &'static str {
        match self {
            Self::RatesCredit => "Rates×Credit",
            Self::RatesVol => "Rates×Vol",
            Self::SpotVol => "Spot×Vol",
            Self::SpotCredit => "Spot×Credit",
            Self::FxVol => "FX×Vol",
            Self::FxRates => "FX×Rates",
        }
    }
}

pub(crate) trait FactorBumper: Send + Sync {
    fn bump_market(
        &self,
        market: &MarketContext,
        as_of: Date,
        direction: f64,
    ) -> Result<MarketContext>;

    fn bump_size(&self) -> f64;

    fn is_applicable(&self, market: &MarketContext, as_of: Date) -> bool;
}

pub(crate) trait BumperFactory: Send + Sync {
    fn create(&self, context: &MetricContext) -> Result<Option<Box<dyn FactorBumper>>>;
}

#[derive(Debug, Clone)]
struct RatesParallelBumper {
    curve_ids: Vec<CurveId>,
    bump_bp: f64,
}

impl FactorBumper for RatesParallelBumper {
    fn bump_market(
        &self,
        market: &MarketContext,
        _as_of: Date,
        direction: f64,
    ) -> Result<MarketContext> {
        let bumps: Vec<MarketBump> = self
            .curve_ids
            .iter()
            .map(|id| MarketBump::Curve {
                id: id.clone(),
                spec: BumpSpec::parallel_bp(self.bump_bp * direction),
            })
            .collect();
        market.bump(bumps)
    }

    fn bump_size(&self) -> f64 {
        self.bump_bp
    }

    fn is_applicable(&self, market: &MarketContext, _as_of: Date) -> bool {
        self.curve_ids.iter().any(|curve_id| {
            market.get_discount(curve_id.as_str()).is_ok()
                || market.get_forward(curve_id.as_str()).is_ok()
        })
    }
}

#[derive(Debug, Clone)]
struct CreditParallelBumper {
    curve_ids: Vec<CurveId>,
    bump_bp: f64,
}

impl FactorBumper for CreditParallelBumper {
    fn bump_market(
        &self,
        market: &MarketContext,
        _as_of: Date,
        direction: f64,
    ) -> Result<MarketContext> {
        let bumps: Vec<MarketBump> = self
            .curve_ids
            .iter()
            .map(|id| MarketBump::Curve {
                id: id.clone(),
                spec: BumpSpec::parallel_bp(self.bump_bp * direction),
            })
            .collect();
        market.bump(bumps)
    }

    fn bump_size(&self) -> f64 {
        self.bump_bp
    }

    fn is_applicable(&self, market: &MarketContext, _as_of: Date) -> bool {
        self.curve_ids
            .iter()
            .any(|curve_id| market.get_hazard(curve_id.as_str()).is_ok())
    }
}

#[derive(Debug, Clone)]
struct VolParallelBumper {
    surface_id: CurveId,
    bump_abs: f64,
}

impl FactorBumper for VolParallelBumper {
    fn bump_market(
        &self,
        market: &MarketContext,
        _as_of: Date,
        direction: f64,
    ) -> Result<MarketContext> {
        bump_surface_vol_absolute(market, self.surface_id.as_str(), self.bump_abs * direction)
    }

    fn bump_size(&self) -> f64 {
        self.bump_abs * 100.0
    }

    fn is_applicable(&self, market: &MarketContext, _as_of: Date) -> bool {
        market.get_surface(self.surface_id.as_str()).is_ok()
    }
}

#[derive(Debug, Clone)]
struct SpotBumper {
    price_id: String,
    bump_pct: f64,
}

impl FactorBumper for SpotBumper {
    fn bump_market(
        &self,
        market: &MarketContext,
        _as_of: Date,
        direction: f64,
    ) -> Result<MarketContext> {
        bump_scalar_price(market, &self.price_id, self.bump_pct * direction)
    }

    fn bump_size(&self) -> f64 {
        self.bump_pct * 100.0
    }

    fn is_applicable(&self, market: &MarketContext, _as_of: Date) -> bool {
        market.get_price(&self.price_id).is_ok()
    }
}

#[derive(Debug, Clone)]
struct FxBumper {
    pairs: Vec<FxPair>,
    bump_pct: f64,
}

impl FactorBumper for FxBumper {
    fn bump_market(
        &self,
        market: &MarketContext,
        as_of: Date,
        direction: f64,
    ) -> Result<MarketContext> {
        let bumps: Vec<MarketBump> = self
            .pairs
            .iter()
            .map(|pair| MarketBump::FxPct {
                base: pair.base,
                quote: pair.quote,
                pct: self.bump_pct * direction * 100.0,
                as_of,
            })
            .collect();
        market.bump(bumps)
    }

    fn bump_size(&self) -> f64 {
        self.bump_pct * 100.0
    }

    fn is_applicable(&self, market: &MarketContext, as_of: Date) -> bool {
        self.pairs.iter().any(|pair| {
            market
                .fx()
                .and_then(|fx| fx.rate(FxQuery::new(pair.base, pair.quote, as_of)).ok())
                .is_some()
        })
    }
}

/// Factory for rate bumpers based on runtime instrument dependencies.
#[derive(Debug, Default)]
pub(crate) struct RatesBumperFactory;

impl BumperFactory for RatesBumperFactory {
    fn create(&self, context: &MetricContext) -> Result<Option<Box<dyn FactorBumper>>> {
        let deps = context.instrument.market_dependencies()?;
        let mut curve_ids = Vec::new();
        for curve_id in deps
            .curves
            .discount_curves
            .iter()
            .chain(&deps.curves.forward_curves)
        {
            if !curve_ids.contains(curve_id) {
                curve_ids.push(curve_id.clone());
            }
        }
        if curve_ids.is_empty() {
            return Ok(None);
        }

        let defaults = sens_config::from_context_or_default(
            context.config(),
            context.pricing_overrides.as_ref(),
        )?;
        Ok(Some(Box::new(RatesParallelBumper {
            curve_ids,
            bump_bp: defaults.rate_bump_bp,
        })))
    }
}

/// Factory for credit bumpers based on runtime instrument dependencies.
#[derive(Debug, Default)]
pub(crate) struct CreditBumperFactory;

impl BumperFactory for CreditBumperFactory {
    fn create(&self, context: &MetricContext) -> Result<Option<Box<dyn FactorBumper>>> {
        let deps = context.instrument.market_dependencies()?;
        if deps.curves.credit_curves.is_empty() {
            return Ok(None);
        }

        let defaults = sens_config::from_context_or_default(
            context.config(),
            context.pricing_overrides.as_ref(),
        )?;
        Ok(Some(Box::new(CreditParallelBumper {
            curve_ids: deps.curves.credit_curves.to_vec(),
            bump_bp: defaults.credit_spread_bump_bp,
        })))
    }
}

/// Factory for volatility bumpers based on runtime instrument dependencies.
#[derive(Debug, Default)]
pub(crate) struct VolBumperFactory;

impl BumperFactory for VolBumperFactory {
    fn create(&self, context: &MetricContext) -> Result<Option<Box<dyn FactorBumper>>> {
        let deps = context.instrument.market_dependencies()?;
        let Some(surface_id) = deps
            .vol_surface_ids
            .iter()
            .find(|surface_id| context.curves.get_surface(surface_id.as_str()).is_ok())
            .or_else(|| deps.vol_surface_ids.first())
        else {
            return Ok(None);
        };

        let defaults = sens_config::from_context_or_default(
            context.config(),
            context.pricing_overrides.as_ref(),
        )?;
        Ok(Some(Box::new(VolParallelBumper {
            surface_id: CurveId::from(surface_id.as_str()),
            bump_abs: defaults.vol_bump_pct,
        })))
    }
}

/// Factory for spot bumpers based on runtime instrument dependencies.
#[derive(Debug, Default)]
pub(crate) struct SpotBumperFactory;

impl BumperFactory for SpotBumperFactory {
    fn create(&self, context: &MetricContext) -> Result<Option<Box<dyn FactorBumper>>> {
        let deps = context.instrument.market_dependencies()?;
        let Some(price_id) = deps.spot_ids.first() else {
            return Ok(None);
        };

        let defaults = sens_config::from_context_or_default(
            context.config(),
            context.pricing_overrides.as_ref(),
        )?;
        Ok(Some(Box::new(SpotBumper {
            price_id: price_id.clone(),
            bump_pct: defaults.spot_bump_pct,
        })))
    }
}

/// Factory for FX bumpers based on runtime instrument dependencies.
#[derive(Debug, Default)]
pub(crate) struct FxBumperFactory;

impl BumperFactory for FxBumperFactory {
    fn create(&self, context: &MetricContext) -> Result<Option<Box<dyn FactorBumper>>> {
        let deps = context.instrument.market_dependencies()?;
        if deps.fx_pairs.is_empty() {
            return Ok(None);
        }

        let defaults = sens_config::from_context_or_default(
            context.config(),
            context.pricing_overrides.as_ref(),
        )?;
        Ok(Some(Box::new(FxBumper {
            pairs: deps.fx_pairs,
            bump_pct: defaults.spot_bump_pct,
        })))
    }
}

/// Generic cross-factor calculator using runtime bumper factories.
pub struct CrossFactorCalculator {
    pair: CrossFactorPair,
    factory_a: Arc<dyn BumperFactory>,
    factory_b: Arc<dyn BumperFactory>,
}

impl CrossFactorCalculator {
    /// Creates a reusable cross-factor calculator for the given pair.
    pub(crate) fn new(
        pair: CrossFactorPair,
        factory_a: Arc<dyn BumperFactory>,
        factory_b: Arc<dyn BumperFactory>,
    ) -> Self {
        Self {
            pair,
            factory_a,
            factory_b,
        }
    }

    /// Returns the pair this calculator represents.
    pub fn pair(&self) -> CrossFactorPair {
        self.pair
    }
}

impl MetricCalculator for CrossFactorCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        if self.pair == CrossFactorPair::SpotVol {
            if let Some(vanna) = context.computed.get(&MetricId::Vanna) {
                return Ok(*vanna);
            }
        }

        let bumper_a = match self.factory_a.create(context)? {
            Some(bumper) => bumper,
            None => return Ok(0.0),
        };
        let bumper_b = match self.factory_b.create(context)? {
            Some(bumper) => bumper,
            None => return Ok(0.0),
        };

        let market = context.curves.as_ref();
        let as_of = context.as_of;
        if !bumper_a.is_applicable(market, as_of) || !bumper_b.is_applicable(market, as_of) {
            return Ok(0.0);
        }

        let h_abs = bumper_a.bump_size();
        let k_abs = bumper_b.bump_size();
        let instrument = Arc::clone(&context.instrument);

        central_mixed(
            || reprice_corner(&instrument, market, as_of, &*bumper_a, &*bumper_b, 1.0, 1.0),
            || {
                reprice_corner(
                    &instrument,
                    market,
                    as_of,
                    &*bumper_a,
                    &*bumper_b,
                    1.0,
                    -1.0,
                )
            },
            || {
                reprice_corner(
                    &instrument,
                    market,
                    as_of,
                    &*bumper_a,
                    &*bumper_b,
                    -1.0,
                    1.0,
                )
            },
            || {
                reprice_corner(
                    &instrument,
                    market,
                    as_of,
                    &*bumper_a,
                    &*bumper_b,
                    -1.0,
                    -1.0,
                )
            },
            h_abs,
            k_abs,
        )
    }

    fn dependencies(&self) -> &[MetricId] {
        match self.pair {
            CrossFactorPair::SpotVol => SPOT_VOL_DEPS,
            _ => NO_DEPS,
        }
    }
}

fn reprice_corner(
    instrument: &Arc<dyn Instrument>,
    market: &MarketContext,
    as_of: Date,
    bumper_a: &dyn FactorBumper,
    bumper_b: &dyn FactorBumper,
    direction_a: f64,
    direction_b: f64,
) -> Result<f64> {
    let bumped_a = bumper_a.bump_market(market, as_of, direction_a)?;
    let bumped_ab = bumper_b.bump_market(&bumped_a, as_of, direction_b)?;
    let bumped_market = Arc::new(bumped_ab);
    Ok(instrument.value(&bumped_market, as_of)?.amount())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use crate::metrics::MetricId;

    use super::CrossFactorPair;

    #[test]
    fn cross_factor_pair_metric_id_roundtrip() {
        let expected = [
            (
                CrossFactorPair::RatesCredit,
                MetricId::CrossGammaRatesCredit,
            ),
            (CrossFactorPair::RatesVol, MetricId::CrossGammaRatesVol),
            (CrossFactorPair::SpotVol, MetricId::CrossGammaSpotVol),
            (CrossFactorPair::SpotCredit, MetricId::CrossGammaSpotCredit),
            (CrossFactorPair::FxVol, MetricId::CrossGammaFxVol),
            (CrossFactorPair::FxRates, MetricId::CrossGammaFxRates),
        ];

        for (pair, metric_id) in expected {
            assert_eq!(pair.metric_id(), metric_id);
            assert!(!pair.metric_id().is_custom());
        }
    }

    #[test]
    fn cross_factor_pair_labels_are_nonempty() {
        for pair in CrossFactorPair::ALL {
            assert!(!pair.label().is_empty());
        }
    }
}
