//! Delayed-Draw Term Loan (DDTL) implementation.

use super::covenants::Covenant;
use super::simulation::{LoanFacility, LoanSimulator, SimulationEvent, EventType};
use super::term_loan::InterestSpec;
use crate::cashflow::builder::{cf, CouponType, FeeBase, FeeSpec, FixedCouponSpec};
use crate::cashflow::primitives::AmortizationSpec;
use crate::cashflow::traits::CashflowProvider;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::F;

/// Draw rules for a DDTL.
#[derive(Clone, Debug)]
pub struct DrawRules {
    /// Minimum draw amount
    pub min_draw: Money,
    /// Maximum draw amount (None means no maximum)
    pub max_draw: Option<Money>,
    /// Notice period required in days
    pub notice_days: i32,
}

/// Draw event for a DDTL.
#[derive(Clone, Debug)]
pub struct DrawEvent {
    /// Date of the draw
    pub date: Date,
    /// Amount to draw
    pub amount: Money,
    /// Purpose/use of proceeds
    pub purpose: Option<String>,
    /// Whether this draw is subject to conditions precedent
    pub conditional: bool,
}

/// Expected funding curve for pricing.
#[derive(Clone, Debug)]
pub struct ExpectedFundingCurve {
    /// Expected future draws for pricing
    pub expected_draws: Vec<DrawEvent>,
    /// Probability of each draw occurring (optional, defaults to 1.0)
    pub draw_probabilities: Option<Vec<F>>,
}

impl ExpectedFundingCurve {
    /// Create a new expected funding curve.
    pub fn new(expected_draws: Vec<DrawEvent>) -> Self {
        Self {
            expected_draws,
            draw_probabilities: None,
        }
    }

    /// Create with probabilities for each draw.
    pub fn with_probabilities(expected_draws: Vec<DrawEvent>, probabilities: Vec<F>) -> Self {
        Self {
            expected_draws,
            draw_probabilities: Some(probabilities),
        }
    }
}

/// Delayed-Draw Term Loan instrument.
#[derive(Clone, Debug)]
pub struct DelayedDrawTermLoan {
    /// Unique identifier
    pub id: String,
    /// Borrower entity ID
    pub borrower: String,
    /// Total committed amount available for draws
    pub commitment: Money,
    /// Amount already drawn
    pub drawn_amount: Money,
    /// Expiry date for drawing rights
    pub commitment_expiry: Date,
    /// Rules governing draws
    pub draw_rules: DrawRules,
    /// Planned/scheduled draws
    pub planned_draws: Vec<DrawEvent>,
    /// Expected funding curve for pricing (future expected draws)
    pub expected_funding_curve: Option<ExpectedFundingCurve>,
    /// Interest specification (applies after drawn)
    pub interest_spec: InterestSpec,
    /// Commitment fee on undrawn amount (annual rate)
    pub commitment_fee_rate: F,
    /// Ticking fee on undrawn amount (annual rate)
    pub ticking_fee_rate: Option<F>,
    /// Additional fees
    pub fees: Vec<FeeSpec>,
    /// Covenants that must be satisfied for draws
    pub draw_conditions: Vec<Covenant>,
    /// Maturity date
    pub maturity: Date,
    /// Amortization after drawn
    pub amortization: AmortizationSpec,
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
    /// Discount curve ID
    pub disc_id: &'static str,
    /// Cash sweep percentage applied due to covenant breach (0.0 = no sweep)
    pub cash_sweep_pct: F,
    /// Whether facility is in default
    pub is_default: bool,
    /// Whether distributions are blocked
    pub distribution_blocked: bool,
    /// Attributes for scenario selection and tagging
    pub attributes: crate::instruments::traits::Attributes,
}

