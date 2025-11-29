//! BCBS-IOSCO regulatory schedule-based IM calculator.
//!
//! Fallback methodology using grid-based rates applied to notional amounts.
//! Simpler but typically more conservative than SIMM.

use crate::instruments::common::traits::Instrument;
use crate::margin::calculators::traits::{ImCalculator, ImResult};
use crate::margin::types::ImMethodology;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use std::collections::HashMap;

/// Asset class for schedule-based IM calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ScheduleAssetClass {
    /// Interest rate derivatives
    InterestRate,
    /// Credit derivatives
    Credit,
    /// Equity derivatives
    Equity,
    /// Commodity derivatives
    Commodity,
    /// Foreign exchange derivatives
    Fx,
    /// Other derivatives
    Other,
}

impl std::fmt::Display for ScheduleAssetClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleAssetClass::InterestRate => write!(f, "interest_rate"),
            ScheduleAssetClass::Credit => write!(f, "credit"),
            ScheduleAssetClass::Equity => write!(f, "equity"),
            ScheduleAssetClass::Commodity => write!(f, "commodity"),
            ScheduleAssetClass::Fx => write!(f, "fx"),
            ScheduleAssetClass::Other => write!(f, "other"),
        }
    }
}

/// BCBS-IOSCO regulatory schedule for IM calculation.
///
/// Provides grid-based rates by asset class and maturity.
#[derive(Debug, Clone)]
pub struct RegulatorySchedule {
    /// IM rates by asset class and maturity bucket
    pub rates: HashMap<(ScheduleAssetClass, MaturityBucket), f64>,
}

/// Maturity bucket for schedule IM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaturityBucket {
    /// Less than 2 years
    Short,
    /// 2-5 years
    Medium,
    /// Greater than 5 years
    Long,
}

impl Default for RegulatorySchedule {
    fn default() -> Self {
        Self::bcbs_iosco()
    }
}

impl RegulatorySchedule {
    /// BCBS-IOSCO standard schedule.
    ///
    /// Reference: BCBS-IOSCO "Margin requirements for non-centrally cleared derivatives"
    /// Annex A, Table 1.
    #[must_use]
    pub fn bcbs_iosco() -> Self {
        let mut rates = HashMap::new();

        // Interest Rate
        rates.insert(
            (ScheduleAssetClass::InterestRate, MaturityBucket::Short),
            0.01,
        ); // 1%
        rates.insert(
            (ScheduleAssetClass::InterestRate, MaturityBucket::Medium),
            0.02,
        ); // 2%
        rates.insert(
            (ScheduleAssetClass::InterestRate, MaturityBucket::Long),
            0.04,
        ); // 4%

        // Credit
        rates.insert((ScheduleAssetClass::Credit, MaturityBucket::Short), 0.02); // 2%
        rates.insert((ScheduleAssetClass::Credit, MaturityBucket::Medium), 0.05); // 5%
        rates.insert((ScheduleAssetClass::Credit, MaturityBucket::Long), 0.10); // 10%

        // Equity
        rates.insert((ScheduleAssetClass::Equity, MaturityBucket::Short), 0.15); // 15%
        rates.insert((ScheduleAssetClass::Equity, MaturityBucket::Medium), 0.15);
        rates.insert((ScheduleAssetClass::Equity, MaturityBucket::Long), 0.15);

        // Commodity
        rates.insert((ScheduleAssetClass::Commodity, MaturityBucket::Short), 0.15);
        rates.insert(
            (ScheduleAssetClass::Commodity, MaturityBucket::Medium),
            0.15,
        );
        rates.insert((ScheduleAssetClass::Commodity, MaturityBucket::Long), 0.15);

        // FX
        rates.insert((ScheduleAssetClass::Fx, MaturityBucket::Short), 0.06); // 6%
        rates.insert((ScheduleAssetClass::Fx, MaturityBucket::Medium), 0.06);
        rates.insert((ScheduleAssetClass::Fx, MaturityBucket::Long), 0.06);

        // Other
        rates.insert((ScheduleAssetClass::Other, MaturityBucket::Short), 0.15);
        rates.insert((ScheduleAssetClass::Other, MaturityBucket::Medium), 0.15);
        rates.insert((ScheduleAssetClass::Other, MaturityBucket::Long), 0.15);

        Self { rates }
    }

    /// Get the IM rate for an asset class and maturity.
    #[must_use]
    pub fn rate(&self, asset_class: ScheduleAssetClass, maturity_years: f64) -> f64 {
        let bucket = if maturity_years < 2.0 {
            MaturityBucket::Short
        } else if maturity_years < 5.0 {
            MaturityBucket::Medium
        } else {
            MaturityBucket::Long
        };

        *self.rates.get(&(asset_class, bucket)).unwrap_or(&0.15)
    }
}

