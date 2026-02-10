//! Variation margin calculator.
//!
//! Implements ISDA CSA variation margin calculation logic including
//! threshold, MTA, and rounding rules.

use crate::margin::types::{CsaSpec, MarginCall, MarginTenor};
use finstack_core::currency::Currency;
use finstack_core::dates::{adjust, BusinessDayConvention, CalendarRegistry, Date, DateExt};
use finstack_core::money::Money;
use finstack_core::Result;
use time::Month;

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
/// Implementation delegates CSA/MTA/rounding logic to
/// [`VmParameters::calculate_margin_call`] to ensure consistent behavior
/// across margin utilities.
///
/// # Example
///
/// ```rust,no_run
/// use finstack_valuations::margin::{VmCalculator, CsaSpec};
/// use finstack_core::money::Money;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::Date;
///
/// # fn main() -> finstack_core::Result<()> {
/// let csa = CsaSpec::usd_regulatory();
/// let calc = VmCalculator::new(csa);
///
/// let exposure = Money::new(5_000_000.0, Currency::USD);
/// let posted = Money::new(3_000_000.0, Currency::USD);
/// let as_of = Date::from_calendar_date(2025, time::Month::January, 15).expect("valid");
///
/// let result = calc.calculate(exposure, posted, as_of)?;
/// println!("Delivery required: {}", result.delivery_amount);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct VmCalculator {
    csa: CsaSpec,
}

