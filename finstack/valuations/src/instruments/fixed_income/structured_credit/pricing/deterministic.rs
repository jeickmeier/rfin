//! Deterministic cashflow simulation wrappers for structured credit instruments.

use crate::cashflow::traits::DatedFlows;
use crate::instruments::fixed_income::structured_credit::pricing::simulation_engine::{
    self, DeterministicPoolFlowSource,
};
use crate::instruments::fixed_income::structured_credit::types::{
    StructuredCredit, TrancheCashflows,
};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::HashMap;
use finstack_core::Result;

/// Run full deterministic cashflow simulation for a structured credit instrument.
pub fn run_simulation(
    instrument: &StructuredCredit,
    context: &MarketContext,
    as_of: Date,
) -> Result<HashMap<String, TrancheCashflows>> {
    let mut source = DeterministicPoolFlowSource;
    simulation_engine::run_simulation_with_source(instrument, context, as_of, &mut source)
}

/// Generate aggregated deterministic cashflows for all tranches.
pub fn generate_cashflows(
    instrument: &StructuredCredit,
    context: &MarketContext,
    as_of: Date,
) -> Result<DatedFlows> {
    let full_results = run_simulation(instrument, context, as_of)?;
    simulation_engine::aggregate_tranche_cashflows(&full_results)
}

/// Generate deterministic cashflows for a specific tranche.
pub fn generate_tranche_cashflows(
    instrument: &StructuredCredit,
    tranche_id: &str,
    context: &MarketContext,
    as_of: Date,
) -> Result<TrancheCashflows> {
    let mut full_results = run_simulation(instrument, context, as_of)?;
    simulation_engine::take_tranche_cashflows(&mut full_results, tranche_id)
}
