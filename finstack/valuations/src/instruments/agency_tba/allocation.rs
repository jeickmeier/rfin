//! Simplified TBA pool allocation (cheapest-to-deliver).
//!
//! This module provides simplified CTD allocation using on-the-run
//! pool characteristics rather than full SIFMA good delivery rules.

use super::AgencyTba;
use crate::instruments::agency_mbs_passthrough::AgencyMbsPassthrough;
use finstack_core::Result;

/// Pool allocation result.
#[derive(Clone, Debug)]
pub struct AllocationResult {
    /// Allocated pool
    pub pool: AgencyMbsPassthrough,
    /// Value advantage/disadvantage vs. generic pool
    pub value_adjustment: f64,
    /// Whether this is a specified pool or generic
    pub is_specified: bool,
}

/// Simplified CTD allocation using on-the-run pool characteristics.
///
/// This function creates an assumed pool for TBA valuation using
/// standard generic pool assumptions rather than evaluating actual
/// deliverable pools.
///
/// # Simplifying Assumptions
///
/// - Pool factor: 1.0 (newly issued)
/// - WAC: TBA coupon + standard fee strip (50 bps)
/// - WAM: Full term (180/240/360 months)
/// - Prepayment: 100% PSA
///
/// For full CTD analysis, users should provide explicit `assumed_pool`
/// on the TBA instrument.
pub fn allocate_generic_pool(tba: &AgencyTba) -> Result<AllocationResult> {
    use crate::instruments::agency_tba::pricer::create_assumed_pool;
    use finstack_core::dates::Date;

    // Use a reference date for pool creation
    let reference_date = Date::from_calendar_date(
        tba.settlement_year,
        time::Month::try_from(tba.settlement_month)
            .map_err(|e| finstack_core::Error::Validation(e.to_string()))?,
        1,
    )
    .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;

    let pool = create_assumed_pool(tba, reference_date)?;

    Ok(AllocationResult {
        pool,
        value_adjustment: 0.0, // Generic pool has no adjustment
        is_specified: false,
    })
}

/// Pool characteristics that affect deliverability and value.
#[derive(Clone, Debug)]
pub struct PoolCharacteristics {
    /// Weighted average coupon
    pub wac: f64,
    /// Weighted average maturity (months)
    pub wam: u32,
    /// Weighted average loan age (months)
    pub wala: u32,
    /// Pool factor (remaining balance / original)
    pub factor: f64,
    /// Average loan size
    pub avg_loan_size: Option<f64>,
    /// Geographic concentration
    pub geographic_concentration: Option<f64>,
}

impl PoolCharacteristics {
    /// Estimate prepayment speed adjustment based on characteristics.
    ///
    /// Returns a PSA multiplier (1.0 = baseline).
    pub fn estimated_psa_multiplier(&self) -> f64 {
        let mut multiplier = 1.0;

        // Higher WAC relative to market tends to prepay faster
        // (This would need market rate context for proper calculation)

        // Seasoned pools (high WALA) may have different prepayment patterns
        if self.wala > 24 {
            // Post-ramp, use burnout adjustment
            multiplier *= 0.95;
        }

        // Lower factors indicate pool has already experienced prepayments
        if self.factor < 0.9 {
            multiplier *= 0.90; // Burnout effect
        }

        // Smaller loans tend to prepay faster (can't refi as easily)
        if let Some(avg_size) = self.avg_loan_size {
            if avg_size < 200_000.0 {
                multiplier *= 0.95;
            } else if avg_size > 500_000.0 {
                multiplier *= 1.05;
            }
        }

        multiplier
    }

    /// Check if pool meets TBA good delivery standards.
    ///
    /// This is a simplified check; full SIFMA rules are more complex.
    pub fn meets_good_delivery(&self, tba_coupon: f64) -> bool {
        // WAC should be within reasonable range of TBA coupon
        let wac_spread = self.wac - tba_coupon;
        if !(0.0025..=0.01).contains(&wac_spread) {
            return false; // Typical servicing spread: 25-100 bps
        }

        // Factor shouldn't be too low (seasoned pools may not deliver)
        if self.factor < 0.50 {
            return false;
        }

        true
    }
}

/// Calculate value adjustment for a specified pool vs. generic.
///
/// Positive adjustment means the specified pool is worth more than generic.
pub fn calculate_pay_up(_characteristics: &PoolCharacteristics, _tba: &AgencyTba) -> f64 {
    // Simplified: no pay-up calculation
    // Full implementation would consider:
    // - WAC vs. market rate (refi incentive)
    // - Loan balance (lower balance = slower prepay = worth more at premium)
    // - Geographic concentration
    // - Loan purpose (purchase vs. refi)
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_generic_pool() {
        let tba = AgencyTba::example();
        let result = allocate_generic_pool(&tba).expect("should allocate");

        assert!(!result.is_specified);
        assert!((result.value_adjustment).abs() < 1e-10);
        assert!((result.pool.current_factor - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_pool_characteristics_psa() {
        let chars = PoolCharacteristics {
            wac: 0.045,
            wam: 348,
            wala: 12,
            factor: 0.98,
            avg_loan_size: Some(300_000.0),
            geographic_concentration: None,
        };

        let psa = chars.estimated_psa_multiplier();
        // Should be around 1.0 for standard characteristics
        assert!(psa > 0.8 && psa < 1.2);
    }

    #[test]
    fn test_pool_characteristics_burnout() {
        let chars = PoolCharacteristics {
            wac: 0.045,
            wam: 300,
            wala: 60,     // 5 years seasoned
            factor: 0.70, // Significant paydown
            avg_loan_size: Some(300_000.0),
            geographic_concentration: None,
        };

        let psa = chars.estimated_psa_multiplier();
        // Should be lower due to burnout
        assert!(psa < 1.0);
    }

    #[test]
    fn test_good_delivery_check() {
        let chars = PoolCharacteristics {
            wac: 0.045, // 4.5% WAC
            wam: 350,
            wala: 10,
            factor: 0.95,
            avg_loan_size: None,
            geographic_concentration: None,
        };

        // With 4.0% TBA coupon, 50 bps spread should pass
        assert!(chars.meets_good_delivery(0.04));

        // With 4.2% TBA coupon, 30 bps spread is within range (25-100 bps)
        assert!(chars.meets_good_delivery(0.042));
    }

    #[test]
    fn test_good_delivery_fails_low_factor() {
        let chars = PoolCharacteristics {
            wac: 0.045,
            wam: 200,
            wala: 160,
            factor: 0.40, // Too low
            avg_loan_size: None,
            geographic_concentration: None,
        };

        assert!(!chars.meets_good_delivery(0.04));
    }
}
