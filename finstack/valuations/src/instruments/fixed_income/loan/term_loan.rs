//! Term loan instrument implementation.

use super::covenants::Covenant;
use super::prepayment::PrepaymentSchedule;
use crate::cashflow::builder::{cf, CouponType, FeeSpec, FixedCouponSpec, FloatingCouponSpec};
use crate::cashflow::primitives::AmortizationSpec;
// impl_attributable provided by macro when we switch to macro-based impls
// use crate::metrics::MetricId; // no longer needed with schedule macro default metrics
use crate::cashflow::traits::CashflowProvider;
use crate::instruments::traits::Attributes;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::MarketContext;

use finstack_core::money::Money;
use finstack_core::F;

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
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
    /// Cash sweep percentage applied due to covenant breach (0.0 = no sweep)
    pub cash_sweep_pct: F,
    /// Whether loan is in default
    pub is_default: bool,
    /// Whether distributions are blocked
    pub distribution_blocked: bool,
}

impl Loan {
    /// Create a new loan builder.
    pub fn builder() -> LoanBuilder {
        LoanBuilder::new()
    }

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
            attributes: Attributes::new(),
            cash_sweep_pct: 0.0,
            is_default: false,
            distribution_blocked: false,
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
            }
            InterestSpec::Floating {
                index_id,
                spread_bp,
                spread_step_ups,
                gearing,
                reset_lag_days,
            } => {
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
            }
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
            }
            InterestSpec::CashPlusPIK {
                cash_rate,
                pik_rate,
            } => {
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
            }
            InterestSpec::PIKToggle {
                cash_rate,
                pik_rate: _,
                toggle_schedule,
            } => {
                // Use payment split program for toggle dates
                let mut payment_steps = Vec::new();
                for &(date, use_pik) in toggle_schedule {
                    let split = if use_pik {
                        CouponType::PIK
                    } else {
                        CouponType::Cash
                    };
                    payment_steps.push((date, split));
                }

                // Add the base rate (will be split according to toggle)
                let spec = FixedCouponSpec {
                    coupon_type: CouponType::Cash, // Default, will be overridden by program
                    rate: *cash_rate,              // Use cash rate as base
                    freq: self.frequency,
                    dc: self.day_count,
                    bdc: self.bdc,
                    calendar_id: self.calendar_id,
                    stub: self.stub,
                };
                builder.fixed_cf(spec);
                builder.payment_split_program(&payment_steps);
            }
        }

        builder.build()
    }
}

