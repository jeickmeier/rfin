//! Inflation-Linked Bond (ILB) types and implementation.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::discountable::Discountable;
use crate::instruments::common::traits::Attributes;
use finstack_core::currency::Currency;
use finstack_core::dates::{
    BusinessDayConvention, Date, DateExt, DayCount, DayCountCtx, StubKind, Tenor,
};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::{InflationIndex, InflationInterpolation, InflationLag};
use finstack_core::market_data::term_structures::InflationCurve;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::types::InstrumentId;
use finstack_core::Result;
use std::sync::Arc;
use time::Duration;

use super::parameters::InflationLinkedBondParams;

/// Indexation method for inflation adjustment
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

impl std::fmt::Display for IndexationMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexationMethod::Canadian => write!(f, "canadian"),
            IndexationMethod::TIPS => write!(f, "tips"),
            IndexationMethod::UK => write!(f, "uk"),
            IndexationMethod::French => write!(f, "french"),
            IndexationMethod::Japanese => write!(f, "japanese"),
        }
    }
}

impl std::str::FromStr for IndexationMethod {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "canadian" => Ok(IndexationMethod::Canadian),
            "tips" | "us" => Ok(IndexationMethod::TIPS),
            "uk" => Ok(IndexationMethod::UK),
            "french" => Ok(IndexationMethod::French),
            "japanese" | "jgb" => Ok(IndexationMethod::Japanese),
            other => Err(format!("Unknown indexation method: {}", other)),
        }
    }
}

impl IndexationMethod {
    /// Get the standard lag for this indexation method.
    ///
    /// # UK Gilt Convention Notes
    ///
    /// For `IndexationMethod::UK`, this returns the **legacy 8-month lag** which applies to
    /// UK Index-Linked Gilts issued **before September 2005**. For modern Gilts issued
    /// **on or after September 2005**, use [`IndexationMethod::standard_lag_modern`] which
    /// returns the 3-month lag consistent with international standards.
    ///
    /// | Issue Date | Indexation Lag | Interpolation |
    /// |------------|----------------|---------------|
    /// | Before Sep 2005 | 8 months | Step (monthly) |
    /// | Sep 2005 onwards | 3 months | Linear (daily) |
    ///
    /// # Production Recommendation
    ///
    /// When pricing UK Index-Linked Gilts, verify the bond's issue date:
    /// - Use [`new_uk_linker`](InflationLinkedBond::new_uk_linker) for legacy (8-month lag)
    /// - Use [`new_uk_linker_modern`](InflationLinkedBond::new_uk_linker_modern) for modern (3-month lag)
    pub fn standard_lag(&self) -> InflationLag {
        match self {
            IndexationMethod::Canadian | IndexationMethod::TIPS => InflationLag::Months(3),
            IndexationMethod::UK => InflationLag::Months(8), // Legacy UK Gilts (pre-Sep 2005)
            IndexationMethod::French => InflationLag::Months(3),
            IndexationMethod::Japanese => InflationLag::Months(3),
        }
    }

    /// Get the modern indexation lag for markets that have transitioned.
    ///
    /// # UK Gilt Modern Convention
    ///
    /// UK Index-Linked Gilts issued **on or after September 2005** use a 3-month lag
    /// with daily linear interpolation, aligning with TIPS and other international linkers.
    ///
    /// For legacy UK Gilts (pre-September 2005), use [`standard_lag`](Self::standard_lag).
    pub fn standard_lag_modern(&self) -> InflationLag {
        match self {
            IndexationMethod::UK => InflationLag::Months(3), // Modern UK Gilts (Sep 2005+)
            _ => self.standard_lag(),
        }
    }

    /// Whether this method uses daily interpolation.
    ///
    /// # UK Gilt Note
    ///
    /// For UK Gilts, this returns `false` (step interpolation) which applies to legacy bonds.
    /// Modern UK Index-Linked Gilts (post-Sep 2005) use daily linear interpolation;
    /// use [`uses_daily_interpolation_modern`](Self::uses_daily_interpolation_modern) for those.
    pub fn uses_daily_interpolation(&self) -> bool {
        matches!(self, IndexationMethod::Canadian | IndexationMethod::TIPS)
    }

