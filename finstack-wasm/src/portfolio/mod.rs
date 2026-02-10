//! Portfolio management bindings for WASM.

mod attribution;
mod builder;
mod cashflows;
mod grouping;
mod margin;
mod metrics;
mod optimization;
mod positions;
mod results;
mod types;
mod valuation;

#[cfg(feature = "scenarios")]
mod scenarios;

pub use builder::JsPortfolioBuilder;
pub use cashflows::{
    js_aggregate_cashflows, js_cashflows_to_base_by_period, js_collapse_cashflows_to_base_by_date,
    JsPortfolioCashflowBuckets, JsPortfolioCashflows,
};
pub use grouping::{js_aggregate_by_attribute, js_group_by_attribute};
pub use margin::{
    JsNettingSet, JsNettingSetId, JsNettingSetManager, JsNettingSetMargin,
    JsPortfolioMarginAggregator, JsPortfolioMarginResult,
};
pub use metrics::{js_aggregate_metrics, js_is_summable, JsAggregatedMetric, JsPortfolioMetrics};
pub use optimization::js_optimize_max_yield_with_ccc_limit;
pub use positions::JsPortfolio;
pub use results::JsPortfolioResult;
pub use types::{
    js_create_position_from_bond, js_create_position_from_deposit, JsEntity, JsPosition,
    JsPositionUnit,
};
pub use valuation::{
    js_value_portfolio, js_value_portfolio_with_options, JsPortfolioValuation,
    JsPortfolioValuationOptions, JsPositionValue,
};

pub use attribution::{js_attribute_portfolio_pnl, JsPnlAttribution, JsPortfolioAttribution};

#[cfg(feature = "scenarios")]
pub use scenarios::{js_apply_and_revalue, js_apply_scenario};
