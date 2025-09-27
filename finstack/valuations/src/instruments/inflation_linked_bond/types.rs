//! Inflation-Linked Bond (ILB) types and implementation.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::discountable::Discountable;
use crate::instruments::common::traits::Attributes;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, DayCountCtx, Frequency, StubKind};
use finstack_core::market_data::scalars::inflation_index::{InflationIndex, InflationInterpolation, InflationLag};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::types::InstrumentId;
use finstack_core::{Result, F};
use time::Duration;

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
    ) -> Result<F> {
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

    /// Build inflation-adjusted cashflow schedule
    pub fn build_schedule(
        &self,
        curves: &MarketContext,
        _as_of: Date,
    ) -> Result<DatedFlows> {
        let inflation_index = curves
            .inflation_index(self.inflation_id.as_str())
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "inflation_linked_bond_quote".to_string(),
                })
            })?;

        // Base coupon dates via shared builder
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
            let year_frac = self
                .dc
                .year_fraction(prev, d, DayCountCtx::default())?
                .max(0.0);
            let base_amount = self.notional * self.real_coupon * year_frac;
            let ratio = self.index_ratio(d, &inflation_index)?;
            flows.push((d, base_amount * ratio));
            prev = d;
        }

        // Principal repayment at maturity (inflation adjusted)
        let principal_ratio = self.index_ratio(self.maturity, &inflation_index)?;
        flows.push((self.maturity, self.notional * principal_ratio));

        Ok(flows)
    }

    /// Calculate real yield (yield in real terms, before inflation)
    pub fn real_yield(
        &self,
        clean_price: F,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<F> {
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
    pub fn real_duration(&self, curves: &MarketContext, as_of: Date) -> Result<F> {
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
        let price_from_yield = |y: f64| -> F {
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
        let disc = curves.get_discount_ref(self.disc_id.clone())?;
        let base_date = disc.base_date();
        // Use curve basis for time mapping
        let dc = disc.day_count();
        flows.npv(disc, base_date, dc)
    }
}

// Explicit Instrument trait implementation (replaces macro for better IDE visibility)
impl crate::instruments::common::traits::Instrument for InflationLinkedBond {
    #[inline]
    fn id(&self) -> &str {
        self.id.as_str()
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

impl crate::instruments::common::traits::InstrumentKind for InflationLinkedBond {
    const TYPE: crate::pricer::InstrumentType = crate::pricer::InstrumentType::InflationLinkedBond;
}

impl CashflowProvider for InflationLinkedBond {
    fn build_schedule(&self, curves: &MarketContext, as_of: Date) -> Result<DatedFlows> {
        self.build_schedule(curves, as_of)
    }
}

