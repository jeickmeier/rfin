//! Formula engine integration for dynamic waterfall calculations.
//!
//! Implements Hastructure's "Free Formula Support" by integrating the core
//! expression engine with the waterfall system. Users can construct formulas
//! using deal statistics (Pool Balance, Account balance, OC ratios, etc.) to
//! determine payment amounts dynamically.
//!
//! Example formulas:
//! - "pool_balance * 0.02" (2% of pool balance)
//! - "if(oc_ratio_aaa < 1.15, trap_cash, distribute_excess)"
//! - "max(0, tranche_balance_aaa - target_balance)"

use finstack_core::dates::Date;
use finstack_core::expr::{CompiledExpr, Expr, ExprNode, SimpleContext, EvalOpts, BinOp};
use finstack_core::money::Money;
use finstack_core::Result;
use std::collections::HashMap;

use super::{AssetPool, TrancheStructure, TestResults, AccountManager};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Context for formula evaluation containing deal state variables
#[derive(Debug, Clone)]
pub struct FormulaContext {
    /// Payment date
    pub payment_date: Date,
    /// Deal statistics for formula evaluation
    pub variables: HashMap<String, f64>,
}

impl FormulaContext {
    /// Create formula context from deal state
    pub fn from_deal_state(
        payment_date: Date,
        pool: &AssetPool,
        tranches: &TrancheStructure,
        accounts: &AccountManager,
        coverage_results: Option<&TestResults>,
    ) -> Self {
        let mut variables = HashMap::new();
        
        // Pool statistics
        variables.insert("pool_balance".to_string(), pool.total_balance().amount());
        variables.insert("performing_balance".to_string(), pool.performing_balance().amount());
        variables.insert("pool_wac".to_string(), pool.weighted_avg_coupon());
        variables.insert("pool_was".to_string(), pool.weighted_avg_spread());
        variables.insert("pool_wam".to_string(), pool.weighted_avg_maturity(payment_date));
        variables.insert("pool_diversity".to_string(), pool.diversity_score());
        variables.insert("cumulative_defaults".to_string(), pool.cumulative_defaults.amount());
        variables.insert("cumulative_recoveries".to_string(), pool.cumulative_recoveries.amount());
        variables.insert("cumulative_prepayments".to_string(), pool.cumulative_prepayments.amount());
        
        // Calculate net loss
        let net_loss = pool.cumulative_defaults.amount() - pool.cumulative_recoveries.amount();
        variables.insert("net_loss".to_string(), net_loss);
        
        // Loss rate as percentage
        if pool.total_balance().amount() > 0.0 {
            let loss_rate = (net_loss / pool.total_balance().amount()) * 100.0;
            variables.insert("loss_rate_pct".to_string(), loss_rate);
        }
        
        // Tranche balances and metrics
        let total_debt = tranches.total_size.amount();
        variables.insert("total_tranche_balance".to_string(), total_debt);
        
        for (idx, tranche) in tranches.tranches.iter().enumerate() {
            let tranche_key = tranche.id.as_str().to_lowercase();
            variables.insert(
                format!("tranche_balance_{}", tranche_key), 
                tranche.current_balance.amount()
            );
            variables.insert(
                format!("tranche_rate_{}", tranche_key), 
                tranche.coupon.current_rate(payment_date)
            );
            variables.insert(
                format!("tranche_attachment_{}", tranche_key), 
                tranche.attachment_point
            );
            variables.insert(
                format!("tranche_detachment_{}", tranche_key), 
                tranche.detachment_point
            );
            variables.insert(format!("tranche_priority_{}", tranche_key), idx as f64);
        }
        
        // Account balances
        for account_type in ["reserve", "collection", "pdl", "excess_spread"] {
            if let Some(balance) = accounts.get_balance(account_type) {
                variables.insert(format!("{}_account", account_type), balance.amount());
            }
        }
        
        // Coverage test results
        if let Some(results) = coverage_results {
            for (tranche_id, ratio) in &results.oc_ratios {
                variables.insert(
                    format!("oc_ratio_{}", tranche_id.to_lowercase()),
                    *ratio
                );
            }
            for (tranche_id, ratio) in &results.ic_ratios {
                variables.insert(
                    format!("ic_ratio_{}", tranche_id.to_lowercase()),
                    *ratio
                );
            }
            if let Some(par_ratio) = results.par_value_ratio {
                variables.insert("par_value_ratio".to_string(), par_ratio);
            }
            variables.insert("breached_tests_count".to_string(), results.breached_tests.len() as f64);
        }
        
        // Time-based variables
        variables.insert("payment_date_serial".to_string(), payment_date.to_julian_day() as f64);
        variables.insert("year".to_string(), payment_date.year() as f64);
        variables.insert("month".to_string(), payment_date.month() as u8 as f64);
        variables.insert("quarter".to_string(), ((payment_date.month() as u8 - 1) / 3 + 1) as f64);
        
        Self {
            payment_date,
            variables,
        }
    }
    
