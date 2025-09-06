//! Inflation-Linked Bond (ILB) instrument implementation.
//!
//! Provides comprehensive support for inflation-indexed bonds including
//! TIPS, UK Index-Linked Gilts, and other inflation-protected securities.

// use crate::results::ValuationResult; // not needed with macro-based impl
use crate::cashflow::traits::DatedFlows;
use crate::instruments::traits::Attributes;
use finstack_core::market_data::inflation_index::{InflationIndex, InflationLag};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::F;

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};

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
    /// Create a new ILB builder.
    pub fn builder() -> ILBBuilder {
        ILBBuilder::new()
    }

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
    pub fn index_ratio(
        &self,
        date: Date,
        inflation_index: &InflationIndex,
    ) -> finstack_core::Result<F> {
        // Apply lag to get reference date
        let reference_date = match self.lag {
            InflationLag::Months(m) => finstack_core::dates::add_months(date, -(m as i32)),
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
            }
            DeflationProtection::AllPayments => ratio.max(1.0),
        })
    }

    /// Build inflation-adjusted cashflow schedule
    pub fn build_schedule(
        &self,
        curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // Get inflation index
        let inflation_index = curves.inflation_index(self.inflation_id).ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "inflation_linked_bond_quote".to_string(),
            })
        })?;

        // Use centralized schedule builder for coupon dates
        let sched = crate::cashflow::builder::build_dates(
            self.issue,
            self.maturity,
            self.freq,
            self.stub,
            self.bdc,
            self.calendar_id,
        );
        let dates = sched.dates;
        if dates.len() < 2 {
            return Ok(vec![]);
        }

        let mut flows = Vec::with_capacity(dates.len());
        let mut prev = dates[0];
        for &d in &dates[1..] {
            // Accrual over the period using standard DayCount
            let year_frac =
                self.dc
                    .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())?;
            let base_amount = self.notional * self.real_coupon * year_frac;

            // Apply inflation adjustment at payment date
            let ratio = self.index_ratio(d, &inflation_index).unwrap_or(1.0);
            let adjusted_amount = base_amount * ratio;
            flows.push((d, adjusted_amount));
            prev = d;
        }

        // Add principal payment at maturity (inflation-adjusted)
        let principal_ratio = self
            .index_ratio(self.maturity, &inflation_index)
            .unwrap_or(1.0);
        flows.push((self.maturity, self.notional * principal_ratio));

        Ok(flows)
    }

    /// Calculate real yield (yield in real terms, before inflation)
    pub fn real_yield(
        &self,
        clean_price: F,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<F> {
        // Real yield calculation requires iterative solving similar to YTM
        // For now, return the coupon rate only if price is at par
        if (clean_price - 100.0).abs() < 1e-6 {
            Ok(self.real_coupon)
        } else {
            // Proper real yield calculation not yet implemented
            Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            ))
        }
    }

    /// Calculate breakeven inflation rate
    pub fn breakeven_inflation(
        &self,
        nominal_bond_yield: F,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<F> {
        let real_yield = self.real_yield(self.quoted_clean.unwrap_or(100.0), curves, as_of)?;

        // Fisher equation: (1 + nominal) = (1 + real) * (1 + inflation)
        // Simplified: breakeven ≈ nominal - real
        Ok(nominal_bond_yield - real_yield)
    }

    /// Calculate inflation-adjusted duration
    pub fn real_duration(&self, _curves: &MarketContext, as_of: Date) -> finstack_core::Result<F> {
        // This would calculate the duration with respect to real yields
        // For now, return a placeholder
        let years_to_maturity = (self.maturity - as_of).whole_days() as F / 365.25;
        Ok(years_to_maturity * 0.8) // Simplified approximation
    }
}

impl_instrument_schedule_pv!(
    InflationLinkedBond, "InflationLinkedBond",
    disc_field: disc_id,
    dc_field: dc
);

// Provide the required CashflowProvider trait implementation used by the macro
impl crate::cashflow::traits::CashflowProvider for InflationLinkedBond {
    fn build_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::traits::DatedFlows> {
        // Delegate to the inherent method defined above
        InflationLinkedBond::build_schedule(self, curves, as_of)
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
            "TIPS", notional, 0.0125, issue, maturity, 250.0, "USD-REAL", "US-CPI-U",
        );

        // Create mock inflation index with deflation and intermediate points
        let observations = vec![
            (issue, 250.0),
            (issue + time::Duration::days(365), 249.0), // Year 1: slight deflation
            (issue + time::Duration::days(365 * 5), 245.0), // Year 5: more deflation
            (maturity, 240.0),                          // Final: deflation scenario
        ];

        let inflation_index = InflationIndex::new("US-CPI-U", observations, Currency::USD).unwrap();

