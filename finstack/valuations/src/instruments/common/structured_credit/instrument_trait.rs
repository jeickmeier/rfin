//! Common trait for structured credit instruments.
//!
//! This trait provides a shared interface for CLO, ABS, RMBS, and CMBS instruments
//! to generate cashflows using a consistent waterfall engine.

use crate::cashflow::traits::DatedFlows;
use crate::instruments::common::structured_credit::{
    AssetPool, CoverageTests, CreditFactors, MarketConditions, MarketFactors,
    PrepaymentBehavior, DefaultBehavior, RecoveryBehavior,
    TrancheStructure, WaterfallEngine, PaymentRecipient,
};
use finstack_core::dates::{Date, Frequency};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for structured credit instrument dates
pub struct InstrumentDates {
    pub closing_date: Date,
    pub first_payment_date: Date,
    pub legal_maturity: Date,
    pub payment_frequency: Frequency,
}

/// Configuration for structured credit instrument models
pub struct InstrumentModels<'a> {
    pub prepayment: &'a Arc<dyn PrepaymentBehavior>,
    pub default: &'a Arc<dyn DefaultBehavior>,
    pub recovery: &'a Arc<dyn RecoveryBehavior>,
    pub market_conditions: &'a MarketConditions,
    pub credit_factors: &'a CreditFactors,
}

/// Common trait for structured credit instruments
pub trait StructuredCreditInstrument {
    /// Get reference to asset pool
    fn pool(&self) -> &AssetPool;

    /// Get reference to tranche structure
    fn tranches(&self) -> &TrancheStructure;
    
    /// Get all date configurations at once
    fn dates(&self) -> InstrumentDates {
        InstrumentDates {
            closing_date: self.closing_date(),
            first_payment_date: self.first_payment_date(),
            legal_maturity: self.legal_maturity(),
            payment_frequency: self.payment_frequency(),
        }
    }
    
    /// Get all model configurations at once
    fn models(&self) -> InstrumentModels {
        InstrumentModels {
            prepayment: self.prepayment_model(),
            default: self.default_model(),
            recovery: self.recovery_model(),
            market_conditions: self.market_conditions(),
            credit_factors: self.credit_factors(),
        }
    }

    /// Get closing date
    fn closing_date(&self) -> Date;

    /// Get first payment date
    fn first_payment_date(&self) -> Date;

    /// Get legal maturity date
    fn legal_maturity(&self) -> Date;

    /// Get payment frequency
    fn payment_frequency(&self) -> Frequency;

    /// Get prepayment model
    fn prepayment_model(&self) -> &Arc<dyn PrepaymentBehavior>;

    /// Get default model
    fn default_model(&self) -> &Arc<dyn DefaultBehavior>;

    /// Get recovery model
    fn recovery_model(&self) -> &Arc<dyn RecoveryBehavior>;

    /// Get market conditions
    fn market_conditions(&self) -> &MarketConditions;

    /// Get credit factors
    fn credit_factors(&self) -> &CreditFactors;

    /// Create instrument-specific waterfall engine
    fn create_waterfall_engine(&self) -> WaterfallEngine;

    /// Get instrument-specific prepayment rate override (if any)
    fn prepayment_rate_override(&self, _pay_date: Date, _seasoning: u32) -> Option<f64> {
        None
    }

    /// Get instrument-specific default rate override (if any)
    fn default_rate_override(&self, _pay_date: Date, _seasoning: u32) -> Option<f64> {
        None
    }

    /// Calculate prepayment rate (SMM)
    fn calculate_prepayment_rate(&self, pay_date: Date, seasoning_months: u32) -> f64 {
        if let Some(override_rate) = self.prepayment_rate_override(pay_date, seasoning_months) {
            return override_rate;
        }

        self.prepayment_model()
            .prepayment_rate(
                pay_date,
                self.closing_date(),
                seasoning_months,
                self.market_conditions(),
            )
            .max(0.0)
    }

    /// Calculate default rate (MDR)
    fn calculate_default_rate(&self, pay_date: Date, seasoning_months: u32) -> f64 {
        if let Some(override_rate) = self.default_rate_override(pay_date, seasoning_months) {
            return override_rate;
        }

        self.default_model()
            .default_rate(
                pay_date,
                self.closing_date(),
                seasoning_months,
                self.credit_factors(),
            )
            .max(0.0)
    }

