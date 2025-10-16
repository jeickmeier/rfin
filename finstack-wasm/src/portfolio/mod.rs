//! Portfolio management bindings for WASM.

mod builder;
mod grouping;
mod metrics;
mod portfolio;
mod results;
mod types;
mod valuation;

#[cfg(feature = "scenarios")]
mod scenarios;

pub use builder::JsPortfolioBuilder;
pub use grouping::{js_aggregate_by_attribute, js_group_by_attribute};
pub use metrics::{js_aggregate_metrics, JsAggregatedMetric, JsPortfolioMetrics};
pub use portfolio::JsPortfolio;
pub use results::JsPortfolioResults;
pub use types::{
    js_create_position_from_bond, js_create_position_from_deposit, JsEntity, JsPosition,
    JsPositionUnit,
};
pub use valuation::{js_value_portfolio, JsPortfolioValuation, JsPositionValue};

#[cfg(feature = "scenarios")]
pub use scenarios::{js_apply_and_revalue, js_apply_scenario};
