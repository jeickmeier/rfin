use pyo3::prelude::*;

use rfin_core::cashflow::leg::CashFlowLeg;
use rfin_core::cashflow::notional::Notional;
use rfin_core::cashflow::npv::{DiscountCurve, Discountable};
use rfin_core::cashflow::primitives::{CFKind, CashFlow as CoreCashFlow};
use rfin_core::dates::ScheduleBuilder;
use std::sync::Arc;

use crate::currency::PyCurrency;
use crate::dates::PyDate;
use crate::daycount::PyDayCount;
use crate::schedule::PyFrequency;

/// Simple flat discount curve used for PV calculations.
struct FlatCurve {
    rate: f64,
}

impl DiscountCurve for FlatCurve {
    fn df(&self, _date: rfin_core::dates::Date) -> f64 {
        // Zero-rate flat curve: simply use exp(-r * t) with t = 0 (today) => 1.0 for now.
        // Future enhancement: compute based on year fraction to payment date.
        1.0 / (1.0 + self.rate)
    }
}

/// Individual cash flow representing a single payment.
///
/// A CashFlow represents a single payment on a specific date with a specific amount
/// and currency. It includes metadata about the type of payment and accrual
/// information for coupon calculations.
///
/// Cash flows are typically generated as part of a `FixedRateLeg` or other
/// financial instrument and represent the individual payments that make up
/// the instrument's payment schedule.
///
/// Examples:
///     >>> from rfin import Currency, Date, DayCount, Frequency, FixedRateLeg
///     >>> leg = FixedRateLeg(
///     ...     notional_amount=1000000,
///     ...     currency=Currency.usd(),
///     ...     rate=0.05,
///     ...     start_date=Date(2023, 1, 1),
///     ...     end_date=Date(2024, 1, 1),
///     ...     frequency=Frequency.SemiAnnual,
///     ...     day_count=DayCount.thirty360()
///     ... )
///     >>> flows = leg.flows()
///     >>> first_flow = flows[0]
///     >>> first_flow.amount
///     25000.0
///     >>> first_flow.currency
///     Currency('USD')
///     >>> first_flow.kind
///     'Fixed'
#[pyclass(name = "CashFlow", module = "rfin.cashflow")]
#[derive(Clone, Copy)]
pub struct PyCashFlow {
    inner: CoreCashFlow,
}

#[pymethods]
impl PyCashFlow {
    /// The payment date of this cash flow.
    ///
    /// Returns:
    ///     Date: The date when this cash flow is due to be paid.
    ///
    /// Examples:
    ///     >>> cash_flow.date
    ///     Date('2023-07-01')
    #[getter]
    fn date(&self) -> PyDate {
        PyDate::from_core(self.inner.date)
    }

    /// The payment amount of this cash flow.
    ///
    /// Returns:
    ///     float: The payment amount (can be positive or negative).
    ///            Positive values represent receipts, negative values represent payments.
    ///
    /// Examples:
    ///     >>> cash_flow.amount
    ///     25000.0
    ///     >>> # For a payer swap, coupon payments would be negative
    ///     >>> payer_flow.amount
    ///     -25000.0
    #[getter]
    fn amount(&self) -> f64 {
        self.inner.amount.amount()
    }

    /// The currency of this cash flow.
    ///
    /// Returns:
    ///     Currency: The currency in which this cash flow is denominated.
    ///
    /// Examples:
    ///     >>> cash_flow.currency
    ///     Currency('USD')
    ///     >>> cash_flow.currency.code
    ///     'USD'
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::from_inner(self.inner.amount.currency())
    }

    /// The type/kind of this cash flow.
    ///
    /// Returns:
    ///     str: The cash flow type. Common values include:
    ///          - "Fixed": Fixed coupon payment
    ///          - "FloatReset": Floating rate coupon payment
    ///          - "Notional": Principal payment
    ///          - "Fee": Fee payment
    ///          - "Stub": Stub period payment
    ///          - "Other": Other payment types
    ///
    /// Examples:
    ///     >>> cash_flow.kind
    ///     'Fixed'
    ///     >>> notional_flow.kind
    ///     'Notional'
    #[getter]
    fn kind(&self) -> String {
        match self.inner.kind {
            CFKind::Fixed => "Fixed",
            CFKind::FloatReset => "FloatReset",
            CFKind::Notional => "Notional",
            CFKind::Fee => "Fee",
            CFKind::Stub => "Stub",
            _ => "Other",
        }
        .to_string()
    }

    /// The accrual factor used for this cash flow.
    ///
    /// The accrual factor represents the fraction of a year over which interest
    /// accrues for this payment, calculated according to the day count convention.
    ///
    /// Returns:
    ///     float: The accrual factor (typically between 0 and 1).
    ///            For example, 0.5 represents a 6-month period.
    ///
    /// Examples:
    ///     >>> # For a 6-month period with ACT/360
    ///     >>> cash_flow.accrual_factor
    ///     0.5055555555555556
    ///     >>> # For a 3-month period with 30/360
    ///     >>> quarterly_flow.accrual_factor
    ///     0.25
    #[getter]
    fn accrual_factor(&self) -> f64 {
        self.inner.accrual_factor
    }

    /// Return string representation of the cash flow.
    ///
    /// Returns:
    ///     str: A formatted string showing the cash flow details.
    ///
    /// Examples:
    ///     >>> str(cash_flow)
    ///     'CashFlow(date=2023-07-01, amount=25000.0000, currency=USD, kind=Fixed)'
    fn __repr__(&self) -> String {
        format!(
            "CashFlow(date={}, amount={:.4}, currency={}, kind={})",
            self.inner.date,
            self.inner.amount.amount(),
            self.inner.amount.currency(),
            self.kind()
        )
    }
}

