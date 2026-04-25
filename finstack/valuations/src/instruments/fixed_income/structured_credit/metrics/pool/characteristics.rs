//! Pool characteristic metrics for structured credit.

use crate::instruments::fixed_income::structured_credit::types::constants;
use crate::instruments::fixed_income::structured_credit::utils::rates::psa_to_cpr;
use crate::instruments::fixed_income::structured_credit::StructuredCredit;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::DateExt;
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
        if let Some(sc) = context
            .instrument
            .as_any()
            .downcast_ref::<StructuredCredit>()
        {
            return Ok(sc.pool.weighted_avg_maturity(as_of));
        }

        // Fallback: return 0
        Ok(0.0)
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
        // Extract CPR from behavior overrides or use deal-type defaults

        if let Some(sc) = context
            .instrument
            .as_any()
            .downcast_ref::<StructuredCredit>()
        {
            // Check overrides first
            if let Some(cpr) = sc.behavior_overrides.cpr_annual {
                return Ok(cpr);
            }

            if let Some(psa_mult) = sc.behavior_overrides.psa_speed_multiplier {
                return Ok(psa_to_cpr(
                    psa_mult,
                    deal_seasoning_month(sc, context.as_of),
                ));
            }

            // Fall back to deal type defaults
            use super::super::super::types::DealType;
            return Ok(match sc.deal_type {
                DealType::RMBS => psa_to_cpr(1.0, deal_seasoning_month(sc, context.as_of)),
                DealType::ABS | DealType::Auto => 0.15, // 15% CPR
                DealType::CMBS => 0.10,                 // 10% CPR (open period)
                DealType::CLO => 0.15,                  // 15% CPR typical
                _ => 0.10,
            });
        }

        Ok(0.0)
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
        // Extract CDR from behavior overrides or use deal-type defaults

        if let Some(sc) = context
            .instrument
            .as_any()
            .downcast_ref::<StructuredCredit>()
        {
            // Check overrides first
            if let Some(cdr) = sc.behavior_overrides.cdr_annual {
                return Ok(cdr);
            }

            if let Some(sda_mult) = sc.behavior_overrides.sda_speed_multiplier {
                return Ok(sda_to_cdr(
                    sda_mult,
                    deal_seasoning_month(sc, context.as_of),
                ));
            }

            // Fall back to deal type defaults
            use super::super::super::types::DealType;
            return Ok(match sc.deal_type {
                DealType::ABS | DealType::Auto => 0.01, // 1% default for ABS
                DealType::RMBS => sda_to_cdr(1.0, deal_seasoning_month(sc, context.as_of)),
                DealType::CMBS => 0.01, // 1% default for CMBS
                DealType::CLO => 0.02,  // 2% CDR base case
                _ => 0.01,
            });
        }

        Ok(0.0)
    }
}

fn deal_seasoning_month(sc: &StructuredCredit, as_of: finstack_core::dates::Date) -> u32 {
    if as_of > sc.closing_date {
        sc.closing_date.months_until(as_of).max(1)
    } else {
        1
    }
}

fn sda_to_cdr(speed_multiplier: f64, month: u32) -> f64 {
    let speed_multiplier = speed_multiplier.max(0.0);
    if speed_multiplier == 0.0 {
        return 0.0;
    }

    let cdr = if month <= constants::SDA_PEAK_MONTH {
        (month as f64 / constants::SDA_PEAK_MONTH as f64) * constants::SDA_PEAK_CDR
    } else if month <= constants::SDA_PEAK_MONTH * 2 {
        let months_past_peak = (month - constants::SDA_PEAK_MONTH) as f64;
        let decline_period = constants::SDA_PEAK_MONTH as f64;
        constants::SDA_PEAK_CDR
            - (months_past_peak / decline_period)
                * (constants::SDA_PEAK_CDR - constants::SDA_TERMINAL_CDR)
    } else {
        constants::SDA_TERMINAL_CDR
    };

    (cdr * speed_multiplier).clamp(0.0, 1.0)
}