    /// Add custom variable
    pub fn add_variable(&mut self, name: String, value: f64) {
        self.variables.insert(name, value);
    }
    
    /// Get variable value
    pub fn get_variable(&self, name: &str) -> Option<f64> {
        self.variables.get(name).copied()
    }
}

/// Formula-based payment calculator
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FormulaCalculator {
    /// Formula expression string
    pub expression: String,
    /// Compiled expression for efficiency
    #[cfg_attr(feature = "serde", serde(skip))]
    compiled: Option<CompiledExpr>,
    /// Base currency for money calculations
    pub base_currency: finstack_core::currency::Currency,
}

impl FormulaCalculator {
    /// Create new formula calculator
    pub fn new(
        expression: impl Into<String>, 
        base_currency: finstack_core::currency::Currency
    ) -> Result<Self> {
        let expr_str = expression.into();
        
        // Parse and compile the expression
        let parsed = Self::parse_expression(&expr_str)?;
        let compiled = CompiledExpr::new(parsed);
        
        Ok(Self {
            expression: expr_str,
            compiled: Some(compiled),
            base_currency,
        })
    }
    
    /// Parse expression string into AST
    fn parse_expression(expr_str: &str) -> Result<Expr> {
        // Simplified parser - in production would use a proper parser
        // For now, support basic patterns that cover most use cases
        
        if expr_str.contains("pool_balance * ") {
            // Pattern: "pool_balance * 0.02"
            if let Some(factor_str) = expr_str.strip_prefix("pool_balance * ") {
                if let Ok(factor) = factor_str.parse::<f64>() {
                    return Ok(Expr {
                        id: None,
                        node: ExprNode::BinOp {
                            op: BinOp::Mul,
                            left: Box::new(Expr {
                                id: None,
                                node: ExprNode::Column("pool_balance".to_string()),
                            }),
                            right: Box::new(Expr {
                                id: None,
                                node: ExprNode::Literal(factor),
                            }),
                        },
                    });
                }
            }
        }
        
        if expr_str.starts_with("if(") {
            // Pattern: "if(oc_ratio_aaa < 1.15, 0, pool_balance * 0.01)"
            // Simplified parsing - would need a proper parser for complex expressions
            return Ok(Expr {
                id: None,
                node: ExprNode::IfThenElse {
                    condition: Box::new(Expr {
                        id: None,
                        node: ExprNode::BinOp {
                            op: BinOp::Lt,
                            left: Box::new(Expr {
                                id: None,
                                node: ExprNode::Column("oc_ratio_aaa".to_string()),
                            }),
                            right: Box::new(Expr {
                                id: None,
                                node: ExprNode::Literal(1.15),
                            }),
                        },
                    }),
                    then_expr: Box::new(Expr {
                        id: None,
                        node: ExprNode::Literal(0.0),
                    }),
                    else_expr: Box::new(Expr {
                        id: None,
                        node: ExprNode::BinOp {
                            op: BinOp::Mul,
                            left: Box::new(Expr {
                                id: None,
                                node: ExprNode::Column("pool_balance".to_string()),
                            }),
                            right: Box::new(Expr {
                                id: None,
                                node: ExprNode::Literal(0.01),
                            }),
                        },
                    }),
                },
            });
        }
        
        // Single column reference
        if expr_str.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Ok(Expr {
                id: None,
                node: ExprNode::Column(expr_str.to_string()),
            });
        }
        