/// Fixed-rate leg of a swap or bond.
///
/// A FixedRateLeg represents a series of fixed coupon payments made at regular
/// intervals. It's commonly used as one leg of an interest rate swap, or to
/// represent the coupon payments of a fixed-rate bond.
///
/// The leg automatically generates a payment schedule based on the start and end
/// dates, frequency, and day count convention. Each payment is calculated as:
/// `payment = notional * rate * accrual_factor`
///
/// Examples:
///     >>> from rfin import Currency, Date, DayCount, Frequency, FixedRateLeg
///     
///     # Create a 2-year fixed rate leg paying 5% semi-annually
///     >>> leg = FixedRateLeg(
///     ...     notional_amount=1000000,  # $1M notional
///     ...     currency=Currency.usd(),
///     ...     rate=0.05,  # 5% annual rate
///     ...     start_date=Date(2023, 1, 1),
///     ...     end_date=Date(2025, 1, 1),
///     ...     frequency=Frequency.SemiAnnual,
///     ...     day_count=DayCount.thirty360()
///     ... )
///     
///     # Get the payment schedule
///     >>> leg.num_flows
///     4
///     >>> flows = leg.flows()
///     >>> flows[0].amount  # First semi-annual payment
///     25000.0
///     
///     # Calculate present value (using flat discount rate of 0%)
///     >>> leg.npv()
///     100000.0
///     
///     # Calculate accrued interest
///     >>> leg.accrued(Date(2023, 3, 1))  # 2 months into first period
///     8333.333333333334
#[pyclass(name = "FixedRateLeg", module = "rfin.cashflow")]
#[derive(Clone)]
pub struct PyFixedRateLeg {
    inner: Arc<CashFlowLeg>,
}

