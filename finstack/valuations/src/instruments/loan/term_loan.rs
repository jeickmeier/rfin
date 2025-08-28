//! Term loan instrument implementation.

use crate::cashflow::builder::{cf, FixedCouponSpec, FloatingCouponSpec, CouponType, FeeSpec};
use crate::cashflow::amortization_notional::AmortizationSpec;
use crate::instruments::loan::prepayment::PrepaymentSchedule;
use crate::instruments::loan::covenants::Covenant;
use crate::metrics::MetricId;
use crate::pricing::result::ValuationResult;
use crate::pricing::discountable::Discountable;
use crate::traits::{CashflowProvider, Priceable};
use finstack_core::dates::{Date, DayCount, Frequency, BusinessDayConvention, StubKind};
use finstack_core::market_data::multicurve::CurveSet;

use finstack_core::money::Money;
use finstack_core::F;
use hashbrown::HashMap;

/// Interest rate specification for loans.
#[derive(Clone, Debug)]
pub enum InterestSpec {
    /// Fixed rate with optional step-ups
    Fixed {
        /// Initial rate
        rate: F,
        /// Optional rate step-ups by date
        step_ups: Option<Vec<(Date, F)>>,
    },
    /// Floating rate based on an index
    Floating {
        /// Index identifier (e.g., "USD-SOFR-3M")
        index_id: &'static str,
        /// Spread in basis points
        spread_bp: F,
        /// Optional spread step-ups by date
        spread_step_ups: Option<Vec<(Date, F)>>,
        /// Gearing factor (multiplier on index rate)
        gearing: F,
        /// Reset lag in days
        reset_lag_days: i32,
    },
    /// Payment-in-kind interest
    PIK {
        /// PIK rate
        rate: F,
    },
    /// Cash plus PIK
    CashPlusPIK {
        /// Cash portion rate
        cash_rate: F,
        /// PIK portion rate
        pik_rate: F,
    },
    /// PIK toggle based on conditions
    PIKToggle {
        /// Cash rate when paying cash
        cash_rate: F,
        /// PIK rate when capitalizing
        pik_rate: F,
        /// Toggle dates and decisions (true = PIK, false = Cash)
        toggle_schedule: Vec<(Date, bool)>,
    },
}

/// Term loan instrument.
#[derive(Clone, Debug)]
pub struct Loan {
    /// Unique identifier
    pub id: String,
    /// Borrower entity ID
    pub borrower: String,
    /// Original loan amount
    pub original_amount: Money,
    /// Current outstanding amount
    pub outstanding: Money,
    /// Issue/origination date
    pub issue_date: Date,
    /// Maturity date
    pub maturity_date: Date,
    /// Interest specification
    pub interest: InterestSpec,
    /// Payment frequency
    pub frequency: Frequency,
    /// Day count convention
    pub day_count: DayCount,
    /// Business day convention
    pub bdc: BusinessDayConvention,
    /// Calendar for adjustments
    pub calendar_id: Option<&'static str>,
    /// Stub handling
    pub stub: StubKind,
    /// Amortization specification
    pub amortization: AmortizationSpec,
    /// Prepayment terms
    pub prepayment: Option<PrepaymentSchedule>,
    /// Fee specifications
    pub fees: Vec<FeeSpec>,
    /// Financial covenants
    pub covenants: Vec<Covenant>,
    /// Discount curve ID for valuation
    pub disc_id: &'static str,
}

impl Loan {
    /// Creates a new term loan.
    pub fn new(
        id: impl Into<String>,
        amount: Money,
        issue_date: Date,
        maturity_date: Date,
        interest: InterestSpec,
    ) -> Self {
        Self {
            id: id.into(),
            borrower: String::new(),
            original_amount: amount,
            outstanding: amount,
            issue_date,
            maturity_date,
            interest,
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("usd"),
            stub: StubKind::None,
            amortization: AmortizationSpec::None,
            prepayment: None,
            fees: Vec::new(),
            covenants: Vec::new(),
            disc_id: "USD-OIS",
        }
    }

    /// Sets the borrower.
    pub fn with_borrower(mut self, borrower: impl Into<String>) -> Self {
        self.borrower = borrower.into();
        self
    }

    /// Sets the payment frequency.
    pub fn with_frequency(mut self, freq: Frequency) -> Self {
        self.frequency = freq;
        self
    }

    /// Sets the day count convention.
    pub fn with_day_count(mut self, dc: DayCount) -> Self {
        self.day_count = dc;
        self
    }

    /// Sets the amortization.
    pub fn with_amortization(mut self, amort: AmortizationSpec) -> Self {
        self.amortization = amort;
        self
    }

    /// Adds a prepayment schedule.
    pub fn with_prepayment(mut self, prepayment: PrepaymentSchedule) -> Self {
        self.prepayment = Some(prepayment);
        self
    }

