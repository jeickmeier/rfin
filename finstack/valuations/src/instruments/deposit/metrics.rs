//! Deposit-specific metric calculators.

use crate::instruments::Instrument;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::F;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

/// Calculates year fraction for deposits.
pub struct YearFractionCalculator;

impl MetricCalculator for YearFractionCalculator {
    fn id(&self) -> MetricId {
        MetricId::Yf
    }
    

    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit = match context.instrument() {
            Instrument::Deposit(deposit) => deposit,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        Ok(DiscountCurve::year_fraction(deposit.start, deposit.end, deposit.day_count))
    }
}

/// Calculates discount factor at start date for deposits.
pub struct DfStartCalculator;

impl MetricCalculator for DfStartCalculator {
    fn id(&self) -> MetricId {
        MetricId::DfStart
    }
    

    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit = match context.instrument() {
            Instrument::Deposit(deposit) => deposit,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        let disc = context.market_data.curves.discount(deposit.disc_id)?;
        let base = disc.base_date();
        
        Ok(DiscountCurve::df_on(&*disc, base, deposit.start, deposit.day_count))
    }
}

/// Calculates discount factor at end date for deposits.
pub struct DfEndCalculator;

impl MetricCalculator for DfEndCalculator {
    fn id(&self) -> MetricId {
        MetricId::DfEnd
    }
    

    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit = match context.instrument() {
            Instrument::Deposit(deposit) => deposit,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        let disc = context.market_data.curves.discount(deposit.disc_id)?;
        let base = disc.base_date();
        
        Ok(DiscountCurve::df_on(&*disc, base, deposit.end, deposit.day_count))
    }
}

/// Calculates par rate for deposits.
pub struct DepositParRateCalculator;

impl MetricCalculator for DepositParRateCalculator {
    fn id(&self) -> MetricId {
        MetricId::DepositParRate
    }
    

    
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::DfStart, MetricId::DfEnd, MetricId::Yf]
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let df_s = context.cache.computed.get(&MetricId::DfStart).copied().unwrap_or(1.0);
        let df_e = context.cache.computed.get(&MetricId::DfEnd).copied().unwrap_or(1.0);
        let yf = context.cache.computed.get(&MetricId::Yf).copied().unwrap_or(0.0);
        
        if yf == 0.0 {
            return Ok(0.0);
        }
        
        Ok((df_s / df_e - 1.0) / yf)
    }
}

/// Calculates implied DF(end) from quoted rate.
pub struct DfEndFromQuoteCalculator;

impl MetricCalculator for DfEndFromQuoteCalculator {
    fn id(&self) -> MetricId {
        MetricId::DfEndFromQuote
    }
    

    
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::DfStart, MetricId::Yf]
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit = match context.instrument() {
            Instrument::Deposit(deposit) => deposit,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        let r = deposit.quote_rate
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        let df_s = context.cache.computed.get(&MetricId::DfStart).copied().unwrap_or(1.0);
        let yf = context.cache.computed.get(&MetricId::Yf).copied().unwrap_or(0.0);
        
        Ok(df_s / (1.0 + r * yf))
    }
}

/// Calculates quoted rate for deposits.
pub struct QuoteRateCalculator;

impl MetricCalculator for QuoteRateCalculator {
    fn id(&self) -> MetricId {
        MetricId::QuoteRate
    }
    

    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit = match context.instrument() {
            Instrument::Deposit(deposit) => deposit,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        deposit.quote_rate
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))
    }
}

/// Register all deposit metrics to a registry.
pub fn register_deposit_metrics(registry: &mut crate::metrics::MetricRegistry) {
    use std::sync::Arc;
    
    let deposit_types = &["Deposit"];
    
    registry
        .register_for_types(Arc::new(YearFractionCalculator), deposit_types)
        .register_for_types(Arc::new(DfStartCalculator), deposit_types)
        .register_for_types(Arc::new(DfEndCalculator), deposit_types)
        .register_for_types(Arc::new(DepositParRateCalculator), deposit_types)
        .register_for_types(Arc::new(DfEndFromQuoteCalculator), deposit_types)
        .register_for_types(Arc::new(QuoteRateCalculator), deposit_types);
}
