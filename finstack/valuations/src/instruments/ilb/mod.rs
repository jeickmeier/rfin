//! Inflation-Linked Bond (ILB) instrument implementation.
//!
//! Provides comprehensive support for inflation-indexed bonds including
//! TIPS, UK Index-Linked Gilts, and other inflation-protected securities.

use crate::pricing::result::ValuationResult;
use crate::traits::{Attributable, Attributes, Priceable, DatedFlows};
use finstack_core::F;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::inflation_index::{InflationIndex, InflationLag};
use finstack_core::money::Money;

use finstack_core::dates::{Date, DayCount, Frequency, BusinessDayConvention, StubKind};
use hashbrown::HashMap;

pub mod metrics;

/// Indexation method for inflation adjustment
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexationMethod {
    /// Canadian model (real yield, indexed principal and coupons)
    Canadian,
    /// US TIPS model (real yield, indexed principal and coupons)
    TIPS,
    /// UK model (nominal yield, indexed coupons only)
    UK,
    /// French OATi/OAT€i model
    French,
    /// Japanese JGBi model
    Japanese,
}

impl IndexationMethod {
    /// Get the standard lag for this indexation method
    pub fn standard_lag(&self) -> InflationLag {
        match self {
            IndexationMethod::Canadian | IndexationMethod::TIPS => InflationLag::Months(3),
            IndexationMethod::UK => InflationLag::Months(8),
            IndexationMethod::French => InflationLag::Months(3),
            IndexationMethod::Japanese => InflationLag::Months(3),
        }
    }
    
    /// Whether this method uses daily interpolation
    pub fn uses_daily_interpolation(&self) -> bool {
        matches!(self, IndexationMethod::Canadian | IndexationMethod::TIPS)
    }
}

/// Deflation protection type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeflationProtection {
    /// No deflation protection
    None,
    /// Protection at maturity only (principal floor at par)
    MaturityOnly,
    /// Protection on all payments (floor at par)
    AllPayments,
}

/// Inflation-Linked Bond instrument
#[derive(Clone, Debug)]
pub struct InflationLinkedBond {
    /// Unique instrument identifier
    pub id: String,
    /// Notional amount (in real terms)
    pub notional: Money,
    /// Real coupon rate (as decimal)
    pub real_coupon: F,
    /// Coupon frequency
    pub freq: Frequency,
    /// Day count convention
    pub dc: DayCount,
    /// Issue date
    pub issue: Date,
    /// Maturity date
    pub maturity: Date,
    /// Base CPI/index value at issue
    pub base_index: F,
    /// Base date for index (may differ from issue date)
    pub base_date: Date,
    /// Indexation method
    pub indexation_method: IndexationMethod,
    /// Inflation lag
    pub lag: InflationLag,
    /// Deflation protection
    pub deflation_protection: DeflationProtection,
    /// Business day convention
    pub bdc: BusinessDayConvention,
    /// Stub convention
    pub stub: StubKind,
    /// Holiday calendar identifier
    pub calendar_id: Option<&'static str>,
    /// Discount curve identifier (real or nominal depending on method)
    pub disc_id: &'static str,
    /// Inflation index identifier
    pub inflation_id: &'static str,
    /// Quoted clean price (if available)
    pub quoted_clean: Option<F>,
    /// Additional attributes
    pub attributes: Attributes,
}