    /// Adds a fee.
    pub fn with_fee(mut self, fee: FeeSpec) -> Self {
        self.fees.push(fee);
        self
    }

    /// Adds a covenant.
    pub fn with_covenant(mut self, covenant: Covenant) -> Self {
        self.covenants.push(covenant);
        self
    }

    /// Sets the discount curve.
    pub fn with_discount_curve(mut self, disc_id: &'static str) -> Self {
        self.disc_id = disc_id;
        self
    }

    /// Builds the cashflow schedule using the builder.
    fn build_cashflows(&self) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        let mut builder = cf();
        builder.principal(self.outstanding, self.issue_date, self.maturity_date);
        builder.amortization(self.amortization.clone());

        // Add fees
        for fee in &self.fees {
            builder.fee(fee.clone());
        }

        // Configure interest based on type
        match &self.interest {
            InterestSpec::Fixed { rate, step_ups } => {
                if let Some(steps) = step_ups {
                    // Use step-up functionality
                    builder.fixed_stepup(
                        steps,
                        crate::cashflow::builder::ScheduleParams {
                            freq: self.frequency,
                            dc: self.day_count,
                            bdc: self.bdc,
                            calendar_id: self.calendar_id,
                            stub: self.stub,
                        },
                        CouponType::Cash,
                    );
                } else {
                    // Simple fixed rate
                    let spec = FixedCouponSpec {
                        coupon_type: CouponType::Cash,
                        rate: *rate,
                        freq: self.frequency,
                        dc: self.day_count,
                        bdc: self.bdc,
                        calendar_id: self.calendar_id,
                        stub: self.stub,
                    };
                    builder.fixed_cf(spec);
                }
            },
            InterestSpec::Floating { index_id, spread_bp, spread_step_ups, gearing, reset_lag_days } => {
                if let Some(steps) = spread_step_ups {
                    // Use margin step-up functionality
                    let base_params = crate::cashflow::builder::FloatCouponParams {
                        index_id,
                        margin_bp: *spread_bp,
                        gearing: *gearing,
                        reset_lag_days: *reset_lag_days,
                    };
                    builder.float_margin_stepup(
                        steps,
                        base_params,
                        crate::cashflow::builder::ScheduleParams {
                            freq: self.frequency,
                            dc: self.day_count,
                            bdc: self.bdc,
                            calendar_id: self.calendar_id,
                            stub: self.stub,
                        },
                        CouponType::Cash,
                    );
                } else {
                    // Simple floating rate
                    let spec = FloatingCouponSpec {
                        index_id,
                        margin_bp: *spread_bp,
                        gearing: *gearing,
                        coupon_type: CouponType::Cash,
                        freq: self.frequency,
                        dc: self.day_count,
                        bdc: self.bdc,
                        calendar_id: self.calendar_id,
                        stub: self.stub,
                        reset_lag_days: *reset_lag_days,
                    };
                    builder.floating_cf(spec);
                }
            },
            InterestSpec::PIK { rate } => {
                let spec = FixedCouponSpec {
                    coupon_type: CouponType::PIK,
                    rate: *rate,
                    freq: self.frequency,
                    dc: self.day_count,
                    bdc: self.bdc,
                    calendar_id: self.calendar_id,
                    stub: self.stub,
                };
                builder.fixed_cf(spec);
            },
            InterestSpec::CashPlusPIK { cash_rate, pik_rate } => {
                let total_rate = cash_rate + pik_rate;
                let cash_pct = cash_rate / total_rate;
                let pik_pct = pik_rate / total_rate;
                
                let spec = FixedCouponSpec {
                    coupon_type: CouponType::Split { cash_pct, pik_pct },
                    rate: total_rate,
                    freq: self.frequency,
                    dc: self.day_count,
                    bdc: self.bdc,
                    calendar_id: self.calendar_id,
                    stub: self.stub,
                };
                builder.fixed_cf(spec);
            },
            InterestSpec::PIKToggle { cash_rate, pik_rate: _, toggle_schedule } => {
                // Use payment split program for toggle dates
                let mut payment_steps = Vec::new();
                for &(date, use_pik) in toggle_schedule {
                    let split = if use_pik { CouponType::PIK } else { CouponType::Cash };
                    payment_steps.push((date, split));
                }
                
                // Add the base rate (will be split according to toggle)
                let spec = FixedCouponSpec {
                    coupon_type: CouponType::Cash, // Default, will be overridden by program
                    rate: *cash_rate, // Use cash rate as base
                    freq: self.frequency,
                    dc: self.day_count,
                    bdc: self.bdc,
                    calendar_id: self.calendar_id,
                    stub: self.stub,
                };
                builder.fixed_cf(spec);
                builder.payment_split_program(&payment_steps);
            },
        }

        builder.build()
    }
}

