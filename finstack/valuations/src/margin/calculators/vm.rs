//! Variation margin calculator.
//!
//! Implements ISDA CSA variation margin calculation logic including
//! threshold, MTA, and rounding rules.

use crate::margin::types::{CsaSpec, MarginCall, MarginFrequency};
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::Result;

/// Variation margin calculation result.
#[derive(Debug, Clone, PartialEq)]
pub struct VmResult {
    /// Calculation date
    pub date: Date,

    /// Gross mark-to-market exposure
    pub gross_exposure: Money,

    /// Net exposure after applying threshold and independent amount
    pub net_exposure: Money,

    /// Delivery amount (positive = we need to post margin)
    pub delivery_amount: Money,

    /// Return amount (positive = we receive margin back)
    pub return_amount: Money,

    /// Settlement date for the margin transfer
    pub settlement_date: Date,
}

impl VmResult {
    /// Get the net margin amount (delivery - return).
    #[must_use]
    pub fn net_margin(&self) -> Money {
        if self.delivery_amount.amount() > 0.0 {
            self.delivery_amount
        } else {
            Money::new(-self.return_amount.amount(), self.return_amount.currency())
        }
    }

    /// Check if a margin call is required.
    #[must_use]
    pub fn requires_call(&self) -> bool {
        self.delivery_amount.amount() > 0.0 || self.return_amount.amount() > 0.0
    }
}

/// Variation margin calculator following ISDA CSA rules.
///
/// Calculates variation margin based on mark-to-market exposure,
/// applying threshold, MTA, independent amount, and rounding rules.
///
/// # ISDA CSA Formula
///
/// ```text
/// Credit Support Amount = max(0, Exposure - Threshold + IA) - Current_Collateral
/// Delivery Amount = max(0, CSA) if CSA ≥ MTA, else 0
/// Return Amount = max(0, -CSA) if |CSA| ≥ MTA, else 0
/// ```
///
/// # Example
///
/// ```rust,ignore
/// use finstack_valuations::margin::{VmCalculator, CsaSpec};
/// use finstack_core::money::Money;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::Date;
///
/// let csa = CsaSpec::usd_regulatory();
/// let calc = VmCalculator::new(csa);
///
/// let exposure = Money::new(5_000_000.0, Currency::USD);
/// let posted = Money::new(3_000_000.0, Currency::USD);
/// let as_of = Date::from_calendar_date(2025, time::Month::January, 15).expect("valid");
///
/// let result = calc.calculate(exposure, posted, as_of)?;
/// println!("Delivery required: {}", result.delivery_amount);
/// ```
#[derive(Debug, Clone)]
pub struct VmCalculator {
    csa: CsaSpec,
}

impl VmCalculator {
    /// Create a new VM calculator with the given CSA specification.
    #[must_use]
    pub fn new(csa: CsaSpec) -> Self {
        Self { csa }
    }

    /// Calculate variation margin given current exposure and posted collateral.
    ///
    /// # Arguments
    ///
    /// * `exposure` - Current mark-to-market exposure (positive = counterparty owes us)
    /// * `posted_collateral` - Value of currently posted collateral
    /// * `as_of` - Calculation date
    ///
    /// # Returns
    ///
    /// [`VmResult`] with delivery and return amounts.
    pub fn calculate(
        &self,
        exposure: Money,
        posted_collateral: Money,
        as_of: Date,
    ) -> Result<VmResult> {
        let currency = self.csa.base_currency;

        // Ensure same currency (would need FX conversion in production)
        debug_assert_eq!(exposure.currency(), currency);
        debug_assert_eq!(posted_collateral.currency(), currency);

        let vm_params = &self.csa.vm_params;

        // Calculate credit support amount
        // CSA = max(0, Exposure - Threshold + IA) - Posted
        let threshold = vm_params.threshold.amount();
        let ia = vm_params.independent_amount.amount();
        let exp = exposure.amount();

        let required = (exp - threshold + ia).max(0.0);
        let credit_support_amount = required - posted_collateral.amount();

        let mta = vm_params.mta.amount();
        let rounding = vm_params.rounding.amount();

        // Apply MTA and determine delivery/return
        let (delivery, ret) = if credit_support_amount >= mta {
            // Delivery required
            let rounded = self.round_to_nearest(credit_support_amount, rounding);
            (rounded, 0.0)
        } else if credit_support_amount <= -mta {
            // Return of excess collateral
            let rounded = self.round_to_nearest(credit_support_amount.abs(), rounding);
            (0.0, rounded)
        } else {
            // Amount below MTA, no action
            (0.0, 0.0)
        };

        // Calculate settlement date
        let settlement_date = self.calculate_settlement_date(as_of)?;

        Ok(VmResult {
            date: as_of,
            gross_exposure: exposure,
            net_exposure: Money::new(required, currency),
            delivery_amount: Money::new(delivery, currency),
            return_amount: Money::new(ret, currency),
            settlement_date,
        })
    }

