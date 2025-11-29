//! Portfolio margin aggregation.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
// Note: InitialMarginMetric and VariationMarginMetric are available but
// netting set margin is calculated via aggregated sensitivities directly.
use finstack_valuations::margin::{
    ImMethodology, Marginable, NettingSetId, SimmCalculator, SimmSensitivities,
};

use crate::margin::netting_set::{NettingSet, NettingSetManager};
use crate::margin::results::{NettingSetMargin, PortfolioMarginResult};
use crate::portfolio::Portfolio;
use crate::position::Position;
use crate::{PortfolioError, PositionId, Result};
use std::sync::Arc;

// ============================================================================
// Helper Functions
// ============================================================================

/// Try to get a reference to the `Marginable` implementation from a dynamic instrument.
///
/// Returns `Some(&dyn Marginable)` if the instrument implements `Marginable`,
/// otherwise returns `None`.
fn as_marginable(instrument: &Arc<dyn Instrument>) -> Option<&dyn Marginable> {
    // Try each known marginable type
    if let Some(irs) = instrument
        .as_any()
        .downcast_ref::<finstack_valuations::instruments::irs::InterestRateSwap>()
    {
        return Some(irs as &dyn Marginable);
    }

    if let Some(cds) = instrument
        .as_any()
        .downcast_ref::<finstack_valuations::instruments::cds::CreditDefaultSwap>()
    {
        return Some(cds as &dyn Marginable);
    }

    if let Some(repo) = instrument
        .as_any()
        .downcast_ref::<finstack_valuations::instruments::repo::Repo>()
    {
        return Some(repo as &dyn Marginable);
    }

    if let Some(cds_index) = instrument
        .as_any()
        .downcast_ref::<finstack_valuations::instruments::cds_index::CDSIndex>(
    ) {
        return Some(cds_index as &dyn Marginable);
    }

    if let Some(eq_trs) = instrument
        .as_any()
        .downcast_ref::<finstack_valuations::instruments::trs::EquityTotalReturnSwap>(
    ) {
        return Some(eq_trs as &dyn Marginable);
    }

    if let Some(fi_trs) = instrument
        .as_any()
        .downcast_ref::<finstack_valuations::instruments::trs::FIIndexTotalReturnSwap>(
    ) {
        return Some(fi_trs as &dyn Marginable);
    }

    None
}

// ============================================================================
// Portfolio Margin Aggregator
// ============================================================================

/// Aggregates margin requirements across a portfolio.
///
/// Organizes positions into netting sets and calculates aggregate
/// margin requirements with proper netting of sensitivities.
#[derive(Debug)]
pub struct PortfolioMarginAggregator {
    /// Netting set manager
    netting_sets: NettingSetManager,
    /// Position references for calculation
    positions: Vec<(PositionId, NettingSetId)>,
    /// Base currency for aggregation
    base_currency: Currency,
    /// Cached SIMM calculator for efficiency
    simm_calculator: SimmCalculator,
}

impl PortfolioMarginAggregator {
    /// Create a new aggregator with a base currency.
    #[must_use]
    pub fn new(base_currency: Currency) -> Self {
        Self {
            netting_sets: NettingSetManager::new(),
            positions: Vec::new(),
            base_currency,
            simm_calculator: SimmCalculator::default(),
        }
    }

    /// Create an aggregator from a portfolio.
    ///
    /// Automatically organizes positions into netting sets based on their
    /// margin specifications.
    #[must_use]
    pub fn from_portfolio(portfolio: &Portfolio) -> Self {
        let mut aggregator = Self::new(portfolio.base_ccy);

        // Iterate through all positions
        for position in &portfolio.positions {
            aggregator.add_position(position);
        }

        aggregator
    }

    /// Add a position to the aggregator.
    ///
    /// The position will be assigned to its appropriate netting set
    /// based on its margin specification.
    pub fn add_position(&mut self, position: &Position) {
        // Try to get netting set ID from the instrument
        let netting_set_id = self.get_netting_set_for_position(position);

        if let Some(ns_id) = netting_set_id {
            self.netting_sets
                .add_position(position, Some(ns_id.clone()));
            self.positions.push((position.position_id.clone(), ns_id));
        }
    }

    /// Get the netting set ID for a position based on its instrument.
    fn get_netting_set_for_position(&self, position: &Position) -> Option<NettingSetId> {
        as_marginable(&position.instrument).and_then(|m| m.netting_set_id())
    }

