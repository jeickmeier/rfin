//! Prepayment schedules and penalty structures for loans.

use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::F;

/// Type of prepayment allowed on the loan.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PrepaymentType {
    /// Prepayment allowed without restrictions
    #[default]
    Allowed,
    /// No prepayment allowed (hard call protection)
    Prohibited,
    /// Prepayment allowed with make-whole premium
    MakeWhole,
    /// Soft call protection with premium
    SoftCall { 
        /// Premium as percentage of prepaid amount
        premium_pct: F 
    },
}

/// Prepayment penalty specification.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PrepaymentPenalty {
    /// Start date when this penalty applies
    pub start: Date,
    /// End date for this penalty (None means it applies until maturity)
    pub end: Option<Date>,
    /// Type of penalty
    pub penalty: PenaltyType,
}

/// Type of prepayment penalty.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PenaltyType {
    /// Fixed amount penalty
    Fixed(Money),
    /// Percentage of prepaid amount
    Percentage(F),
    /// Make-whole premium based on benchmark curve
    MakeWhole { 
        /// Benchmark curve ID
        benchmark_curve: String,
        /// Spread in basis points over benchmark
        spread_bp: F 
    },
    /// Yield maintenance penalty
    YieldMaintenance {
        /// Reference rate for calculation
        reference_rate: F
    },
}

/// Complete prepayment schedule for a loan.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PrepaymentSchedule {
    /// Type of prepayment allowed
    pub prepayment_type: PrepaymentType,
    /// Optional lockout period during which no prepayment is allowed
    pub lockout_period: Option<(Date, Date)>,
    /// Schedule of penalties by date range
    pub penalties: Vec<PrepaymentPenalty>,
}



impl PrepaymentSchedule {
    /// Creates a new prepayment schedule.
    pub fn new(prepayment_type: PrepaymentType) -> Self {
        Self {
            prepayment_type,
            lockout_period: None,
            penalties: Vec::new(),
        }
    }

    /// Sets the lockout period.
    pub fn with_lockout(mut self, start: Date, end: Date) -> Self {
        self.lockout_period = Some((start, end));
        self
    }

    /// Adds a penalty period.
    pub fn with_penalty(mut self, penalty: PrepaymentPenalty) -> Self {
        self.penalties.push(penalty);
        self
    }

    /// Checks if prepayment is allowed on a given date.
    pub fn is_prepayment_allowed(&self, date: Date) -> bool {
        // Check lockout period
        if let Some((start, end)) = self.lockout_period {
            if date >= start && date <= end {
                return false;
            }
        }

        // Check prepayment type
        !matches!(self.prepayment_type, PrepaymentType::Prohibited)
    }

    /// Calculates the prepayment penalty for a given date and amount.
    pub fn calculate_penalty(&self, date: Date, amount: Money) -> finstack_core::Result<Money> {
        if !self.is_prepayment_allowed(date) {
            return Err(finstack_core::error::InputError::Invalid.into());
        }

        // Find applicable penalty
        for penalty in &self.penalties {
            let in_range = date >= penalty.start && 
                          penalty.end.map_or(true, |end| date <= end);
            
            if in_range {
                return match &penalty.penalty {
                    PenaltyType::Fixed(fee) => Ok(*fee),
                    PenaltyType::Percentage(pct) => {
                        Ok(Money::new(amount.amount() * pct, amount.currency()))
                    },
                    PenaltyType::MakeWhole { .. } => {
                        // Make-whole calculation would require market data
                        // For now, return a placeholder
                        Ok(Money::new(amount.amount() * 0.03, amount.currency())) // 3% placeholder
                    },
                    PenaltyType::YieldMaintenance { reference_rate } => {
                        // Simplified yield maintenance
                        Ok(Money::new(amount.amount() * reference_rate * 0.5, amount.currency()))
                    },
                };
            }
        }

        // No penalty if no matching period
        Ok(Money::new(0.0, amount.currency()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_prepayment_lockout() {
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2025, Month::June, 30).unwrap();
        
        let schedule = PrepaymentSchedule::new(PrepaymentType::Allowed)
            .with_lockout(start, end);

        // During lockout
        assert!(!schedule.is_prepayment_allowed(
            Date::from_calendar_date(2025, Month::March, 15).unwrap()
        ));

        // After lockout
        assert!(schedule.is_prepayment_allowed(
            Date::from_calendar_date(2025, Month::July, 1).unwrap()
        ));
    }

    #[test]
    fn test_prepayment_penalty_calculation() {
        let schedule = PrepaymentSchedule::new(PrepaymentType::SoftCall { premium_pct: 0.02 })
            .with_penalty(PrepaymentPenalty {
                start: Date::from_calendar_date(2025, Month::January, 1).unwrap(),
                end: Some(Date::from_calendar_date(2025, Month::December, 31).unwrap()),
                penalty: PenaltyType::Percentage(0.03), // 3% penalty
            });

        let amount = Money::new(1_000_000.0, Currency::USD);
        let date = Date::from_calendar_date(2025, Month::June, 15).unwrap();
        
        let penalty = schedule.calculate_penalty(date, amount).unwrap();
        assert_eq!(penalty.amount(), 30_000.0); // 3% of 1M
    }
}
