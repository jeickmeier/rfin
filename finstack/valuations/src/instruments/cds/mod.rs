//! Credit Default Swap (CDS) instrument implementation with ISDA conventions.
//!
//! Provides comprehensive CDS valuation including par spread calculation,
//! risky PV01, CS01, and protection leg valuation.

use crate::pricing::result::ValuationResult;
use crate::traits::{Attributable, Attributes, Priceable, DatedFlows};

use finstack_core::F;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::term_structures::credit_curve::CreditCurve;
use finstack_core::market_data::traits::Discount;
use finstack_core::money::Money;
use finstack_core::dates::{Date, DayCount, BusinessDayConvention, Frequency, StubKind};
use hashbrown::HashMap;

pub mod metrics;

/// CDS payment types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayReceive {
    /// Protection buyer pays premium leg
    PayProtection,
    /// Protection seller receives premium leg
    ReceiveProtection,
}

/// ISDA CDS conventions
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CDSConvention {
    /// Standard North American convention (quarterly, Act/360)
    IsdaNa,
    /// Standard European convention (quarterly, Act/360)
    IsdaEu,
    /// Standard Asian convention (quarterly, Act/365)
    IsdaAs,
    /// Custom convention
    Custom,
}

impl CDSConvention {
    /// Get the standard day count convention
    pub fn day_count(&self) -> DayCount {
        match self {
            CDSConvention::IsdaNa | CDSConvention::IsdaEu => DayCount::Act360,
            CDSConvention::IsdaAs => DayCount::Act365F,
            CDSConvention::Custom => DayCount::Act360, // Default
        }
    }

    /// Get the standard payment frequency
    pub fn frequency(&self) -> Frequency {
        Frequency::quarterly()
    }

    /// Get the standard business day convention
    pub fn business_day_convention(&self) -> BusinessDayConvention {
        BusinessDayConvention::Following
    }

    /// Get the standard stub convention
    pub fn stub_convention(&self) -> StubKind {
        StubKind::ShortFront
    }
}

/// Settlement type for CDS protection payment
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettlementType {
    /// Physical delivery of defaulted bonds
    Physical,
    /// Cash settlement based on recovery rate
    Cash,
    /// Auction-based settlement
    Auction,
}

/// Premium leg specification
#[derive(Clone, Debug)]
pub struct PremiumLegSpec {
    /// Start date of protection
    pub start: Date,
    /// End date of protection
    pub end: Date,
    /// Payment frequency
    pub freq: Frequency,
    /// Stub convention
    pub stub: StubKind,
    /// Business day convention
    pub bdc: BusinessDayConvention,
    /// Holiday calendar identifier
    pub calendar_id: Option<&'static str>,
    /// Day count convention
    pub dc: DayCount,
    /// Fixed spread in basis points
    pub spread_bp: F,
    /// Discount curve identifier
    pub disc_id: &'static str,
}

/// Protection leg specification
#[derive(Clone, Debug)]
pub struct ProtectionLegSpec {
    /// Credit curve identifier for default probabilities
    pub credit_id: &'static str,
    /// Recovery rate (0.0 to 1.0)
    pub recovery_rate: F,
    /// Settlement type on default
    pub settlement: SettlementType,
    /// Settlement delay in business days
    pub settlement_delay: u16,
}

/// Credit Default Swap instrument
#[derive(Clone, Debug)]
pub struct CreditDefaultSwap {
    /// Unique instrument identifier
    pub id: String,
    /// Notional amount
    pub notional: Money,
    /// Reference entity (issuer being protected)
    pub reference_entity: String,
    /// Buyer/seller perspective
    pub side: PayReceive,
    /// ISDA convention
    pub convention: CDSConvention,
    /// Premium leg specification
    pub premium: PremiumLegSpec,
    /// Protection leg specification
    pub protection: ProtectionLegSpec,
    /// Upfront payment (if any)
    pub upfront: Option<Money>,
    /// Additional attributes
    pub attributes: Attributes,
}

impl CreditDefaultSwap {
    /// Create a new CDS with standard ISDA conventions
    #[allow(clippy::too_many_arguments)]
    pub fn new_isda(
        id: impl Into<String>,
        notional: Money,
        reference_entity: impl Into<String>,
        side: PayReceive,
        convention: CDSConvention,
        start: Date,
        end: Date,
        spread_bp: F,
        credit_id: &'static str,
        recovery_rate: F,
        disc_id: &'static str,
    ) -> Self {
        let dc = convention.day_count();
        let freq = convention.frequency();
        let bdc = convention.business_day_convention();
        let stub = convention.stub_convention();

        Self {
            id: id.into(),
            notional,
            reference_entity: reference_entity.into(),
            side,
            convention,
            premium: PremiumLegSpec {
                start,
                end,
                freq,
                stub,
                bdc,
                calendar_id: None,
                dc,
                spread_bp,
                disc_id,
            },
            protection: ProtectionLegSpec {
                credit_id,
                recovery_rate,
                settlement: SettlementType::Cash,
                settlement_delay: 3,
            },
            upfront: None,
            attributes: Attributes::new(),
        }
    }