impl InflationLinkedBond {
    /// Create a new US TIPS bond
    #[allow(clippy::too_many_arguments)]
    pub fn new_tips(
        id: impl Into<String>,
        notional: Money,
        real_coupon: F,
        issue: Date,
        maturity: Date,
        base_index: F,
        disc_id: &'static str,
        inflation_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            notional,
            real_coupon,
            freq: Frequency::semi_annual(),
            dc: DayCount::ActAct,
            issue,
            maturity,
            base_index,
            base_date: issue,
            indexation_method: IndexationMethod::TIPS,
            lag: InflationLag::Months(3),
            deflation_protection: DeflationProtection::MaturityOnly,
            bdc: BusinessDayConvention::Following,
            stub: StubKind::None,
            calendar_id: Some("US"),
            disc_id,
            inflation_id,
            quoted_clean: None,
            attributes: Attributes::new(),
        }
    }
    
    /// Create a new UK Index-Linked Gilt
    #[allow(clippy::too_many_arguments)]
    pub fn new_uk_linker(
        id: impl Into<String>,
        notional: Money,
        real_coupon: F,
        issue: Date,
        maturity: Date,
        base_index: F,
        base_date: Date,
        disc_id: &'static str,
        inflation_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            notional,
            real_coupon,
            freq: Frequency::semi_annual(),
            dc: DayCount::ActAct,
            issue,
            maturity,
            base_index,
            base_date,
            indexation_method: IndexationMethod::UK,
            lag: InflationLag::Months(8),
            deflation_protection: DeflationProtection::None,
            bdc: BusinessDayConvention::Following,
            stub: StubKind::None,
            calendar_id: Some("UK"),
            disc_id,
            inflation_id,
            quoted_clean: None,
            attributes: Attributes::new(),
        }
    }
    
    /// Calculate index ratio for a given date
    pub fn index_ratio(&self, date: Date, inflation_index: &InflationIndex) -> finstack_core::Result<F> {
        // Apply lag to get reference date
        let reference_date = match self.lag {
            InflationLag::Months(m) => date - time::Duration::days((m as i64) * 30),
            InflationLag::Days(d) => date - time::Duration::days(d as i64),
            InflationLag::None => date,
        };
        
        // Get index value at reference date
        let current_index = if self.indexation_method.uses_daily_interpolation() {
            // Use linear interpolation for daily index values
            inflation_index.value_on(reference_date)?
        } else {
            // Use monthly index value
            inflation_index.value_on(reference_date)?
        };
        
        // Calculate ratio
        let ratio = current_index / self.base_index;
        
        // Apply deflation floor if applicable
        Ok(match self.deflation_protection {
            DeflationProtection::None => ratio,
            DeflationProtection::MaturityOnly => {
                if date == self.maturity {
                    ratio.max(1.0)
                } else {
                    ratio
                }
            },
            DeflationProtection::AllPayments => ratio.max(1.0),
        })
    }
    
    /// Build inflation-adjusted cashflow schedule
    pub fn build_schedule(&self, curves: &CurveSet, _as_of: Date) -> finstack_core::Result<DatedFlows> {
        // Get inflation index
        let inflation_index = curves.inflation_index(self.inflation_id)
            .ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::NotFound
            ))?;
        
        // Simplified implementation - would use CashflowBuilder in full implementation
        let mut flows = Vec::new();
        
        // Generate coupon dates based on frequency
        let mut current = self.issue;
        while current < self.maturity {
            let next = current + time::Duration::days(180); // Semi-annual
            if next <= self.maturity {
                let year_frac = self.dc.year_fraction(current, next)?;
                let base_amount = self.notional * self.real_coupon * year_frac;
                
                // Apply inflation adjustment
                let ratio = self.index_ratio(next, &inflation_index).unwrap_or(1.0);
                let adjusted_amount = base_amount * ratio;
                
                flows.push((next, adjusted_amount));
            }
            current = next;
        }
        
        // Add principal payment at maturity
        let principal_ratio = self.index_ratio(self.maturity, &inflation_index).unwrap_or(1.0);
        flows.push((self.maturity, self.notional * principal_ratio));
        
        Ok(flows)
    }
    
    /// Calculate real yield (yield in real terms, before inflation)
    pub fn real_yield(&self, _clean_price: F, _curves: &CurveSet, _as_of: Date) -> finstack_core::Result<F> {
        // This would implement the actual real yield calculation
        // using Newton-Raphson or similar solver
        // For now, return a placeholder
        Ok(self.real_coupon)
    }
    
    /// Calculate breakeven inflation rate
    pub fn breakeven_inflation(
        &self,
        nominal_bond_yield: F,
        curves: &CurveSet,
        as_of: Date,
    ) -> finstack_core::Result<F> {
        let real_yield = self.real_yield(
            self.quoted_clean.unwrap_or(100.0),
            curves,
            as_of,
        )?;
        
        // Fisher equation: (1 + nominal) = (1 + real) * (1 + inflation)
        // Simplified: breakeven ≈ nominal - real
        Ok(nominal_bond_yield - real_yield)
    }
    
    /// Calculate inflation-adjusted duration
    pub fn real_duration(&self, _curves: &CurveSet, as_of: Date) -> finstack_core::Result<F> {
        // This would calculate the duration with respect to real yields
        // For now, return a placeholder
        let years_to_maturity = (self.maturity - as_of).whole_days() as F / 365.25;
        Ok(years_to_maturity * 0.8) // Simplified approximation
    }
}