impl VmCalculator {
    fn default_calendar_id_for_currency(currency: Currency) -> &'static str {
        match currency {
            Currency::USD => "USNY",
            Currency::EUR => "TARGET2",
            Currency::GBP => "GBLO",
            Currency::JPY => "JPTO",
            Currency::CHF => "CHZU",
            Currency::CAD => "CATO",
            Currency::AUD => "AUSY",
            _ => crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID,
        }
    }

    fn calendar_for_csa(&self) -> Option<&'static dyn finstack_core::dates::HolidayCalendar> {
        let cal_id = Self::default_calendar_id_for_currency(self.csa.base_currency);
        CalendarRegistry::global().resolve_str(cal_id)
    }

    fn add_business_days(&self, date: Date, days: i32) -> Date {
        if days == 0 {
            return date;
        }
        if let Some(cal) = self.calendar_for_csa() {
            if let Ok(d) = date.add_business_days(days, cal) {
                return d;
            }
        }
        date.add_weekdays(days)
    }

    fn adjust_to_business_day(&self, date: Date) -> Date {
        if let Some(cal) = self.calendar_for_csa() {
            return adjust(date, BusinessDayConvention::Following, cal).unwrap_or(date);
        }
        date
    }

    fn add_month_clamped(&self, date: Date) -> Date {
        let (y, m, d) = date.to_calendar_date();
        let m_num = m as i32;
        let mut target_year = y;
        let mut target_month = m_num + 1;
        if target_month > 12 {
            target_month = 1;
            target_year += 1;
        }
        let month = Month::try_from(target_month as u8).unwrap_or(Month::December);
        let mut day = d;
        loop {
            if let Ok(candidate) = Date::from_calendar_date(target_year, month, day) {
                return self.adjust_to_business_day(candidate);
            }
            // Clamp to last valid day of month
            day = u8::try_from((day as i32 - 1).max(1)).unwrap_or(1);
        }
    }

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

        // Required margin before applying posted collateral and MTA
        let threshold = vm_params.threshold.amount();
        let ia = vm_params.independent_amount.amount();
        let required = (exposure.amount() - threshold + ia).max(0.0);

        // Delegate CSA/MTA/rounding logic to VmParameters for consistency
        let net_call = vm_params.calculate_margin_call(exposure, posted_collateral);
        let (delivery, ret) = if net_call.amount() > 0.0 {
            (net_call, Money::new(0.0, currency))
        } else if net_call.amount() < 0.0 {
            (
                Money::new(0.0, currency),
                Money::new(net_call.amount().abs(), currency),
            )
        } else {
            (Money::new(0.0, currency), Money::new(0.0, currency))
        };

        // Calculate settlement date
        let settlement_date = self.calculate_settlement_date(as_of)?;

        Ok(VmResult {
            date: as_of,
            gross_exposure: exposure,
            net_exposure: Money::new(required, currency),
            delivery_amount: delivery,
            return_amount: ret,
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
                    current_collateral = current_collateral.checked_add(result.delivery_amount)?;
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
        let adjusted_start = self.adjust_to_business_day(start);
        let mut current = adjusted_start;

        while current <= end {
            dates.push(current);
            current = match self.csa.vm_params.frequency {
                MarginTenor::Daily => self.add_business_days(current, 1),
                MarginTenor::Weekly => self.add_business_days(current, 5),
                MarginTenor::Monthly => self.add_month_clamped(current),
                MarginTenor::OnDemand => {
                    // For on-demand, just return start and end
                    if current == adjusted_start {
                        self.adjust_to_business_day(end)
                    } else {
                        break;
                    }
                }
            };
        }

        dates
    }

    /// Calculate settlement date based on lag.
    fn calculate_settlement_date(&self, call_date: Date) -> Result<Date> {
        let lag = self.csa.vm_params.settlement_lag as i32;
        Ok(self.add_business_days(call_date, lag))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::margin::types::{EligibleCollateralSchedule, MarginCallTiming, VmParameters};
    use crate::margin::MarginCallType;
    use finstack_core::currency::Currency;
    use finstack_core::types::CurveId;
    use time::Month;

    fn test_date(y: i32, m: u8, d: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).expect("valid month"), d)
            .expect("valid date")
    }

    fn threshold_csa() -> CsaSpec {
        CsaSpec {
            id: "TEST".to_string(),
            base_currency: Currency::USD,
            vm_params: VmParameters::with_threshold(
                Money::new(1_000_000.0, Currency::USD),
                Money::new(100_000.0, Currency::USD),
            ),
            im_params: None,
            eligible_collateral: EligibleCollateralSchedule::default(),
            call_timing: MarginCallTiming::default(),
            collateral_curve_id: CurveId::new("USD-OIS"),
        }
    }

    #[test]
    fn vm_calculator_no_threshold() {
        let csa = CsaSpec::usd_regulatory();
        let calc = VmCalculator::new(csa);

        let exposure = Money::new(5_000_000.0, Currency::USD);
        let posted = Money::new(3_000_000.0, Currency::USD);
        let result = calc
            .calculate(exposure, posted, test_date(2025, 1, 15))
            .expect("calc ok");

        // With zero threshold, delivery = exposure - posted = 2M
        assert_eq!(result.delivery_amount.amount(), 2_000_000.0);
        assert_eq!(result.return_amount.amount(), 0.0);
    }

    #[test]
    fn vm_calculator_with_threshold() {
        let csa = threshold_csa();
        let calc = VmCalculator::new(csa);

        // Exposure below threshold: no margin call
        let exposure = Money::new(500_000.0, Currency::USD);
        let posted = Money::new(0.0, Currency::USD);
        let result = calc
            .calculate(exposure, posted, test_date(2025, 1, 15))
            .expect("calc ok");

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
        let result = calc
            .calculate(exposure, posted, test_date(2025, 1, 15))
            .expect("calc ok");

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
        let result = calc
            .calculate(exposure, posted, test_date(2025, 1, 15))
            .expect("calc ok");

        // 300K < 500K MTA, no call
        assert!(!result.requires_call());
    }

    #[test]
    fn vm_calculator_matches_vm_params() {
        let csa = CsaSpec::usd_regulatory();
        let calc = VmCalculator::new(csa.clone());
        let as_of = test_date(2025, 1, 15);

        let exposure = Money::new(2_000_000.0, Currency::USD);
        let posted = Money::new(0.0, Currency::USD);

        let params_call = csa.vm_params.calculate_margin_call(exposure, posted);
        let result = calc.calculate(exposure, posted, as_of).expect("calc ok");

        assert_eq!(result.delivery_amount, params_call);
        assert_eq!(result.return_amount.amount(), 0.0);

        // Now flip to a return scenario
        let exposure = Money::new(500_000.0, Currency::USD);
        let posted = Money::new(3_000_000.0, Currency::USD);

        let params_call = csa.vm_params.calculate_margin_call(exposure, posted);
        let result = calc.calculate(exposure, posted, as_of).expect("calc ok");

        assert_eq!(result.delivery_amount.amount(), 0.0);
        assert_eq!(
            result.return_amount,
            Money::new(params_call.amount().abs(), Currency::USD)
        );
    }

    #[test]
    fn generate_margin_call_series() {
        let csa = CsaSpec::usd_regulatory();
        let calc = VmCalculator::new(csa);

        let exposures = vec![
            (
                test_date(2025, 1, 15),
                Money::new(1_000_000.0, Currency::USD),
            ),
            (
                test_date(2025, 1, 16),
                Money::new(2_000_000.0, Currency::USD),
            ),
            (
                test_date(2025, 1, 17),
                Money::new(1_500_000.0, Currency::USD),
            ),
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

    #[test]
    fn settlement_lag_uses_business_days() {
        let csa = CsaSpec::usd_regulatory(); // settlement_lag = 1
        let calc = VmCalculator::new(csa);
        let friday = test_date(2025, 1, 10);
        let exposure = Money::new(1_000_000.0, Currency::USD);
        let posted = Money::new(0.0, Currency::USD);

        let result = calc.calculate(exposure, posted, friday).expect("calc ok");
        // T+1 business day from Friday should be Monday.
        assert_eq!(result.settlement_date, test_date(2025, 1, 13));
    }

    #[test]
    fn daily_margin_call_dates_skip_weekends() {
        let csa = CsaSpec::usd_regulatory();
        let calc = VmCalculator::new(csa);
        let dates = calc.margin_call_dates(test_date(2025, 1, 10), test_date(2025, 1, 14));
        assert_eq!(
            dates,
            vec![
                test_date(2025, 1, 10),
                test_date(2025, 1, 13),
                test_date(2025, 1, 14)
            ]
        );
    }
}
