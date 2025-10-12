//! Common trait for structured credit instruments.
//!
//! This trait provides a shared interface for CLO, ABS, RMBS, and CMBS instruments
//! to generate cashflows using a consistent waterfall engine.

use crate::cashflow::traits::DatedFlows;
use crate::instruments::common::structured_credit::{
    AssetPool, CoverageTests, CreditFactors, DefaultBehavior, MarketConditions, MarketFactors,
    PaymentRecipient, PrepaymentBehavior, RecoveryBehavior, TrancheStructure, WaterfallEngine,
};
use finstack_core::dates::utils::add_months;
use finstack_core::dates::{Date, Frequency};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use std::collections::HashMap;
use std::sync::Arc;

/// Update tranche balance after payment (helper function)
fn update_tranche_balance(
    tranche_balances: &mut HashMap<String, Money>,
    tranche_id: &str,
    payment: Money,
    interest_portion: Money,
) -> finstack_core::Result<()> {
    let principal_payment = payment
        .checked_sub(interest_portion)
        .unwrap_or(Money::new(0.0, payment.currency()));

    if let Some(current) = tranche_balances.get_mut(tranche_id) {
        *current = current.checked_sub(principal_payment).unwrap_or(*current);
    }
    
    Ok(())
}

/// Common trait for structured credit instruments
pub trait StructuredCreditInstrument {
    /// Get reference to asset pool
    fn pool(&self) -> &AssetPool;

    /// Get reference to tranche structure
    fn tranches(&self) -> &TrancheStructure;

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

    /// Calculate period interest collections from pool assets
    fn calculate_period_interest_collections(
        &self,
        pay_date: Date,
        months_per_period: f64,
        context: &MarketContext,
    ) -> finstack_core::Result<Money> {
        let pool = self.pool();
        let base_ccy = pool.base_currency();
        let mut interest_collections = Money::new(0.0, base_ccy);
        
        for asset in &pool.assets {
            let asset_rate = if let Some(idx) = &asset.index_id {
                match context.get_forward_ref(idx.as_str()) {
                    Ok(fwd) => {
                        let base = fwd.base_date();
                        let dc = finstack_core::dates::DayCount::Act365F;
                        let t2 = dc
                            .year_fraction(base, pay_date, finstack_core::dates::DayCountCtx::default())
                            .unwrap_or(0.0);
                        let t1 = (t2 - 0.25).max(0.0);
                        let idx_rate = fwd.rate_period(t1, t2);
                        idx_rate + (asset.spread_bps().max(0.0) / 10_000.0)
                    }
                    Err(_) => asset.rate,
                }
            } else {
                asset.rate
            };
            
            let ir = Money::new(
                asset.balance.amount() * asset_rate * (months_per_period / 12.0),
                base_ccy,
            );
            interest_collections = interest_collections.checked_add(ir)?;
        }
        
        Ok(interest_collections)
    }

    /// Calculate prepayments and defaults for a period
    fn calculate_period_prepayments_and_defaults(
        &self,
        pay_date: Date,
        seasoning_months: u32,
        pool_outstanding: Money,
        months_per_period: f64,
    ) -> finstack_core::Result<(Money, Money, Money)> {
        let base_ccy = pool_outstanding.currency();
        let smm = self.calculate_prepayment_rate(pay_date, seasoning_months);
        let mdr = self.calculate_default_rate(pay_date, seasoning_months);

        // Adjust for payment period frequency
        let period_smm = 1.0 - (1.0 - smm).powi(months_per_period as i32);
        let period_mdr = 1.0 - (1.0 - mdr).powi(months_per_period as i32);

        let prepay_amt = Money::new(pool_outstanding.amount() * period_smm, base_ccy);
        let default_amt = Money::new(pool_outstanding.amount() * period_mdr, base_ccy);

        let recovery_rate = self.recovery_model().recovery_rate(
            pay_date,
            super::constants::DEFAULT_RESOLUTION_LAG_MONTHS,
            None,
            default_amt,
            &MarketFactors::default(),
        );
        let recovery_amt = Money::new(default_amt.amount() * recovery_rate, base_ccy);

        Ok((prepay_amt, default_amt, recovery_amt))
    }