        // Test deflation floor at maturity
        let ratio_at_maturity = tips.index_ratio(maturity, &inflation_index).unwrap();
        assert_eq!(ratio_at_maturity, 1.0); // Should be floored at 1.0

        // Test no floor before maturity (accounting for 3-month TIPS lag)
        let test_date = issue + time::Duration::days(365 + 90); // 1 year + 3 months to account for lag
        let ratio_before = tips.index_ratio(test_date, &inflation_index).unwrap();

        // Debug: check the reference date after applying lag
        let reference_date = test_date - time::Duration::days(90); // 3-month lag
        println!(
            "Test date: {:?}, Reference date (after lag): {:?}, Ratio: {}",
            test_date, reference_date, ratio_before
        );

        assert!(
            ratio_before < 1.0,
            "Ratio {} should be < 1.0 for deflation scenario",
            ratio_before
        ); // Should not be floored
    }
}

/// Builder pattern for ILB instruments
#[derive(Default)]
pub struct ILBBuilder {
    id: Option<String>,
    notional: Option<Money>,
    real_coupon: Option<F>,
    freq: Option<Frequency>,
    dc: Option<DayCount>,
    issue: Option<Date>,
    maturity: Option<Date>,
    base_index: Option<F>,
    base_date: Option<Date>,
    indexation_method: Option<IndexationMethod>,
    lag: Option<InflationLag>,
    deflation_protection: Option<DeflationProtection>,
    bdc: Option<BusinessDayConvention>,
    stub: Option<StubKind>,
    calendar_id: Option<&'static str>,
    disc_id: Option<&'static str>,
    inflation_id: Option<&'static str>,
    quoted_clean: Option<F>,
}

impl ILBBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }

    pub fn notional(mut self, value: Money) -> Self {
        self.notional = Some(value);
        self
    }

    pub fn real_coupon(mut self, value: F) -> Self {
        self.real_coupon = Some(value);
        self
    }

    pub fn freq(mut self, value: Frequency) -> Self {
        self.freq = Some(value);
        self
    }

    pub fn dc(mut self, value: DayCount) -> Self {
        self.dc = Some(value);
        self
    }

    pub fn issue(mut self, value: Date) -> Self {
        self.issue = Some(value);
        self
    }

    pub fn maturity(mut self, value: Date) -> Self {
        self.maturity = Some(value);
        self
    }

    pub fn base_index(mut self, value: F) -> Self {
        self.base_index = Some(value);
        self
    }

    pub fn base_date(mut self, value: Date) -> Self {
        self.base_date = Some(value);
        self
    }

    pub fn indexation_method(mut self, value: IndexationMethod) -> Self {
        self.indexation_method = Some(value);
        self
    }

    pub fn lag(mut self, value: InflationLag) -> Self {
        self.lag = Some(value);
        self
    }

    pub fn deflation_protection(mut self, value: DeflationProtection) -> Self {
        self.deflation_protection = Some(value);
        self
    }

    pub fn bdc(mut self, value: BusinessDayConvention) -> Self {
        self.bdc = Some(value);
        self
    }

    pub fn stub(mut self, value: StubKind) -> Self {
        self.stub = Some(value);
        self
    }

    pub fn calendar_id(mut self, value: &'static str) -> Self {
        self.calendar_id = Some(value);
        self
    }

    pub fn disc_id(mut self, value: &'static str) -> Self {
        self.disc_id = Some(value);
        self
    }

    pub fn inflation_id(mut self, value: &'static str) -> Self {
        self.inflation_id = Some(value);
        self
    }

    pub fn quoted_clean(mut self, value: F) -> Self {
        self.quoted_clean = Some(value);
        self
    }

    pub fn build(self) -> finstack_core::Result<InflationLinkedBond> {
        let issue = self
            .issue
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;

        Ok(InflationLinkedBond {
            id: self.id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            notional: self.notional.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            real_coupon: self.real_coupon.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            freq: self.freq.unwrap_or_else(Frequency::semi_annual),
            dc: self.dc.unwrap_or(DayCount::ActAct),
            issue,
            maturity: self.maturity.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            base_index: self.base_index.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            base_date: self.base_date.unwrap_or(issue),
            indexation_method: self.indexation_method.unwrap_or(IndexationMethod::TIPS),
            lag: self.lag.unwrap_or(InflationLag::Months(3)),
            deflation_protection: self
                .deflation_protection
                .unwrap_or(DeflationProtection::MaturityOnly),
            bdc: self.bdc.unwrap_or(BusinessDayConvention::Following),
            stub: self.stub.unwrap_or(StubKind::None),
            calendar_id: self.calendar_id,
            disc_id: self.disc_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            inflation_id: self.inflation_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            quoted_clean: self.quoted_clean,
            attributes: Attributes::new(),
        })
    }
}
