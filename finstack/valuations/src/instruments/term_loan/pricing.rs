//! Discounting pricer for Term Loans (deterministic v1).

use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

use super::cashflows::generate_cashflows;
use super::types::TermLoan;

#[derive(Default)]
pub struct TermLoanDiscountingPricer;

impl TermLoanDiscountingPricer {
    /// Price a term loan using deterministic cashflows and discounting.
    pub fn price(
        loan: &TermLoan,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        let schedule = generate_cashflows(loan, market, as_of)?;

        // Get discount curve
        let disc = market.get_discount_ref(loan.discount_curve_id.as_str())?;
        let disc_dc = disc.day_count();
        let base_date = disc.base_date();

        let t_as_of = disc_dc.year_fraction(
            base_date,
            as_of,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let df_as_of = disc.df(t_as_of);

        let mut pv = Money::new(0.0, loan.currency);

        for cf in &schedule.flows {
            if cf.date <= as_of {
                continue;
            }
            let t_cf = disc_dc.year_fraction(
                base_date,
                cf.date,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let df = {
                let df_abs = disc.df(t_cf);
                if df_as_of != 0.0 {
                    df_abs / df_as_of
                } else {
                    1.0
                }
            };
            pv = pv.checked_add(cf.amount * df)?;
        }

        Ok(pv)
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
        _as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        let loan = instrument
            .as_any()
            .downcast_ref::<TermLoan>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::TermLoan, instrument.key())
            })?;

        // Use discount curve base date as valuation date
        let disc = market
            .get_discount_ref(loan.discount_curve_id.as_str())
            .map_err(|e| PricingError::model_failure(e.to_string()))?;
        let as_of = disc.base_date();

        let pv = Self::price(loan, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        Ok(ValuationResult::stamped(loan.id(), as_of, pv))
    }
}
