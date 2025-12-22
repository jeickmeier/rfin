use std::sync::Arc;

use crate::collections::HashMap;
use crate::types::CurveId;

use super::curve_storage::CurveStorage;
use super::MarketContext;

use crate::market_data::{
    dividends::DividendSchedule,
    scalars::{inflation_index::InflationIndex, MarketScalar, ScalarTimeSeries},
    surfaces::vol_surface::VolSurface,
};
use crate::money::fx::{providers::SimpleFxProvider, FxMatrix, FxMatrixState, FxProvider};

// -----------------------------------------------------------------------------
// Serde: CurveState and (De)Serialize impls
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Serializable state representation for any curve type.
///
/// Produced when the crate is compiled with the `serde` feature to persist
/// market data snapshots.
pub enum CurveState {
    /// Discount curve state
    Discount(crate::market_data::term_structures::discount_curve::DiscountCurve),
    /// Forward curve state
    Forward(crate::market_data::term_structures::forward_curve::ForwardCurve),
    /// Hazard curve state
    Hazard(crate::market_data::term_structures::hazard_curve::HazardCurve),
    /// Inflation curve state
    Inflation(crate::market_data::term_structures::inflation::InflationCurve),
    /// Base correlation curve state
    BaseCorrelation(crate::market_data::term_structures::base_correlation::BaseCorrelationCurve),
    /// Volatility index curve state (VIX, VXN, VSTOXX)
    VolIndex(crate::market_data::term_structures::vol_index_curve::VolatilityIndexCurve),
}

fn curve_state_id(s: &CurveState) -> &CurveId {
    match s {
        CurveState::Discount(c) => c.id(),
        CurveState::Forward(c) => c.id(),
        CurveState::Hazard(c) => c.id(),
        CurveState::Inflation(c) => c.id(),
        CurveState::BaseCorrelation(c) => c.id(),
        CurveState::VolIndex(c) => c.id(),
    }
}

impl CurveStorage {
    /// Convert to serializable state.
    ///
    /// This conversion is infallible - all curve types can be converted to their state representation.
    pub fn to_state(&self) -> CurveState {
        match self {
            Self::Discount(curve) => CurveState::Discount((**curve).clone()),
            Self::Forward(curve) => CurveState::Forward((**curve).clone()),
            Self::Hazard(curve) => CurveState::Hazard((**curve).clone()),
            Self::Inflation(curve) => CurveState::Inflation((**curve).clone()),
            Self::BaseCorrelation(curve) => CurveState::BaseCorrelation((**curve).clone()),
            Self::VolIndex(curve) => CurveState::VolIndex((**curve).clone()),
        }
    }

    /// Reconstruct from serializable state.
    ///
    /// This conversion is infallible - all state variants map directly to storage variants.
    pub fn from_state(state: CurveState) -> Self {
        match state {
            CurveState::Discount(c) => Self::Discount(Arc::new(c)),
            CurveState::Forward(c) => Self::Forward(Arc::new(c)),
            CurveState::Hazard(c) => Self::Hazard(Arc::new(c)),
            CurveState::Inflation(c) => Self::Inflation(Arc::new(c)),
            CurveState::BaseCorrelation(c) => Self::BaseCorrelation(Arc::new(c)),
            CurveState::VolIndex(c) => Self::VolIndex(Arc::new(c)),
        }
    }
}

impl serde::Serialize for CurveStorage {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_state().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for CurveStorage {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let state = CurveState::deserialize(deserializer)?;
        Ok(Self::from_state(state))
    }
}

// -----------------------------------------------------------------------------
// Credit Index State (for serialization of CreditIndexData)
// -----------------------------------------------------------------------------

/// Serializable state for credit index data.
///
/// Instead of serializing `Arc<Curve>` directly, we store curve IDs that
/// reference curves present in the `MarketContextState`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreditIndexState {
    /// Unique identifier for this credit index
    pub id: String,
    /// Number of constituents
    pub num_constituents: u16,
    /// Recovery rate
    pub recovery_rate: f64,
    /// ID of the index hazard curve (must exist in context curves)
    pub index_credit_curve_id: String,
    /// ID of the base correlation curve (must exist in context curves)
    pub base_correlation_curve_id: String,
    /// Optional map of issuer ID → hazard curve ID
    pub issuer_credit_curve_ids: Option<std::collections::BTreeMap<String, String>>,
    /// Optional map of issuer ID → recovery rate
    pub issuer_recovery_rates: Option<std::collections::BTreeMap<String, f64>>,
    /// Optional map of issuer ID → weight
    pub issuer_weights: Option<std::collections::BTreeMap<String, f64>>,
}

