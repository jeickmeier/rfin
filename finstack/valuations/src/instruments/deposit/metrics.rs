//! Deposit-specific metric calculators.

use crate::metrics::{MetricCalculator, MetricContext};
use super::Deposit;
use finstack_core::F;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

/// Calculates year fraction for deposits.
pub struct YearFractionCalculator;

impl MetricCalculator for YearFractionCalculator {
    fn id(&self) -> &str {
        "yf"
    }
    
    fn description(&self) -> &str {
        "Year fraction of the deposit period"
    }
    
    fn is_applicable(&self, instrument_type: &str) -> bool {
        instrument_type == "Deposit"
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit = context.instrument_as::<Deposit>()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        
        Ok(DiscountCurve::year_fraction(deposit.start, deposit.end, deposit.day_count))
    }
}

/// Calculates discount factor at start date for deposits.
pub struct DfStartCalculator;

impl MetricCalculator for DfStartCalculator {
    fn id(&self) -> &str {
        "df_start"
    }
    
    fn description(&self) -> &str {
        "Discount factor at start date"
    }
    
    fn is_applicable(&self, instrument_type: &str) -> bool {
        instrument_type == "Deposit"
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit = context.instrument_as::<Deposit>()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        
        let disc = context.curves.discount(deposit.disc_id)?;
        let base = disc.base_date();
        
        Ok(DiscountCurve::df_on(&*disc, base, deposit.start, deposit.day_count))
    }
}

/// Calculates discount factor at end date for deposits.
pub struct DfEndCalculator;

impl MetricCalculator for DfEndCalculator {
    fn id(&self) -> &str {
        "df_end"
    }
    
    fn description(&self) -> &str {
        "Discount factor at end date"
    }
    
    fn is_applicable(&self, instrument_type: &str) -> bool {
        instrument_type == "Deposit"
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit = context.instrument_as::<Deposit>()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        
        let disc = context.curves.discount(deposit.disc_id)?;
        let base = disc.base_date();
        
        Ok(DiscountCurve::df_on(&*disc, base, deposit.end, deposit.day_count))
    }
}

/// Calculates par rate for deposits.
pub struct DepositParRateCalculator;

impl MetricCalculator for DepositParRateCalculator {
    fn id(&self) -> &str {
        "deposit_par_rate"
    }
    
    fn description(&self) -> &str {
        "Deposit par rate (simple rate that makes NPV zero)"
    }
    
    fn is_applicable(&self, instrument_type: &str) -> bool {
        instrument_type == "Deposit"
    }
    
    fn dependencies(&self) -> Vec<&str> {
        vec!["df_start", "df_end", "yf"]
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let df_s = context.computed.get("df_start").copied().unwrap_or(1.0);
        let df_e = context.computed.get("df_end").copied().unwrap_or(1.0);
        let yf = context.computed.get("yf").copied().unwrap_or(0.0);
        
        if yf == 0.0 {
            return Ok(0.0);
        }
        
        Ok((df_s / df_e - 1.0) / yf)
    }
}

/// Calculates implied DF(end) from quoted rate.
pub struct DfEndFromQuoteCalculator;

impl MetricCalculator for DfEndFromQuoteCalculator {
    fn id(&self) -> &str {
        "df_end_from_quote"
    }
    
    fn description(&self) -> &str {
        "Implied discount factor at end date from quoted rate"
    }
    
    fn is_applicable(&self, instrument_type: &str) -> bool {
        instrument_type == "Deposit"
    }
    
    fn dependencies(&self) -> Vec<&str> {
        vec!["df_start", "yf"]
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit = context.instrument_as::<Deposit>()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        
        let r = deposit.quote_rate
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        let df_s = context.computed.get("df_start").copied().unwrap_or(1.0);
        let yf = context.computed.get("yf").copied().unwrap_or(0.0);
        
        Ok(df_s / (1.0 + r * yf))
    }
}

/// Calculates quoted rate for deposits.
pub struct QuoteRateCalculator;

impl MetricCalculator for QuoteRateCalculator {
    fn id(&self) -> &str {
        "quote_rate"
    }
    
    fn description(&self) -> &str {
        "Quoted simple rate"
    }
    
    fn is_applicable(&self, instrument_type: &str) -> bool {
        instrument_type == "Deposit"
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit = context.instrument_as::<Deposit>()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        
        deposit.quote_rate
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))
    }
}

/// Register all deposit metrics to a registry.
pub fn register_deposit_metrics(registry: &mut crate::metrics::MetricRegistry) {
    use std::sync::Arc;
    
    registry
        .register(Arc::new(YearFractionCalculator))
        .register(Arc::new(DfStartCalculator))
        .register(Arc::new(DfEndCalculator))
        .register(Arc::new(DepositParRateCalculator))
        .register(Arc::new(DfEndFromQuoteCalculator))
        .register(Arc::new(QuoteRateCalculator));
}