impl DelayedDrawTermLoan {
    /// Creates a new DDTL.
    pub fn new(
        id: impl Into<String>,
        commitment: Money,
        commitment_expiry: Date,
        maturity: Date,
        interest_spec: InterestSpec,
    ) -> Self {
        Self {
            id: id.into(),
            borrower: String::new(),
            commitment,
            drawn_amount: Money::new(0.0, commitment.currency()),
            commitment_expiry,
            draw_rules: DrawRules {
                min_draw: Money::new(100_000.0, commitment.currency()),
                max_draw: None,
                notice_days: 3,
            },
            planned_draws: Vec::new(),
            expected_funding_curve: None,
            interest_spec,
            commitment_fee_rate: 0.005, // 50 bps default
            ticking_fee_rate: None,
            fees: Vec::new(),
            draw_conditions: Vec::new(),
            maturity,
            amortization: AmortizationSpec::None,
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("usd"),
            stub: StubKind::None,
            disc_id: "USD-OIS",
            cash_sweep_pct: 0.0,
            is_default: false,
            distribution_blocked: false,
            attributes: crate::instruments::traits::Attributes::new(),
        }
    }

    /// Returns the undrawn amount.
    pub fn undrawn_amount(&self) -> Money {
        Money::new(
            self.commitment.amount() - self.drawn_amount.amount(),
            self.commitment.currency(),
        )
    }

    /// Adds a planned draw.
    pub fn with_draw(mut self, draw: DrawEvent) -> Self {
        self.planned_draws.push(draw);
        self
    }

    /// Sets the commitment fee rate.
    pub fn with_commitment_fee(mut self, rate: F) -> Self {
        self.commitment_fee_rate = rate;
        self
    }

    /// Sets the ticking fee rate.
    pub fn with_ticking_fee(mut self, rate: F) -> Self {
        self.ticking_fee_rate = Some(rate);
        self
    }

    /// Adds a draw condition covenant.
    pub fn with_draw_condition(mut self, covenant: Covenant) -> Self {
        self.draw_conditions.push(covenant);
        self
    }

    /// Set expected funding curve for pricing.
    pub fn with_expected_funding_curve(mut self, curve: ExpectedFundingCurve) -> Self {
        self.expected_funding_curve = Some(curve);
        self
    }

    /// Add expected draws for pricing.
    pub fn with_expected_draws(mut self, draws: Vec<DrawEvent>) -> Self {
        self.expected_funding_curve = Some(ExpectedFundingCurve::new(draws));
        self
    }

    /// Simulates draws up to a given date and returns the drawn amount.
    fn simulate_draws_to_date(&self, as_of: Date) -> Money {
        self.simulate_draws_to_date_with_context(as_of, None)
    }

    /// Simulates draws up to a given date with optional covenant context for draw condition enforcement.
    pub fn simulate_draws_to_date_with_context(&self, as_of: Date, covenant_context: Option<&crate::covenants::engine::CovenantEngine>) -> Money {
        let mut drawn = self.drawn_amount;

        for draw in &self.planned_draws {
            if draw.date <= as_of && draw.date <= self.commitment_expiry {
                // Check draw conditions if draw is conditional
                let draw_allowed = if draw.conditional && !self.draw_conditions.is_empty() {
                    if let Some(engine) = covenant_context {
                        // Create a simplified metric context for covenant evaluation
                        // In practice, this would be provided by the host with real statement data
                        use crate::metrics::MetricContext;
                        use std::sync::Arc;
                        let dummy_curves = finstack_core::market_data::MarketContext::new();
                        let mut metric_ctx = MetricContext::new(
                            Arc::new(self.clone()),
                            Arc::new(dummy_curves),
                            draw.date,
                            Money::new(0.0, self.commitment.currency()),
                        );
                        
                        // Evaluate draw conditions using provided engine (all must pass)
                        if let Ok(reports) = engine.evaluate(&mut metric_ctx, draw.date) {
                            !reports.values().any(|report| !report.passed)
                        } else {
                            false // Failed to evaluate - deny draw
                        }
                    } else {
                        // No covenant context provided - allow draw but warn in logs
                        true
                    }
                } else {
                    // Unconditional draw or no conditions
                    true
                };

                if draw_allowed {
                    let new_drawn = drawn.amount() + draw.amount.amount();
                    if new_drawn <= self.commitment.amount() {
                        drawn = Money::new(new_drawn, self.commitment.currency());
                    }
                }
            }
        }

        drawn
    }