// -----------------------------------------------------------------------------
// Market Context State (complete snapshot)
// -----------------------------------------------------------------------------

/// Complete serializable state of a MarketContext.
///
/// Provides a stable, versioned snapshot of all market data that can be
/// persisted to JSON and reconstructed deterministically.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketContextState {
    /// All curves (discount, forward, hazard, inflation, base correlation)
    pub curves: Vec<CurveState>,
    /// FX matrix state (optional)
    pub fx: Option<FxMatrixState>,
    /// Volatility surfaces
    pub surfaces: Vec<VolSurface>,
    /// Market scalars and prices
    pub prices: std::collections::BTreeMap<String, MarketScalar>,
    /// Generic time series
    pub series: Vec<ScalarTimeSeries>,
    /// Inflation indices
    pub inflation_indices: Vec<InflationIndex>,
    /// Dividend schedules
    pub dividends: Vec<DividendSchedule>,
    /// Credit index aggregates (references curves by ID)
    pub credit_indices: Vec<CreditIndexState>,
    /// Collateral CSA mappings
    pub collateral: std::collections::BTreeMap<String, String>,
}

impl From<&MarketContext> for MarketContextState {
    fn from(ctx: &MarketContext) -> Self {
        // Convert all curves (sort deterministically by id for stable snapshots).
        let mut curves: Vec<CurveState> = ctx
            .curves
            .values()
            .map(|storage| storage.to_state())
            .collect();
        curves.sort_by(|a, b| curve_state_id(a).cmp(curve_state_id(b)));

        // Convert FX (if present)
        let fx = ctx.fx.as_ref().map(|fx| fx.get_serializable_state());

        // Convert all surfaces (sort deterministically by key id).
        let mut surfaces_pairs: Vec<(CurveId, VolSurface)> = ctx
            .surfaces
            .iter()
            .map(|(id, surf)| (id.clone(), (**surf).clone()))
            .collect();
        surfaces_pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let surfaces: Vec<_> = surfaces_pairs.into_iter().map(|(_, surf)| surf).collect();

        // Convert prices (CurveId → String)
        let prices: std::collections::BTreeMap<String, _> = ctx
            .prices
            .iter()
            .map(|(id, scalar)| (id.to_string(), scalar.clone()))
            .collect();

        // Convert series (sort deterministically by key id).
        let mut series_pairs: Vec<(CurveId, ScalarTimeSeries)> = ctx
            .series
            .iter()
            .map(|(id, series)| (id.clone(), series.clone()))
            .collect();
        series_pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let series: Vec<_> = series_pairs.into_iter().map(|(_, series)| series).collect();

        // Convert inflation indices (sort deterministically by key id).
        let mut inflation_pairs: Vec<(CurveId, InflationIndex)> = ctx
            .inflation_indices
            .iter()
            .map(|(id, idx)| (id.clone(), (**idx).clone()))
            .collect();
        inflation_pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let inflation_indices: Vec<_> = inflation_pairs.into_iter().map(|(_, idx)| idx).collect();

        // Convert credit indices (extract IDs from Arc references; sort deterministically by id).
        let mut credit_pairs: Vec<(CurveId, CreditIndexState)> = ctx
            .credit_indices
            .iter()
            .map(|(id, data)| {
                let issuer_ids: Option<std::collections::BTreeMap<String, String>> =
                    data.issuer_credit_curves.as_ref().map(|map| {
                        map.iter()
                            .map(|(issuer, curve)| (issuer.clone(), curve.id().to_string()))
                            .collect()
                    });
                let issuer_recovery_rates: Option<std::collections::BTreeMap<String, f64>> = data
                    .issuer_recovery_rates
                    .as_ref()
                    .map(|m| m.iter().map(|(k, v)| (k.clone(), *v)).collect());
                let issuer_weights: Option<std::collections::BTreeMap<String, f64>> = data
                    .issuer_weights
                    .as_ref()
                    .map(|m| m.iter().map(|(k, v)| (k.clone(), *v)).collect());

                (
                    id.clone(),
                    CreditIndexState {
                        id: id.to_string(),
                        num_constituents: data.num_constituents,
                        recovery_rate: data.recovery_rate,
                        index_credit_curve_id: data.index_credit_curve.id().to_string(),
                        base_correlation_curve_id: data.base_correlation_curve.id().to_string(),
                        issuer_credit_curve_ids: issuer_ids,
                        issuer_recovery_rates,
                        issuer_weights,
                    },
                )
            })
            .collect();
        credit_pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let credit_indices: Vec<CreditIndexState> =
            credit_pairs.into_iter().map(|(_, s)| s).collect();

        // Convert dividends (sort deterministically by id).
        let mut dividend_pairs: Vec<(CurveId, DividendSchedule)> = ctx
            .dividends
            .iter()
            .map(|(id, divs)| (id.clone(), (**divs).clone()))
            .collect();
        dividend_pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let dividends: Vec<_> = dividend_pairs.into_iter().map(|(_, d)| d).collect();

        // Convert collateral mappings
        let collateral: std::collections::BTreeMap<String, String> = ctx
            .collateral
            .iter()
            .map(|(csa, curve_id)| (csa.clone(), curve_id.to_string()))
            .collect();

        MarketContextState {
            curves,
            fx,
            surfaces,
            prices,
            series,
            inflation_indices,
            dividends,
            credit_indices,
            collateral,
        }
    }
}

