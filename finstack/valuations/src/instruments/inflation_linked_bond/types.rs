//! Inflation-Linked Bond (ILB) types and implementation.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::discountable::Discountable;
use crate::instruments::common::traits::Attributes;
use finstack_core::currency::Currency;
use finstack_core::dates::{
    BusinessDayConvention, Date, DayCount, DayCountCtx, Frequency, StubKind,
};
use finstack_core::market_data::scalars::inflation_index::{
    InflationIndex, InflationInterpolation, InflationLag,
};
use finstack_core::market_data::term_structures::inflation::InflationCurve;
use finstack_core::market_data::MarketContext;
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
    pub freq: Frequency,
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
    pub fn example() -> Self {
        Self {
            id: InstrumentId::new("TIPS-10Y"),
            notional: Money::new(1_000_000.0, Currency::USD),
            real_coupon: 0.025,
            freq: Frequency::semi_annual(),
            dc: DayCount::Act365F,
            issue: Date::from_calendar_date(2024, time::Month::January, 15).expect("Valid example date"),
            maturity: Date::from_calendar_date(2034, time::Month::January, 15).expect("Valid example date"),
            base_index: 100.0,
            base_date: Date::from_calendar_date(2024, time::Month::January, 15).expect("Valid example date"),
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

    /// Create a new UK Index-Linked Gilt using parameter structs
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
            lag: IndexationMethod::UK.standard_lag(),
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

    fn inflation_source(&self, curves: &MarketContext) -> Result<InflationSource> {
        InflationSource::from_market(curves, &self.inflation_index_id)
    }

    /// Calculate index ratio for a given date
    pub fn index_ratio(&self, date: Date, inflation_index: &InflationIndex) -> Result<f64> {
        // Validate interpolation policy vs indexation method for common standards
        match self.indexation_method {
            IndexationMethod::TIPS | IndexationMethod::Canadian => {
                if inflation_index.interpolation() != InflationInterpolation::Linear {
                    return Err(finstack_core::error::InputError::Invalid.into());
                }
            }
            IndexationMethod::UK => {
                if inflation_index.interpolation() != InflationInterpolation::Step {
                    return Err(finstack_core::error::InputError::Invalid.into());
                }
            }
            _ => {}
        }

        // Apply lag to obtain the reference date in index space
        let reference_date = match self.lag {
            InflationLag::Months(m) => finstack_core::dates::add_months(date, -(m as i32)),
            InflationLag::Days(d) => date - Duration::days(d as i64),
            InflationLag::None => date,
            _ => date,
        };

        // Value on reference date (interpolation policy controlled by index)
        let current_index = inflation_index.value_on(reference_date)?;

        // Ratio vs base
        if self.base_index <= 0.0 {
            return Err(finstack_core::error::InputError::NonPositiveValue.into());
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
            InflationLag::Months(m) => finstack_core::dates::add_months(date, -(m as i32)),
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
            return Err(finstack_core::error::InputError::NonPositiveValue.into());
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
        );
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

    /// Calculate real yield (yield in real terms, before inflation)
    pub fn real_yield(&self, clean_price: f64, curves: &MarketContext, as_of: Date) -> Result<f64> {
        use crate::instruments::bond::pricing::helpers::YieldCompounding;
        use crate::instruments::bond::pricing::ytm_solver::{solve_ytm, YtmPricingSpec};

        if !clean_price.is_finite() || clean_price <= 0.0 {
            return Err(finstack_core::error::InputError::Invalid.into());
        }

        // Build real cashflows (already inflation-adjusted amounts at payment dates)
        // Note: Here we approximate by using the instrument schedule with ILB cash amounts;
        // accrued real interest is small relative to coupon accuracy. A future enhancement can
        // compute real accrued to convert clean→dirty precisely.
        let flows = self.build_schedule(curves, as_of)?;
        if flows.is_empty() {
            return Err(finstack_core::error::InputError::TooFewPoints.into());
        }

        // Convert clean price (per 100) to Money in instrument currency
        let target_price = Money::new(
            clean_price / 100.0 * self.notional.amount(),
            self.notional.currency(),
        );

        let spec = YtmPricingSpec {
            day_count: self.dc,
            notional: self.notional,
            coupon_rate: self.real_coupon,
            compounding: YieldCompounding::Street,
            frequency: self.freq,
        };
        // Solve yield that matches the target price to PV of flows on (as_of)
        let y = solve_ytm(&flows, as_of, target_price, spec)?;
        // Clamp extreme values to avoid explosive outputs
        Ok(y.clamp(-0.99, 2.0))
    }

    /// Calculate breakeven inflation rate
    pub fn breakeven_inflation(
        &self,
        nominal_bond_yield: f64,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<f64> {
        let real_yield = self.real_yield(self.quoted_clean.unwrap_or(100.0), curves, as_of)?;

        // Fisher equation: (1 + nominal) = (1 + real) * (1 + inflation)
        // Simplified: breakeven ≈ nominal - real
        Ok(nominal_bond_yield - real_yield)
    }

    /// Calculate inflation-adjusted duration
    pub fn real_duration(&self, curves: &MarketContext, as_of: Date) -> Result<f64> {
        // Determine a base clean price to center the bump around
        let base_clean = self.quoted_clean.unwrap_or(100.0);
        // Compute base yield
        let y0 = self.real_yield(base_clean, curves, as_of)?;
        // Bump yield by 1bp in decimal terms
        let bp = 1e-4;
        // Price function from yield using helper
        use crate::instruments::bond::pricing::helpers::{
            price_from_ytm_compounded_params, YieldCompounding,
        };
        let flows = self.build_schedule(curves, as_of)?;
        // Convert price from ytm helpers returns currency units; convert back to clean per-100 notionally
        let price_from_yield = |y: f64| -> f64 {
            price_from_ytm_compounded_params(
                self.dc,
                self.freq,
                &flows,
                as_of,
                y,
                YieldCompounding::Street,
            )
            .unwrap_or(0.0)
                / self.notional.amount()
                * 100.0
        };
        let p_up = price_from_yield(y0 + bp);
        let p_dn = price_from_yield(y0 - bp);
        let dp_dy = (p_up - p_dn) / (2.0 * bp);
        // Modified duration in years per 1 delta in yield: D = - (1/P) * dP/dy
        let p0 = base_clean.max(1e-6);
        Ok(-(dp_dy / p0))
    }

    /// Present value using standard cashflow discounting
    pub fn npv(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        let flows = self.build_schedule(curves, as_of)?;
        let disc = curves.get_discount_ref(&self.discount_curve_id)?;
        let base_date = disc.base_date();
        // Use curve basis for time mapping
        let dc = disc.day_count();
        flows.npv(disc, base_date, dc)
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
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        // Route through helper for schedule-based PV calculation
        crate::instruments::common::helpers::schedule_pv_impl(
            self,
            curves,
            as_of,
            &self.discount_curve_id,
            self.dc,
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
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
        )
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for InflationLinkedBond {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

impl CashflowProvider for InflationLinkedBond {
    fn build_schedule(&self, curves: &MarketContext, as_of: Date) -> Result<DatedFlows> {
        self.build_schedule(curves, as_of)
    }
}

impl crate::instruments::common::traits::CurveDependencies for InflationLinkedBond {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}
