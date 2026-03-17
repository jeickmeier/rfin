//! Portfolio margin aggregation.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::fx::FxQuery;
use finstack_core::money::Money;
use finstack_margin::{ImMethodology, NettingSetId, SimmCalculator, SimmSensitivities};

use crate::margin::netting_set::{NettingSet, NettingSetManager};
use crate::margin::results::{NettingSetMargin, PortfolioMarginResult};
use crate::portfolio::Portfolio;
use crate::position::Position;
use crate::{Error, PositionId, Result};

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
        position
            .instrument
            .as_marginable()
            .and_then(|m| m.netting_set_id())
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
            // Currency mismatch is impossible here since we create all margins
            // with self.base_currency, but we handle the error for API consistency.
            result.add_netting_set(ns_margin).map_err(|e| {
                Error::validation(format!(
                    "Unexpected currency mismatch in margin aggregation: {}",
                    e
                ))
            })?;
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
        if let Some(marginable) = position.instrument.as_marginable() {
            marginable
                .simm_sensitivities(market, as_of)
                .map_err(|e| Error::valuation(position.position_id.clone(), e.to_string()))
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
        // Calculate aggregated VM from position MTMs, FX-converting to base currency
        let mut total_mtm = 0.0;
        let mut position_count = 0;

        for pos_id in &netting_set.positions {
            if let Some(position) = portfolio.get_position(pos_id.as_str()) {
                if let Ok(mtm) = self.get_position_mtm(position, market, as_of) {
                    // FX-convert MTM to base currency if necessary
                    let mtm_base = if mtm.currency() == self.base_currency {
                        mtm.amount()
                    } else {
                        self.convert_to_base(mtm, market, as_of)
                    };
                    total_mtm += mtm_base;
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

    /// Get MTM for a position in its native currency.
    fn get_position_mtm(
        &self,
        position: &Position,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        if let Some(marginable) = position.instrument.as_marginable() {
            marginable
                .mtm_for_vm(market, as_of)
                .map_err(|e| Error::valuation(position.position_id.clone(), e.to_string()))
        } else {
            // Default: return zero
            Ok(Money::new(0.0, self.base_currency))
        }
    }

    /// Convert a monetary amount to base currency using the FX matrix.
    ///
    /// Falls back to 1.0 if FX rate is unavailable (logs warning).
    fn convert_to_base(&self, amount: Money, market: &MarketContext, as_of: Date) -> f64 {
        if let Some(fx_matrix) = market.fx() {
            let query = FxQuery::new(amount.currency(), self.base_currency, as_of);
            if let Ok(rate_result) = fx_matrix.rate(query) {
                return amount.amount() * rate_result.rate;
            }
        }
        tracing::warn!(
            from = %amount.currency(),
            to = %self.base_currency,
            "Unable to FX-convert VM to base currency; using native amount"
        );
        amount.amount()
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
    ) -> finstack_core::HashMap<String, Money> {
        let (_, breakdown) = self.calculate_simm_with_breakdown(sensitivities);
        breakdown
    }

    /// Calculate SIMM and breakdown in a single pass.
    ///
    /// Returns (total_im, breakdown_by_risk_class).
    fn calculate_simm_with_breakdown(
        &self,
        sensitivities: &SimmSensitivities,
    ) -> (Money, finstack_core::HashMap<String, Money>) {
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