    /// Whether modern issuances of this method use daily interpolation.
    ///
    /// Modern UK Index-Linked Gilts (September 2005 onwards) switched to daily linear
    /// interpolation, matching TIPS and other international linkers.
    pub fn uses_daily_interpolation_modern(&self) -> bool {
        matches!(
            self,
            IndexationMethod::Canadian
                | IndexationMethod::TIPS
                | IndexationMethod::UK
                | IndexationMethod::French
        )
    }
}

/// Deflation protection type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DeflationProtection {
    /// No deflation protection
    None,
    /// Protection at maturity only (principal floor at par)
    MaturityOnly,
    /// Protection on all payments (floor at par)
    AllPayments,
}

impl std::fmt::Display for DeflationProtection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeflationProtection::None => write!(f, "none"),
            DeflationProtection::MaturityOnly => write!(f, "maturity_only"),
            DeflationProtection::AllPayments => write!(f, "all_payments"),
        }
    }
}

impl std::str::FromStr for DeflationProtection {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "none" => Ok(DeflationProtection::None),
            "maturity_only" | "maturity" => Ok(DeflationProtection::MaturityOnly),
            "all_payments" | "all" => Ok(DeflationProtection::AllPayments),
            other => Err(format!("Unknown deflation protection: {}", other)),
        }
    }
}

#[derive(Clone)]
enum InflationSource {
    Index(Arc<InflationIndex>),
    Curve(Arc<InflationCurve>),
}

impl InflationSource {
    fn from_market(curves: &MarketContext, id: &CurveId) -> Result<Self> {
        if let Some(index) = curves.inflation_index(id.as_str()) {
            Ok(Self::Index(index))
        } else {
            let curve = curves.get_inflation(id.as_str())?;
            Ok(Self::Curve(curve))
        }
    }

    fn ratio(&self, bond: &InflationLinkedBond, date: Date) -> Result<f64> {
        match self {
            Self::Index(index) => bond.index_ratio(date, index.as_ref()),
            Self::Curve(curve) => bond.index_ratio_from_curve(date, curve.as_ref()),
        }
    }
}

/// Inflation-Linked Bond instrument
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct InflationLinkedBond {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Notional amount (in real terms)
    pub notional: Money,
    /// Real coupon rate (as decimal)
    pub real_coupon: f64,
    /// Coupon frequency
    pub freq: Tenor,
    /// Day count convention
    pub dc: DayCount,
    /// Issue date
    pub issue: Date,
    /// Maturity date
    pub maturity: Date,
    /// Base CPI/index value at issue
    pub base_index: f64,
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
    pub calendar_id: Option<String>,
    /// Discount curve identifier (real or nominal depending on method)
    pub discount_curve_id: CurveId,
    /// Inflation index identifier
    pub inflation_index_id: CurveId,
    /// Quoted clean price (if available)
    pub quoted_clean: Option<f64>,
    /// Additional attributes
    pub attributes: Attributes,
}

impl InflationLinkedBond {
    /// Create a canonical example US TIPS inflation-linked bond.
    ///
    /// Returns a 10-year TIPS with semi-annual coupons and standard 3-month lag.
    ///
    /// # Market Conventions (US TIPS)
    ///
    /// - **Day Count**: ACT/ACT ICMA (per Treasury market standards)
    /// - **Frequency**: Semi-annual
    /// - **Indexation Lag**: 3 months
    /// - **Interpolation**: Linear (daily)
    /// - **Deflation Protection**: Maturity only (principal floor at par)
    pub fn example() -> Self {
        use time::macros::date;
        Self {
            id: InstrumentId::new("TIPS-10Y"),
            notional: Money::new(1_000_000.0, Currency::USD),
            real_coupon: 0.025,
            freq: Tenor::semi_annual(),
            dc: DayCount::ActActIsma, // US Treasury convention
            issue: date!(2024 - 01 - 15),
            maturity: date!(2034 - 01 - 15),
            base_index: 100.0,
            base_date: date!(2024 - 01 - 15),
            indexation_method: IndexationMethod::TIPS,
            lag: IndexationMethod::TIPS.standard_lag(),
            deflation_protection: DeflationProtection::MaturityOnly,
            bdc: BusinessDayConvention::Following,
            stub: StubKind::None,
            calendar_id: None,
            discount_curve_id: CurveId::new("USD-TIPS"),
            inflation_index_id: CurveId::new("US-CPI"),
            quoted_clean: None,
            attributes: Attributes::new(),
        }
    }

