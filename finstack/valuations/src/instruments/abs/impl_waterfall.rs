//! ABS waterfall implementation

use super::types::Abs;
use crate::cashflow::traits::DatedFlows;
use crate::instruments::common::structured_credit::{
    EnhancedCoverageTests, PaymentRecipient, WaterfallEngine, PaymentRule, 
    PaymentCalculation,
};
use finstack_core::dates::{Date, utils::add_months};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use std::collections::HashMap;

impl Abs {
    /// Generate complete tranche-specific cashflows using waterfall engine
    pub(super) fn generate_tranche_cashflows_abs(
        &self,
        _context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        let base_ccy = self.pool.base_currency();
        let mut pool_outstanding = self.pool.total_balance();
        
        if pool_outstanding.amount() <= 0.0 {
            return Ok(Vec::new());
        }

        let mut tranche_balances: HashMap<String, Money> = self.tranches
            .tranches
            .iter()
            .map(|t| (t.id.to_string(), t.current_balance))
            .collect();

        let mut tranche_cashflow_map: HashMap<String, Vec<(Date, Money)>> = HashMap::new();
        for tranche in &self.tranches.tranches {
            tranche_cashflow_map.insert(tranche.id.to_string(), Vec::new());
        }

        let mut waterfall_engine = self.create_abs_waterfall_engine();
        
        let mut _coverage_tests = EnhancedCoverageTests {
            oc_tests: HashMap::new(),
            ic_tests: HashMap::new(),
            par_value_test: None,
            diversity_test: None,
            warf_test: None,
            was_test: None,
        };

        let months_per_period = self.payment_frequency.months().unwrap_or(1) as f64;
        let mut pay_date = self.first_payment_date.max(as_of);

        while pay_date <= self.legal_maturity && pool_outstanding.amount() > 100.0 {
            let seasoning_months = {
                let m = (pay_date.year() - self.closing_date.year()) * 12
                    + (pay_date.month() as i32 - self.closing_date.month() as i32);
                m.max(0) as u32
            };

            let wac = self.pool.weighted_avg_coupon();
            let period_rate = wac * (months_per_period / 12.0);
            let interest_collections = Money::new(pool_outstanding.amount() * period_rate, base_ccy);

            // Apply ABS-specific prepayment and default rates
            let smm = if let Some(abs_speed) = self.abs_speed {
                abs_speed
            } else {
                self.premium_smm(pay_date, seasoning_months)
            };

            let mdr = if let Some(cdr) = self.cdr_annual {
                crate::instruments::common::structured_credit::cdr_to_mdr(cdr)
            } else {
                self.premium_mdr(pay_date, seasoning_months)
            };
            
            let prepay_amt = Money::new(pool_outstanding.amount() * smm, base_ccy);
            let default_amt = Money::new(pool_outstanding.amount() * mdr, base_ccy);
            
            let recovery_rate = self.recovery_model.recovery_rate(
                pay_date,
                6,
                None,
                default_amt,
                &crate::instruments::common::structured_credit::MarketFactors::default(),
            );
            let recovery_amt = Money::new(default_amt.amount() * recovery_rate, base_ccy);

            let scheduled_prin = Money::new(0.0, base_ccy);
            let total_principal = scheduled_prin
                .checked_add(prepay_amt)?
                .checked_add(recovery_amt)?;

            let total_cash = interest_collections.checked_add(total_principal)?;

            let waterfall_result = waterfall_engine.apply_waterfall(
                total_cash,
                pay_date,
                &self.tranches,
                pool_outstanding,
            )?;

            for tranche in &self.tranches.tranches {
                let tranche_id = tranche.id.to_string();
                
                if let Some(payment) = waterfall_result.distributions.get(&PaymentRecipient::Tranche(tranche_id.clone())) {
                    if payment.amount() > 0.0 {
                        tranche_cashflow_map
                            .get_mut(&tranche_id)
                            .unwrap()
                            .push((pay_date, *payment));
                    }
                    
                    let interest_portion = Money::new(
                        tranche_balances[&tranche_id].amount() * tranche.coupon.current_rate(pay_date) * (months_per_period / 12.0),
                        base_ccy
                    );
                    let principal_payment = payment.checked_sub(interest_portion).unwrap_or(Money::new(0.0, base_ccy));
                    
                    if let Some(current) = tranche_balances.get_mut(&tranche_id) {
                        *current = current.checked_sub(principal_payment).unwrap_or(*current);
                    }
                }
            }

            pool_outstanding = pool_outstanding
                .checked_sub(prepay_amt)?
                .checked_sub(default_amt)?;

            pay_date = add_months(pay_date, self.payment_frequency.months().unwrap_or(1) as i32);
        }

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

    /// Create waterfall engine for ABS
    pub(super) fn create_abs_waterfall_engine(&self) -> WaterfallEngine {
        let mut engine = WaterfallEngine::new(self.pool.base_currency());
        
        engine.payment_rules.push(PaymentRule {
            id: "servicing_fees".to_string(),
            priority: 1,
            recipient: PaymentRecipient::ServiceProvider("Servicer".to_string()),
            calculation: PaymentCalculation::PercentageOfCollateral {
                rate: 0.005, // 50 bps servicing
                annual: true,
            },
            conditions: vec![],
            divertible: false,
        });
        
        let mut sorted_tranches = self.tranches.tranches.clone();
        sorted_tranches.sort_by_key(|t| t.payment_priority);
        
        let mut priority = 2;
        for tranche in &sorted_tranches {
            engine.payment_rules.push(PaymentRule {
                id: format!("{}_interest", tranche.id.as_str()),
                priority,
                recipient: PaymentRecipient::Tranche(tranche.id.to_string()),
                calculation: PaymentCalculation::TrancheInterest {
                    tranche_id: tranche.id.to_string(),
                },
                conditions: vec![],
                divertible: false,
            });
            priority += 1;
        }
        
        for tranche in &sorted_tranches {
            engine.payment_rules.push(PaymentRule {
                id: format!("{}_principal", tranche.id.as_str()),
                priority,
                recipient: PaymentRecipient::Tranche(tranche.id.to_string()),
                calculation: PaymentCalculation::TranchePrincipal {
                    tranche_id: tranche.id.to_string(),
                    target_balance: Some(Money::new(0.0, self.pool.base_currency())),
                },
                conditions: vec![],
                divertible: true,
            });
            priority += 1;
        }
        
        engine
    }
}