    /// Generate complete tranche-specific cashflows using waterfall engine
    ///
    /// This is the shared implementation that eliminates duplication across
    /// CLO, ABS, CMBS, and RMBS instruments.
    fn generate_tranche_cashflows(
        &self,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        let pool = self.pool();
        let tranches = self.tranches();
        let base_ccy = pool.base_currency();
        let mut pool_outstanding = pool.total_balance();

        if pool_outstanding.amount() <= 0.0 {
            return Ok(Vec::new());
        }

        // Get date configurations
        let dates_closing_date = self.closing_date();
        let dates_first_payment_date = self.first_payment_date();
        let dates_legal_maturity = self.legal_maturity();
        let dates_payment_frequency = self.payment_frequency();

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

        let months_per_period = dates_payment_frequency.months().unwrap_or(3) as f64;
        let mut pay_date = dates_first_payment_date.max(as_of);

        // Simulate period-by-period
        while pay_date <= dates_legal_maturity
            && pool_outstanding.amount() > super::constants::POOL_BALANCE_CLEANUP_THRESHOLD
        {
            // Cache seasoning calculation for this period
            let seasoning_months = {
                let m = (pay_date.year() - dates_closing_date.year()) * 12
                    + (pay_date.month() as i32 - dates_closing_date.month() as i32);
                m.max(0) as u32
            };

            // Step 1: Calculate pool collections
            let interest_collections = self.calculate_period_interest_collections(
                pay_date,
                months_per_period,
                context,
            )?;

            // Step 2: Apply prepayments and defaults
            let (prepay_amt, default_amt, recovery_amt) = 
                self.calculate_period_prepayments_and_defaults(
                    pay_date,
                    seasoning_months,
                    pool_outstanding,
                    months_per_period,
                )?;

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
                    let coupon_rate = tranche
                        .coupon
                        .current_rate_with_index(pay_date, context);
                    let interest_portion = Money::new(
                        tranche_balances[&tranche_id].amount()
                            * coupon_rate
                            * (months_per_period / 12.0),
                        base_ccy,
                    );

                    update_tranche_balance(
                        &mut tranche_balances,
                        &tranche_id,
                        *payment,
                        interest_portion,
                    )?;
                }
            }

            // Step 5: Update pool balance
            pool_outstanding = pool_outstanding
                .checked_sub(prepay_amt)?
                .checked_sub(default_amt)?;

