//! Deposit-specific metric calculators.
//! 
//! Provides metric calculators for deposit instruments including year fractions,
//! discount factors, par rates, and quoted rates. These metrics are essential
//! for valuing simple interest-bearing deposits and understanding their pricing.

use crate::instruments::Instrument;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::F;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

/// Calculates year fraction for deposits.
/// 
/// Computes the time period between start and end dates using the deposit's
/// day count convention. This is fundamental for all other deposit calculations.
/// 
/// # Example
/// ```rust
/// use finstack_valuations::instruments::fixed_income::deposit::metrics::YearFractionCalculator;
/// use finstack_valuations::metrics::traits::MetricCalculator;
/// use finstack_valuations::instruments::fixed_income::deposit::Deposit;
/// use finstack_valuations::instruments::Instrument;
/// use finstack_core::dates::{Date, DayCount};
/// use finstack_core::money::Money;
/// use finstack_core::currency::Currency;
/// use time::Month;
/// 
/// let deposit = Deposit {
///     id: "DEP001".to_string(),
///     notional: Money::new(1000.0, Currency::USD),
///     start: Date::from_calendar_date(2025, Month::January, 1).unwrap(),
///     end: Date::from_calendar_date(2025, Month::July, 1).unwrap(),
///     day_count: DayCount::Act365F,
///     disc_id: "USD-OIS",
///     quote_rate: Some(0.05),
///     attributes: Default::default(),
/// };
/// 
/// let calculator = YearFractionCalculator;
/// // Note: Would need proper context to test calculation
/// ```
pub struct YearFractionCalculator;

impl MetricCalculator for YearFractionCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit = match &*context.instrument {
            Instrument::Deposit(deposit) => deposit,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        Ok(DiscountCurve::year_fraction(deposit.start, deposit.end, deposit.day_count))
    }
}

/// Calculates discount factor at start date for deposits.
/// 
/// Computes the present value of $1 received at the deposit start date,
/// using the deposit's discount curve and day count convention.
/// 
/// # Example
/// ```rust
/// use finstack_valuations::instruments::fixed_income::deposit::metrics::DfStartCalculator;
/// use finstack_valuations::metrics::traits::MetricCalculator;
/// 
/// let calculator = DfStartCalculator;
/// // Note: Would need proper context with curves to test calculation
/// ```
pub struct DfStartCalculator;

impl MetricCalculator for DfStartCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit = match &*context.instrument {
            Instrument::Deposit(deposit) => deposit,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        let disc = context.curves.discount(deposit.disc_id)?;
        let base = disc.base_date();
        
        Ok(DiscountCurve::df_on(&*disc, base, deposit.start, deposit.day_count))
    }
}

/// Calculates discount factor at end date for deposits.
/// 
/// Computes the present value of $1 received at the deposit end date,
/// using the deposit's discount curve and day count convention.
/// 
/// # Example
/// ```rust
/// use finstack_valuations::instruments::fixed_income::deposit::metrics::DfEndCalculator;
/// use finstack_valuations::metrics::traits::MetricCalculator;
/// 
/// let calculator = DfEndCalculator;
/// // Note: Would need proper context with curves to test calculation
/// ```
pub struct DfEndCalculator;

impl MetricCalculator for DfEndCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit = match &*context.instrument {
            Instrument::Deposit(deposit) => deposit,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        let disc = context.curves.discount(deposit.disc_id)?;
        let base = disc.base_date();
        
        Ok(DiscountCurve::df_on(&*disc, base, deposit.end, deposit.day_count))
    }
}

/// Calculates par rate for deposits.
/// 
/// Computes the rate that makes the deposit worth par (face value) at inception.
/// Uses the formula: (DF(start) / DF(end) - 1) / year_fraction.
/// 
/// # Dependencies
/// Requires `DfStart`, `DfEnd`, and `Yf` metrics to be computed first.
/// 
/// # Example
/// ```rust
/// use finstack_valuations::instruments::fixed_income::deposit::metrics::DepositParRateCalculator;
/// use finstack_valuations::metrics::traits::MetricCalculator;
/// 
/// let calculator = DepositParRateCalculator;
/// // Note: Would need proper context with computed dependencies to test calculation
/// ```
pub struct DepositParRateCalculator;