impl TryFrom<MarketContextState> for MarketContext {
    type Error = crate::Error;

    fn try_from(state: MarketContextState) -> crate::Result<Self> {
        let mut ctx = MarketContext::new();

        // Reconstruct all curves
        for curve_state in state.curves {
            let storage = CurveStorage::from_state(curve_state);
            ctx.curves.insert(storage.id().clone(), storage);
        }

        // Reconstruct all surfaces
        for surface in state.surfaces {
            ctx.surfaces.insert(surface.id().clone(), Arc::new(surface));
        }

        // Reconstruct FX matrix if present (using a simple provider to host cached quotes)
        if let Some(fx_state) = state.fx {
            let provider: Arc<dyn FxProvider> = Arc::new(SimpleFxProvider::new());
            let matrix = FxMatrix::with_config(Arc::clone(&provider), fx_state.config);
            matrix.load_from_state(&fx_state);
            ctx.fx = Some(Arc::new(matrix));
        }

        // Reconstruct prices
        for (id_str, scalar) in state.prices {
            ctx.prices.insert(CurveId::from(id_str), scalar);
        }

        // Reconstruct series
        for series in state.series {
            ctx.series.insert(series.id().clone(), series);
        }

        // Reconstruct inflation indices
        for idx in state.inflation_indices {
            let id = CurveId::from(idx.id.clone());
            ctx.inflation_indices.insert(id, Arc::new(idx));
        }

        // Reconstruct dividends
        for schedule in state.dividends {
            let id = schedule.id.clone();
            ctx.dividends.insert(id, Arc::new(schedule));
        }

        // Reconstruct credit indices (resolve curve references)
        for credit_state in state.credit_indices {
            // Resolve hazard curve
            let index_curve = ctx.get_hazard(&credit_state.index_credit_curve_id)?;

            // Resolve base correlation curve
            let base_corr = ctx.get_base_correlation(&credit_state.base_correlation_curve_id)?;

            // Resolve issuer curves if present
            let issuer_curves = if let Some(issuer_ids) = credit_state.issuer_credit_curve_ids {
                let mut map = HashMap::default();
                for (issuer, curve_id) in issuer_ids {
                    let curve = ctx.get_hazard(&curve_id)?;
                    map.insert(issuer, curve);
                }
                Some(map)
            } else {
                None
            };

            let data = crate::market_data::term_structures::credit_index::CreditIndexData {
                num_constituents: credit_state.num_constituents,
                recovery_rate: credit_state.recovery_rate,
                index_credit_curve: index_curve,
                base_correlation_curve: base_corr,
                issuer_credit_curves: issuer_curves,
                issuer_recovery_rates: credit_state
                    .issuer_recovery_rates
                    .map(|m| m.into_iter().collect::<HashMap<_, _>>()),
                issuer_weights: credit_state
                    .issuer_weights
                    .map(|m| m.into_iter().collect::<HashMap<_, _>>()),
            };

            ctx.credit_indices
                .insert(CurveId::from(credit_state.id), Arc::new(data));
        }

        // Reconstruct collateral mappings
        for (csa, curve_id_str) in state.collateral {
            ctx.collateral.insert(csa, CurveId::from(curve_id_str));
        }

        Ok(ctx)
    }
}

// -----------------------------------------------------------------------------
// Optional Serialize/Deserialize for MarketContext (via State)
// -----------------------------------------------------------------------------

impl serde::Serialize for MarketContext {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.market_history.is_some() {
            return Err(serde::ser::Error::custom(
                "market_history is runtime-only and cannot be serialized",
            ));
        }
        let state: MarketContextState = self.into();
        state.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for MarketContext {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let state = MarketContextState::deserialize(deserializer)?;
        Self::try_from(state).map_err(serde::de::Error::custom)
    }
}