#[pymethods]
impl PyFixedRateLeg {
    /// Create a new fixed-rate leg.
    ///
    /// Args:
    ///     notional_amount (float): The notional principal amount.
    ///     currency (Currency): The currency of the payments.
    ///     rate (float): The annual fixed interest rate (as a decimal, e.g., 0.05 for 5%).
    ///     start_date (Date): The start date of the leg (first accrual period begins).
    ///     end_date (Date): The end date of the leg (last payment date).
    ///     frequency (Frequency): The payment frequency (e.g., Frequency.SemiAnnual).
    ///     day_count (DayCount): The day count convention for accrual calculations.
    ///
    /// Returns:
    ///     FixedRateLeg: A new fixed-rate leg instance.
    ///
    /// Raises:
    ///     ValueError: If the leg cannot be constructed (e.g., invalid dates).
    ///
    /// Examples:
    ///     >>> from rfin import Currency, Date, DayCount, Frequency, FixedRateLeg
    ///     
    ///     # Standard 5% fixed rate leg
    ///     >>> leg = FixedRateLeg(
    ///     ...     notional_amount=1000000,
    ///     ...     currency=Currency.usd(),
    ///     ...     rate=0.05,
    ///     ...     start_date=Date(2023, 1, 1),
    ///     ...     end_date=Date(2024, 1, 1),
    ///     ...     frequency=Frequency.SemiAnnual,
    ///     ...     day_count=DayCount.thirty360()
    ///     ... )
    ///     
    ///     # Quarterly payments in EUR
    ///     >>> eur_leg = FixedRateLeg(
    ///     ...     notional_amount=500000,
    ///     ...     currency=Currency.eur(),
    ///     ...     rate=0.03,
    ///     ...     start_date=Date(2023, 1, 1),
    ///     ...     end_date=Date(2023, 12, 31),
    ///     ...     frequency=Frequency.Quarterly,
    ///     ...     day_count=DayCount.act360()
    ///     ... )
    #[new]
    #[pyo3(
        text_signature = "(notional_amount, currency, rate, start_date, end_date, frequency, day_count)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        notional_amount: f64,
        currency: &PyCurrency,
        rate: f64,
        start_date: &PyDate,
        end_date: &PyDate,
        frequency: &PyFrequency,
        day_count: &PyDayCount,
    ) -> PyResult<Self> {
        let core_currency = currency.inner();

        let notional = Notional::par(notional_amount, core_currency);

        let freq = frequency.inner();
        let dc = day_count.inner();

        let sched = ScheduleBuilder::new(start_date.inner(), end_date.inner())
            .frequency(freq)
            .build_raw();

        let leg = CashFlowLeg::fixed_rate(notional, rate, sched, dc).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Failed to build leg: {:?}", e))
        })?;

        Ok(Self {
            inner: Arc::new(leg),
        })
    }

    /// Calculate the net present value of the leg.
    ///
    /// Currently uses a flat discount rate of 0% (no discounting).
    /// Future versions will support proper discount curves.
    ///
    /// Returns:
    ///     float: The net present value of all cash flows in the leg.
    ///
    /// Examples:
    ///     >>> leg = FixedRateLeg(
    ///     ...     notional_amount=1000000,
    ///     ...     currency=Currency.usd(),
    ///     ...     rate=0.05,
    ///     ...     start_date=Date(2023, 1, 1),
    ///     ...     end_date=Date(2024, 1, 1),
    ///     ...     frequency=Frequency.SemiAnnual,
    ///     ...     day_count=DayCount.thirty360()
    ///     ... )
    ///     >>> leg.npv()
    ///     50000.0  # Two payments of $25,000 each
    #[pyo3(text_signature = "(self)")]
    fn npv(&self) -> f64 {
        let curve = FlatCurve { rate: 0.0 };
        self.inner.npv(&curve).amount()
    }

    /// Calculate accrued interest up to a valuation date.
    ///
    /// Calculates the amount of interest that has accrued on the current coupon
    /// period up to (but not including) the specified valuation date.
    ///
    /// Args:
    ///     val_date (Date): The valuation date (exclusive).
    ///
    /// Returns:
    ///     float: The accrued interest amount.
    ///
    /// Examples:
    ///     >>> from rfin import Currency, Date, DayCount, Frequency, FixedRateLeg
    ///     >>> leg = FixedRateLeg(
    ///     ...     notional_amount=1000000,
    ///     ...     currency=Currency.usd(),
    ///     ...     rate=0.05,
    ///     ...     start_date=Date(2023, 1, 1),
    ///     ...     end_date=Date(2024, 1, 1),
    ///     ...     frequency=Frequency.SemiAnnual,
    ///     ...     day_count=DayCount.thirty360()
    ///     ... )
    ///     
    ///     # Accrued interest after 2 months (60 days with 30/360)
    ///     >>> leg.accrued(Date(2023, 3, 1))
    ///     8333.333333333334
    ///     
    ///     # Accrued interest after 3 months (90 days with 30/360)
    ///     >>> leg.accrued(Date(2023, 4, 1))
    ///     12500.0
    #[pyo3(text_signature = "(self, val_date)")]
    fn accrued(&self, val_date: &PyDate) -> f64 {
        self.inner.accrued(val_date.inner()).amount()
    }

    /// The number of cash flows in this leg.
    ///
    /// Returns:
    ///     int: The total number of cash flows (payments) in the leg.
    ///
    /// Examples:
    ///     >>> # 2-year semi-annual leg has 4 payments
    ///     >>> leg.num_flows
    ///     4
    ///     >>> # 1-year quarterly leg has 4 payments
    ///     >>> quarterly_leg.num_flows
    ///     4
    #[getter]
    fn num_flows(&self) -> usize {
        self.inner.flows.len()
    }

    /// Get all cash flows in the leg.
    ///
    /// Returns:
    ///     List[CashFlow]: A list of all cash flows in the leg, ordered by payment date.
    ///
    /// Examples:
    ///     >>> leg = FixedRateLeg(
    ///     ...     notional_amount=1000000,
    ///     ...     currency=Currency.usd(),
    ///     ...     rate=0.05,
    ///     ...     start_date=Date(2023, 1, 1),
    ///     ...     end_date=Date(2024, 1, 1),
    ///     ...     frequency=Frequency.SemiAnnual,
    ///     ...     day_count=DayCount.thirty360()
    ///     ... )
    ///     >>> flows = leg.flows()
    ///     >>> len(flows)
    ///     2
    ///     >>> flows[0].date
    ///     Date('2023-07-01')
    ///     >>> flows[0].amount
    ///     25000.0
    ///     >>> flows[1].date
    ///     Date('2024-01-01')
    ///     >>> flows[1].amount
    ///     25000.0
    #[pyo3(text_signature = "(self)")]
    fn flows(&self) -> Vec<PyCashFlow> {
        self.inner
            .flows
            .iter()
            .copied()
            .map(|cf| PyCashFlow { inner: cf })
            .collect()
    }

    /// Return string representation of the leg.
    ///
    /// Returns:
    ///     str: A formatted string showing the number of cash flows.
    ///
    /// Examples:
    ///     >>> str(leg)
    ///     'FixedRateLeg(n_flows=4)'
    fn __repr__(&self) -> String {
        format!("FixedRateLeg(n_flows={})", self.inner.flows.len())
    }
}