impl CashflowProvider for Loan {
    fn build_schedule(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<Vec<(Date, Money)>> {
        // For floating rate loans, we need special handling
        if let InterestSpec::Floating { index_id, spread_bp, .. } = &self.interest {
            // Build a simplified floating rate schedule
            let mut flows = Vec::new();
            
            // Get the forward curve
            let fwd_curve = curves.forecast(index_id)?;
            
            // Generate payment dates
            let period_schedule = crate::cashflow::builder::schedule_utils::build_dates(
                self.issue_date,
                self.maturity_date,
                self.frequency,
                self.stub,
                self.bdc,
                self.calendar_id,
            );
            let periods = period_schedule.dates;
            
            // Calculate floating rate coupons
            let mut remaining_notional = self.outstanding.amount();
            
            for i in 1..periods.len() {
                let start = periods[i - 1];
                let end = periods[i];
                
                // Get forward rate for the period - convert dates to year fractions from as_of date
                let t1 = self.day_count.year_fraction(as_of, start)?;
                let t2 = self.day_count.year_fraction(as_of, end)?;
                let fwd_rate = fwd_curve.rate_period(t1, t2);
                let total_rate = fwd_rate + spread_bp / 10000.0;
                
                // Calculate accrual
                let yf = self.day_count.year_fraction(start, end)?;
                let interest = remaining_notional * total_rate * yf;
                
                flows.push((end, Money::new(interest, self.outstanding.currency())));
                
                // Apply amortization if any
                if let AmortizationSpec::LinearTo { final_notional } = &self.amortization {
                    // Simplified linear amortization
                    let amort_amount = (self.outstanding.amount() - final_notional.amount()) / (periods.len() - 1) as f64;
                    remaining_notional -= amort_amount;
                    flows.push((end, Money::new(amort_amount, self.outstanding.currency())));
                }
            }
            
            // Add final principal if remaining
            if remaining_notional > 0.0 {
                flows.push((self.maturity_date, Money::new(remaining_notional, self.outstanding.currency())));
            }
            
            return Ok(flows);
        }
        
        // For non-floating rate loans, use the standard builder
        let schedule = self.build_cashflows()?;
        
        // Convert to dated flows
        let mut flows = Vec::new();
        for cf in &schedule.flows {
            flows.push((cf.date, cf.amount));
        }
        
        // Add prepayment penalty if applicable
        // This would be computed based on prepayment schedule and market conditions
        
        Ok(flows)
    }
}

impl Priceable for Loan {
    fn value(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<Money> {
        let flows = self.build_schedule(curves, as_of)?;
        let disc = curves.discount(self.disc_id)?;
        flows.npv(&*disc, disc.base_date(), self.day_count)
    }

    fn price_with_metrics(
        &self,
        curves: &CurveSet,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        
        let mut result = ValuationResult::stamped(&self.id, as_of, base_value);
        
        // For now, just return the base value
        // In a full implementation, we would compute requested metrics
        let mut measures = HashMap::new();
        for metric in metrics {
            if metric == &MetricId::Ytm {
                // Simplified YTM calculation placeholder
                measures.insert("ytm".to_string(), 0.05);
            }
        }
        
        result = result.with_measures(measures);
        Ok(result)
    }

    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        Ok(ValuationResult::stamped(&self.id, as_of, base_value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_loan_creation() {
        let loan = Loan::new(
            "LOAN-001",
            Money::new(10_000_000.0, Currency::USD),
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            InterestSpec::Fixed { rate: 0.065, step_ups: None },
        );

        assert_eq!(loan.id, "LOAN-001");
        assert_eq!(loan.original_amount.amount(), 10_000_000.0);
    }

    #[test]
    fn test_loan_with_step_ups() {
        let step_ups = vec![
            (Date::from_calendar_date(2026, Month::January, 1).unwrap(), 0.07),
            (Date::from_calendar_date(2027, Month::January, 1).unwrap(), 0.075),
        ];

        let loan = Loan::new(
            "LOAN-002",
            Money::new(5_000_000.0, Currency::USD),
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            InterestSpec::Fixed { rate: 0.065, step_ups: Some(step_ups) },
        );

        // Build cashflows to ensure it works
        let schedule = loan.build_cashflows().unwrap();
        assert!(!schedule.flows.is_empty());
    }

    #[test]
    fn test_loan_with_pik_toggle() {
        let toggle_schedule = vec![
            (Date::from_calendar_date(2025, Month::January, 1).unwrap(), false), // Cash
            (Date::from_calendar_date(2026, Month::January, 1).unwrap(), true),  // PIK
            (Date::from_calendar_date(2027, Month::January, 1).unwrap(), false), // Cash
        ];

        let loan = Loan::new(
            "LOAN-003",
            Money::new(5_000_000.0, Currency::USD),
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            InterestSpec::PIKToggle {
                cash_rate: 0.06,
                pik_rate: 0.065,
                toggle_schedule,
            },
        );

        let schedule = loan.build_cashflows().unwrap();
        assert!(!schedule.flows.is_empty());
    }
}