            // Advance to next period
            pay_date = add_months(
                pay_date,
                dates_payment_frequency.months().unwrap_or(3) as i32,
            );
        }

        // Aggregate all tranche cashflows into single schedule
        // For now, sum across all tranches; in production would track separately
        let mut all_flows: DatedFlows = Vec::new();
        let mut flow_map: HashMap<Date, Money> = HashMap::new();

        for (_tranche_id, flows) in tranche_cashflow_map {
            for (date, amount) in flows {
                flow_map
                    .entry(date)
                    .and_modify(|existing| {
                        *existing = existing.checked_add(amount).unwrap_or(*existing)
                    })
                    .or_insert(amount);
            }
        }

        for (date, amount) in flow_map {
            all_flows.push((date, amount));
        }
        all_flows.sort_by_key(|(d, _)| *d);

        Ok(all_flows)
    }

    /// Generate cashflows for a specific tranche after waterfall allocation
    ///
    /// This method runs the full waterfall simulation but returns only the
    /// cashflows allocated to the specified tranche, properly separating
    /// interest and principal components.
    fn generate_specific_tranche_cashflows(
        &self,
        tranche_id: &str,
        _context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<super::tranche_valuation::TrancheCashflowResult> {
        let pool = self.pool();
        let tranches = self.tranches();
        let base_ccy = pool.base_currency();
        let mut pool_outstanding = pool.total_balance();

        // Verify tranche exists
        let target_tranche = tranches
            .tranches
            .iter()
            .find(|t| t.id.as_str() == tranche_id)
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: format!("tranche:{}", tranche_id),
                })
            })?;

        if pool_outstanding.amount() <= 0.0 {
            return Ok(super::tranche_valuation::TrancheCashflowResult {
                tranche_id: tranche_id.to_string(),
                cashflows: Vec::new(),
                detailed_flows: Vec::new(),
                interest_flows: Vec::new(),
                principal_flows: Vec::new(),
                pik_flows: Vec::new(),
                final_balance: target_tranche.current_balance,
                total_interest: Money::new(0.0, base_ccy),
                total_principal: Money::new(0.0, base_ccy),
                total_pik: Money::new(0.0, base_ccy),
            });
        }

        // Get date configurations
        let dates_closing_date = self.closing_date();
        let dates_first_payment_date = self.first_payment_date();
        let dates_legal_maturity = self.legal_maturity();
        let dates_payment_frequency = self.payment_frequency();

        // Track tranche balances over time
        let mut tranche_balances: HashMap<String, Money> = tranches
            .tranches
            .iter()
            .map(|t| (t.id.to_string(), t.current_balance))
            .collect();

        // Store cashflows for the target tranche
        let mut target_cashflows: Vec<(Date, Money)> = Vec::new();
        let mut target_interest_flows: Vec<(Date, Money)> = Vec::new();
        let mut target_principal_flows: Vec<(Date, Money)> = Vec::new();
        let mut total_interest = Money::new(0.0, base_ccy);
        let mut total_principal = Money::new(0.0, base_ccy);

        // Initialize waterfall engine with instrument-specific rules
        let mut waterfall_engine = self.create_waterfall_engine();

        // Initialize coverage tests
        let mut _coverage_tests = CoverageTests::new();

        let months_per_period = dates_payment_frequency.months().unwrap_or(3) as f64;
        let mut pay_date = dates_first_payment_date.max(as_of);

        // Simulate period-by-period
        while pay_date <= dates_legal_maturity
            && pool_outstanding.amount() > super::constants::POOL_BALANCE_CLEANUP_THRESHOLD
        {
            let seasoning_months = {
                let m = (pay_date.year() - dates_closing_date.year()) * 12
                    + (pay_date.month() as i32 - dates_closing_date.month() as i32);
                m.max(0) as u32
            };

            // Step 1: Calculate pool collections
            let interest_collections = self.calculate_period_interest_collections(
                pay_date,
                months_per_period,
                _context,
            )?;

            // Step 2: Apply prepayments and defaults
            let (prepay_amt, default_amt, recovery_amt) = 
                self.calculate_period_prepayments_and_defaults(
                    pay_date,
                    seasoning_months,
                    pool_outstanding,
                    months_per_period,
                )?;

            // Total principal available
            let scheduled_prin = Money::new(0.0, base_ccy);
            let total_principal_available = scheduled_prin
                .checked_add(prepay_amt)?
                .checked_add(recovery_amt)?;

            // Total cash available for distribution
            let total_cash = interest_collections.checked_add(total_principal_available)?;

            // Step 3: Run waterfall to distribute cash to tranches
            let waterfall_result = waterfall_engine.apply_waterfall(
                total_cash,
                pay_date,
                tranches,
                pool_outstanding,
            )?;

            // Step 4: Record cashflows for the target tranche only
            if let Some(payment) = waterfall_result
                .distributions
                .get(&PaymentRecipient::Tranche(tranche_id.to_string()))
            {
                if payment.amount() > 0.0 {
                    // Calculate interest portion based on current balance
                    let current_balance = tranche_balances
                        .get(tranche_id)
                        .copied()
                        .unwrap_or(Money::new(0.0, base_ccy));
                    
                    let coupon_rate = target_tranche
                        .coupon
                        .current_rate_with_index(pay_date, _context);
                    let interest_portion = Money::new(
                        current_balance.amount() * coupon_rate * (months_per_period / 12.0),
                        base_ccy,
                    );
                    
                    let principal_payment = payment
                        .checked_sub(interest_portion)
                        .unwrap_or(Money::new(0.0, base_ccy));

                    // Record flows
                    target_cashflows.push((pay_date, *payment));
                    
                    if interest_portion.amount() > 0.0 {
                        target_interest_flows.push((pay_date, interest_portion));
                        total_interest = total_interest.checked_add(interest_portion)?;
                    }
                    
                    if principal_payment.amount() > 0.0 {
                        target_principal_flows.push((pay_date, principal_payment));
                        total_principal = total_principal.checked_add(principal_payment)?;
                    }

                    // Update tranche balance
                    update_tranche_balance(
                        &mut tranche_balances,
                        tranche_id,
                        *payment,
                        interest_portion,
                    )?;
                }
            }

            // Update all tranche balances for accurate interest calculations
            for tranche in &tranches.tranches {
                let tid = tranche.id.to_string();
                if tid == tranche_id {
                    continue; // Already handled
                }
                
                if let Some(payment) = waterfall_result
                    .distributions
                    .get(&PaymentRecipient::Tranche(tid.clone()))
                {
                    let coupon_rate = tranche
                        .coupon
                        .current_rate_with_index(pay_date, _context);
                    let interest_portion = Money::new(
                        tranche_balances[&tid].amount() * coupon_rate * (months_per_period / 12.0),
                        base_ccy,
                    );
                    update_tranche_balance(
                        &mut tranche_balances,
                        &tid,
                        *payment,
                        interest_portion,
                    )?;
                }
            }

            // Step 5: Update pool balance
            pool_outstanding = pool_outstanding
                .checked_sub(prepay_amt)?
                .checked_sub(default_amt)?;

            // Advance to next period
            pay_date = add_months(
                pay_date,
                dates_payment_frequency.months().unwrap_or(3) as i32,
            );
        }

        let final_balance = tranche_balances
            .get(tranche_id)
            .copied()
            .unwrap_or(Money::new(0.0, base_ccy));

        // Create detailed cashflows using CFKind classification
        let mut detailed_flows = Vec::new();
        let total_pik = Money::new(0.0, base_ccy);

        // Add interest flows
        for (date, amount) in &target_interest_flows {
            if let Ok(cf) = finstack_core::cashflow::CashFlow::fixed_cf(*date, *amount) {
                detailed_flows.push(cf);
            }
        }

        // Add principal flows  
        for (date, amount) in &target_principal_flows {
            if let Ok(cf) = finstack_core::cashflow::CashFlow::amort_cf(*date, *amount) {
                detailed_flows.push(cf);
            }
        }

        Ok(super::tranche_valuation::TrancheCashflowResult {
            tranche_id: tranche_id.to_string(),
            cashflows: target_cashflows,
            detailed_flows,
            interest_flows: target_interest_flows,
            principal_flows: target_principal_flows,
            pik_flows: Vec::new(), // PIK flows to be added when Z-bond support is implemented
            final_balance,
            total_interest,
            total_principal,
            total_pik,
        })
    }
}

#[cfg(test)]
mod tests {
    // Tests would go here
}