    /// Build premium leg cashflows
    pub fn build_premium_schedule(&self, _curves: &CurveSet, _as_of: Date) -> finstack_core::Result<DatedFlows> {
        // Simplified implementation - would use CashflowBuilder in full implementation
        let mut flows = Vec::new();
        
        // Generate payment dates based on frequency
        let mut current = self.premium.start;
        while current < self.premium.end {
            let next = current + time::Duration::days(90); // Quarterly
            if next <= self.premium.end {
                let year_frac = self.premium.dc.year_fraction(current, next)?;
                let amount = self.notional * (self.premium.spread_bp / 10000.0) * year_frac;
                flows.push((next, amount));
            }
            current = next;
        }
        
        Ok(flows)
    }

    /// Calculate premium leg PV
    pub fn pv_premium_leg(&self, disc: &dyn Discount, surv: &CreditCurve) -> finstack_core::Result<Money> {
        let flows = self.build_premium_schedule(&CurveSet::default(), disc.base_date())?;
        
        // Calculate risky PV by adjusting for survival probability
        let mut pv = Money::new(0.0, self.notional.currency());
        
        for (pay_date, amount) in flows.iter() {
            let t = self.premium.dc.year_fraction(disc.base_date(), *pay_date)?;
            let df = disc.df(t);
            let surv_prob = survival_probability(surv, t)?;
            pv = (pv + *amount * (df * surv_prob))?;
        }
        
        Ok(pv)
    }

    /// Calculate protection leg PV
    pub fn pv_protection_leg(&self, disc: &dyn Discount, credit: &CreditCurve) -> finstack_core::Result<Money> {
        // Protection payment = Notional * (1 - Recovery) * Default Probability
        let lgd = 1.0 - self.protection.recovery_rate; // Loss given default
        
        // Discretize protection leg calculation (quarterly for accuracy)
        let dt = 0.25; // Quarterly steps
        let num_steps = ((self.premium.end - self.premium.start).whole_days() as F / 365.25 / dt).ceil() as usize;
        
        let mut pv = Money::new(0.0, self.notional.currency());
        let _base_date = disc.base_date();
        
        for i in 0..num_steps {
            let t1 = i as F * dt;
            let t2 = ((i + 1) as F * dt).min(
                (self.premium.end - self.premium.start).whole_days() as F / 365.25
            );
            
            if t2 <= t1 {
                break;
            }
            
            // Survival probabilities
            let surv1 = survival_probability(credit, t1)?;
            let surv2 = survival_probability(credit, t2)?;
            
            // Default probability in period
            let default_prob = surv1 - surv2;
            
            // Discount factor at mid-point (assuming default at mid-period)
            let t_mid = (t1 + t2) / 2.0;
            let df = disc.df(t_mid);
            
            // Protection payment
            pv = (pv + self.notional * (lgd * default_prob * df))?;
        }
        
        Ok(pv)
    }

    /// Calculate par spread (spread that makes PV = 0)
    pub fn par_spread(&self, disc: &dyn Discount, credit: &CreditCurve) -> finstack_core::Result<F> {
        // Par spread = Protection Leg PV / Risky Annuity
        let protection_pv = self.pv_protection_leg(disc, credit)?;
        let risky_annuity = self.risky_annuity(disc, credit)?;
        
        if risky_annuity == 0.0 {
            return Err(finstack_core::Error::from(
                finstack_core::error::InputError::NonPositiveValue
            ));
        }
        
        // Convert to basis points
        Ok(protection_pv.amount() / risky_annuity * 10000.0)
    }

    /// Calculate risky annuity (premium leg PV per bp)
    pub fn risky_annuity(&self, disc: &dyn Discount, credit: &CreditCurve) -> finstack_core::Result<F> {
        // Create a 1bp CDS to get the annuity
        let mut unit_cds = self.clone();
        unit_cds.premium.spread_bp = 1.0;
        unit_cds.notional = Money::new(1.0, self.notional.currency());
        
        let pv = unit_cds.pv_premium_leg(disc, credit)?;
        Ok(pv.amount())
    }

    /// Calculate risky PV01 (change in PV for 1bp spread change)
    pub fn risky_pv01(&self, disc: &dyn Discount, credit: &CreditCurve) -> finstack_core::Result<F> {
        self.risky_annuity(disc, credit)
    }