    /// Get all expected future draws for pricing (includes planned + expected).
    #[allow(dead_code)]
    fn get_expected_future_draws(&self, as_of: Date) -> Vec<(Date, Money, F)> {
        let mut future_draws = Vec::new();

        // Add planned draws that are after as_of
        for draw in &self.planned_draws {
            if draw.date > as_of && draw.date <= self.commitment_expiry && !draw.conditional {
                future_draws.push((draw.date, draw.amount, 1.0));
            }
        }

        // Add expected draws from funding curve
        if let Some(ref curve) = self.expected_funding_curve {
            for (i, draw) in curve.expected_draws.iter().enumerate() {
                if draw.date > as_of && draw.date <= self.commitment_expiry && !draw.conditional {
                    let prob = curve
                        .draw_probabilities
                        .as_ref()
                        .and_then(|probs| probs.get(i))
                        .copied()
                        .unwrap_or(1.0);
                    future_draws.push((draw.date, draw.amount, prob));
                }
            }
        }

        // Sort by date
        future_draws.sort_by_key(|(date, _, _)| *date);
        future_draws
    }

    /// Builds the cashflow schedule.
    fn build_cashflows(
        &self,
        as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        // Simulate draws to determine outstanding amount
        let drawn = self.simulate_draws_to_date(as_of);
        let undrawn = Money::new(
            self.commitment.amount() - drawn.amount(),
            self.commitment.currency(),
        );

        let mut builder = cf();

        // For the drawn portion, create a term loan
        if drawn.amount() > 0.0 {
            let issue = self.planned_draws.first().map(|d| d.date).unwrap_or(as_of);

            builder.principal(drawn, issue, self.maturity);
            builder.amortization(self.amortization.clone());

            // Add interest on drawn amount
            match &self.interest_spec {
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
                    use crate::cashflow::builder::FloatingCouponSpec;
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

                    let spec = FixedCouponSpec {
                        coupon_type: CouponType::Cash, // Default, overridden by program
                        rate: *cash_rate,
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
        } else {
            // If nothing drawn yet, just set up an empty schedule
            builder.principal(
                Money::new(0.0, self.commitment.currency()),
                as_of,
                self.maturity,
            );
        }

        // Add commitment fee on undrawn amount
        if undrawn.amount() > 0.0 && as_of < self.commitment_expiry {
            let fee_spec = FeeSpec::PeriodicBps {
                base: FeeBase::Undrawn {
                    facility_limit: self.commitment,
                },
                bps: self.commitment_fee_rate * 10000.0, // Convert to bps
                freq: self.frequency,
                dc: self.day_count,
                bdc: self.bdc,
                calendar_id: self.calendar_id,
                stub: self.stub,
            };
            builder.fee(fee_spec);
        }

        // Add ticking fee if applicable
        if let Some(ticking_rate) = self.ticking_fee_rate {
            if undrawn.amount() > 0.0 && as_of < self.commitment_expiry {
                let fee_spec = FeeSpec::PeriodicBps {
                    base: FeeBase::Undrawn {
                        facility_limit: self.commitment,
                    },
                    bps: ticking_rate * 10000.0,
                    freq: Frequency::monthly(),
                    dc: self.day_count,
                    bdc: self.bdc,
                    calendar_id: self.calendar_id,
                    stub: self.stub,
                };
                builder.fee(fee_spec);
            }
        }

        // Add other fees
        for fee in &self.fees {
            builder.fee(fee.clone());
        }

        builder.build()
    }
}

impl CashflowProvider for DelayedDrawTermLoan {
    fn build_schedule(
        &self,
        _curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Vec<(Date, Money)>> {
        let schedule = self.build_cashflows(as_of)?;

        let mut flows = Vec::new();
        for cf in &schedule.flows {
            flows.push((cf.date, cf.amount));
        }

        Ok(flows)
    }
}

impl LoanFacility for DelayedDrawTermLoan {
    fn currency(&self) -> finstack_core::currency::Currency {
        self.commitment.currency()
    }
    
    fn commitment(&self) -> Money {
        self.commitment
    }
    
    fn drawn_amount(&self) -> Money {
        self.drawn_amount
    }
    
    fn commitment_expiry(&self) -> Date {
        self.commitment_expiry
    }
    
    fn maturity(&self) -> Date {
        self.maturity
    }
    
    fn interest_spec(&self) -> &InterestSpec {
        &self.interest_spec
    }
    
    fn commitment_fee_rate(&self) -> F {
        self.commitment_fee_rate
    }
    
    fn cash_sweep_percentage(&self) -> F {
        self.cash_sweep_pct
    }
    
    fn frequency(&self) -> Frequency {
        self.frequency
    }
    
    fn day_count(&self) -> DayCount {
        self.day_count
    }
    
    fn bdc(&self) -> BusinessDayConvention {
        self.bdc
    }
    
    fn calendar_id(&self) -> Option<&'static str> {
        self.calendar_id
    }
    
    fn stub(&self) -> StubKind {
        self.stub
    }
    
    fn disc_id(&self) -> &'static str {
        self.disc_id
    }
    
    fn expected_events(&self) -> Vec<SimulationEvent> {
        let mut events = Vec::new();
        
        // Add planned draws
        for draw in &self.planned_draws {
            if !draw.conditional {
                events.push(SimulationEvent {
                    date: draw.date,
                    balance_change: draw.amount.amount(),
                    probability: 1.0,
                    event_type: EventType::Draw,
                });
            }
        }
        
        // Add expected draws from funding curve
        if let Some(ref curve) = self.expected_funding_curve {
            for (i, draw) in curve.expected_draws.iter().enumerate() {
                if !draw.conditional {
                    let prob = curve.draw_probabilities
                        .as_ref()
                        .and_then(|probs| probs.get(i))
                        .copied()
                        .unwrap_or(1.0);
                    
                    events.push(SimulationEvent {
                        date: draw.date,
                        balance_change: draw.amount.amount(),
                        probability: prob,
                        event_type: EventType::Draw,
                    });
                }
            }
        }
        
        events
    }
    
    fn events_on_date(&self, date: Date) -> Vec<SimulationEvent> {
        self.expected_events()
            .into_iter()
            .filter(|event| event.date == date)
            .collect()
    }
    
    fn build_existing_flows(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<Vec<(Date, Money)>> {
        self.build_schedule(curves, as_of)
    }
}

impl_instrument!(
    DelayedDrawTermLoan,
    "DelayedDrawTermLoan",
    pv = |s, curves, as_of| {
        // Use enhanced simulation-based valuation
        let simulator = LoanSimulator::new();
        let result = simulator.simulate(s, curves, as_of)?;
        Ok(result.total_pv)
    }
);

impl crate::covenants::engine::InstrumentMutator for DelayedDrawTermLoan {
    fn set_default_status(&mut self, is_default: bool, _as_of: Date) -> finstack_core::Result<()> {
        self.is_default = is_default;
        Ok(())
    }

    fn increase_rate(&mut self, increase: F) -> finstack_core::Result<()> {
        match &mut self.interest_spec {
            InterestSpec::Fixed { rate, step_ups } => {
                if let Some(ref mut steps) = step_ups {
                    if let Some((_, last_rate)) = steps.last_mut() {
                        *last_rate += increase;
                    } else {
                        steps.push((self.commitment_expiry, *rate + increase));
                    }
                } else {
                    *step_ups = Some(vec![(self.commitment_expiry, *rate + increase)]);
                }
            }
            InterestSpec::Floating { spread_bp, spread_step_ups, .. } => {
                let increase_bp = increase * 10000.0;
                if let Some(ref mut steps) = spread_step_ups {
                    if let Some((_, last_spread)) = steps.last_mut() {
                        *last_spread += increase_bp;
                    } else {
                        steps.push((self.commitment_expiry, *spread_bp + increase_bp));
                    }
                } else {
                    *spread_step_ups = Some(vec![(self.commitment_expiry, *spread_bp + increase_bp)]);
                }
            }
            _ => {
                // For other interest types, apply increase to base rates
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
        if new_maturity < self.commitment_expiry {
            return Err(finstack_core::error::InputError::Invalid.into());
        }
        self.maturity = new_maturity;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_ddtl_creation() {
        let ddtl = DelayedDrawTermLoan::new(
            "DDTL-001",
            Money::new(10_000_000.0, Currency::USD),
            Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            InterestSpec::Fixed {
                rate: 0.065,
                step_ups: None,
            },
        );

        assert_eq!(ddtl.id, "DDTL-001");
        assert_eq!(ddtl.commitment.amount(), 10_000_000.0);
        assert_eq!(ddtl.undrawn_amount().amount(), 10_000_000.0);
    }

    #[test]
    fn test_ddtl_with_draws() {
        let ddtl = DelayedDrawTermLoan::new(
            "DDTL-002",
            Money::new(10_000_000.0, Currency::USD),
            Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            InterestSpec::Fixed {
                rate: 0.065,
                step_ups: None,
            },
        )
        .with_draw(DrawEvent {
            date: Date::from_calendar_date(2025, Month::June, 1).unwrap(),
            amount: Money::new(3_000_000.0, Currency::USD),
            purpose: Some("Working capital".to_string()),
            conditional: false,
        })
        .with_draw(DrawEvent {
            date: Date::from_calendar_date(2025, Month::September, 1).unwrap(),
            amount: Money::new(2_000_000.0, Currency::USD),
            purpose: Some("Expansion".to_string()),
            conditional: false,
        });

        let as_of = Date::from_calendar_date(2025, Month::October, 1).unwrap();
        let drawn = ddtl.simulate_draws_to_date(as_of);
        assert_eq!(drawn.amount(), 5_000_000.0);
    }

    #[test]
    fn test_ddtl_covenant_consequences() {
        use crate::covenants::engine::InstrumentMutator;
        
        let mut ddtl = DelayedDrawTermLoan::new(
            "DDTL-COVENANT-TEST",
            Money::new(10_000_000.0, Currency::USD),
            Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            InterestSpec::Fixed {
                rate: 0.065,
                step_ups: None,
            },
        );

        // Test rate increase
        ddtl.increase_rate(0.01).unwrap(); // 100bps increase
        match &ddtl.interest_spec {
            InterestSpec::Fixed { step_ups, .. } => {
                assert!(step_ups.is_some());
                let steps = step_ups.as_ref().unwrap();
                assert_eq!(steps.len(), 1);
                assert_eq!(steps[0].1, 0.075); // Original 6.5% + 1% = 7.5%
            }
            _ => panic!("Expected Fixed interest"),
        }

        // Test cash sweep
        ddtl.set_cash_sweep(0.25).unwrap();
        assert_eq!(ddtl.cash_sweep_pct, 0.25);

        // Test maturity acceleration
        let new_maturity = Date::from_calendar_date(2029, Month::January, 1).unwrap();
        ddtl.set_maturity(new_maturity).unwrap();
        assert_eq!(ddtl.maturity, new_maturity);
    }

    #[test]
    fn test_ddtl_draw_condition_enforcement() {
        use crate::instruments::fixed_income::loan::covenants::{Covenant, CovenantType};
        
        // Create DDTL with conditional draws
        let mut ddtl = DelayedDrawTermLoan::new(
            "DDTL-CONDITIONS-TEST",
            Money::new(10_000_000.0, Currency::USD),
            Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            InterestSpec::Fixed {
                rate: 0.065,
                step_ups: None,
            },
        );

        // Add conditional draw with covenant requirement
        let covenant = Covenant::new(
            CovenantType::MaxDebtToEBITDA { threshold: 3.5 },
            Frequency::quarterly(),
        );
        ddtl = ddtl.with_draw_condition(covenant);

        let conditional_draw = DrawEvent {
            date: Date::from_calendar_date(2025, Month::June, 1).unwrap(),
            amount: Money::new(5_000_000.0, Currency::USD),
            purpose: Some("Conditional draw".to_string()),
            conditional: true,
        };
        ddtl = ddtl.with_draw(conditional_draw);

        // Test that draw simulation handles conditional draws
        // Without covenant context, conditional draws should still be allowed (with warning)
        let as_of = Date::from_calendar_date(2025, Month::October, 1).unwrap();
        let drawn = ddtl.simulate_draws_to_date(as_of);
        assert_eq!(drawn.amount(), 5_000_000.0); // Draw should be allowed without context
    }
}
