use crate::engine::ExecutionContext;
use crate::error::Result;
use crate::spec::OperationSpec;
use finstack_core::market_data::bumps::MarketBump;

use finstack_core::market_data::term_structures::{
    DiscountCurve, ForwardCurve, HazardCurve, InflationCurve,
};
use std::sync::Arc;

/// Represents the outcome of a scenario operation that can be collected and applied later.
/// This allows the engine to separate the "decision" phase from the "mutation" phase.
#[derive(Debug)]
pub enum ScenarioEffect {
    /// A market data bump to be applied to the context.
    MarketBump(MarketBump),
    /// A warning message to be recorded.
    Warning(String),

    /// Update a discount curve in the market.
    #[allow(dead_code)] // Engine will use this
    UpdateDiscountCurve {
        /// The curve identifier.
        id: String,
        /// The updated curve instance.
        curve: Arc<DiscountCurve>,
    },
    /// Update a forward curve in the market.
    #[allow(dead_code)]
    UpdateForwardCurve {
        /// The curve identifier.
        id: String,
        /// The updated curve instance.
        curve: Arc<ForwardCurve>,
    },
    /// Update a hazard curve in the market.
    #[allow(dead_code)]
    UpdateHazardCurve {
        /// The curve identifier.
        id: String,
        /// The updated curve instance.
        curve: Arc<HazardCurve>,
    },
    /// Update an inflation curve in the market.
    #[allow(dead_code)]
    UpdateInflationCurve {
        /// The curve identifier.
        id: String,
        /// The updated curve instance.
        curve: Arc<InflationCurve>,
    },

    /// Apply a forecast adjustment to the statement model (% change).
    StmtForecastPercent {
        /// The ID of the statement item node.
        node_id: String,
        /// The percentage change to apply.
        pct: f64,
    },
    /// Apply a forecast assignment to the statement model (absolute value).
    StmtForecastAssign {
        /// The ID of the statement item node.
        node_id: String,
        /// The value to assign.
        value: f64,
    },

    /// Update rate binding.
    RateBinding {
        /// The ID of the statement item node.
        node_id: String,
        /// The ID of the curve to bind to.
        curve_id: String,
    },

    /// Apply a price shock to instruments.
    InstrumentPriceShock {
        /// Filter by instrument types (if present).
        types: Option<Vec<finstack_valuations::pricer::InstrumentType>>,
        /// Filter by attributes (if present).
        attrs: Option<indexmap::IndexMap<String, String>>,
        /// The percentage shock to apply.
        pct: f64,
    },
    /// Apply a spread shock to instruments.
    InstrumentSpreadShock {
        /// Filter by instrument types (if present).
        types: Option<Vec<finstack_valuations::pricer::InstrumentType>>,
        /// Filter by attributes (if present).
        attrs: Option<indexmap::IndexMap<String, String>>,
        /// The spread shock in basis points.
        bp: f64,
    },
}

/// Trait for adapters that can convert an OperationSpec into effects.
pub trait ScenarioAdapter {
    /// Try to handle an operation. Returns `Ok(None)` if this adapter doesn't handle the op type.
    fn try_generate_effects(
        &self,
        op: &OperationSpec,
        ctx: &ExecutionContext,
    ) -> Result<Option<Vec<ScenarioEffect>>>;
}
