//! Inflation-Linked Bond (ILB) types and implementation.

use crate::cashflow::traits::DatedFlows;
use crate::instruments::common::InflationLinkedBondParams;
use crate::instruments::traits::Attributes;
use finstack_core::market_data::scalars::inflation_index::{InflationIndex, InflationLag};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::F;

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};

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
    pub fn builder() -> crate::instruments::fixed_income::inflation_linked_bond::builder::ILBBuilder
    {
        crate::instruments::fixed_income::inflation_linked_bond::builder::ILBBuilder::new()
    }

    /// Create a new US TIPS bond using parameter structs
    pub fn new_tips(
        id: impl Into<String>,
        bond_params: &InflationLinkedBondParams,
        disc_id: &'static str,
        inflation_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            notional: bond_params.notional,
            real_coupon: bond_params.real_coupon,
            freq: bond_params.frequency,
            dc: bond_params.day_count,
            issue: bond_params.issue,
            maturity: bond_params.maturity,
            base_index: bond_params.base_index,
            base_date: bond_params.issue,
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

    /// Create a new UK Index-Linked Gilt using parameter structs
    pub fn new_uk_linker(
        id: impl Into<String>,
        bond_params: &InflationLinkedBondParams,
        base_date: Date,
        disc_id: &'static str,
        inflation_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            notional: bond_params.notional,
            real_coupon: bond_params.real_coupon,
            freq: bond_params.frequency,
            dc: bond_params.day_count,
            issue: bond_params.issue,
            maturity: bond_params.maturity,
            base_index: bond_params.base_index,
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
            _ => date,
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
