//! Delayed-Draw Term Loan (DDTL) implementation.

use crate::cashflow::builder::{cf, FixedCouponSpec, CouponType, FeeSpec, FeeBase};
use crate::cashflow::amortization_notional::AmortizationSpec;
use crate::instruments::loan::term_loan::InterestSpec;
use crate::instruments::loan::covenants::Covenant;
use crate::pricing::result::ValuationResult;
use crate::pricing::discountable::Discountable;
use crate::traits::{CashflowProvider, Priceable};
use finstack_core::dates::{Date, DayCount, Frequency, BusinessDayConvention, StubKind};
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::money::Money;
use finstack_core::F;
use hashbrown::HashMap;

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
        }
    }

    /// Returns the undrawn amount.
    pub fn undrawn_amount(&self) -> Money {
        Money::new(
            self.commitment.amount() - self.drawn_amount.amount(),
            self.commitment.currency()
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
        let mut drawn = self.drawn_amount;
        
        for draw in &self.planned_draws {
            if draw.date <= as_of && draw.date <= self.commitment_expiry {
                // In a full implementation, would check draw conditions here
                if !draw.conditional {
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
                    let prob = curve.draw_probabilities.as_ref()
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
    fn build_cashflows(&self, as_of: Date) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        // Simulate draws to determine outstanding amount
        let drawn = self.simulate_draws_to_date(as_of);
        let undrawn = Money::new(
            self.commitment.amount() - drawn.amount(),
            self.commitment.currency()
        );

        let mut builder = cf();
        
        // For the drawn portion, create a term loan
        if drawn.amount() > 0.0 {
            let issue = self.planned_draws.first()
                .map(|d| d.date)
                .unwrap_or(as_of);
            
            builder.principal(drawn, issue, self.maturity);
            builder.amortization(self.amortization.clone());

            // Add interest on drawn amount
            match &self.interest_spec {
                InterestSpec::Fixed { rate, .. } => {
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
                },
                _ => {
                    // Other interest types would be handled similarly
                }
            }
        } else {
            // If nothing drawn yet, just set up an empty schedule
            builder.principal(
                Money::new(0.0, self.commitment.currency()),
                as_of,
                self.maturity
            );
        }

        // Add commitment fee on undrawn amount
        if undrawn.amount() > 0.0 && as_of < self.commitment_expiry {
            let fee_spec = FeeSpec::PeriodicBps {
                base: FeeBase::Undrawn { 
                    facility_limit: self.commitment 
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
                        facility_limit: self.commitment 
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
    fn build_schedule(&self, _curves: &CurveSet, as_of: Date) -> finstack_core::Result<Vec<(Date, Money)>> {
        let schedule = self.build_cashflows(as_of)?;
        
        let mut flows = Vec::new();
        for cf in &schedule.flows {
            flows.push((cf.date, cf.amount));
        }
        
        Ok(flows)
    }
}

impl Priceable for DelayedDrawTermLoan {
    fn value(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<Money> {
        let disc = curves.discount(self.disc_id)?;
        let mut total_npv = 0.0;
        
        // 1. Value existing drawn amount
        let existing_flows = self.build_schedule(curves, as_of)?;
        let existing_npv = existing_flows.npv(&*disc, disc.base_date(), self.day_count)?;
        total_npv += existing_npv.amount();
        
        // 2. Value expected future draws
        let future_draws = self.get_expected_future_draws(as_of);
        for (draw_date, draw_amount, probability) in future_draws {
            // For each future draw, we need to value:
            // a) The negative cashflow of the draw itself (funding outflow from lender perspective)
            let draw_yf = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::year_fraction(disc.base_date(), draw_date, self.day_count);
            let draw_df = disc.df(draw_yf);
            total_npv -= draw_amount.amount() * draw_df * probability;
            
            // b) The positive value of future interest payments on that draw
            // This is simplified - in practice would build full schedule from draw_date to maturity
            let remaining_years = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::year_fraction(draw_date, self.maturity, self.day_count);
            if let InterestSpec::Fixed { rate, .. } = &self.interest_spec {
                // Approximate value of future interest payments
                let interest_value = draw_amount.amount() * rate * remaining_years;
                total_npv += interest_value * draw_df * probability;
                
                // Add principal repayment at maturity
                let maturity_yf = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::year_fraction(disc.base_date(), self.maturity, self.day_count);
                let maturity_df = disc.df(maturity_yf);
                total_npv += draw_amount.amount() * maturity_df * probability;
            }
        }
        
        // 3. Value commitment fees on undrawn amounts
        // This would be more complex in practice, accounting for the changing undrawn balance
        
        Ok(Money::new(total_npv, self.commitment.currency()))
    }

    fn price_with_metrics(
        &self,
        curves: &CurveSet,
        as_of: Date,
        _metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        
        let mut result = ValuationResult::stamped(&self.id, as_of, base_value);
        
        // Add some basic metrics
        let mut measures = HashMap::new();
        measures.insert("drawn".to_string(), self.simulate_draws_to_date(as_of).amount());
        measures.insert("undrawn".to_string(), self.undrawn_amount().amount());
        measures.insert("commitment".to_string(), self.commitment.amount());
        
        result = result.with_measures(measures);
        Ok(result)
    }

    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<ValuationResult> {
        self.price_with_metrics(curves, as_of, &[])
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
            InterestSpec::Fixed { rate: 0.065, step_ups: None },
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
            InterestSpec::Fixed { rate: 0.065, step_ups: None },
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
}
