//! Inflation-Linked Bond (ILB) types and implementation.

use crate::cashflow::traits::DatedFlows;
use crate::instruments::common::traits::Attributes;
use finstack_core::market_data::scalars::inflation_index::{InflationIndex, InflationLag};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::types::InstrumentId;
use finstack_core::F;

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};

use super::parameters::InflationLinkedBondParams;

/// Indexation method for inflation adjustment
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexationMethod {
    /// Canadian model (real yield, indexed principal and coupons)
    Canadian,
    /// US TIPS model (real yield, indexed principal and coupons)
    TIPS,
    /// UK model (nominal yield; indexed principal and coupons, no deflation floor)
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
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct InflationLinkedBond {
    /// Unique instrument identifier
    pub id: InstrumentId,
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
    pub disc_id: CurveId,
    /// Inflation index identifier
    pub inflation_id: CurveId,
    /// Quoted clean price (if available)
    pub quoted_clean: Option<F>,
    /// Additional attributes
    pub attributes: Attributes,
}

impl InflationLinkedBond {
    /// Create a new US TIPS bond using parameter structs
    pub fn new_tips(
        id: impl Into<InstrumentId>,
        bond_params: &InflationLinkedBondParams,
        disc_id: impl Into<CurveId>,
        inflation_id: impl Into<CurveId>,
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
            lag: IndexationMethod::TIPS.standard_lag(),
            deflation_protection: DeflationProtection::MaturityOnly,
            bdc: BusinessDayConvention::Following,
            stub: StubKind::None,
            calendar_id: None,
            disc_id: disc_id.into(),
            inflation_id: inflation_id.into(),
            quoted_clean: None,
            attributes: Attributes::new(),
        }
    }

    /// Create a new UK Index-Linked Gilt using parameter structs
    pub fn new_uk_linker(
        id: impl Into<InstrumentId>,
        bond_params: &InflationLinkedBondParams,
        base_date: Date,
        disc_id: impl Into<CurveId>,
        inflation_id: impl Into<CurveId>,
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
            lag: IndexationMethod::UK.standard_lag(),
            deflation_protection: DeflationProtection::None,
            bdc: BusinessDayConvention::Following,
            stub: StubKind::None,
            calendar_id: None,
            disc_id: disc_id.into(),
            inflation_id: inflation_id.into(),
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
        crate::instruments::inflation_linked_bond::pricing::InflationLinkedBondEngine::index_ratio(
            self,
            date,
            inflation_index,
        )
    }

    /// Build inflation-adjusted cashflow schedule
    pub fn build_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        crate::instruments::inflation_linked_bond::pricing::InflationLinkedBondEngine::build_schedule(
            self,
            curves,
            as_of,
        )
    }

    /// Calculate real yield (yield in real terms, before inflation)
    pub fn real_yield(
        &self,
        clean_price: F,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<F> {
        crate::instruments::inflation_linked_bond::pricing::InflationLinkedBondEngine::real_yield(
            self,
            clean_price,
            curves,
            as_of,
        )
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
    pub fn real_duration(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<F> {
        crate::instruments::inflation_linked_bond::pricing::InflationLinkedBondEngine::real_duration(
            self, curves, as_of,
        )
    }
}

// Explicit Instrument trait implementation (replaces macro for better IDE visibility)
impl crate::instruments::common::traits::Instrument for InflationLinkedBond {
    #[inline]
    fn id(&self) -> &str {
        self.id.as_str()
    }

    #[inline]
    fn instrument_type(&self) -> &'static str {
        "InflationLinkedBond"
    }

    #[inline]
    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    #[inline]
    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    #[inline]
    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    #[inline]
    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        // Route through helper for schedule-based PV calculation
        crate::instruments::common::helpers::schedule_pv_impl(
            self, curves, as_of, &self.disc_id, self.dc,
        )
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            self, curves, as_of, base_value, metrics,
        )
    }
}

impl crate::instruments::common::HasDiscountCurve for InflationLinkedBond {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.disc_id
    }
}

// CashflowProvider trait impl is defined in pricing engine to centralize pricing logic