impl CashflowProvider for Loan {
    fn build_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Vec<(Date, Money)>> {
        // For floating rate loans, we need special handling
        if let InterestSpec::Floating {
            index_id,
            spread_bp,
            ..
        } = &self.interest
        {
            // Build a simplified floating rate schedule
            let mut flows = Vec::new();

            // Get the forward curve
            let fwd_curve = curves.fwd(index_id)?;

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
                let t1 = self.day_count.year_fraction(as_of, start, finstack_core::dates::DayCountCtx::default())?;
                let t2 = self.day_count.year_fraction(as_of, end, finstack_core::dates::DayCountCtx::default())?;
                let fwd_rate = fwd_curve.rate_period(t1, t2);
                let total_rate = fwd_rate + spread_bp / 10000.0;

                // Calculate accrual
                let yf = self.day_count.year_fraction(start, end, finstack_core::dates::DayCountCtx::default())?;
                let interest = remaining_notional * total_rate * yf;

                flows.push((end, Money::new(interest, self.outstanding.currency())));

                // Apply amortization if any
                if let AmortizationSpec::LinearTo { final_notional } = &self.amortization {
                    // Simplified linear amortization
                    let amort_amount = (self.outstanding.amount() - final_notional.amount())
                        / (periods.len() - 1) as f64;
                    remaining_notional -= amort_amount;
                    flows.push((end, Money::new(amort_amount, self.outstanding.currency())));
                }
            }

            // Add final principal if remaining
            if remaining_notional > 0.0 {
                flows.push((
                    self.maturity_date,
                    Money::new(remaining_notional, self.outstanding.currency()),
                ));
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

impl_instrument_schedule_pv!(
    Loan, "Loan",
    disc_field: disc_id,
    dc_field: day_count
);

// Conversions and Attributable provided by macro

impl crate::covenants::engine::InstrumentMutator for Loan {
    fn set_default_status(&mut self, is_default: bool, _as_of: Date) -> finstack_core::Result<()> {
        self.is_default = is_default;
        Ok(())
    }

    fn increase_rate(&mut self, increase: F) -> finstack_core::Result<()> {
        match &mut self.interest {
            InterestSpec::Fixed { rate, step_ups } => {
                if let Some(ref mut steps) = step_ups {
                    // Add a step-up from today forward with the increased rate
                    let new_rate = *rate + increase;
                    // Find if there's already a step for today or later, update the latest one
                    if let Some((_, last_rate)) = steps.last_mut() {
                        *last_rate += increase;
                    } else {
                        steps.push((self.issue_date, new_rate));
                    }
                } else {
                    // Create new step-ups starting from today
                    let new_rate = *rate + increase;
                    *step_ups = Some(vec![(self.issue_date, new_rate)]);
                }
            }
            InterestSpec::Floating {
                spread_bp,
                spread_step_ups,
                ..
            } => {
                let increase_bp = increase * 10000.0;
                if let Some(ref mut steps) = spread_step_ups {
                    // Add spread step-up
                    if let Some((_, last_spread)) = steps.last_mut() {
                        *last_spread += increase_bp;
                    } else {
                        steps.push((self.issue_date, *spread_bp + increase_bp));
                    }
                } else {
                    *spread_step_ups = Some(vec![(self.issue_date, *spread_bp + increase_bp)]);
                }
            }
            InterestSpec::PIK { rate } => {
                *rate += increase;
            }
            InterestSpec::CashPlusPIK {
                cash_rate,
                pik_rate,
            } => {
                // Apply increase proportionally
                let total = *cash_rate + *pik_rate;
                if total > 0.0 {
                    let cash_portion = *cash_rate / total;
                    let pik_portion = *pik_rate / total;
                    *cash_rate += increase * cash_portion;
                    *pik_rate += increase * pik_portion;
                } else {
                    *cash_rate += increase * 0.5;
                    *pik_rate += increase * 0.5;
                }
            }
            InterestSpec::PIKToggle {
                cash_rate,
                pik_rate,
                ..
            } => {
                *cash_rate += increase;
                *pik_rate += increase;
            }
        }
        Ok(())
    }

    fn set_cash_sweep(&mut self, percentage: F) -> finstack_core::Result<()> {
        self.cash_sweep_pct = percentage.clamp(0.0, 1.0);
        Ok(())
    }

    fn set_distribution_block(&mut self, blocked: bool) -> finstack_core::Result<()> {
        self.distribution_blocked = blocked;
        Ok(())
    }

    fn set_maturity(&mut self, new_maturity: Date) -> finstack_core::Result<()> {
        if new_maturity < self.issue_date {
            return Err(finstack_core::error::InputError::Invalid.into());
        }
        self.maturity_date = new_maturity;
        Ok(())
    }
}

/// Builder pattern for Loan instruments
#[derive(Default)]
pub struct LoanBuilder {
    id: Option<String>,
    borrower: Option<String>,
    original_amount: Option<Money>,
    outstanding: Option<Money>,
    issue_date: Option<Date>,
    maturity_date: Option<Date>,
    interest: Option<InterestSpec>,
    frequency: Option<Frequency>,
    day_count: Option<DayCount>,
    bdc: Option<BusinessDayConvention>,
    calendar_id: Option<&'static str>,
    stub: Option<StubKind>,
    amortization: Option<AmortizationSpec>,
    prepayment: Option<PrepaymentSchedule>,
    fees: Option<Vec<FeeSpec>>,
    covenants: Option<Vec<Covenant>>,
    disc_id: Option<&'static str>,
}

impl LoanBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }

    pub fn borrower(mut self, value: impl Into<String>) -> Self {
        self.borrower = Some(value.into());
        self
    }

    pub fn original_amount(mut self, value: Money) -> Self {
        self.original_amount = Some(value);
        self
    }

    pub fn outstanding(mut self, value: Money) -> Self {
        self.outstanding = Some(value);
        self
    }

    pub fn issue_date(mut self, value: Date) -> Self {
        self.issue_date = Some(value);
        self
    }

    pub fn maturity_date(mut self, value: Date) -> Self {
        self.maturity_date = Some(value);
        self
    }

    pub fn interest(mut self, value: InterestSpec) -> Self {
        self.interest = Some(value);
        self
    }

    pub fn frequency(mut self, value: Frequency) -> Self {
        self.frequency = Some(value);
        self
    }

    pub fn day_count(mut self, value: DayCount) -> Self {
        self.day_count = Some(value);
        self
    }

    pub fn bdc(mut self, value: BusinessDayConvention) -> Self {
        self.bdc = Some(value);
        self
    }

    pub fn calendar_id(mut self, value: &'static str) -> Self {
        self.calendar_id = Some(value);
        self
    }

    pub fn stub(mut self, value: StubKind) -> Self {
        self.stub = Some(value);
        self
    }

    pub fn amortization(mut self, value: AmortizationSpec) -> Self {
        self.amortization = Some(value);
        self
    }

    pub fn prepayment(mut self, value: PrepaymentSchedule) -> Self {
        self.prepayment = Some(value);
        self
    }

    pub fn fees(mut self, value: Vec<FeeSpec>) -> Self {
        self.fees = Some(value);
        self
    }

    pub fn covenants(mut self, value: Vec<Covenant>) -> Self {
        self.covenants = Some(value);
        self
    }

    pub fn disc_id(mut self, value: &'static str) -> Self {
        self.disc_id = Some(value);
        self
    }

    pub fn build(self) -> finstack_core::Result<Loan> {
        let id = self
            .id
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let original_amount = self
            .original_amount
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let issue_date = self
            .issue_date
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let maturity_date = self
            .maturity_date
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let interest = self
            .interest
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;

        Ok(Loan {
            id,
            borrower: self.borrower.unwrap_or_default(),
            original_amount,
            outstanding: self.outstanding.unwrap_or(original_amount),
            issue_date,
            maturity_date,
            interest,
            frequency: self.frequency.unwrap_or_else(Frequency::quarterly),
            day_count: self.day_count.unwrap_or(DayCount::Act360),
            bdc: self.bdc.unwrap_or(BusinessDayConvention::ModifiedFollowing),
            calendar_id: self.calendar_id.or(Some("usd")),
            stub: self.stub.unwrap_or(StubKind::None),
            amortization: self.amortization.unwrap_or(AmortizationSpec::None),
            prepayment: self.prepayment,
            fees: self.fees.unwrap_or_default(),
            covenants: self.covenants.unwrap_or_default(),
            disc_id: self.disc_id.unwrap_or("USD-OIS"),
            attributes: Attributes::new(),
            cash_sweep_pct: 0.0,
            is_default: false,
            distribution_blocked: false,
        })
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
            InterestSpec::Fixed {
                rate: 0.065,
                step_ups: None,
            },
        );

        assert_eq!(loan.id, "LOAN-001");
        assert_eq!(loan.original_amount.amount(), 10_000_000.0);
    }