    /// Calculate margin requirements for the portfolio.
    ///
    /// Returns aggregated margin results by netting set.
    pub fn calculate(
        &mut self,
        portfolio: &Portfolio,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<PortfolioMarginResult> {
        let mut result = PortfolioMarginResult::new(as_of, self.base_currency);

        // Calculate sensitivities for each position and aggregate by netting set
        for (pos_id, ns_id) in &self.positions {
            if let Some(position) = portfolio.get_position(pos_id.as_str()) {
                // Calculate sensitivities and aggregate
                if let Ok(sensitivities) =
                    self.calculate_position_sensitivities(position, market, as_of)
                {
                    self.netting_sets.merge_sensitivities(ns_id, &sensitivities);
                }
            }
        }

        // Calculate margin for each netting set
        for (_ns_id, netting_set) in self.netting_sets.iter() {
            let ns_margin =
                self.calculate_netting_set_margin(netting_set, portfolio, market, as_of)?;
            result.add_netting_set(ns_margin);
        }

        // Count positions without margin
        result.positions_without_margin = portfolio
            .positions
            .len()
            .saturating_sub(result.total_positions);

        Ok(result)
    }

    /// Calculate sensitivities for a single position.
    fn calculate_position_sensitivities(
        &self,
        position: &Position,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<SimmSensitivities> {
        if let Some(marginable) = as_marginable(&position.instrument) {
            marginable
                .simm_sensitivities(market, as_of)
                .map_err(|e| PortfolioError::valuation(position.position_id.clone(), e.to_string()))
        } else {
            // Default: return empty sensitivities
            Ok(SimmSensitivities::new(self.base_currency))
        }
    }

    /// Calculate margin for a netting set.
    fn calculate_netting_set_margin(
        &self,
        netting_set: &NettingSet,
        portfolio: &Portfolio,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<NettingSetMargin> {
        // Calculate aggregated VM from position MTMs
        let mut total_mtm = 0.0;
        let mut position_count = 0;

        for pos_id in &netting_set.positions {
            if let Some(position) = portfolio.get_position(pos_id.as_str()) {
                if let Ok(mtm) = self.get_position_mtm(position, market, as_of) {
                    total_mtm += mtm.amount();
                    position_count += 1;
                }
            }
        }

        // VM is the net MTM (positive = we owe, negative = they owe)
        let vm = Money::new(total_mtm, self.base_currency);

        // Calculate IM from aggregated sensitivities
        let im = if let Some(ref sensitivities) = netting_set.aggregated_sensitivities {
            self.calculate_simm_from_sensitivities(sensitivities)
        } else {
            Money::new(0.0, self.base_currency)
        };

        // Determine methodology
        let methodology = if netting_set.is_cleared() {
            ImMethodology::ClearingHouse
        } else {
            ImMethodology::Simm
        };

        let mut result = NettingSetMargin::new(
            netting_set.id.clone(),
            as_of,
            im,
            vm,
            position_count,
            methodology,
        );

        // Add SIMM breakdown if available
        if let Some(ref sensitivities) = netting_set.aggregated_sensitivities {
            let breakdown = self.calculate_simm_breakdown(sensitivities);
            result = result.with_simm_breakdown(sensitivities.clone(), breakdown);
        }

        Ok(result)
    }

    /// Get MTM for a position.
    fn get_position_mtm(
        &self,
        position: &Position,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        if let Some(marginable) = as_marginable(&position.instrument) {
            marginable
                .mtm_for_vm(market, as_of)
                .map_err(|e| PortfolioError::valuation(position.position_id.clone(), e.to_string()))
        } else {
            // Default: return zero
            Ok(Money::new(0.0, self.base_currency))
        }
    }

    /// Calculate SIMM from aggregated sensitivities.
    ///
    /// Uses the cached `SimmCalculator` for efficiency.
    fn calculate_simm_from_sensitivities(&self, sensitivities: &SimmSensitivities) -> Money {
        let (total, _) = self.calculate_simm_with_breakdown(sensitivities);
        total
    }

    /// Calculate SIMM breakdown by risk class.
    fn calculate_simm_breakdown(
        &self,
        sensitivities: &SimmSensitivities,
    ) -> std::collections::HashMap<String, Money> {
        let (_, breakdown) = self.calculate_simm_with_breakdown(sensitivities);
        breakdown
    }

    /// Calculate SIMM and breakdown in a single pass.
    ///
    /// Returns (total_im, breakdown_by_risk_class).
    fn calculate_simm_with_breakdown(
        &self,
        sensitivities: &SimmSensitivities,
    ) -> (Money, std::collections::HashMap<String, Money>) {
        let (total_im, breakdown) = self
            .simm_calculator
            .calculate_from_sensitivities(sensitivities, self.base_currency);
        (Money::new(total_im, self.base_currency), breakdown)
    }

    /// Get the number of netting sets.
    #[must_use]
    pub fn netting_set_count(&self) -> usize {
        self.netting_sets.count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregator_creation() {
        let aggregator = PortfolioMarginAggregator::new(Currency::USD);
        assert_eq!(aggregator.netting_set_count(), 0);
    }
}
