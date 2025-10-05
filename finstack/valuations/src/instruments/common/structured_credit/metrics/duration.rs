//! Duration calculators for structured credit.

use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::DayCountCtx;
use finstack_core::Result;

/// Calculates Macaulay duration for structured credit.
///
/// Macaulay duration measures the weighted average time to receive cashflows,
/// where weights are the present values of each cashflow. This is the fundamental
/// measure of interest rate sensitivity.
///
/// # Formula
///
/// Macaulay Duration = Σ(PV_i × t_i) / Price
///
/// Where:
/// - PV_i = present value of cashflow i
/// - t_i = time in years to cashflow i
/// - Price = total present value (dirty price)
///
/// # Market Conventions
///
/// - **CLO (floating)**: Typically 0.1-0.3 years (very low IR duration)
/// - **ABS (fixed)**: Typically 2-4 years
/// - **RMBS (fixed)**: Typically 3-6 years (depends on prepayments)
/// - **CMBS (fixed)**: Typically 4-7 years
///
pub struct MacaulayDurationCalculator;

impl MetricCalculator for MacaulayDurationCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Get cashflows
        let flows = context.cashflows.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "context.cashflows".to_string(),
            })
        })?;
        
        // Get discount curve
        let disc_curve_id = context.discount_curve_id.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "discount_curve_id".to_string(),
            })
        })?;
        
        let disc = context.curves.get_discount_ref(disc_curve_id.as_str())?;
        
        // Use Act/365F for time calculation
        let day_count = finstack_core::dates::DayCount::Act365F;
        
        let mut weighted_pv = 0.0;
        let mut total_pv = 0.0;
        
        for (date, amount) in flows {
            if *date <= context.as_of {
                continue;
            }
            
            // Calculate time in years
            let years = day_count
                .year_fraction(context.as_of, *date, DayCountCtx::default())
                .unwrap_or(0.0);
            
            // Get discount factor
            let df = disc.df_on_date_curve(*date);
            
            // Calculate present value
            let pv = amount.amount() * df;
            
            // Accumulate weighted PV
            weighted_pv += pv * years;
            total_pv += pv;
        }
        
        // Calculate Macaulay duration
        if total_pv > 0.0 {
            Ok(weighted_pv / total_pv)
        } else {
            Ok(0.0)
        }
    }
    
    fn dependencies(&self) -> &[MetricId] {
        &[] // Uses cashflows and discount curve from context
    }
}

/// Calculates modified duration for structured credit.
///
/// Modified duration measures the percentage price change for a 1% change in yield.
/// It's the primary measure used for interest rate risk management.
///
/// # Formula
///
/// Modified Duration = Macaulay Duration / (1 + y)
///
/// Where y is the yield. For simplicity, we approximate using a small yield bump
/// and measure the actual price sensitivity.
///
/// # Interpretation
///
/// A modified duration of 3.5 means that for a 1% (100bp) increase in yield,
/// the price would decrease by approximately 3.5%.
///
pub struct ModifiedDurationCalculator;

impl MetricCalculator for ModifiedDurationCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // For structured credit, we use a numerical approach:
        // Calculate price sensitivity to a small yield shift
        
        // Get base NPV
        let base_npv = context.base_value.amount();
        
        if base_npv == 0.0 {
            return Ok(0.0);
        }
        
        // Get cashflows
        let flows = context.cashflows.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "context.cashflows".to_string(),
            })
        })?;
        
        // Get discount curve
        let disc_curve_id = context.discount_curve_id.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "discount_curve_id".to_string(),
            })
        })?;
        
        let disc = context.curves.get_discount_ref(disc_curve_id.as_str())?;
        
        // Use Act/365F for time calculation
        let day_count = finstack_core::dates::DayCount::Act365F;
        let base_date = disc.base_date();
        
        // Shift yield by 1bp (0.01%)
        let yield_shift = 0.0001;
        
        // Calculate PV with shifted discount factors
        let mut shifted_npv = 0.0;
        
        for (date, amount) in flows {
            if *date <= context.as_of {
                continue;
            }
            
            // Calculate time from curve base date
            let t = day_count
                .year_fraction(base_date, *date, DayCountCtx::default())
                .unwrap_or(0.0);
            
            // Get base discount factor
            let df = disc.df_on_date_curve(*date);
            
            // Apply yield shift: df_shifted = df * exp(-shift * t)
            let df_shifted = df * (-yield_shift * t).exp();
            
            shifted_npv += amount.amount() * df_shifted;
        }
        
        // Modified duration = -(dP/dy) / P
        // Where dP = shifted_npv - base_npv, dy = yield_shift
        let price_change = shifted_npv - base_npv;
        let modified_duration = -(price_change / base_npv) / yield_shift;
        
        Ok(modified_duration)
    }
    
    fn dependencies(&self) -> &[MetricId] {
        &[] // Uses cashflows and discount curve from context
    }
}