    /// Generate complete tranche-specific cashflows using waterfall engine
    ///
    /// This is the shared implementation that eliminates duplication across
    /// CLO, ABS, CMBS, and RMBS instruments.
    fn generate_tranche_cashflows(
        &self,
        _context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        let pool = self.pool();
        let tranches = self.tranches();
        let base_ccy = pool.base_currency();
        let mut pool_outstanding = pool.total_balance();

        if pool_outstanding.amount() <= 0.0 {
            return Ok(Vec::new());
        }
        
        // Get all date and model configurations at once
        let dates = self.dates();
        let models = self.models();

        // Track tranche balances over time
        let mut tranche_balances: HashMap<String, Money> = tranches
            .tranches
            .iter()
            .map(|t| (t.id.to_string(), t.current_balance))
            .collect();

        // Store all tranche cashflows by tranche ID
        let mut tranche_cashflow_map: HashMap<String, Vec<(Date, Money)>> = HashMap::new();
        for tranche in &tranches.tranches {
            tranche_cashflow_map.insert(tranche.id.to_string(), Vec::new());
        }

        // Initialize waterfall engine with instrument-specific rules
        let mut waterfall_engine = self.create_waterfall_engine();

        // Initialize coverage tests
        let mut _coverage_tests = CoverageTests::new();

        let months_per_period = dates.payment_frequency.months().unwrap_or(3) as f64;
        let mut pay_date = dates.first_payment_date.max(as_of);

        // Simulate period-by-period
        while pay_date <= dates.legal_maturity && pool_outstanding.amount() > 100.0 {
            let seasoning_months = {
                let m = (pay_date.year() - dates.closing_date.year()) * 12
                    + (pay_date.month() as i32 - dates.closing_date.month() as i32);
                m.max(0) as u32
            };

            // Step 1: Calculate pool collections
            let wac = pool.weighted_avg_coupon();
            let period_rate = wac * (months_per_period / 12.0);
            let interest_collections =
                Money::new(pool_outstanding.amount() * period_rate, base_ccy);

            // Step 2: Apply prepayments and defaults
            let smm = self.calculate_prepayment_rate(pay_date, seasoning_months);
            let mdr = self.calculate_default_rate(pay_date, seasoning_months);

            let prepay_amt = Money::new(pool_outstanding.amount() * smm, base_ccy);
            let default_amt = Money::new(pool_outstanding.amount() * mdr, base_ccy);

            let recovery_rate = models.recovery.recovery_rate(
                pay_date,
                6,
                None,
                default_amt,
                &MarketFactors::default(),
            );
            let recovery_amt = Money::new(default_amt.amount() * recovery_rate, base_ccy);

            // Total principal available = prepayments + recoveries + scheduled (0 for now)
            let scheduled_prin = Money::new(0.0, base_ccy);
            let total_principal = scheduled_prin
                .checked_add(prepay_amt)?
                .checked_add(recovery_amt)?;

            // Total cash available for distribution
            let total_cash = interest_collections.checked_add(total_principal)?;

            // Step 3: Run waterfall to distribute cash to tranches
            let waterfall_result = waterfall_engine.apply_waterfall(
                total_cash,
                pay_date,
                tranches,
                pool_outstanding,
            )?;

            // Step 4: Record tranche-specific cashflows
            for tranche in &tranches.tranches {
                let tranche_id = tranche.id.to_string();

                // Get payment to this tranche from waterfall
                if let Some(payment) = waterfall_result
                    .distributions
                    .get(&PaymentRecipient::Tranche(tranche_id.clone()))
                {
                    if payment.amount() > 0.0 {
                        tranche_cashflow_map
                            .get_mut(&tranche_id)
                            .unwrap()
                            .push((pay_date, *payment));
                    }

                    // Update tranche balance (assuming payments reduce balance)
                    let interest_portion = Money::new(
                        tranche_balances[&tranche_id].amount()
                            * tranche.coupon.current_rate(pay_date)
                            * (months_per_period / 12.0),
                        base_ccy,
                    );
                    let principal_payment = payment
                        .checked_sub(interest_portion)
                        .unwrap_or(Money::new(0.0, base_ccy));

                    if let Some(current) = tranche_balances.get_mut(&tranche_id) {
                        *current = current.checked_sub(principal_payment).unwrap_or(*current);
                    }
                }
            }

            // Step 5: Update pool balance
            pool_outstanding = pool_outstanding
                .checked_sub(prepay_amt)?
                .checked_sub(default_amt)?;

            // Advance to next period
            pay_date = add_months(
                pay_date,
                dates.payment_frequency.months().unwrap_or(3) as i32,
            );
        }

        // Aggregate all tranche cashflows into single schedule
        // For now, sum across all tranches; in production would track separately
        let mut all_flows: DatedFlows = Vec::new();
        let mut flow_map: HashMap<Date, Money> = HashMap::new();

        for (_tranche_id, flows) in tranche_cashflow_map {
            for (date, amount) in flows {
                *flow_map.entry(date).or_insert(Money::new(0.0, base_ccy)) =
                    flow_map[&date].checked_add(amount)?;
            }
        }

        for (date, amount) in flow_map {
            all_flows.push((date, amount));
        }
        all_flows.sort_by_key(|(d, _)| *d);

        Ok(all_flows)
    }
}

/// Helper function to add months to a date
fn add_months(date: Date, months: i32) -> Date {
    let year = date.year();
    let month = date.month() as i32;
    
    let total_months = month + months;
    let new_year = year + ((total_months - 1) / 12);
    let new_month = ((total_months - 1) % 12) + 1;
    
    Date::from_calendar_date(
        new_year,
        time::Month::try_from(new_month as u8).unwrap_or(time::Month::January),
        date.day().min(28), // Avoid end-of-month issues
    )
    .unwrap_or(date)
}

#[cfg(test)]
mod tests {
    // Tests would go here
}
