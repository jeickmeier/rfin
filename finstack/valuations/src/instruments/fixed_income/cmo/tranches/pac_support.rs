//! PAC/Support tranche logic.
//!
//! PAC (Planned Amortization Class) tranches receive principal according
//! to a pre-determined schedule as long as prepayments stay within the
//! PAC collar. Support tranches absorb prepayment variability to protect
//! the PAC.

use crate::instruments::agency_cmo::types::PacCollar;

/// PAC amortization schedule.
#[derive(Clone, Debug)]
pub struct PacSchedule {
    /// Monthly scheduled principal payments
    pub scheduled_payments: Vec<f64>,
    /// PAC collar
    pub collar: PacCollar,
}

impl PacSchedule {
    /// Generate PAC schedule from collateral characteristics.
    ///
    /// The PAC schedule is the minimum principal at each period
    /// across the collar range.
    ///
    /// # Arguments
    ///
    /// * `pac_balance` - Initial PAC tranche balance
    /// * `wam` - Weighted average maturity in months
    /// * `collar` - PAC collar (PSA bounds)
    pub fn generate(pac_balance: f64, wam: u32, collar: PacCollar) -> Self {
        let mut schedule = Vec::new();
        let mut remaining = pac_balance;

        for month in 1..=wam {
            if remaining <= 0.0 {
                break;
            }

            // Calculate principal at lower and upper PSA
            let lower_smm = psa_to_smm(collar.lower_psa, month);
            let upper_smm = psa_to_smm(collar.upper_psa, month);

            // Simplified: PAC gets the minimum of the two
            // In practice, this is more complex with scheduled amortization
            let lower_principal = remaining * lower_smm;
            let upper_principal = remaining * upper_smm;

            let scheduled = lower_principal.min(upper_principal);
            schedule.push(scheduled);

            remaining -= scheduled;
        }

        Self {
            scheduled_payments: schedule,
            collar,
        }
    }

    /// Check if current prepayment is within collar.
    pub fn is_within_collar(&self, actual_psa: f64) -> bool {
        actual_psa >= self.collar.lower_psa && actual_psa <= self.collar.upper_psa
    }

    /// Get scheduled payment for a period.
    pub fn scheduled_at(&self, period: usize) -> f64 {
        self.scheduled_payments.get(period).cloned().unwrap_or(0.0)
    }

    /// Total scheduled principal.
    pub fn total_scheduled(&self) -> f64 {
        self.scheduled_payments.iter().sum()
    }
}

/// Convert PSA speed to SMM for a given month.
///
/// PSA model: CPR ramps from 0% to 6% over first 30 months,
/// then stays at 6%. Speed multiplier scales this.
fn psa_to_smm(psa_speed: f64, month: u32) -> f64 {
    let base_cpr = if month <= 30 {
        0.06 * (month as f64 / 30.0)
    } else {
        0.06
    };

    let cpr = base_cpr * psa_speed;

    // Convert CPR to SMM
    1.0 - (1.0 - cpr).powf(1.0 / 12.0)
}

/// Allocate principal between PAC and support tranches.
///
/// # Arguments
///
/// * `available_principal` - Total principal available
/// * `pac_balance` - Current PAC balance
/// * `support_balance` - Current support balance
/// * `pac_scheduled` - PAC scheduled amount for this period
/// * `actual_psa` - Actual prepayment speed (PSA)
/// * `collar` - PAC collar
///
/// # Returns
///
/// (pac_allocation, support_allocation)
pub fn allocate_pac_support(
    available_principal: f64,
    pac_balance: f64,
    support_balance: f64,
    pac_scheduled: f64,
    actual_psa: f64,
    collar: &PacCollar,
) -> (f64, f64) {
    if available_principal <= 0.0 {
        return (0.0, 0.0);
    }

    let is_within_collar = actual_psa >= collar.lower_psa && actual_psa <= collar.upper_psa;

    if is_within_collar {
        // PAC gets scheduled, support gets excess
        let pac_alloc = pac_scheduled.min(pac_balance).min(available_principal);
        let support_alloc = (available_principal - pac_alloc).min(support_balance);
        (pac_alloc, support_alloc)
    } else if actual_psa < collar.lower_psa {
        // Slow prepay: PAC may not get full schedule, support depletes first
        // Support should absorb shortfall first
        let total_needed = pac_scheduled.min(pac_balance);
        if available_principal >= total_needed {
            (total_needed, available_principal - total_needed)
        } else {
            // Not enough for PAC schedule
            (available_principal, 0.0)
        }
    } else {
        // Fast prepay (above upper collar): Support absorbs excess first
        let support_alloc = available_principal.min(support_balance);
        let remaining = available_principal - support_alloc;
        let pac_alloc = remaining.min(pac_balance);
        (pac_alloc, support_alloc)
    }
}

/// Check if PAC collar is "broken" (support depleted).
pub fn is_collar_broken(support_balance: f64) -> bool {
    support_balance <= 0.0
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_pac_schedule_generation() {
        let schedule = PacSchedule::generate(100_000.0, 360, PacCollar::standard());

        assert!(!schedule.scheduled_payments.is_empty());
        assert!(schedule.total_scheduled() > 0.0);
        assert!(schedule.total_scheduled() <= 100_000.0);
    }

    #[test]
    fn test_within_collar() {
        let schedule = PacSchedule::generate(100_000.0, 360, PacCollar::standard());

        // 100% PSA is within 100-300 collar
        assert!(schedule.is_within_collar(1.0));

        // 200% PSA is within collar
        assert!(schedule.is_within_collar(2.0));

        // 50% PSA is below collar
        assert!(!schedule.is_within_collar(0.5));

        // 400% PSA is above collar
        assert!(!schedule.is_within_collar(4.0));
    }

    #[test]
    fn test_pac_support_allocation_within_collar() {
        let collar = PacCollar::standard();

        // Within collar: PAC gets schedule, support gets excess
        let (pac, support) = allocate_pac_support(
            10_000.0, // available
            50_000.0, // pac balance
            50_000.0, // support balance
            5_000.0,  // pac scheduled
            2.0,      // actual PSA (within collar)
            &collar,
        );

        assert!((pac - 5_000.0).abs() < 1.0);
        assert!((support - 5_000.0).abs() < 1.0);
    }

    #[test]
    fn test_pac_support_allocation_fast_prepay() {
        let collar = PacCollar::standard();

        // Above collar: Support absorbs excess first
        let (_pac, support) = allocate_pac_support(
            10_000.0, 50_000.0, 20_000.0, // Limited support
            5_000.0, 4.0, // Fast prepay (above collar)
            &collar,
        );

        // Support should get more than normal
        assert!(support >= 10_000.0); // Support gets everything first
    }

    #[test]
    fn test_psa_to_smm() {
        // 100% PSA at month 30 should give ~0.5% SMM
        let smm = psa_to_smm(1.0, 30);
        assert!(smm > 0.004 && smm < 0.006);

        // 200% PSA should be about double
        let smm_200 = psa_to_smm(2.0, 30);
        assert!(smm_200 > smm * 1.5);
    }
}
