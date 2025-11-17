//! Discounting pricer for Term Loans (deterministic v1).

use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

use super::cashflows::generate_cashflows;
use super::types::TermLoan;

#[derive(Default)]
/// Term loan pricer using discounting of projected cashflows
pub struct TermLoanDiscountingPricer;

impl TermLoanDiscountingPricer {
    /// Price a term loan using deterministic cashflows and discounting.
    pub fn price(
        loan: &TermLoan,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        // Build full cashflow schedule
        let schedule = generate_cashflows(loan, market, as_of)?;

        // Retrieve discount curve and discount flows to `as_of` using date-based DF mapping.
        // This ensures valuation is anchored on the valuation date rather than the curve's
        // internal base date.
        let disc = market.get_discount_ref(loan.discount_curve_id.as_str())?;
        let flows: Vec<(finstack_core::dates::Date, Money)> = schedule
            .flows
            .iter()
            .map(|cf| (cf.date, cf.amount))
            .collect();

        crate::instruments::common::discountable::npv_by_date(disc, as_of, &flows)
    }
}

impl Pricer for TermLoanDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::TermLoan, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        let loan = instrument
            .as_any()
            .downcast_ref::<TermLoan>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::TermLoan, instrument.key())
            })?;

        // Use the provided as_of date for valuation
        let pv = Self::price(loan, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        Ok(ValuationResult::stamped(loan.id(), as_of, pv))
    }
}