    /// Create a new US TIPS bond using parameter structs
    pub fn new_tips(
        id: impl Into<InstrumentId>,
        bond_params: &InflationLinkedBondParams,
        discount_curve_id: impl Into<CurveId>,
        inflation_index_id: impl Into<CurveId>,
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
            discount_curve_id: discount_curve_id.into(),
            inflation_index_id: inflation_index_id.into(),
            quoted_clean: None,
            attributes: Attributes::new(),
        }
    }

    /// Create a **legacy** UK Index-Linked Gilt (pre-September 2005) using parameter structs.
    ///
    /// # ⚠️ Important: Legacy vs Modern UK Gilts
    ///
    /// This constructor creates a linker with the **8-month lag** convention used for
    /// UK Index-Linked Gilts issued **before September 2005**. For gilts issued on or
    /// after September 2005, use [`new_uk_linker_modern`](Self::new_uk_linker_modern).
    ///
    /// # Market Conventions (Legacy UK Index-Linked Gilt)
    ///
    /// - **Day Count**: User-specified (typically ACT/ACT ICMA)
    /// - **Frequency**: User-specified (typically semi-annual)
    /// - **Indexation Lag**: 8 months
    /// - **Interpolation**: Step (monthly, no daily interpolation)
    /// - **Deflation Protection**: None (no floor)
    /// - **Index**: UK RPI (Retail Price Index)
    ///
    /// # Example Legacy Gilts
    ///
    /// - 2.5% IL Treasury Gilt 2020 (ISIN: GB0009081828)
    /// - 4.125% IL Treasury Gilt 2030 (ISIN: GB0031790826)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::fixed_income::inflation_linked_bond::{
    ///     InflationLinkedBond, InflationLinkedBondParams,
    /// };
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{DayCount, Tenor};
    /// use finstack_core::money::Money;
    /// use time::macros::date;
    ///
    /// let params = InflationLinkedBondParams {
    ///     notional: Money::new(1_000_000.0, Currency::GBP),
    ///     real_coupon: 0.025,  // 2.5% real coupon
    ///     frequency: Tenor::semi_annual(),
    ///     day_count: DayCount::ActActIsma,
    ///     issue: date!(1999-07-26),  // Pre-2005 issue
    ///     maturity: date!(2020-07-26),
    ///     base_index: 162.9,
    /// };
    ///
    /// let gilt = InflationLinkedBond::new_uk_linker(
    ///     "UKTI-2020",
    ///     &params,
    ///     date!(1999-07-26),
    ///     "GBP-REAL",
    ///     "UK-RPI",
    /// );
    /// ```
    pub fn new_uk_linker(
        id: impl Into<InstrumentId>,
        bond_params: &InflationLinkedBondParams,
        base_date: Date,
        discount_curve_id: impl Into<CurveId>,
        inflation_index_id: impl Into<CurveId>,
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
            lag: IndexationMethod::UK.standard_lag(), // 8-month lag for legacy gilts
            deflation_protection: DeflationProtection::None,
            bdc: BusinessDayConvention::Following,
            stub: StubKind::None,
            calendar_id: None,
            discount_curve_id: discount_curve_id.into(),
            inflation_index_id: inflation_index_id.into(),
            quoted_clean: None,
            attributes: Attributes::new(),
        }
    }

    /// Create a **modern** UK Index-Linked Gilt (September 2005 onwards) using parameter structs.
    ///
    /// # Market Conventions (Modern UK Index-Linked Gilt)
    ///
    /// UK Index-Linked Gilts issued on or after **September 2005** adopted a new
    /// indexation methodology aligned with international standards (TIPS, Canadian RRBs):
    ///
    /// - **Day Count**: User-specified (typically ACT/ACT ICMA)
    /// - **Frequency**: User-specified (typically semi-annual)
    /// - **Indexation Lag**: 3 months (changed from 8 months)
    /// - **Interpolation**: Linear (daily interpolation between monthly readings)
    /// - **Deflation Protection**: None (no floor)
    /// - **Index**: UK RPI (Retail Price Index)
    ///
    /// # Why the Change?
    ///
    /// The UK Debt Management Office (DMO) changed the indexation methodology to:
    /// 1. Improve pricing transparency with daily accruals
    /// 2. Align with international inflation-linked bond standards
    /// 3. Reduce basis risk vs TIPS and other linkers
    ///
    /// # Example Modern Gilts
    ///
    /// - 0.125% IL Treasury Gilt 2024 (ISIN: GB00B3LZBF68)
    /// - 0.125% IL Treasury Gilt 2029 (ISIN: GB00B3Y1JG82)
    /// - 0.625% IL Treasury Gilt 2040 (ISIN: GB00B3MYD345)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::fixed_income::inflation_linked_bond::{
    ///     InflationLinkedBond, InflationLinkedBondParams,
    /// };
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{DayCount, Tenor};
    /// use finstack_core::money::Money;
    /// use time::macros::date;
    ///
    /// let params = InflationLinkedBondParams {
    ///     notional: Money::new(1_000_000.0, Currency::GBP),
    ///     real_coupon: 0.00125,  // 0.125% real coupon
    ///     frequency: Tenor::semi_annual(),
    ///     day_count: DayCount::ActActIsma,
    ///     issue: date!(2019-11-22),  // Post-2005 issue
    ///     maturity: date!(2029-11-22),
    ///     base_index: 294.318,  // RPI at issue
    /// };
    ///
    /// let gilt = InflationLinkedBond::new_uk_linker_modern(
    ///     "UKTI-2029",
    ///     &params,
    ///     "GBP-REAL",
    ///     "UK-RPI",
    /// );
    /// ```
    ///
    /// # Production Note
    ///
    /// When pricing modern UK gilts, ensure the inflation index provided uses
    /// **linear interpolation** (daily), not step interpolation. The pricer will
    /// validate this at runtime.
    pub fn new_uk_linker_modern(
        id: impl Into<InstrumentId>,
        bond_params: &InflationLinkedBondParams,
        discount_curve_id: impl Into<CurveId>,
        inflation_index_id: impl Into<CurveId>,
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
            base_date: bond_params.issue, // Modern gilts use issue date as base
            indexation_method: IndexationMethod::UK,
            lag: IndexationMethod::UK.standard_lag_modern(), // 3-month lag for modern gilts
            deflation_protection: DeflationProtection::None,
            bdc: BusinessDayConvention::Following,
            stub: StubKind::None,
            calendar_id: None,
            discount_curve_id: discount_curve_id.into(),
            inflation_index_id: inflation_index_id.into(),
            quoted_clean: None,
            attributes: Attributes::new(),
        }
    }

    /// Create a new Japanese Government Inflation-Linked Bond (JGBi) using parameter structs
    ///
    /// # Market Conventions (JGBi)
    ///
    /// - **Day Count**: ACT/365F (per MOF Japan standards)
    /// - **Frequency**: Semi-annual
    /// - **Indexation Lag**: 3 months
    /// - **Interpolation**: Step (monthly, no daily interpolation)
    /// - **Deflation Protection**: Maturity only (principal floor at par)
    /// - **Index**: Japan CPI (ex-fresh food)
    ///
    /// # Arguments
    ///
    /// * `id` - Unique instrument identifier
    /// * `bond_params` - Bond parameters (notional, coupon, dates, base_index)
    /// * `discount_curve_id` - Real rate discount curve identifier
    /// * `inflation_index_id` - Japan CPI index identifier
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::fixed_income::inflation_linked_bond::{
    ///     InflationLinkedBond, InflationLinkedBondParams,
    /// };
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{DayCount, Tenor};
    /// use finstack_core::money::Money;
    /// use time::macros::date;
    ///
    /// let params = InflationLinkedBondParams {
    ///     notional: Money::new(100_000_000.0, Currency::JPY),
    ///     real_coupon: 0.001,  // 0.1% real coupon
    ///     frequency: Tenor::semi_annual(),
    ///     day_count: DayCount::Act365F,
    ///     issue: date!(2024-03-10),
    ///     maturity: date!(2034-03-10),
    ///     base_index: 105.0,
    /// };
    ///
    /// let jgbi = InflationLinkedBond::new_jgbi("JGBi-10Y", &params, "JPY-REAL", "JP-CPI");
    /// ```
    pub fn new_jgbi(
        id: impl Into<InstrumentId>,
        bond_params: &InflationLinkedBondParams,
        discount_curve_id: impl Into<CurveId>,
        inflation_index_id: impl Into<CurveId>,
    ) -> Self {
        Self {
            id: id.into(),
            notional: bond_params.notional,
            real_coupon: bond_params.real_coupon,
            freq: bond_params.frequency,
            dc: DayCount::Act365F, // JGB standard day count
            issue: bond_params.issue,
            maturity: bond_params.maturity,
            base_index: bond_params.base_index,
            base_date: bond_params.issue,
            indexation_method: IndexationMethod::Japanese,
            lag: IndexationMethod::Japanese.standard_lag(),
            deflation_protection: DeflationProtection::MaturityOnly,
            bdc: BusinessDayConvention::Following,
            stub: StubKind::None,
            calendar_id: Some("jpto".to_string()),
            discount_curve_id: discount_curve_id.into(),
            inflation_index_id: inflation_index_id.into(),
            quoted_clean: None,
            attributes: Attributes::new(),
        }
    }

    fn inflation_source(&self, curves: &MarketContext) -> Result<InflationSource> {
        InflationSource::from_market(curves, &self.inflation_index_id)
    }

    /// Calculate index ratio for a given date
    ///
    /// Validates that the inflation index interpolation method matches the
    /// market convention for the bond's indexation method and lag:
    ///
    /// | Method | Lag | Interpolation |
    /// |--------|-----|---------------|
    /// | TIPS/Canadian | 3 months | Linear (daily) |
    /// | UK Gilt (legacy) | 8 months | Step (monthly) |
    /// | UK Gilt (modern) | 3 months | Linear (daily) |
    /// | French OAT€i | 3 months | Linear (daily) |
    /// | Japanese JGBi | 3 months | Step (monthly) |
    ///
    /// # Validation
    ///
    /// The method validates that the provided `InflationIndex` uses the correct
    /// interpolation method. For UK gilts, the lag value is used to determine
    /// whether legacy (8-month) or modern (3-month) conventions apply.
    pub fn index_ratio(&self, date: Date, inflation_index: &InflationIndex) -> Result<f64> {
        // Validate interpolation policy vs indexation method for market standards
        // For UK gilts, determine legacy vs modern based on the lag
        let is_modern_uk = matches!(self.indexation_method, IndexationMethod::UK)
            && matches!(self.lag, InflationLag::Months(m) if m <= 3);

        match self.indexation_method {
            IndexationMethod::TIPS | IndexationMethod::Canadian | IndexationMethod::French => {
                // TIPS, Canadian RRBs, and French OAT€i/OATi use daily linear interpolation
                if inflation_index.interpolation() != InflationInterpolation::Linear {
                    return Err(finstack_core::InputError::Invalid.into());
                }
            }
            IndexationMethod::UK => {
                // UK gilts: legacy (8-month) uses Step, modern (3-month) uses Linear
                let expected_interp = if is_modern_uk {
                    InflationInterpolation::Linear
                } else {
                    InflationInterpolation::Step
                };
                if inflation_index.interpolation() != expected_interp {
                    return Err(finstack_core::InputError::Invalid.into());
                }
            }
            IndexationMethod::Japanese => {
                // Japanese JGBi uses step (monthly) interpolation
                if inflation_index.interpolation() != InflationInterpolation::Step {
                    return Err(finstack_core::InputError::Invalid.into());
                }
            }
        }

        // Apply lag to obtain the reference date in index space
        let reference_date = match self.lag {
            InflationLag::Months(m) => date.add_months(-(m as i32)),
            InflationLag::Days(d) => date - Duration::days(d as i64),
            InflationLag::None => date,
            _ => date,
        };

        // Value on reference date (interpolation policy controlled by index)
        let current_index = inflation_index.value_on(reference_date)?;

        // Ratio vs base
        if self.base_index <= 0.0 {
            return Err(finstack_core::InputError::NonPositiveValue.into());
        }
        let ratio = current_index / self.base_index;

        // Apply deflation protection per instrument policy
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

    /// Calculate index ratio using an inflation term structure when no index is available
    pub fn index_ratio_from_curve(
        &self,
        date: Date,
        inflation_curve: &InflationCurve,
    ) -> Result<f64> {
        let reference_date = match self.lag {
            InflationLag::Months(m) => date.add_months(-(m as i32)),
            InflationLag::Days(d) => date - Duration::days(d as i64),
            InflationLag::None => date,
            _ => date,
        };

        let current_index = if reference_date <= self.base_date {
            inflation_curve.base_cpi()
        } else {
            let t = DayCount::ActAct.year_fraction(
                self.base_date,
                reference_date,
                DayCountCtx::default(),
            )?;
            inflation_curve.cpi(t)
        };

        if self.base_index <= 0.0 {
            return Err(finstack_core::InputError::NonPositiveValue.into());
        }
        let ratio = current_index / self.base_index;

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

    /// Calculate index ratio sourcing inflation data from the market context
    pub fn index_ratio_from_market(&self, date: Date, curves: &MarketContext) -> Result<f64> {
        let source = self.inflation_source(curves)?;
        source.ratio(self, date)
    }

    /// Build inflation-adjusted cashflow schedule
    pub fn build_schedule(&self, curves: &MarketContext, _as_of: Date) -> Result<DatedFlows> {
        let inflation_source = self.inflation_source(curves)?;

        // Base coupon dates via shared builder
        let sched = crate::cashflow::builder::build_dates(
            self.issue,
            self.maturity,
            self.freq,
            self.stub,
            self.bdc,
            self.calendar_id.as_deref(),
        )?;
        let dates = sched.dates;
        if dates.len() < 2 {
            return Ok(vec![]);
        }

        let mut flows = Vec::with_capacity(dates.len());
        let mut prev = dates[0];
        for &d in &dates[1..] {
            let year_frac = self
                .dc
                .year_fraction(prev, d, DayCountCtx::default())?
                .max(0.0);
            let base_amount = self.notional * self.real_coupon * year_frac;
            let ratio = inflation_source.ratio(self, d)?;
            flows.push((d, base_amount * ratio));
            prev = d;
        }

        // Principal repayment at maturity (inflation adjusted)
        let principal_ratio = inflation_source.ratio(self, self.maturity)?;
        flows.push((self.maturity, self.notional * principal_ratio));

        Ok(flows)
    }

    /// Calculate real accrued interest at the given date
    fn accrued_real_interest(&self, as_of: Date) -> Result<f64> {
        // Reconstruct the date schedule
        let sched = crate::cashflow::builder::build_dates(
            self.issue,
            self.maturity,
            self.freq,
            self.stub,
            self.bdc,
            self.calendar_id.as_deref(),
        )?;
        let dates = sched.dates;
        if dates.len() < 2 {
            return Ok(0.0);
        }

        // Find the active period
        // Dates array includes issue date at [0] and subsequent coupon dates
        for window in dates.windows(2) {
            let start = window[0];
            let end = window[1];

            if start <= as_of && as_of < end {
                // Found the active period
                let total_yf = self.dc.year_fraction(start, end, DayCountCtx::default())?;
                let elapsed_yf = self
                    .dc
                    .year_fraction(start, as_of, DayCountCtx::default())?;

                if total_yf <= 0.0 {
                    return Ok(0.0);
                }

                // Real coupon amount for the full period
                let full_coupon = self.notional.amount() * self.real_coupon * total_yf;

                // Linear accrual: Coupon * (elapsed / total)
                // Note: This matches standard bond accrual for fixed coupons.
                // If we need exact day-based fraction (e.g. Act/Act), year_fraction handles it roughly,
                // but strictly generic accrual uses Coupon * (AccrualDays / PeriodDays).
                // For Act/Act, year_fraction(start, as_of) / year_fraction(start, end) is the standard ratio.
                return Ok(full_coupon * (elapsed_yf / total_yf));
            }
        }

        // If we are past maturity or before issue
        Ok(0.0)
    }

    /// Build unadjusted real cashflow schedule (no inflation indexation)
    pub fn build_real_schedule(&self, _as_of: Date) -> Result<DatedFlows> {
        // Base coupon dates via shared builder
        let sched = crate::cashflow::builder::build_dates(
            self.issue,
            self.maturity,
            self.freq,
            self.stub,
            self.bdc,
            self.calendar_id.as_deref(),
        )?;
        let dates = sched.dates;
        if dates.len() < 2 {
            return Ok(vec![]);
        }

        let mut flows = Vec::with_capacity(dates.len());
        let mut prev = dates[0];
        for &d in &dates[1..] {
            let year_frac = self
                .dc
                .year_fraction(prev, d, DayCountCtx::default())?
                .max(0.0);
            let base_amount = self.notional.amount() * self.real_coupon * year_frac;
            // No inflation adjustment
            flows.push((d, Money::new(base_amount, self.notional.currency())));
            prev = d;
        }

        // Principal repayment at maturity (unadjusted real principal)
        // Note: Deflation protection applies to the final payment in nominal terms,
        // but Real Yield is typically defined on the base real flows.
        flows.push((self.maturity, self.notional));

        Ok(flows)
    }

    /// Calculate real yield (yield in real terms, before inflation)
    ///
    /// Computes the internal rate of return of the **unadjusted (real) cashflows**
    /// against the **real price** (clean price + real accrued interest).
    ///
    /// This is the standard "Real Yield" quoted for TIPS and other linkers.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The clean price is non-positive or non-finite
    /// - There are no cashflows remaining
    /// - The YTM solver fails to converge
    pub fn real_yield(
        &self,
        clean_price: f64,
        _curves: &MarketContext,
        as_of: Date,
    ) -> Result<f64> {
        use crate::instruments::bond::pricing::quote_engine::YieldCompounding;
        use crate::instruments::bond::pricing::ytm_solver::{solve_ytm, YtmPricingSpec};

        if !clean_price.is_finite() || clean_price <= 0.0 {
            return Err(finstack_core::InputError::Invalid.into());
        }

        // 1. Build real cashflows (unadjusted for inflation)
        let flows = self.build_real_schedule(as_of)?;
        if flows.is_empty() {
            return Err(finstack_core::InputError::TooFewPoints.into());
        }

        // 2. Calculate Real Accrued Interest
        // Needed to convert Clean Real Price -> Dirty Real Price
        let real_accrued = self.accrued_real_interest(as_of)?;

        // 3. Calculate Target Dirty Real Price
        // Price is per 100 notional.
        let target_dirty_price_val = (clean_price / 100.0 * self.notional.amount()) + real_accrued;
        let target_price = Money::new(target_dirty_price_val, self.notional.currency());

        let spec = YtmPricingSpec {
            day_count: self.dc,
            notional: self.notional,
            coupon_rate: self.real_coupon,
            compounding: YieldCompounding::Street,
            frequency: self.freq,
        };

        // 4. Solve yield that matches the target real price to PV of real flows
        // The solver handles convergence internally; we propagate any solver errors
        // rather than clamping, so callers can detect and handle extreme cases.
        solve_ytm(&flows, as_of, target_price, spec)
    }

    /// Calculate breakeven inflation rate
    ///
    /// Uses the exact Fisher equation:
    /// `(1 + nominal) = (1 + real) × (1 + inflation)`
    ///
    /// Solving for inflation:
    /// `breakeven = (1 + nominal) / (1 + real) - 1`
    ///
    /// This is more accurate than the simplified approximation (`nominal - real`)
    /// at higher inflation levels where the cross-term becomes significant.
    pub fn breakeven_inflation(
        &self,
        nominal_bond_yield: f64,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<f64> {
        let real_yield = self.real_yield(self.quoted_clean.unwrap_or(100.0), curves, as_of)?;

        // Fisher equation: (1 + nominal) = (1 + real) * (1 + inflation)
        // Exact solution: breakeven = (1 + nominal) / (1 + real) - 1
        // Guard against division by zero for extreme negative real yields
        let denominator = 1.0 + real_yield;
        if denominator <= 0.0 {
            return Err(finstack_core::InputError::NonPositiveValue.into());
        }
        Ok((1.0 + nominal_bond_yield) / denominator - 1.0)
    }

    /// Calculate inflation-adjusted duration (Real Duration)
    ///
    /// Computes the modified duration of the bond based on its real (unadjusted)
    /// cashflows. This measures sensitivity to changes in real yield.
    pub fn real_duration(&self, curves: &MarketContext, as_of: Date) -> Result<f64> {
        use crate::instruments::bond::pricing::quote_engine::{
            price_from_ytm_compounded_params, YieldCompounding,
        };

        // Determine a base clean price to center the bump around
        let base_clean = self.quoted_clean.unwrap_or(100.0);
        // Compute base yield
        let y0 = self.real_yield(base_clean, curves, as_of)?;
        // Bump yield by 1bp in decimal terms
        let bp = 1e-4;

        // Use real schedule to calculate sensitivity to real yield (Real Duration)
        // This assumes the "Duration" metric refers to the duration of the real bond component.
        let flows = self.build_real_schedule(as_of)?;

        // Helper to compute price from yield, propagating errors
        let price_from_yield = |y: f64| -> Result<f64> {
            let price = price_from_ytm_compounded_params(
                self.dc,
                self.freq,
                &flows,
                as_of,
                y,
                YieldCompounding::Street,
            )?;
            Ok(price / self.notional.amount() * 100.0)
        };

        let p_up = price_from_yield(y0 + bp)?;
        let p_dn = price_from_yield(y0 - bp)?;
        let dp_dy = (p_up - p_dn) / (2.0 * bp);

        // Modified duration in years per 1 delta in yield: D = - (1/P) * dP/dy
        let p0 = base_clean.max(1e-6);
        Ok(-(dp_dy / p0))
    }

    /// Present value using standard cashflow discounting
    pub fn npv(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        let flows = self.build_schedule(curves, as_of)?;
        let disc = curves.get_discount(&self.discount_curve_id)?;
        let base_date = disc.base_date();
        // Use curve basis for time mapping
        let dc = disc.day_count();
        flows.npv(disc.as_ref(), base_date, Some(dc))
    }
}

// Explicit Instrument trait implementation (replaces macro for better IDE visibility)
impl crate::instruments::common::traits::Instrument for InflationLinkedBond {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::InflationLinkedBond
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        // Route through helper for schedule-based PV calculation using curve basis
        crate::instruments::common::helpers::schedule_pv_using_curve_dc(
            self,
            curves,
            as_of,
            &self.discount_curve_id,
        )
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
            None,
            None,
        )
    }

    fn as_cashflow_provider(&self) -> Option<&dyn crate::cashflow::traits::CashflowProvider> {
        Some(self)
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for InflationLinkedBond {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

impl CashflowProvider for InflationLinkedBond {
    fn notional(&self) -> Option<Money> {
        Some(self.notional)
    }

    fn build_full_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<crate::cashflow::builder::CashFlowSchedule> {
        let flows = self.build_schedule(curves, as_of)?;
        Ok(crate::cashflow::traits::schedule_from_dated_flows(
            flows,
            self.notional(),
            self.dc,
        ))
    }
}

impl crate::instruments::common::traits::CurveDependencies for InflationLinkedBond {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}
