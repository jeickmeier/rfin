//! Revolving Credit Facility implementation.

use crate::cashflow::builder::{cf, FloatingCouponSpec, CouponType, FeeSpec, FeeBase};
use super::term_loan::InterestSpec;
use super::covenants::Covenant;
use crate::pricing::result::ValuationResult;
use crate::pricing::discountable::Discountable;
use crate::traits::{CashflowProvider, Priceable};
use finstack_core::dates::{Date, DayCount, Frequency, BusinessDayConvention, StubKind};
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::money::Money;
use finstack_core::F;
use hashbrown::HashMap;

/// Utilization fee tier.
#[derive(Clone, Debug)]
pub struct UtilizationTier {
    /// Minimum utilization percentage for this tier
    pub min_utilization: F,
    /// Maximum utilization percentage for this tier
    pub max_utilization: F,
    /// Fee rate in basis points for this tier
    pub fee_rate_bp: F,
}

/// Utilization-based fee schedule.
#[derive(Clone, Debug)]
pub struct UtilizationFeeSchedule {
    /// Tiers in ascending order of utilization
    pub tiers: Vec<UtilizationTier>,
}

impl UtilizationFeeSchedule {
    /// Creates a new utilization fee schedule.
    pub fn new() -> Self {
        Self { tiers: Vec::new() }
    }

    /// Adds a tier to the schedule.
    pub fn with_tier(mut self, min: F, max: F, rate_bp: F) -> Self {
        self.tiers.push(UtilizationTier {
            min_utilization: min,
            max_utilization: max,
            fee_rate_bp: rate_bp,
        });
        self
    }

    /// Gets the fee rate for a given utilization percentage.
    pub fn get_rate(&self, utilization: F) -> F {
        for tier in &self.tiers {
            if utilization >= tier.min_utilization && utilization < tier.max_utilization {
                return tier.fee_rate_bp;
            }
        }
        0.0
    }
}

impl Default for UtilizationFeeSchedule {
    fn default() -> Self {
        Self::new()
    }
}

/// Draw or repayment event.
#[derive(Clone, Debug)]
pub struct DrawRepayEvent {
    /// Date of the event
    pub date: Date,
    /// Amount (positive for draws, negative for repayments)
    pub amount: Money,
    /// Whether this is mandatory
    pub mandatory: bool,
    /// Purpose/description
    pub description: Option<String>,
}

/// Expected funding curve for revolving credit facility pricing.
#[derive(Clone, Debug)]
pub struct RevolverFundingCurve {
    /// Expected future draw/repay events for pricing
    pub expected_events: Vec<DrawRepayEvent>,
    /// Probability of each event occurring (optional, defaults to 1.0)
    pub event_probabilities: Option<Vec<F>>,
}

impl RevolverFundingCurve {
    /// Create a new expected funding curve.
    pub fn new(expected_events: Vec<DrawRepayEvent>) -> Self {
        Self {
            expected_events,
            event_probabilities: None,
        }
    }
    
    /// Create with probabilities for each event.
    pub fn with_probabilities(expected_events: Vec<DrawRepayEvent>, probabilities: Vec<F>) -> Self {
        Self {
            expected_events,
            event_probabilities: Some(probabilities),
        }
    }
}

