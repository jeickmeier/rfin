//! Margin calculation result types.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_valuations::margin::{ImMethodology, NettingSetId, SimmSensitivities};
use std::collections::HashMap;

/// Margin results for a single netting set.
#[derive(Debug, Clone)]
pub struct NettingSetMargin {
    /// Netting set identifier
    pub netting_set_id: NettingSetId,
    /// Calculation date
    pub as_of: Date,
    /// Initial margin requirement
    pub initial_margin: Money,
    /// Variation margin requirement
    pub variation_margin: Money,
    /// Total margin (IM + positive VM)
    pub total_margin: Money,
    /// Number of positions in the netting set
    pub position_count: usize,
    /// IM methodology used
    pub im_methodology: ImMethodology,
    /// Aggregated sensitivities (for SIMM breakdown)
    pub sensitivities: Option<SimmSensitivities>,
    /// Breakdown by risk class (for SIMM)
    pub im_breakdown: HashMap<String, Money>,
}

impl NettingSetMargin {
    /// Create a new netting set margin result.
    #[must_use]
    pub fn new(
        netting_set_id: NettingSetId,
        as_of: Date,
        initial_margin: Money,
        variation_margin: Money,
        position_count: usize,
        im_methodology: ImMethodology,
    ) -> Self {
        let currency = initial_margin.currency();
        let total = Money::new(
            initial_margin.amount() + variation_margin.amount().max(0.0),
            currency,
        );

        Self {
            netting_set_id,
            as_of,
            initial_margin,
            variation_margin,
            total_margin: total,
            position_count,
            im_methodology,
            sensitivities: None,
            im_breakdown: HashMap::new(),
        }
    }

    /// Add SIMM breakdown information.
    pub fn with_simm_breakdown(
        mut self,
        sensitivities: SimmSensitivities,
        breakdown: HashMap<String, Money>,
    ) -> Self {
        self.sensitivities = Some(sensitivities);
        self.im_breakdown = breakdown;
        self
    }

    /// Check if this is a cleared netting set.
    #[must_use]
    pub fn is_cleared(&self) -> bool {
        self.netting_set_id.is_cleared()
    }
}

/// Portfolio-wide margin calculation results.
#[derive(Debug, Clone)]
pub struct PortfolioMarginResult {
    /// Calculation date
    pub as_of: Date,
    /// Base currency for aggregated figures
    pub base_currency: Currency,
    /// Total initial margin across all netting sets
    pub total_initial_margin: Money,
    /// Total variation margin across all netting sets
    pub total_variation_margin: Money,
    /// Total margin requirement
    pub total_margin: Money,
    /// Results by netting set
    pub by_netting_set: HashMap<NettingSetId, NettingSetMargin>,
    /// Number of positions included in margin calculation
    pub total_positions: usize,
    /// Number of positions without margin specs (excluded)
    pub positions_without_margin: usize,
}

impl PortfolioMarginResult {
    /// Create a new portfolio margin result.
    #[must_use]
    pub fn new(as_of: Date, base_currency: Currency) -> Self {
        Self {
            as_of,
            base_currency,
            total_initial_margin: Money::new(0.0, base_currency),
            total_variation_margin: Money::new(0.0, base_currency),
            total_margin: Money::new(0.0, base_currency),
            by_netting_set: HashMap::new(),
            total_positions: 0,
            positions_without_margin: 0,
        }
    }

    /// Add a netting set margin result.
    pub fn add_netting_set(&mut self, result: NettingSetMargin) {
        // Convert to base currency if needed (simplified - assumes same currency)
        // In production, would use FX rates for cross-currency conversion
        // TODO: Implement FX conversion when currencies differ
        let im = result.initial_margin.amount();
        let vm = result.variation_margin.amount();

        self.total_initial_margin = Money::new(
            self.total_initial_margin.amount() + im,
            self.base_currency,
        );
        self.total_variation_margin = Money::new(
            self.total_variation_margin.amount() + vm,
            self.base_currency,
        );
        self.total_margin = Money::new(
            self.total_margin.amount() + result.total_margin.amount(),
            self.base_currency,
        );
        self.total_positions += result.position_count;
        self.by_netting_set.insert(result.netting_set_id.clone(), result);
    }

    /// Get the number of netting sets.
    #[must_use]
    pub fn netting_set_count(&self) -> usize {
        self.by_netting_set.len()
    }

    /// Get results for cleared vs bilateral netting sets.
    #[must_use]
    pub fn cleared_bilateral_split(&self) -> (Money, Money) {
        let mut cleared = 0.0;
        let mut bilateral = 0.0;

        for result in self.by_netting_set.values() {
            if result.is_cleared() {
                cleared += result.total_margin.amount();
            } else {
                bilateral += result.total_margin.amount();
            }
        }

        (
            Money::new(cleared, self.base_currency),
            Money::new(bilateral, self.base_currency),
        )
    }

    /// Iterate over netting set results.
    pub fn iter(&self) -> impl Iterator<Item = (&NettingSetId, &NettingSetMargin)> {
        self.by_netting_set.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    fn test_date() -> Date {
        Date::from_calendar_date(2024, Month::June, 15).expect("valid date")
    }

    #[test]
    fn test_netting_set_margin_creation() {
        let id = NettingSetId::bilateral("BANK_A", "CSA_001");
        let result = NettingSetMargin::new(
            id,
            test_date(),
            Money::new(5_000_000.0, Currency::USD),
            Money::new(1_000_000.0, Currency::USD),
            10,
            ImMethodology::Simm,
        );

        assert_eq!(result.initial_margin.amount(), 5_000_000.0);
        assert_eq!(result.variation_margin.amount(), 1_000_000.0);
        assert_eq!(result.total_margin.amount(), 6_000_000.0);
        assert!(!result.is_cleared());
    }

    #[test]
    fn test_portfolio_margin_aggregation() {
        let mut portfolio_result = PortfolioMarginResult::new(test_date(), Currency::USD);

        // Add bilateral netting set
        let bilateral = NettingSetMargin::new(
            NettingSetId::bilateral("BANK_A", "CSA_001"),
            test_date(),
            Money::new(5_000_000.0, Currency::USD),
            Money::new(1_000_000.0, Currency::USD),
            10,
            ImMethodology::Simm,
        );
        portfolio_result.add_netting_set(bilateral);

        // Add cleared netting set
        let cleared = NettingSetMargin::new(
            NettingSetId::cleared("LCH"),
            test_date(),
            Money::new(3_000_000.0, Currency::USD),
            Money::new(500_000.0, Currency::USD),
            5,
            ImMethodology::ClearingHouse,
        );
        portfolio_result.add_netting_set(cleared);

        assert_eq!(portfolio_result.netting_set_count(), 2);
        assert_eq!(portfolio_result.total_initial_margin.amount(), 8_000_000.0);
        assert_eq!(portfolio_result.total_variation_margin.amount(), 1_500_000.0);
        assert_eq!(portfolio_result.total_positions, 15);

        let (cleared_total, bilateral_total) = portfolio_result.cleared_bilateral_split();
        assert_eq!(cleared_total.amount(), 3_500_000.0);
        assert_eq!(bilateral_total.amount(), 6_000_000.0);
    }
}