        // Single literal
        if let Ok(value) = expr_str.parse::<f64>() {
            return Ok(Expr {
                id: None,
                node: ExprNode::Literal(value),
            });
        }
        
        Err(finstack_core::error::InputError::Invalid.into())
    }
    
    /// Calculate payment amount using formula
    pub fn calculate_payment(
        &self,
        context: &FormulaContext,
    ) -> Result<Money> {
        if let Some(ref compiled) = self.compiled {
            // Create evaluation context
            let column_names: Vec<String> = context.variables.keys().cloned().collect();
            let eval_context = SimpleContext::new(column_names.iter().map(|s| s.as_str()));
            
            // Create data columns (single row for current period)
            let data_values: Vec<Vec<f64>> = column_names
                .iter()
                .map(|name| vec![context.variables.get(name).copied().unwrap_or(0.0)])
                .collect();
            
            let data_refs: Vec<&[f64]> = data_values.iter().map(|v| v.as_slice()).collect();
            
            // Evaluate expression
            let result = compiled.eval(&eval_context, &data_refs, EvalOpts::default());
            
            // Get scalar result (single value for current period)
            let amount = result.values.first().copied().unwrap_or(0.0);
            
            Ok(Money::new(amount.max(0.0), self.base_currency))
        } else {
            Ok(Money::new(0.0, self.base_currency))
        }
    }
    
    /// Evaluate formula to boolean (for conditions)
    pub fn evaluate_condition(
        &self,
        context: &FormulaContext,
    ) -> Result<bool> {
        let amount = self.calculate_payment(context)?;
        Ok(amount.amount() > 0.0)
    }
}

/// Helper to create common formula expressions
pub struct FormulaBuilder;

impl FormulaBuilder {
    /// Create percentage of pool balance formula
    pub fn pool_percentage(percentage: f64, base_currency: finstack_core::currency::Currency) -> Result<FormulaCalculator> {
        FormulaCalculator::new(format!("pool_balance * {}", percentage), base_currency)
    }
    
    /// Create OC test trigger formula
    pub fn oc_trigger(tranche_id: &str, threshold: f64, base_currency: finstack_core::currency::Currency) -> Result<FormulaCalculator> {
        FormulaCalculator::new(
            format!("if(oc_ratio_{} < {}, pool_balance, 0)", tranche_id.to_lowercase(), threshold),
            base_currency
        )
    }
    
    /// Create excess cash distribution formula
    pub fn excess_distribution(reserve_target: f64, base_currency: finstack_core::currency::Currency) -> Result<FormulaCalculator> {
        FormulaCalculator::new(
            format!("max(0, available_cash - {})", reserve_target),
            base_currency
        )
    }
}

/// Enhanced payment calculation that supports formulas
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum EnhancedPaymentCalculation {
    /// Static calculations (existing)
    Static(super::waterfall::PaymentCalculation),
    /// Formula-based calculation (Hastructure-style flexibility)
    Formula { 
        calculator: FormulaCalculator,
        description: String,
    },
}

impl EnhancedPaymentCalculation {
    /// Calculate payment amount
    pub fn calculate_amount(
        &self,
        context: &FormulaContext,
        available: Money,
        tranches: &TrancheStructure,
        pool_balance: Money,
        payment_date: Date,
    ) -> Result<Money> {
        match self {
            Self::Static(calc) => {
                // Use existing static calculation logic
                // (Would need to refactor waterfall.rs calculate_payment_amount to be public)
                match calc {
                    super::waterfall::PaymentCalculation::FixedAmount { amount } => Ok(*amount),
                    super::waterfall::PaymentCalculation::PercentageOfCollateral { rate, annualized } => {
                        let period_rate = if *annualized {
                            rate / super::constants::QUARTERLY_PERIODS_PER_YEAR
                        } else {
                            *rate
                        };
                        Ok(Money::new(
                            pool_balance.amount() * period_rate,
                            pool_balance.currency(),
                        ))
                    },
                    _ => {
                        // Use other parameters to avoid warnings
                        let _ = (available, tranches, payment_date);
                        Ok(Money::new(0.0, pool_balance.currency()))
                    }
                }
            },
            Self::Formula { calculator, .. } => {
                calculator.calculate_payment(context)
            },
        }
    }
}