    /// Calculate CS01 (change in PV for 1bp credit spread change)
    pub fn cs01(&self, curves: &CurveSet) -> finstack_core::Result<F> {
        let disc = curves.discount(self.premium.disc_id)?;
        let credit = curves.credit(self.protection.credit_id)?;
        
        // Base PV
        let base_pv = self.value(curves, disc.base_date())?;
        
        // Bump credit spread by 1bp
        let mut bumped_credit = (*credit).clone();
        for spread in &mut bumped_credit.spreads_bp {
            *spread += 1.0;
        }
        
        // Create bumped curve set
        let mut bumped_curves = curves.clone();
        bumped_curves.add_credit(bumped_credit);
        
        // Bumped PV
        let bumped_pv = self.value(&bumped_curves, disc.base_date())?;
        
        // CS01 is the difference
        Ok((bumped_pv - base_pv)?.amount())
    }
}

impl Priceable for CreditDefaultSwap {
    /// Compute the present value of the CDS
    fn value(&self, curves: &CurveSet, _as_of: Date) -> finstack_core::Result<Money> {
        let disc = curves.discount(self.premium.disc_id)?;
        let credit = curves.credit(self.protection.credit_id)?;
        
        let pv_premium = self.pv_premium_leg(&*disc, &credit)?;
        let pv_protection = self.pv_protection_leg(&*disc, &credit)?;
        
        let pv = match self.side {
            PayReceive::PayProtection => (pv_protection - pv_premium)?,
            PayReceive::ReceiveProtection => (pv_premium - pv_protection)?,
        };
        
        // Add upfront payment if any
        if let Some(upfront) = self.upfront {
            Ok((pv + upfront)?)
        } else {
            Ok(pv)
        }
    }

    /// Compute value with specific metrics using the metrics framework
    fn price_with_metrics(
        &self,
        curves: &CurveSet,
        as_of: Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        use crate::instruments::Instrument;
        use crate::metrics::{MetricContext, standard_registry};
        use std::sync::Arc;
        
        // Compute base value
        let base_value = self.value(curves, as_of)?;
        
        // Create metric context
        let mut context = MetricContext::new(
            Arc::new(Instrument::CDS(self.clone())),
            Arc::new(curves.clone()),
            as_of,
            base_value,
        );
        
        // Get registry and compute requested metrics
        let registry = standard_registry();
        let metric_measures = registry.compute(metrics, &mut context)?;
        
        // Convert MetricId keys to String keys for ValuationResult
        let measures: HashMap<String, F> = metric_measures
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v))
            .collect();
        
        // Create result
        let mut result = ValuationResult::stamped(self.id.clone(), as_of, base_value);
        result.measures = measures;
        
        Ok(result)
    }

    /// Compute full valuation with all standard CDS metrics
    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<ValuationResult> {
        use crate::metrics::MetricId;
        
        let standard_metrics = vec![
            MetricId::ParSpread,
            MetricId::RiskyPv01,
            MetricId::Cs01,
            MetricId::ProtectionLegPv,
            MetricId::PremiumLegPv,
        ];
        
        self.price_with_metrics(curves, as_of, &standard_metrics)
    }
}

impl Attributable for CreditDefaultSwap {
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }
    
    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

// Helper function to calculate survival probability from credit curve
fn survival_probability(credit: &CreditCurve, t: F) -> finstack_core::Result<F> {
    if t <= 0.0 {
        return Ok(1.0);
    }
    
    // Convert spread to hazard rate (simplified)
    let spread_decimal = credit.spread_bp(t) / 10000.0;
    let hazard_rate = spread_decimal / (1.0 - credit.recovery_rate);
    
    // Survival probability = exp(-hazard_rate * t)
    Ok((-hazard_rate * t).exp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_cds_creation() {
        let notional = Money::new(10_000_000.0, Currency::USD);
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2030, Month::January, 1).unwrap();
        
        let cds = CreditDefaultSwap::new_isda(
            "CDS001",
            notional,
            "ABC Corp",
            PayReceive::PayProtection,
            CDSConvention::IsdaNa,
            start,
            end,
            100.0, // 100bp spread
            "ABC-SENIOR",
            0.4, // 40% recovery
            "USD-OIS",
        );
        
        assert_eq!(cds.id, "CDS001");
        assert_eq!(cds.reference_entity, "ABC Corp");
        assert_eq!(cds.premium.spread_bp, 100.0);
        assert_eq!(cds.protection.recovery_rate, 0.4);
    }

    #[test]
    fn test_isda_conventions() {
        assert_eq!(CDSConvention::IsdaNa.day_count(), DayCount::Act360);
        assert_eq!(CDSConvention::IsdaEu.day_count(), DayCount::Act360);
        assert_eq!(CDSConvention::IsdaAs.day_count(), DayCount::Act365F);
        assert_eq!(CDSConvention::IsdaNa.frequency(), Frequency::quarterly());
    }
}
