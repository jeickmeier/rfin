//! Pool characteristic metrics for structured credit.

use crate::instruments::structured_credit::StructuredCredit;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Calculates WAM (Weighted Average Maturity) for the underlying pool.
///
/// WAM measures the weighted average time until assets in the pool mature,
/// based on the original legal maturities (not expected prepayment life like WAL).
///
/// # Formula
///
/// WAM = Σ(Balance_i × Years_to_Maturity_i) / Σ(Balance_i)
///
/// # Typical Values
///
/// - **CLO**: 5-7 years
/// - **ABS**: 3-5 years
/// - **RMBS**: 25-30 years (legal maturity, much longer than WAL)
/// - **CMBS**: 7-10 years
///
pub struct WamCalculator;

impl MetricCalculator for WamCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Try to extract WAM from pool
        // This uses the pool's weighted_avg_maturity method if available
        
        let as_of = context.as_of;
        
        // Single check for unified structured credit type
        if let Some(sc) = context.instrument.as_any().downcast_ref::<StructuredCredit>() {
            return Ok(sc.pool.weighted_avg_maturity(as_of));
        }
        
        // Fallback: return 0
        Ok(0.0)
    }
    
    fn dependencies(&self) -> &[MetricId] {
        &[] // No dependencies
    }
}

/// Calculates CPR (Constant Prepayment Rate) assumption.
///
/// CPR is the annualized rate at which the pool is assumed to prepay.
/// This is an assumption/input rather than a calculated output.
///
/// # Formula
///
/// CPR = annualized prepayment rate (e.g., 0.15 = 15% CPR)
///
/// # Sources
///
/// - **RMBS**: Derived from PSA speed
/// - **ABS**: From ABS speed assumption
/// - **CMBS**: From open period CPR
/// - **CLO**: Default prepayment assumption
///
pub struct CprCalculator;

impl MetricCalculator for CprCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Extract CPR from instrument-specific fields
        
        use crate::instruments::structured_credit::InstrumentSpecificFields;
        
        if let Some(sc) = context.instrument.as_any().downcast_ref::<StructuredCredit>() {
            return Ok(match &sc.specific {
                InstrumentSpecificFields::Rmbs { psa_speed, .. } => {
                    // PSA model: 100% PSA = 6% CPR at 30 months
                    psa_speed * 0.06
                }
                InstrumentSpecificFields::Abs { abs_speed, .. } => {
                    // Use ABS speed if available
                    abs_speed.unwrap_or(0.15)
                }
                InstrumentSpecificFields::Cmbs { open_cpr, .. } => {
                    // Use open CPR if available
                    open_cpr.unwrap_or(0.10)
                }
                InstrumentSpecificFields::Clo { .. } => {
                    // CLO default prepayment
                    0.15 // 15% CPR typical
                }
            });
        }
        
        Ok(0.0)
    }
    
    fn dependencies(&self) -> &[MetricId] {
        &[] // No dependencies
    }
}

/// Calculates CDR (Constant Default Rate) assumption.
///
/// CDR is the annualized rate at which the pool is assumed to default.
/// This is an assumption/input rather than a calculated output.
///
/// # Formula
///
/// CDR = annualized default rate (e.g., 0.02 = 2% CDR)
///
/// # Typical Values
///
/// - **CLO**: 2-3% base case, 5-10% stress
/// - **ABS (auto)**: 1-2% base case
/// - **RMBS**: 0.5-1% base case (agency)
/// - **CMBS**: 0.5-1% base case
///
pub struct CdrCalculator;

impl MetricCalculator for CdrCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Extract CDR from instrument-specific fields
        
        use crate::instruments::structured_credit::InstrumentSpecificFields;
        
        if let Some(sc) = context.instrument.as_any().downcast_ref::<StructuredCredit>() {
            return Ok(match &sc.specific {
                InstrumentSpecificFields::Abs { cdr_annual, .. } => {
                    cdr_annual.unwrap_or(0.01) // 1% default for ABS
                }
                InstrumentSpecificFields::Rmbs { sda_speed, .. } => {
                    // Derive from SDA speed
                    // SDA 100% ≈ 0.6% CDR at peak
                    sda_speed * 0.006
                }
                InstrumentSpecificFields::Cmbs { cdr_annual, .. } => {
                    cdr_annual.unwrap_or(0.01) // 1% default for CMBS
                }
                InstrumentSpecificFields::Clo { cdr_annual, .. } => {
                    // CLO default assumption
                    cdr_annual.unwrap_or(0.02) // 2% CDR base case
                }
            });
        }
        
        Ok(0.0)
    }
    
    fn dependencies(&self) -> &[MetricId] {
        &[] // No dependencies
    }
}