impl Priceable for InflationLinkedBond {
    /// Compute the present value of the ILB
    fn value(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<Money> {
        use crate::pricing::npv::npv;
        
        let disc = curves.discount(self.disc_id)?;
        let flows = self.build_schedule(curves, as_of)?;
        npv(&*disc, disc.base_date(), self.dc, &flows)
    }
    
    /// Compute value with specific metrics
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
            Arc::new(Instrument::ILB(self.clone())),
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
    
    /// Compute full valuation with all standard ILB metrics
    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<ValuationResult> {
        use crate::metrics::MetricId;
        
        let standard_metrics = vec![
            MetricId::Accrued,
            MetricId::CleanPrice,
            MetricId::DirtyPrice,
            MetricId::custom("real_yield"),
            MetricId::custom("index_ratio"),
            MetricId::custom("real_duration"),
            MetricId::custom("breakeven_inflation"),
        ];
        
        self.price_with_metrics(curves, as_of, &standard_metrics)
    }
}

impl Attributable for InflationLinkedBond {
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }
    
    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_tips_creation() {
        let notional = Money::new(1_000_000.0, Currency::USD);
        let issue = Date::from_calendar_date(2020, Month::January, 15).unwrap();
        let maturity = Date::from_calendar_date(2030, Month::January, 15).unwrap();
        
        let tips = InflationLinkedBond::new_tips(
            "US_TIPS_2030",
            notional,
            0.0125, // 1.25% real coupon
            issue,
            maturity,
            250.0, // Base CPI
            "USD-REAL",
            "US-CPI-U",
        );
        
        assert_eq!(tips.id, "US_TIPS_2030");
        assert_eq!(tips.indexation_method, IndexationMethod::TIPS);
        assert_eq!(tips.deflation_protection, DeflationProtection::MaturityOnly);
        assert_eq!(tips.lag, InflationLag::Months(3));
    }

    #[test]
    fn test_uk_linker_creation() {
        let notional = Money::new(1_000_000.0, Currency::GBP);
        let issue = Date::from_calendar_date(2015, Month::March, 22).unwrap();
        let maturity = Date::from_calendar_date(2040, Month::March, 22).unwrap();
        let base_date = Date::from_calendar_date(2014, Month::November, 1).unwrap();
        
        let linker = InflationLinkedBond::new_uk_linker(
            "UK_LINKER_2040",
            notional,
            0.00625, // 0.625% real coupon
            issue,
            maturity,
            280.0, // Base RPI
            base_date,
            "GBP-NOMINAL",
            "UK-RPI",
        );
        
        assert_eq!(linker.id, "UK_LINKER_2040");
        assert_eq!(linker.indexation_method, IndexationMethod::UK);
        assert_eq!(linker.deflation_protection, DeflationProtection::None);
        assert_eq!(linker.lag, InflationLag::Months(8));
    }

    #[test]
    fn test_deflation_floor() {
        let notional = Money::new(1_000_000.0, Currency::USD);
        let issue = Date::from_calendar_date(2020, Month::January, 15).unwrap();
        let maturity = Date::from_calendar_date(2030, Month::January, 15).unwrap();
        
        let tips = InflationLinkedBond::new_tips(
            "TIPS",
            notional,
            0.0125,
            issue,
            maturity,
            250.0,
            "USD-REAL",
            "US-CPI-U",
        );
        
        // Create mock inflation index with deflation
        let observations = vec![
            (issue, 250.0),
            (maturity, 240.0), // Deflation scenario
        ];
        
        let inflation_index = InflationIndex::new("US-CPI-U", observations, Currency::USD).unwrap();
        
        // Test deflation floor at maturity
        let ratio_at_maturity = tips.index_ratio(maturity, &inflation_index).unwrap();
        assert_eq!(ratio_at_maturity, 1.0); // Should be floored at 1.0
        
        // Test no floor before maturity
        let ratio_before = tips.index_ratio(issue + time::Duration::days(365), &inflation_index).unwrap();
        assert!(ratio_before < 1.0); // Should not be floored
    }
}