    #[test]
    fn test_loan_with_step_ups() {
        let step_ups = vec![
            (
                Date::from_calendar_date(2026, Month::January, 1).unwrap(),
                0.07,
            ),
            (
                Date::from_calendar_date(2027, Month::January, 1).unwrap(),
                0.075,
            ),
        ];

        let loan = Loan::new(
            "LOAN-002",
            Money::new(5_000_000.0, Currency::USD),
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            InterestSpec::Fixed {
                rate: 0.065,
                step_ups: Some(step_ups),
            },
        );

        // Build cashflows to ensure it works
        let schedule = loan.build_cashflows().unwrap();
        assert!(!schedule.flows.is_empty());
    }

    #[test]
    fn test_loan_with_pik_toggle() {
        let toggle_schedule = vec![
            (
                Date::from_calendar_date(2025, Month::January, 1).unwrap(),
                false,
            ), // Cash
            (
                Date::from_calendar_date(2026, Month::January, 1).unwrap(),
                true,
            ), // PIK
            (
                Date::from_calendar_date(2027, Month::January, 1).unwrap(),
                false,
            ), // Cash
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

    #[test]
    fn test_loan_builder_pattern() {
        let amount = Money::new(5_000_000.0, Currency::USD);
        let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

        let loan = Loan::builder()
            .id("LOAN-BUILDER-001")
            .borrower("Test Borrower LLC")
            .original_amount(amount)
            .outstanding(amount)
            .issue_date(issue)
            .maturity_date(maturity)
            .interest(InterestSpec::Fixed {
                rate: 0.075,
                step_ups: None,
            })
            .frequency(Frequency::quarterly())
            .day_count(DayCount::Act360)
            .disc_id("USD-OIS")
            .build()
            .unwrap();

        assert_eq!(loan.id, "LOAN-BUILDER-001");
        assert_eq!(loan.borrower, "Test Borrower LLC");
        assert_eq!(loan.original_amount.amount(), 5_000_000.0);
        assert_eq!(loan.outstanding.amount(), 5_000_000.0);
        assert_eq!(loan.issue_date, issue);
        assert_eq!(loan.maturity_date, maturity);
        assert_eq!(loan.day_count, DayCount::Act360);
        assert_eq!(loan.disc_id, "USD-OIS");

        match loan.interest {
            InterestSpec::Fixed { rate, .. } => assert_eq!(rate, 0.075),
            _ => panic!("Expected Fixed interest"),
        }
    }

    #[test]
    fn test_covenant_consequence_rate_increase() {
        use crate::covenants::engine::InstrumentMutator;

        let mut loan = Loan::new(
            "LOAN-COVENANT-TEST",
            Money::new(1_000_000.0, Currency::USD),
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            InterestSpec::Fixed {
                rate: 0.05,
                step_ups: None,
            },
        );

        // Apply rate increase
        loan.increase_rate(0.01).unwrap(); // 100bps increase

        // Verify rate increase was applied
        match &loan.interest {
            InterestSpec::Fixed { step_ups, .. } => {
                assert!(step_ups.is_some());
                let steps = step_ups.as_ref().unwrap();
                assert_eq!(steps.len(), 1);
                assert!((steps[0].1 - 0.06).abs() < 1e-10); // Original 5% + 1% = 6%
            }
            _ => panic!("Expected Fixed interest"),
        }
    }

    #[test]
    fn test_covenant_consequence_cash_sweep() {
        use crate::covenants::engine::InstrumentMutator;

        let mut loan = Loan::new(
            "LOAN-SWEEP-TEST",
            Money::new(1_000_000.0, Currency::USD),
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            InterestSpec::Fixed {
                rate: 0.05,
                step_ups: None,
            },
        );

        assert_eq!(loan.cash_sweep_pct, 0.0);

        // Apply cash sweep
        loan.set_cash_sweep(0.5).unwrap(); // 50% sweep
        assert_eq!(loan.cash_sweep_pct, 0.5);

        // Test clamping
        loan.set_cash_sweep(1.5).unwrap(); // Should be clamped to 1.0
        assert_eq!(loan.cash_sweep_pct, 1.0);
    }

    #[test]
    fn test_covenant_consequence_default_status() {
        use crate::covenants::engine::InstrumentMutator;

        let mut loan = Loan::new(
            "LOAN-DEFAULT-TEST",
            Money::new(1_000_000.0, Currency::USD),
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            InterestSpec::Fixed {
                rate: 0.05,
                step_ups: None,
            },
        );

        assert!(!loan.is_default);

        loan.set_default_status(
            true,
            Date::from_calendar_date(2025, Month::June, 1).unwrap(),
        )
        .unwrap();
        assert!(loan.is_default);
    }
}
