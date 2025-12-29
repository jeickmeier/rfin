use super::pool::AssetPool as Pool;
use finstack_core::dates::{Date, DayCount};

/// Structure of Arrays (SoA) layout for pool assets to improve cache locality
/// and enable vectorization during pricing.
#[derive(Debug, Clone)]
pub struct PoolState {
    /// Asset identifiers
    pub ids: Vec<String>,
    /// Current outstanding balances
    pub balances: Vec<f64>,
    /// Interest rates (decimal)
    pub rates: Vec<f64>,
    /// Spread over index (basis points)
    pub spread_bps: Vec<Option<f64>>,
    /// Index identifiers for floating rate assets
    pub index_ids: Vec<Option<String>>,
    /// Maturity dates
    pub maturities: Vec<Date>,
    /// Day count conventions
    pub day_counts: Vec<Option<DayCount>>,
    /// Default status
    pub is_defaulted: Vec<bool>,
    /// Recovery amounts for defaulted assets
    pub recovery_amounts: Vec<Option<f64>>,
    /// SMM overrides
    pub smm_overrides: Vec<Option<f64>>,
    /// MDR overrides
    pub mdr_overrides: Vec<Option<f64>>,
    /// Integer indices for curve lookups (optimization)
    pub curve_indices: Vec<Option<usize>>,
    /// Unique curve identifiers (referenced by curve_indices)
    pub unique_curves: Vec<String>,
}

impl PoolState {
    /// Create a new PoolState from a Pool (AoS to SoA conversion).
    pub fn from_pool(pool: &Pool) -> Self {
        let n = pool.assets.len();
        let mut ids = Vec::with_capacity(n);
        let mut balances = Vec::with_capacity(n);
        let mut rates = Vec::with_capacity(n);
        let mut spread_bps = Vec::with_capacity(n);
        let mut index_ids = Vec::with_capacity(n);
        let mut maturities = Vec::with_capacity(n);
        let mut day_counts: Vec<Option<DayCount>> = Vec::with_capacity(n);
        let mut is_defaulted = Vec::with_capacity(n);
        let mut recovery_amounts = Vec::with_capacity(n);
        let mut smm_overrides = Vec::with_capacity(n);
        let mut mdr_overrides = Vec::with_capacity(n);

        for asset in &pool.assets {
            ids.push(asset.id.to_string());
            balances.push(asset.balance.amount());
            rates.push(asset.rate);
            spread_bps.push(asset.spread_bps);
            index_ids.push(asset.index_id.clone());
            maturities.push(asset.maturity);
            day_counts.push(Some(asset.day_count));
            is_defaulted.push(asset.is_defaulted);
            recovery_amounts.push(asset.recovery_amount.map(|m| m.amount()));
            smm_overrides.push(asset.smm_override);
            mdr_overrides.push(asset.mdr_override);
        }

        // Build unique curve index
        let mut unique_curves = Vec::new();
        let mut curve_map = finstack_core::HashMap::default();
        let mut curve_indices = Vec::with_capacity(n);

        for id_opt in &index_ids {
            if let Some(id) = id_opt {
                if !curve_map.contains_key(id) {
                    curve_map.insert(id.clone(), unique_curves.len());
                    unique_curves.push(id.clone());
                }
                curve_indices.push(Some(curve_map[id]));
            } else {
                curve_indices.push(None);
            }
        }

        Self {
            ids,
            balances,
            rates,
            spread_bps,
            index_ids,
            maturities,
            day_counts,
            is_defaulted,
            recovery_amounts,
            smm_overrides,
            mdr_overrides,
            curve_indices,
            unique_curves,
        }
    }

    /// Get the number of assets in the pool.
    pub fn len(&self) -> usize {
        self.balances.len()
    }

    /// Check if the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.balances.is_empty()
    }
}
