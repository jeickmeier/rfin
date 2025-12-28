//! Dollar roll carry and implied financing calculations.
//!
//! The dollar roll drop implies a financing rate that can be compared
//! to repo rates to assess roll "specialness".

use super::DollarRoll;
use finstack_core::Result;

/// Carry calculation result.
#[derive(Clone, Debug)]
pub struct CarryResult {
    /// Implied financing rate (annualized)
    pub implied_rate: f64,
    /// Dollar drop (front price - back price)
    pub drop: f64,
    /// Days between settlements
    pub settlement_days: i64,
    /// Expected coupon income during roll period
    pub coupon_income: f64,
    /// Expected principal paydown during roll period
    pub principal_paydown: f64,
}

/// Calculate implied financing rate from dollar roll drop.
///
/// The implied financing rate is the annualized cost/benefit of
/// executing the roll versus holding the security and financing
/// it in repo.
///
/// # Formula
///
/// ```text
/// implied_rate = (drop + coupon - paydown) / price × (365 / days)
/// ```
///
/// # Arguments
///
/// * `roll` - Dollar roll instrument
/// * `prepay_rate` - Expected monthly prepayment rate (SMM)
///
/// # Returns
///
/// Annualized implied financing rate
pub fn implied_financing_rate(roll: &DollarRoll, prepay_rate: f64) -> Result<CarryResult> {
    let days = roll.settlement_days()?;
    let drop = roll.drop();

    // Estimate coupon income during roll period (roughly 1 month)
    // Monthly coupon = notional × coupon / 12
    let monthly_coupon = roll.coupon / 12.0;
    let coupon_income = monthly_coupon * 100.0; // Per $100 face

    // Estimate principal paydown (scheduled + prepay)
    // Simplified: use SMM applied to balance
    let principal_paydown = prepay_rate * 100.0; // Per $100 face

    // Net benefit of rolling = drop + coupon - paydown
    // If positive, implies financing is cheaper than repo
    let net_benefit = drop + coupon_income - principal_paydown;

    // Annualize the rate
    // implied_rate = net_benefit / price × (365 / days)
    let price = roll.front_price;
    let implied_rate = (net_benefit / price) * (365.0 / days as f64);

    Ok(CarryResult {
        implied_rate,
        drop,
        settlement_days: days,
        coupon_income,
        principal_paydown,
    })
}

/// Calculate roll specialness (implied rate vs. repo rate).
///
/// # Arguments
///
/// * `roll` - Dollar roll instrument
/// * `prepay_rate` - Expected monthly prepayment rate (SMM)
/// * `repo_rate` - Prevailing repo rate (annualized)
///
/// # Returns
///
/// Roll specialness in basis points (positive = roll is special)
pub fn roll_specialness(roll: &DollarRoll, prepay_rate: f64, repo_rate: f64) -> Result<f64> {
    let carry = implied_financing_rate(roll, prepay_rate)?;

    // Specialness = repo_rate - implied_rate
    // If positive, roll financing is cheaper than repo
    let specialness = repo_rate - carry.implied_rate;

    Ok(specialness * 10_000.0) // Convert to bps
}

/// Calculate break-even drop given a target financing rate.
///
/// # Arguments
///
/// * `target_rate` - Target financing rate (annualized)
/// * `front_price` - Front-month price
/// * `coupon_income` - Expected coupon income per $100 face
/// * `principal_paydown` - Expected paydown per $100 face
/// * `days` - Days between settlements
///
/// # Returns
///
/// Break-even drop (in price points)
pub fn break_even_drop(
    target_rate: f64,
    front_price: f64,
    coupon_income: f64,
    principal_paydown: f64,
    days: i64,
) -> f64 {
    // Solve: target_rate = (drop + coupon - paydown) / price × (365 / days)
    // drop = target_rate × price × days / 365 - coupon + paydown
    let required_net = target_rate * front_price * (days as f64 / 365.0);
    required_net - coupon_income + principal_paydown
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_implied_financing_rate() {
        let roll = DollarRoll::example();
        let prepay_rate = 0.005; // 0.5% SMM

        let result = implied_financing_rate(&roll, prepay_rate).expect("should calculate");

        // Should return a reasonable financing rate
        assert!(result.implied_rate > -0.20);
        assert!(result.implied_rate < 0.20);

        // Drop should match the roll's drop
        assert!((result.drop - roll.drop()).abs() < 1e-10);
    }

    #[test]
    fn test_roll_specialness() {
        let roll = DollarRoll::example();
        let prepay_rate = 0.005;
        let repo_rate = 0.05; // 5% repo rate

        let specialness =
            roll_specialness(&roll, prepay_rate, repo_rate).expect("should calculate");

        // Specialness should be reasonable (positive or negative)
        assert!(specialness > -500.0);
        assert!(specialness < 500.0);
    }

    #[test]
    fn test_break_even_drop() {
        let target_rate = 0.04; // 4% target
        let front_price = 98.5;
        let coupon_income = 0.333; // ~4% / 12
        let principal_paydown = 0.5; // SMM applied
        let days = 30;

        let break_even = break_even_drop(
            target_rate,
            front_price,
            coupon_income,
            principal_paydown,
            days,
        );

        // Should be a reasonable drop
        assert!(break_even.abs() < 2.0);
    }
}