/// Revolving Credit Facility instrument.
#[derive(Clone, Debug)]
pub struct RevolvingCreditFacility {
    /// Unique identifier
    pub id: String,
    /// Borrower entity ID
    pub borrower: String,
    /// Total commitment amount
    pub commitment: Money,
    /// Currently drawn amount
    pub drawn_amount: Money,
    /// Period during which draws are allowed
    pub availability_start: Date,
    pub availability_end: Date,
    /// Final maturity date
    pub maturity: Date,
    /// Interest specification on drawn amounts
    pub interest_spec: InterestSpec,
    /// Commitment fee rate on undrawn amounts (annual)
    pub commitment_fee_rate: F,
    /// Utilization-based fee schedule
    pub utilization_fees: Option<UtilizationFeeSchedule>,
    /// Draw and repayment schedule
    pub draw_repay_schedule: Vec<DrawRepayEvent>,
    /// Expected funding curve for pricing (future expected draws/repayments)
    pub expected_funding_curve: Option<RevolverFundingCurve>,
    /// Financial covenants
    pub covenants: Vec<Covenant>,
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

impl RevolvingCreditFacility {
    /// Creates a new revolving credit facility.
    pub fn new(
        id: impl Into<String>,
        commitment: Money,
        availability_start: Date,
        availability_end: Date,
        maturity: Date,
    ) -> Self {
        Self {
            id: id.into(),
            borrower: String::new(),
            commitment,
            drawn_amount: Money::new(0.0, commitment.currency()),
            availability_start,
            availability_end,
            maturity,
            interest_spec: InterestSpec::Floating {
                index_id: "USD-SOFR-3M",
                spread_bp: 250.0,
                spread_step_ups: None,
                gearing: 1.0,
                reset_lag_days: 2,
            },
            commitment_fee_rate: 0.0035, // 35 bps default
            utilization_fees: None,
            draw_repay_schedule: Vec::new(),
            expected_funding_curve: None,
            covenants: Vec::new(),
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

    /// Returns the utilization percentage.
    pub fn utilization(&self) -> F {
        if self.commitment.amount() > 0.0 {
            self.drawn_amount.amount() / self.commitment.amount()
        } else {
            0.0
        }
    }

    /// Sets the interest specification.
    pub fn with_interest(mut self, spec: InterestSpec) -> Self {
        self.interest_spec = spec;
        self
    }

    /// Sets the commitment fee rate.
    pub fn with_commitment_fee(mut self, rate: F) -> Self {
        self.commitment_fee_rate = rate;
        self
    }

    /// Sets the utilization fee schedule.
    pub fn with_utilization_fees(mut self, schedule: UtilizationFeeSchedule) -> Self {
        self.utilization_fees = Some(schedule);
        self
    }

    /// Adds a draw or repayment event.
    pub fn with_event(mut self, event: DrawRepayEvent) -> Self {
        self.draw_repay_schedule.push(event);
        self
    }

    /// Adds a covenant.
    pub fn with_covenant(mut self, covenant: Covenant) -> Self {
        self.covenants.push(covenant);
        self
    }
    
    /// Set expected funding curve for pricing.
    pub fn with_expected_funding_curve(mut self, curve: RevolverFundingCurve) -> Self {
        self.expected_funding_curve = Some(curve);
        self
    }
    
    /// Add expected events for pricing.
    pub fn with_expected_events(mut self, events: Vec<DrawRepayEvent>) -> Self {
        self.expected_funding_curve = Some(RevolverFundingCurve::new(events));
        self
    }

    /// Get all expected future events for pricing (includes scheduled + expected).
    fn get_expected_future_events(&self, as_of: Date) -> Vec<(Date, Money, F)> {
        let mut future_events = Vec::new();
        
        // Add scheduled events that are after as_of
        for event in &self.draw_repay_schedule {
            if event.date > as_of && event.date <= self.maturity {
                future_events.push((event.date, event.amount, 1.0));
            }
        }
        
        // Add expected events from funding curve
        if let Some(ref curve) = self.expected_funding_curve {
            for (i, event) in curve.expected_events.iter().enumerate() {
                if event.date > as_of && event.date <= self.maturity {
                    let prob = curve.event_probabilities.as_ref()
                        .and_then(|probs| probs.get(i))
                        .copied()
                        .unwrap_or(1.0);
                    future_events.push((event.date, event.amount, prob));
                }
            }
        }
        
        // Sort by date
        future_events.sort_by_key(|(date, _, _)| *date);
        future_events
    }
    
    /// Simulates the drawn amount up to a given date.
    fn simulate_drawn_to_date(&self, as_of: Date) -> Money {
        let mut drawn = self.drawn_amount;
        
        for event in &self.draw_repay_schedule {
            if event.date <= as_of {
                let new_drawn = drawn.amount() + event.amount.amount();
                // Ensure we don't go negative or exceed commitment
                let new_drawn = new_drawn.max(0.0).min(self.commitment.amount());
                drawn = Money::new(new_drawn, self.commitment.currency());
            }
        }
        
        drawn
    }

    /// Builds the cashflow schedule.
    fn build_cashflows(&self, as_of: Date) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        let drawn = self.simulate_drawn_to_date(as_of);
        let undrawn = Money::new(
            self.commitment.amount() - drawn.amount(),
            self.commitment.currency()
        );

        let mut builder = cf();
        
        // Set up the principal (drawn amount, repaid at maturity)
        builder.principal(drawn, self.availability_start, self.maturity);

        // Add interest on drawn amount
        if drawn.amount() > 0.0 {
            match &self.interest_spec {
                InterestSpec::Floating { index_id, spread_bp, gearing, reset_lag_days, .. } => {
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
                },
                _ => {
                    // Handle other interest types
                }
            }
        }

        // Add commitment fee on undrawn amount
        if undrawn.amount() > 0.0 && as_of <= self.availability_end {
            let fee_spec = FeeSpec::PeriodicBps {
                base: FeeBase::Undrawn { 
                    facility_limit: self.commitment 
                },
                bps: self.commitment_fee_rate * 10000.0,
                freq: self.frequency,
                dc: self.day_count,
                bdc: self.bdc,
                calendar_id: self.calendar_id,
                stub: self.stub,
            };
            builder.fee(fee_spec);
        }

        // Add utilization fee if applicable
        if let Some(util_schedule) = &self.utilization_fees {
            let utilization = self.utilization();
            let util_rate_bp = util_schedule.get_rate(utilization);
            if util_rate_bp > 0.0 {
                let fee_spec = FeeSpec::PeriodicBps {
                    base: FeeBase::Drawn,
                    bps: util_rate_bp,
                    freq: self.frequency,
                    dc: self.day_count,
                    bdc: self.bdc,
                    calendar_id: self.calendar_id,
                    stub: self.stub,
                };
                builder.fee(fee_spec);
            }
        }

        builder.build()
    }
}

impl CashflowProvider for RevolvingCreditFacility {
    fn build_schedule(&self, _curves: &CurveSet, as_of: Date) -> finstack_core::Result<Vec<(Date, Money)>> {
        let schedule = self.build_cashflows(as_of)?;
        
        let mut flows = Vec::new();
        for cf in &schedule.flows {
            flows.push((cf.date, cf.amount));
        }
        
        Ok(flows)
    }
}

impl Priceable for RevolvingCreditFacility {
    fn value(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<Money> {
        let disc = curves.discount(self.disc_id)?;
        let mut total_npv = 0.0;
        
        // 1. Value existing drawn amount and scheduled cashflows
        let existing_flows = self.build_schedule(curves, as_of)?;
        let existing_npv = existing_flows.npv(&*disc, disc.base_date(), self.day_count)?;
        total_npv += existing_npv.amount();
        
        // 2. Value expected future draws and repayments
        let future_events = self.get_expected_future_events(as_of);
        let mut projected_drawn = self.simulate_drawn_to_date(as_of);
        
        for (event_date, event_amount, probability) in future_events {
            // Update projected drawn amount
            let new_drawn = projected_drawn.amount() + event_amount.amount();
            
            // Ensure drawn amount stays within bounds [0, commitment]
            let new_drawn = new_drawn.max(0.0).min(self.commitment.amount());
            projected_drawn = Money::new(new_drawn, self.commitment.currency());
            
            // For each future event, value the change in interest payments
            let event_df = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::df_on(
                &*disc,
                disc.base_date(),
                event_date,
                self.day_count,
            );
            
            if event_amount.amount() > 0.0 {
                // Draw event - value as negative cashflow (funding outflow)
                total_npv -= event_amount.amount() * event_df * probability;
                
                // Add value of future interest on the additional drawn amount
                if let InterestSpec::Fixed { rate, .. } = &self.interest_spec {
                    let remaining_years = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::year_fraction(event_date, self.maturity, self.day_count);
                    let interest_value = event_amount.amount() * rate * remaining_years;
                    total_npv += interest_value * event_df * probability;
                }
            } else {
                // Repayment event - value as positive cashflow
                total_npv += event_amount.amount().abs() * event_df * probability;
                
                // Reduce future interest payments
                if let InterestSpec::Fixed { rate, .. } = &self.interest_spec {
                    let remaining_years = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::year_fraction(event_date, self.maturity, self.day_count);
                    let interest_savings = event_amount.amount().abs() * rate * remaining_years;
                    total_npv -= interest_savings * event_df * probability;
                }
            }
        }
        
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
        
        // Add facility metrics
        let mut measures = HashMap::new();
        measures.insert("drawn".to_string(), self.simulate_drawn_to_date(as_of).amount());
        measures.insert("undrawn".to_string(), self.undrawn_amount().amount());
        measures.insert("commitment".to_string(), self.commitment.amount());
        measures.insert("utilization".to_string(), self.utilization());
        
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
    fn test_revolver_creation() {
        let revolver = RevolvingCreditFacility::new(
            "RCF-001",
            Money::new(50_000_000.0, Currency::USD),
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Date::from_calendar_date(2028, Month::January, 1).unwrap(),
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        );

        assert_eq!(revolver.id, "RCF-001");
        assert_eq!(revolver.commitment.amount(), 50_000_000.0);
        assert_eq!(revolver.undrawn_amount().amount(), 50_000_000.0);
        assert_eq!(revolver.utilization(), 0.0);
    }

    #[test]
    fn test_utilization_fee_schedule() {
        let schedule = UtilizationFeeSchedule::new()
            .with_tier(0.0, 0.33, 10.0)   // < 33% utilization: 10 bps
            .with_tier(0.33, 0.66, 15.0)  // 33-66% utilization: 15 bps
            .with_tier(0.66, 1.0, 25.0);  // > 66% utilization: 25 bps

        assert_eq!(schedule.get_rate(0.2), 10.0);
        assert_eq!(schedule.get_rate(0.5), 15.0);
        assert_eq!(schedule.get_rate(0.8), 25.0);
    }

    #[test]
    fn test_revolver_with_draws() {
        let revolver = RevolvingCreditFacility::new(
            "RCF-002",
            Money::new(50_000_000.0, Currency::USD),
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Date::from_calendar_date(2028, Month::January, 1).unwrap(),
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        )
        .with_event(DrawRepayEvent {
            date: Date::from_calendar_date(2025, Month::March, 1).unwrap(),
            amount: Money::new(10_000_000.0, Currency::USD),
            mandatory: false,
            description: Some("Initial draw".to_string()),
        })
        .with_event(DrawRepayEvent {
            date: Date::from_calendar_date(2025, Month::June, 1).unwrap(),
            amount: Money::new(5_000_000.0, Currency::USD),
            mandatory: false,
            description: Some("Additional draw".to_string()),
        })
        .with_event(DrawRepayEvent {
            date: Date::from_calendar_date(2025, Month::September, 1).unwrap(),
            amount: Money::new(-3_000_000.0, Currency::USD),
            mandatory: true,
            description: Some("Mandatory repayment".to_string()),
        });

        let as_of = Date::from_calendar_date(2025, Month::October, 1).unwrap();
        let drawn = revolver.simulate_drawn_to_date(as_of);
        
        assert_eq!(drawn.amount(), 12_000_000.0); // 10M + 5M - 3M
        assert_eq!(revolver.simulate_drawn_to_date(as_of).amount(), 12_000_000.0);
    }
}
