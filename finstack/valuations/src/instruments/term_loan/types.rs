//! Term loan types and instrument trait implementation.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use crate::instruments::common::traits::Attributes;
use crate::instruments::pricing_overrides::PricingOverrides;
use crate::cashflow::builder::types::CouponType;
use super::spec::{AmortizationSpec, CovenantSpec, DdtlSpec, LoanCallSchedule};

/// Minimal rate spec placeholder (extended in later tasks)
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RateSpec {
    /// Fixed annual rate in basis points
    Fixed { rate_bp: i32 },
    /// Floating index with margin; full shape added later
    Floating {
        index_id: CurveId,
        margin_bp: i32,
        floor_bp: Option<i32>,
        reset_freq: Frequency,
        reset_lag_days: i32,
    },
}

/// Term Loan instrument (DDTL features added via later tasks)
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TermLoan {
    /// Unique instrument identifier
    pub id: InstrumentId,

    /// Currency for all cashflows
    pub currency: Currency,

    /// Maximum commitment / notional limit
    pub notional_limit: Money,

    /// Issue (effective) date
    pub issue: Date,

    /// Maturity date
    pub maturity: Date,

    /// Rate specification (fixed or floating)
    pub rate: RateSpec,

    /// Payment frequency for coupons/fees
    pub pay_freq: Frequency,

    /// Day count convention
    pub day_count: DayCount,

    /// Business day convention
    pub bdc: BusinessDayConvention,

    /// Optional calendar id for adjustments
    pub calendar_id: Option<String>,

    /// Stub rule
    pub stub: StubKind,

    /// Discount curve identifier
    pub disc_id: CurveId,

    /// Amortization specification
    pub amortization: AmortizationSpec,

    /// Coupon split type (Cash/PIK/Split)
    pub coupon_type: CouponType,

    /// Upfront fee at issue (if any)
    pub upfront_fee: Option<Money>,

    /// Optional DDTL parameters; None => plain term loan
    pub ddtl: Option<DdtlSpec>,

    /// Optional covenant spec
    pub covenants: Option<CovenantSpec>,

    /// Pricing overrides (quoted price, seed, etc.)
    pub pricing_overrides: PricingOverrides,

    /// Optional call schedule (borrower callability)
    pub call_schedule: Option<LoanCallSchedule>,

    /// Attributes for tagging and scenarios
    pub attributes: Attributes,
}

impl crate::instruments::common::traits::Instrument for TermLoan {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::TermLoan
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
    ) -> finstack_core::Result<Money> {
        // Delegate to discounting pricer (deterministic v1)
        crate::instruments::term_loan::pricing::TermLoanDiscountingPricer::price(self, curves, as_of)
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

impl crate::cashflow::traits::CashflowProvider for TermLoan {
	fn build_schedule(
		&self,
		curves: &finstack_core::market_data::MarketContext,
		as_of: finstack_core::dates::Date,
	) -> finstack_core::Result<crate::cashflow::DatedFlows> {
		let sched = crate::instruments::term_loan::cashflows::generate_cashflows(self, curves, as_of)?;
		Ok(crate::instruments::term_loan::cashflows::build_dated_flows(&sched))
	}

	fn build_full_schedule(
		&self,
		curves: &finstack_core::market_data::MarketContext,
		as_of: finstack_core::dates::Date,
	) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
		crate::instruments::term_loan::cashflows::generate_cashflows(self, curves, as_of)
	}
}

// Allow generic metric calculators to fetch discount curve id
impl crate::instruments::common::pricing::HasDiscountCurve for TermLoan {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.disc_id
    }
}