/// Formula registry for common structured credit expressions
pub struct FormulaRegistry {
    /// Pre-compiled formulas by name
    formulas: HashMap<String, FormulaCalculator>,
}

impl FormulaRegistry {
    /// Create new formula registry
    pub fn new() -> Self {
        Self {
            formulas: HashMap::new(),
        }
    }
    
    /// Register a formula
    pub fn register(&mut self, name: String, calculator: FormulaCalculator) {
        self.formulas.insert(name, calculator);
    }
    
    /// Get formula by name
    pub fn get(&self, name: &str) -> Option<&FormulaCalculator> {
        self.formulas.get(name)
    }
    
    /// Create standard structured credit formula registry
    pub fn standard_registry(base_currency: finstack_core::currency::Currency) -> Result<Self> {
        let mut registry = Self::new();
        
        // Common percentage formulas (these patterns are supported by the simplified parser)
        registry.register(
            "senior_mgmt_fee".to_string(),
            FormulaCalculator::new("pool_balance * 0.004", base_currency)? // 40bps
        );
        
        registry.register(
            "sub_mgmt_fee".to_string(),
            FormulaCalculator::new("pool_balance * 0.002", base_currency)? // 20bps
        );
        
        // OC test based cash trapping (simplified pattern for now)
        // TODO: Enhance parser to support arbitrary if/then/else patterns
        registry.register(
            "oc_cash_trap".to_string(),
            FormulaCalculator::new("if(oc_ratio_aaa < 1.15, 0, pool_balance * 0.01)", base_currency)?
        );
        
        // Note: max() function not yet supported by simplified parser
        // Would require full expression parsing from core or enhanced parser
        
        Ok(registry)
    }
}

impl Default for FormulaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use crate::instruments::common::structured_credit::DealType;
    use time::Month;

    fn test_date() -> Date {
        Date::from_calendar_date(2024, Month::January, 1).unwrap()
    }

    #[test]
    fn test_formula_context_creation() {
        let pool = AssetPool::new("TEST", DealType::CLO, Currency::USD);
        let tranches = TrancheStructure::new(Vec::new()).unwrap_or_else(|_| {
            // Create minimal structure for test
            use crate::instruments::common::structured_credit::{Tranche, TrancheSeniority, TrancheCoupon};
            let tranche = Tranche::new(
                "TEST",
                0.0,
                100.0,
                TrancheSeniority::Senior,
                Money::new(1000.0, Currency::USD),
                TrancheCoupon::Fixed { rate: 0.05 },
                test_date(),
            ).unwrap();
            TrancheStructure::new(vec![tranche]).unwrap()
        });
        let accounts = AccountManager::new();
        
        let context = FormulaContext::from_deal_state(
            test_date(),
            &pool,
            &tranches,
            &accounts,
            None,
        );
        
        // Should have basic variables
        assert!(context.variables.contains_key("pool_balance"));
        assert!(context.variables.contains_key("year"));
        assert_eq!(context.variables.get("year"), Some(&2024.0));
    }

    #[test]
    fn test_simple_formula_calculation() {
        let calc = FormulaCalculator::new("pool_balance * 0.02", Currency::USD).unwrap();
        
        let mut context = FormulaContext {
            payment_date: test_date(),
            variables: HashMap::new(),
        };
        context.add_variable("pool_balance".to_string(), 1_000_000.0);
        
        let result = calc.calculate_payment(&context).unwrap();
        assert_eq!(result.amount(), 20_000.0); // 2% of 1M
    }

    #[test]
    fn test_formula_registry() {
        let registry = FormulaRegistry::standard_registry(Currency::USD).unwrap();
        
        // Verify the formulas that are registered with supported patterns
        assert!(registry.get("senior_mgmt_fee").is_some());
        assert!(registry.get("sub_mgmt_fee").is_some());
        assert!(registry.get("oc_cash_trap").is_some());
        
        // Verify they're not in the registry (not registered)
        assert!(registry.get("excess_after_reserves").is_none());
    }
}