impl MetricCalculator for DepositParRateCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::DfStart, MetricId::DfEnd, MetricId::Yf]
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let df_s = context.computed.get(&MetricId::DfStart).copied()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        let df_e = context.computed.get(&MetricId::DfEnd).copied()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        let yf = context.computed.get(&MetricId::Yf).copied()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        if yf == 0.0 {
            return Ok(0.0);
        }
        
        Ok((df_s / df_e - 1.0) / yf)
    }
}

/// Calculates implied DF(end) from quoted rate.
/// 
/// Computes the discount factor at the end date implied by the quoted rate,
/// using the formula: DF(start) / (1 + rate * year_fraction).
/// 
/// # Dependencies
/// Requires `DfStart` and `Yf` metrics to be computed first.
/// 
/// # Example
/// ```rust
/// use finstack_valuations::instruments::fixed_income::deposit::metrics::DfEndFromQuoteCalculator;
/// use finstack_valuations::metrics::traits::MetricCalculator;
/// 
/// let calculator = DfEndFromQuoteCalculator;
/// // Note: Would need proper context with computed dependencies to test calculation
/// ```
pub struct DfEndFromQuoteCalculator;

impl MetricCalculator for DfEndFromQuoteCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::DfStart, MetricId::Yf]
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit = match &*context.instrument {
            Instrument::Deposit(deposit) => deposit,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        let r = deposit.quote_rate
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        let df_s = context.computed.get(&MetricId::DfStart).copied()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        let yf = context.computed.get(&MetricId::Yf).copied()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        Ok(df_s / (1.0 + r * yf))
    }
}

/// Calculates quoted rate for deposits.
/// 
/// Returns the quoted rate from the deposit instrument. This is a simple
/// pass-through metric that extracts the rate from the instrument data.
/// 
/// # Example
/// ```rust
/// use finstack_valuations::instruments::fixed_income::deposit::metrics::QuoteRateCalculator;
/// use finstack_valuations::metrics::traits::MetricCalculator;
/// 
/// let calculator = QuoteRateCalculator;
/// // Note: Would need proper context with deposit data to test calculation
/// ```
pub struct QuoteRateCalculator;

impl MetricCalculator for QuoteRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit = match &*context.instrument {
            Instrument::Deposit(deposit) => deposit,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        deposit.quote_rate
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))
    }
}

/// Registers all deposit metrics to a registry.
/// 
/// This function adds all deposit-specific metrics to the provided metric
/// registry. Each metric is registered with the "Deposit" instrument type
/// to ensure proper applicability filtering.
/// 
/// # Arguments
/// * `registry` - Metric registry to add deposit metrics to
/// 
/// # Example
/// ```rust
/// use finstack_valuations::metrics::registry::MetricRegistry;
/// use finstack_valuations::instruments::fixed_income::deposit::metrics::register_deposit_metrics;
/// 
/// let mut registry = MetricRegistry::new();
/// register_deposit_metrics(&mut registry);
/// 
/// // Check that deposit metrics are registered
/// assert!(registry.has_metric(finstack_valuations::metrics::MetricId::Yf));
/// assert!(registry.has_metric(finstack_valuations::metrics::MetricId::DfStart));
/// assert!(registry.has_metric(finstack_valuations::metrics::MetricId::DfEnd));
/// ```
pub fn register_deposit_metrics(registry: &mut crate::metrics::MetricRegistry) {
    use std::sync::Arc;
    use crate::metrics::MetricId;
    
    registry
        .register_metric(MetricId::Yf, Arc::new(YearFractionCalculator), &["Deposit"])
        .register_metric(MetricId::DfStart, Arc::new(DfStartCalculator), &["Deposit"])
        .register_metric(MetricId::DfEnd, Arc::new(DfEndCalculator), &["Deposit"])
        .register_metric(MetricId::DepositParRate, Arc::new(DepositParRateCalculator), &["Deposit"])
        .register_metric(MetricId::DfEndFromQuote, Arc::new(DfEndFromQuoteCalculator), &["Deposit"])
        .register_metric(MetricId::QuoteRate, Arc::new(QuoteRateCalculator), &["Deposit"]);
}