    /// Generate a series of margin calls from an exposure time series.
    ///
    /// # Arguments
    ///
    /// * `exposures` - Time series of (date, exposure) pairs
    /// * `initial_collateral` - Initially posted collateral
    ///
    /// # Returns
    ///
    /// Vector of [`MarginCall`] events.
    pub fn generate_margin_calls(
        &self,
        exposures: &[(Date, Money)],
        initial_collateral: Money,
    ) -> Result<Vec<MarginCall>> {
        let mut calls = Vec::new();
        let mut current_collateral = initial_collateral;
        let currency = self.csa.base_currency;

        for (date, exposure) in exposures {
            let result = self.calculate(*exposure, current_collateral, *date)?;

            if result.requires_call() {
                let settlement_date = result.settlement_date;

                if result.delivery_amount.amount() > 0.0 {
                    calls.push(MarginCall::vm_delivery(
                        *date,
                        settlement_date,
                        result.delivery_amount,
                        *exposure,
                        self.csa.vm_params.threshold,
                        self.csa.vm_params.mta,
                    ));
                    current_collateral = (current_collateral + result.delivery_amount)?;
                } else if result.return_amount.amount() > 0.0 {
                    calls.push(MarginCall::vm_return(
                        *date,
                        settlement_date,
                        result.return_amount,
                        *exposure,
                        self.csa.vm_params.threshold,
                        self.csa.vm_params.mta,
                    ));
                    current_collateral = Money::new(
                        (current_collateral.amount() - result.return_amount.amount()).max(0.0),
                        currency,
                    );
                }
            }
        }

        Ok(calls)
    }

    /// Generate margin call dates based on frequency.
    pub fn margin_call_dates(&self, start: Date, end: Date) -> Vec<Date> {
        let mut dates = Vec::new();
        let mut current = start;

        while current <= end {
            dates.push(current);
            current = match self.csa.vm_params.frequency {
                MarginFrequency::Daily => {
                    // Add 1 day (simplified - should use calendar)
                    current + time::Duration::days(1)
                }
                MarginFrequency::Weekly => current + time::Duration::weeks(1),
                MarginFrequency::Monthly => {
                    // Add approximately 1 month
                    current + time::Duration::days(30)
                }
                MarginFrequency::OnDemand => {
                    // For on-demand, just return start and end
                    if current == start {
                        end
                    } else {
                        break;
                    }
                }
            };
        }

        dates
    }

    /// Round to nearest increment.
    fn round_to_nearest(&self, amount: f64, rounding: f64) -> f64 {
        if rounding <= 0.0 {
            amount
        } else {
            (amount / rounding).round() * rounding
        }
    }

    /// Calculate settlement date based on lag.
    fn calculate_settlement_date(&self, call_date: Date) -> Result<Date> {
        // Simplified: add business days (should use calendar)
        let lag = self.csa.vm_params.settlement_lag as i64;
        Ok(call_date + time::Duration::days(lag))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::margin::MarginCallType;
    use finstack_core::currency::Currency;
    use time::Month;

    fn test_date(y: i32, m: u8, d: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).expect("valid month"), d)
            .expect("valid date")
    }

    #[test]
    fn vm_calculator_no_threshold() {
        let csa = CsaSpec::usd_regulatory();
        let calc = VmCalculator::new(csa);

        let exposure = Money::new(5_000_000.0, Currency::USD);
        let posted = Money::new(3_000_000.0, Currency::USD);
        let result = calc.calculate(exposure, posted, test_date(2025, 1, 15)).expect("calc ok");

        // With zero threshold, delivery = exposure - posted = 2M
        assert_eq!(result.delivery_amount.amount(), 2_000_000.0);
        assert_eq!(result.return_amount.amount(), 0.0);
    }

    #[test]
    fn vm_calculator_with_threshold() {
        let csa = CsaSpec::bilateral_legacy("TEST", Currency::USD, 1_000_000.0, 100_000.0);
        let calc = VmCalculator::new(csa);

        // Exposure below threshold: no margin call
        let exposure = Money::new(500_000.0, Currency::USD);
        let posted = Money::new(0.0, Currency::USD);
        let result = calc.calculate(exposure, posted, test_date(2025, 1, 15)).expect("calc ok");

        assert_eq!(result.delivery_amount.amount(), 0.0);
        assert!(!result.requires_call());
    }

    #[test]
    fn vm_calculator_return_excess() {
        let csa = CsaSpec::usd_regulatory();
        let calc = VmCalculator::new(csa);

        // Exposure dropped, have excess collateral
        let exposure = Money::new(1_000_000.0, Currency::USD);
        let posted = Money::new(3_000_000.0, Currency::USD);
        let result = calc.calculate(exposure, posted, test_date(2025, 1, 15)).expect("calc ok");

        // Return = posted - required = 3M - 1M = 2M
        assert_eq!(result.delivery_amount.amount(), 0.0);
        assert_eq!(result.return_amount.amount(), 2_000_000.0);
    }

    #[test]
    fn vm_calculator_below_mta() {
        let csa = CsaSpec::usd_regulatory(); // MTA = 500K
        let calc = VmCalculator::new(csa);

        let exposure = Money::new(300_000.0, Currency::USD);
        let posted = Money::new(0.0, Currency::USD);
        let result = calc.calculate(exposure, posted, test_date(2025, 1, 15)).expect("calc ok");

        // 300K < 500K MTA, no call
        assert!(!result.requires_call());
    }

    #[test]
    fn generate_margin_call_series() {
        let csa = CsaSpec::usd_regulatory();
        let calc = VmCalculator::new(csa);

        let exposures = vec![
            (test_date(2025, 1, 15), Money::new(1_000_000.0, Currency::USD)),
            (test_date(2025, 1, 16), Money::new(2_000_000.0, Currency::USD)),
            (test_date(2025, 1, 17), Money::new(1_500_000.0, Currency::USD)),
        ];

        let calls = calc
            .generate_margin_calls(&exposures, Money::new(0.0, Currency::USD))
            .expect("margin calls ok");

        // Three calls: 2 deliveries (1M, then 1M more), then 1 return (0.5M excess)
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].call_type, MarginCallType::VariationMarginDelivery);
        assert_eq!(calls[1].call_type, MarginCallType::VariationMarginDelivery);
        assert_eq!(calls[2].call_type, MarginCallType::VariationMarginReturn);
    }
}