/// Schedule-based IM calculator.
///
/// Calculates initial margin using the BCBS-IOSCO regulatory schedule approach.
/// This is a simpler alternative to SIMM that applies grid-based rates to notional.
///
/// # Formula
///
/// ```text
/// IM = Notional × Schedule_Rate(asset_class, maturity)
/// ```
///
/// # Example
///
/// ```rust,ignore
/// use finstack_valuations::margin::{ScheduleImCalculator, ScheduleAssetClass};
///
/// let calc = ScheduleImCalculator::bcbs_standard();
/// let im = calc.calculate(&swap, &context, as_of)?;
/// ```
#[derive(Debug, Clone)]
pub struct ScheduleImCalculator {
    /// Regulatory schedule
    pub schedule: RegulatorySchedule,
    /// Default asset class to assume
    pub default_asset_class: ScheduleAssetClass,
    /// Default maturity in years
    pub default_maturity_years: f64,
    /// Margin period of risk (days)
    pub mpor_days: u32,
}

impl Default for ScheduleImCalculator {
    fn default() -> Self {
        Self::bcbs_standard()
    }
}

impl ScheduleImCalculator {
    /// Create calculator with BCBS-IOSCO standard schedule.
    #[must_use]
    pub fn bcbs_standard() -> Self {
        Self {
            schedule: RegulatorySchedule::bcbs_iosco(),
            default_asset_class: ScheduleAssetClass::InterestRate,
            default_maturity_years: 5.0,
            mpor_days: 10,
        }
    }

    /// Set default asset class.
    #[must_use]
    pub fn with_asset_class(mut self, asset_class: ScheduleAssetClass) -> Self {
        self.default_asset_class = asset_class;
        self
    }

    /// Set default maturity.
    #[must_use]
    pub fn with_maturity(mut self, years: f64) -> Self {
        self.default_maturity_years = years;
        self
    }

    /// Calculate IM for a given notional, asset class, and maturity.
    pub fn calculate_for_notional(
        &self,
        notional: Money,
        asset_class: ScheduleAssetClass,
        maturity_years: f64,
    ) -> Money {
        let rate = self.schedule.rate(asset_class, maturity_years);
        Money::new(notional.amount().abs(), notional.currency()) * rate
    }

    /// Get the schedule rate for an asset class and maturity.
    #[must_use]
    pub fn rate(&self, asset_class: ScheduleAssetClass, maturity_years: f64) -> f64 {
        self.schedule.rate(asset_class, maturity_years)
    }
}

impl ImCalculator for ScheduleImCalculator {
    fn calculate(
        &self,
        instrument: &dyn Instrument,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<ImResult> {
        // Use PV as proxy for notional (simplified)
        let pv = instrument.value(context, as_of)?;
        let currency = pv.currency();
        let notional = Money::new(pv.amount().abs(), currency);

        let rate = self
            .schedule
            .rate(self.default_asset_class, self.default_maturity_years);
        let im_amount = notional * rate;

        let mut breakdown = HashMap::new();
        breakdown.insert(self.default_asset_class.to_string(), im_amount);

        Ok(ImResult::with_breakdown(
            im_amount,
            ImMethodology::Schedule,
            as_of,
            self.mpor_days,
            breakdown,
        ))
    }

    fn methodology(&self) -> ImMethodology {
        ImMethodology::Schedule
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;

    #[test]
    fn bcbs_schedule_rates() {
        let schedule = RegulatorySchedule::bcbs_iosco();

        // Interest rate
        assert_eq!(schedule.rate(ScheduleAssetClass::InterestRate, 1.0), 0.01); // 1%
        assert_eq!(schedule.rate(ScheduleAssetClass::InterestRate, 3.0), 0.02); // 2%
        assert_eq!(schedule.rate(ScheduleAssetClass::InterestRate, 10.0), 0.04); // 4%

        // Credit
        assert_eq!(schedule.rate(ScheduleAssetClass::Credit, 1.0), 0.02);
        assert_eq!(schedule.rate(ScheduleAssetClass::Credit, 10.0), 0.10);

        // Equity (constant)
        assert_eq!(schedule.rate(ScheduleAssetClass::Equity, 1.0), 0.15);
        assert_eq!(schedule.rate(ScheduleAssetClass::Equity, 10.0), 0.15);
    }

    #[test]
    fn schedule_im_calculation() {
        let calc = ScheduleImCalculator::bcbs_standard();

        let notional = Money::new(100_000_000.0, Currency::USD);
        let im = calc.calculate_for_notional(notional, ScheduleAssetClass::InterestRate, 5.0);

        // 5y IR uses long bucket (4%) since maturity >= 5.0
        assert_eq!(im.amount(), 4_000_000.0);
    }

    #[test]
    fn credit_schedule_im() {
        let calc = ScheduleImCalculator::bcbs_standard()
            .with_asset_class(ScheduleAssetClass::Credit)
            .with_maturity(7.0);

        let notional = Money::new(50_000_000.0, Currency::USD);
        let im = calc.calculate_for_notional(notional, ScheduleAssetClass::Credit, 7.0);

        // 7y credit uses long bucket (10%)
        assert_eq!(im.amount(), 5_000_000.0);
    }
}
