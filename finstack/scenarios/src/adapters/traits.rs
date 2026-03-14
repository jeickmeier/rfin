use crate::engine::ExecutionContext;
use crate::error::Result;
use crate::spec::{OperationSpec, RateBindingSpec};
use finstack_core::market_data::bumps::MarketBump;
use finstack_statements::NodeId;

/// Represents the outcome of a scenario operation that can be collected and applied later.
/// This allows the engine to separate the "decision" phase from the "mutation" phase.
#[derive(Debug)]
pub enum ScenarioEffect {
    /// A market data bump to be applied to the context.
    MarketBump(MarketBump),
    /// A warning message to be recorded.
    Warning(String),

    /// Update a curve in the market (discount, forward, hazard, inflation, or vol-index).
    UpdateCurve(finstack_core::market_data::context::CurveStorage),

    /// Apply a forecast adjustment to the statement model (% change).
    StmtForecastPercent {
        /// The ID of the statement item node.
        node_id: NodeId,
        /// The percentage change to apply.
        pct: f64,
    },
    /// Apply a forecast assignment to the statement model (absolute value).
    StmtForecastAssign {
        /// The ID of the statement item node.
        node_id: NodeId,
        /// The value to assign.
        value: f64,
    },

    /// Update rate binding.
    RateBinding {
        /// Binding specification to apply.
        binding: RateBindingSpec,
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

    /// Shock asset correlation on StructuredCredit instruments.
    AssetCorrelationShock {
        /// Additive shock in correlation points.
        delta_pts: f64,
    },
    /// Shock prepay-default correlation on StructuredCredit instruments.
    PrepayDefaultCorrelationShock {
        /// Additive shock in correlation points.
        delta_pts: f64,
    },
    /// Shock recovery-default correlation on StructuredCredit instruments.
    RecoveryCorrelationShock {
        /// Additive shock in correlation points.
        delta_pts: f64,
    },
    /// Shock prepay factor loading on StructuredCredit instruments.
    PrepayFactorLoadingShock {
        /// Additive shock to factor loading.
        delta_pts: f64,
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
