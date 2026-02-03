//! Zero-coupon Inflation Swap types and pricing implementation.

use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::common_impl::validation;
use finstack_core::dates::{
    BusinessDayConvention, Date, DateExt, DayCount, DayCountCtx, StubKind, Tenor,
};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::InflationLag;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId, Rate};

/// Direction from the perspective of paying fixed real vs receiving inflation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PayReceiveInflation {
    /// Pay fixed (real) leg, receive inflation leg
    PayFixed,
    /// Receive fixed (real) leg, pay inflation leg
    ReceiveFixed,
}

impl std::fmt::Display for PayReceiveInflation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PayReceiveInflation::PayFixed => write!(f, "pay_fixed"),
            PayReceiveInflation::ReceiveFixed => write!(f, "receive_fixed"),
        }
    }
}

impl std::str::FromStr for PayReceiveInflation {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "pay_fixed" | "pay" => Ok(PayReceiveInflation::PayFixed),
            "receive_fixed" | "receive" => Ok(PayReceiveInflation::ReceiveFixed),
            other => Err(format!("Unknown inflation swap pay/receive: {}", other)),
        }
    }
}

/// Zero-coupon Inflation Swap instrument.
///
/// Represents a zero-coupon inflation swap where one party pays a fixed real rate
/// and the other receives the cumulative inflation over the swap's life. At maturity:
///
/// ```text
/// Inflation leg = Notional × [CPI(T_mat - Lag) / CPI(T_start - Lag) - 1]
/// Fixed leg     = Notional × [(1 + fixed_rate)^τ - 1]
/// ```
///
/// # Market Conventions
///
/// - **Lag**: Standard 3-month lag for US CPI, EUR HICP; 8-month for UK RPI
/// - **Day Count**: Typically ACT/ACT for accrual, curve-specific for discounting
/// - **Business Day**: Payment dates adjusted per calendar; index observation typically unadjusted
///
/// # Validation
///
/// Call [`validate()`](Self::validate) to check structural invariants before pricing.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct InflationSwap {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Notional in quote currency
    pub notional: Money,
    /// Start date of indexation
    pub start: Date,
    /// Maturity date
    pub maturity: Date,
    /// Fixed real rate (as decimal)
    pub fixed_rate: f64,
    /// Inflation index identifier (e.g., US-CPI-U)
    pub inflation_index_id: CurveId,
    /// Discount curve identifier (quote currency)
    pub discount_curve_id: CurveId,
    /// Day count for accrual calculation (fixed leg compounding)
    pub dc: DayCount,
    /// Trade side
    pub side: PayReceiveInflation,
    /// Optional contract-level lag override (if set, overrides index lag)
    #[builder(optional)]
    pub lag_override: Option<InflationLag>,
    /// Explicit Base CPI (reference index level at start with lag applied).
    /// If not provided, it will be looked up/calculated from start date.
    #[builder(optional)]
    pub base_cpi: Option<f64>,
    /// Business day convention for payment date adjustment.
    /// Defaults to `Following` if not specified.
    #[builder(optional)]
    pub bdc: Option<BusinessDayConvention>,
    /// Holiday calendar identifier for payment date adjustment.
    /// If not specified, payment dates are used unadjusted.
    #[builder(optional)]
    pub calendar_id: Option<String>,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl InflationSwap {
    /// Create a canonical example zero-coupon inflation swap (US CPI, 5Y).
    ///
    /// Returns a 5-year USD inflation swap with standard 3-month CPI lag.
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use time::Month;
        InflationSwapBuilder::new()
            .id(InstrumentId::new("INFLSWAP-USD-5Y"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start(Date::from_calendar_date(2024, Month::January, 15).expect("Valid example date"))
            .maturity(
                Date::from_calendar_date(2029, Month::January, 15).expect("Valid example date"),
            )
            .fixed_rate(0.02)
            .inflation_index_id(CurveId::new("US-CPI"))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .dc(DayCount::Act365F)
            .side(PayReceiveInflation::PayFixed)
            .lag_override(InflationLag::Months(3))
            .bdc(BusinessDayConvention::Following)
            .attributes(Attributes::new())
            .build()
            .expect("Example InflationSwap construction should not fail")
    }

    /// Validate structural invariants of the inflation swap.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `start >= maturity` (invalid date ordering)
    /// - `notional` is non-positive
    /// - `base_cpi` is provided but non-positive
    pub fn validate(&self) -> finstack_core::Result<()> {
        validation::require_or(
            self.start < self.maturity,
            finstack_core::InputError::InvalidDateRange,
        )?;
        validation::require_or(
            self.notional.amount() > 0.0,
            finstack_core::InputError::NonPositiveValue,
        )?;
        if let Some(base) = self.base_cpi {
            validation::require_or(base > 0.0, finstack_core::InputError::NonPositiveValue)?;
        }
        Ok(())
    }

    /// Apply lag to a date according to the instrument's lag policy.
    ///
    /// Uses `lag_override` if set, otherwise falls back to `default_lag`.
    ///
    /// # Supported Lag Types
    ///
    /// - `InflationLag::None`: No lag applied
    /// - `InflationLag::Months(n)`: Subtract n months from the date
    /// - `InflationLag::Days(n)`: Subtract n days from the date
    ///
    /// # Note
    ///
    /// The `InflationLag` enum is `#[non_exhaustive]`, so unknown variants
    /// fall back to no lag with a debug assertion. This ensures forward
    /// compatibility while catching unexpected variants in development.
    pub(crate) fn apply_lag(&self, date: Date, default_lag: InflationLag) -> Date {
        let lag_policy = self.lag_override.unwrap_or(default_lag);
        match lag_policy {
            InflationLag::None => date,
            InflationLag::Months(m) => date.add_months(-(m as i32)),
            InflationLag::Days(d) => date - time::Duration::days(d as i64),
            // InflationLag is #[non_exhaustive], so we must handle unknown variants.
            // Debug assert to catch new variants during development.
            #[allow(unreachable_patterns)]
            unknown => {
                debug_assert!(
                    false,
                    "Unhandled InflationLag variant: {:?}. Falling back to no lag.",
                    unknown
                );
                date
            }
        }
    }

    /// Get the effective lag policy, using index lag as default if available.
    ///
    /// Priority order:
    /// 1. Instrument's `lag_override` if set
    /// 2. Index's lag if an InflationIndex is in the market context
    /// 3. `InflationLag::None` as fallback (no lag applied)
    ///
    /// Note: For production use, either set `lag_override` explicitly or ensure
    /// an InflationIndex with the correct lag is in the market context.
    fn effective_lag(&self, curves: &MarketContext) -> InflationLag {
        if let Some(lag) = self.lag_override {
            return lag;
        }
        if let Some(index) = curves.inflation_index(self.inflation_index_id.as_str()) {
            return index.lag();
        }
        // Default to no lag when no index is available
        // This ensures consistency with curve-only contexts (e.g., calibration)
        InflationLag::None
    }

    /// Calculate the projected index ratio I(T_mat - Lag) / I(T_start - Lag).
    ///
    /// When using an InflationIndex, the index applies its own lag internally via `value_on()`,
    /// so we pass unlagged dates. When falling back to the inflation curve, we must apply
    /// the lag ourselves since the curve works in year fractions from its base date.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Inflation curve is not found
    /// - Base CPI (start index) is non-positive
    pub(crate) fn projected_index_ratio(
        &self,
        curves: &MarketContext,
        discount_base: Date,
    ) -> finstack_core::Result<f64> {
        let inflation_index = curves.inflation_index(self.inflation_index_id.as_str());
        let inflation_curve = curves.get_inflation(self.inflation_index_id.as_str())?;

        let i_start = if let Some(base) = self.base_cpi {
            base
        } else if let Some(index) = inflation_index {
            // InflationIndex.value_on() applies its own lag internally
            index.value_on(self.start)?
        } else {
            // Fall back to curve-based lookup - we must apply lag ourselves
            let default_lag = self.effective_lag(curves);
            let lagged_start = self.apply_lag(self.start, default_lag);
            // Compute signed year fraction (can be negative if lagged_start < discount_base)
            let t_start = Self::signed_year_fraction(discount_base, lagged_start);
            // cpi(t) returns base_cpi for t <= 0, which is correct for historical dates
            inflation_curve.cpi(t_start)
        };

        if i_start <= 0.0 {
            return Err(finstack_core::InputError::NonPositiveValue.into());
        }

        // For maturity projection, use curve-based lookup with lag applied
        // (The index may not have future projections, so we always use the curve for maturity)
        let default_lag = self.effective_lag(curves);
        let lagged_maturity = self.apply_lag(self.maturity, default_lag);

        // Compute signed year fraction (can be negative if matured)
        let t_maturity_infl = Self::signed_year_fraction(discount_base, lagged_maturity);
        // cpi(t) returns base_cpi for t <= 0, which is correct for matured swaps
        let i_maturity_projected = inflation_curve.cpi(t_maturity_infl);

        Ok(i_maturity_projected / i_start)
    }

    /// Compute signed year fraction (positive if end > start, negative if end < start).
    ///
    /// This is needed for inflation curve lookups where dates may be before the base date.
    ///
    /// # Day Count Convention
    ///
    /// Uses `Act365F` (Actual/365 Fixed) regardless of the instrument's `dc` field because:
    ///
    /// 1. **Inflation curves use time in years**: Inflation curve knots are expressed in
    ///    year fractions from the base date. Using a consistent day count ensures proper
    ///    interpolation alignment.
    ///
    /// 2. **Market convention**: Inflation curves are typically constructed with Act365F
    ///    or Act/Act, making Act365F a reasonable default for curve time calculations.
    ///
    /// 3. **Separation of concerns**: The instrument's `dc` field controls fixed leg
    ///    accrual calculation, while inflation curve lookups use curve-native conventions.
    ///
    /// Note: The instrument's `dc` field is used for fixed leg compounding
    /// (see `pv_fixed_leg`), while this function is used only for inflation curve lookups.
    fn signed_year_fraction(start: Date, end: Date) -> f64 {
        if end >= start {
            DayCount::Act365F
                .year_fraction(start, end, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0)
        } else {
            // Negative year fraction for dates before the base
            -DayCount::Act365F
                .year_fraction(end, start, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0)
        }
    }

    /// Get the adjusted payment date based on business day convention and calendar.
    ///
    /// If no calendar is specified, returns the unadjusted date.
    fn adjusted_payment_date(&self, date: Date) -> Date {
        let bdc = self.bdc.unwrap_or(BusinessDayConvention::Following);
        if let Some(ref cal_id) = self.calendar_id {
            use finstack_core::dates::CalendarRegistry;
            if let Some(cal) = CalendarRegistry::global().resolve_str(cal_id) {
                return finstack_core::dates::adjust(date, bdc, cal).unwrap_or(date);
            }
        }
        // No calendar specified - return unadjusted (common for inflation swaps)
        date
    }

    /// Calculate PV of the fixed leg (real rate leg).
    ///
    /// The fixed leg pays `Notional × [(1 + fixed_rate)^τ - 1]` at maturity,
    /// where τ is the accrual year fraction using the instrument's day count.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Discount curve is not found
    /// - Year fraction calculation fails
    pub fn pv_fixed_leg(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let disc = curves.get_discount(self.discount_curve_id.as_str())?;

        // Use instrument day count for accrual period
        let tau_accrual = self.dc.year_fraction(
            self.start,
            self.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        let fixed_payment = self.notional * ((1.0 + self.fixed_rate).powf(tau_accrual) - 1.0);

        // Use curve's day count for discounting (market standard)
        let payment_date = self.adjusted_payment_date(self.maturity);
        let curve_dc = disc.day_count();
        let t_discount = curve_dc.year_fraction(
            as_of,
            payment_date,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let df = disc.df(t_discount);

        Ok(fixed_payment * df)
    }

    /// Calculate PV of the inflation leg.
    ///
    /// The inflation leg pays `Notional × [I(T_mat - Lag) / I(T_start - Lag) - 1]`
    /// at maturity, where I(t) is the inflation index value.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Discount curve is not found
    /// - Inflation curve is not found
    /// - Index ratio calculation fails
    /// - Year fraction calculation fails
    pub fn pv_inflation_leg(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let disc = curves.get_discount(self.discount_curve_id.as_str())?;
        let index_ratio = self.projected_index_ratio(curves, as_of)?;
        let inflation_payment = self.notional * (index_ratio - 1.0);

        // Use curve's day count for discounting (market standard)
        let payment_date = self.adjusted_payment_date(self.maturity);
        let curve_dc = disc.day_count();
        let t_discount = curve_dc.year_fraction(
            as_of,
            payment_date,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let df = disc.df(t_discount);

        Ok(inflation_payment * df)
    }

    /// Fixed rate that sets the swap's present value to zero (par real rate / breakeven).
    ///
    /// For a zero-coupon inflation swap, the par rate K satisfies:
    /// ```text
    /// (1 + K)^τ = I(T_mat - Lag) / I(T_start - Lag)
    /// K = [I(T_mat - Lag) / I(T_start - Lag)]^(1/τ) - 1
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Discount curve is not found
    /// - Index ratio is non-positive
    /// - Year fraction calculation fails
    pub fn par_rate(&self, curves: &MarketContext) -> finstack_core::Result<f64> {
        let disc = curves.get_discount(self.discount_curve_id.as_str())?;
        let base = disc.base_date();
        let index_ratio = self.projected_index_ratio(curves, base)?;

        if index_ratio <= 0.0 {
            return Err(finstack_core::InputError::NonPositiveValue.into());
        }

        let tau = self.dc.year_fraction(
            self.start,
            self.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if tau <= 0.0 {
            return Ok(0.0);
        }

        Ok(index_ratio.powf(1.0 / tau) - 1.0)
    }
}

impl InflationSwapBuilder {
    /// Set the fixed rate using a typed rate.
    pub fn fixed_rate_rate(mut self, rate: Rate) -> Self {
        self.fixed_rate = Some(rate.as_decimal());
        self
    }
}

impl crate::instruments::common_impl::traits::Instrument for InflationSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::InflationSwap
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common_impl::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common_impl::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common_impl::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        // Matured swaps have zero PV
        if as_of >= self.maturity {
            return Ok(finstack_core::money::Money::new(
                0.0,
                self.notional.currency(),
            ));
        }

        let pv_fixed = self.pv_fixed_leg(curves, as_of)?;
        let pv_inflation = self.pv_inflation_leg(curves, as_of)?;
        match self.side {
            PayReceiveInflation::ReceiveFixed => pv_fixed.checked_sub(pv_inflation),
            PayReceiveInflation::PayFixed => pv_inflation.checked_sub(pv_fixed),
        }
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common_impl::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
            None,
            None,
        )
    }

    fn expiry(&self) -> Option<finstack_core::dates::Date> {
        Some(self.maturity)
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        Some(self.start)
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common_impl::traits::CurveDependencies for InflationSwap {
    fn curve_dependencies(&self) -> crate::instruments::common_impl::traits::InstrumentCurves {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.inflation_index_id.clone())
            .build()
    }
}

/// Year-on-year (YoY) Inflation Swap instrument.
///
/// Pays periodic inflation rates (CPI ratios over each period) versus a fixed rate.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct YoYInflationSwap {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Notional in quote currency
    pub notional: Money,
    /// Start date of the first accrual period
    pub start: Date,
    /// Maturity date
    pub maturity: Date,
    /// Fixed rate (decimal)
    pub fixed_rate: f64,
    /// Payment frequency
    pub frequency: Tenor,
    /// Inflation index identifier (e.g., US-CPI-U)
    pub inflation_index_id: CurveId,
    /// Discount curve identifier (quote currency)
    pub discount_curve_id: CurveId,
    /// Day count for fixed leg accrual calculation
    pub dc: DayCount,
    /// Trade side
    pub side: PayReceiveInflation,
    /// Optional contract-level lag override (if set, overrides index lag)
    #[builder(optional)]
    pub lag_override: Option<InflationLag>,
    /// Business day convention for payment date adjustment.
    #[builder(optional)]
    pub bdc: Option<BusinessDayConvention>,
    /// Holiday calendar identifier for payment date adjustment.
    #[builder(optional)]
    pub calendar_id: Option<String>,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl YoYInflationSwap {
    /// Validate structural invariants of the YoY inflation swap.
    pub fn validate(&self) -> finstack_core::Result<()> {
        validation::require_or(
            self.start < self.maturity,
            finstack_core::InputError::InvalidDateRange,
        )?;
        validation::require_or(
            self.notional.amount() > 0.0,
            finstack_core::InputError::NonPositiveValue,
        )?;
        validation::require_or(
            self.frequency.count != 0,
            finstack_core::InputError::Invalid,
        )?;
        Ok(())
    }

    fn effective_lag(&self, curves: &MarketContext) -> InflationLag {
        if let Some(lag) = self.lag_override {
            return lag;
        }
        if let Some(index) = curves.inflation_index(self.inflation_index_id.as_str()) {
            return index.lag();
        }
        InflationLag::None
    }

    /// Apply lag to a date.
    ///
    /// # Note
    ///
    /// The `InflationLag` enum is `#[non_exhaustive]`, so unknown variants
    /// fall back to no lag with a debug assertion.
    fn apply_lag(date: Date, lag: InflationLag) -> Date {
        match lag {
            InflationLag::None => date,
            InflationLag::Months(m) => date.add_months(-(m as i32)),
            InflationLag::Days(d) => date - time::Duration::days(d as i64),
            // InflationLag is #[non_exhaustive], so we must handle unknown variants.
            #[allow(unreachable_patterns)]
            unknown => {
                debug_assert!(
                    false,
                    "Unhandled InflationLag variant: {:?}. Falling back to no lag.",
                    unknown
                );
                date
            }
        }
    }

    /// Compute signed year fraction for inflation curve lookups.
    ///
    /// Uses Act365F for inflation curve time calculations (see `InflationSwap::signed_year_fraction`
    /// for detailed rationale). The instrument's `dc` field is used for fixed leg accrual only.
    fn signed_year_fraction(start: Date, end: Date) -> f64 {
        if end >= start {
            DayCount::Act365F
                .year_fraction(start, end, DayCountCtx::default())
                .unwrap_or(0.0)
        } else {
            -DayCount::Act365F
                .year_fraction(end, start, DayCountCtx::default())
                .unwrap_or(0.0)
        }
    }

    fn cpi_value(
        &self,
        curves: &MarketContext,
        as_of: Date,
        date: Date,
    ) -> finstack_core::Result<f64> {
        if let Some(index) = curves.inflation_index(self.inflation_index_id.as_str()) {
            if let Ok(value) = index.value_on(date) {
                return Ok(value);
            }
        }

        let lag = self.effective_lag(curves);
        let lagged_date = Self::apply_lag(date, lag);
        let curve = curves.get_inflation(self.inflation_index_id.as_str())?;
        let t = Self::signed_year_fraction(as_of, lagged_date);
        Ok(curve.cpi(t))
    }

    fn schedule(&self) -> finstack_core::Result<Vec<(Date, Date, Date)>> {
        let bdc = self.bdc.unwrap_or(BusinessDayConvention::Following);
        let periods = crate::cashflow::builder::periods::build_periods(
            crate::cashflow::builder::periods::BuildPeriodsParams {
                start: self.start,
                end: self.maturity,
                frequency: self.frequency,
                stub: StubKind::None,
                bdc,
                calendar_id: self
                    .calendar_id
                    .as_deref()
                    .unwrap_or(crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID),
                end_of_month: false,
                day_count: self.dc,
                payment_lag_days: 0,
                reset_lag_days: None,
            },
        )?;

        if periods.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::InputError::Invalid,
            ));
        }

        Ok(periods
            .into_iter()
            .map(|period| {
                (
                    period.accrual_start,
                    period.accrual_end,
                    period.payment_date,
                )
            })
            .collect())
    }

    /// Calculates the raw present value (f64) of the YoY inflation swap.
    pub fn npv_raw(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        let disc = curves.get_discount(self.discount_curve_id.as_str())?;
        let mut pv = 0.0_f64;

        for (start, end, pay) in self.schedule()? {
            let accrual = self.dc.year_fraction(start, end, DayCountCtx::default())?;

            let cpi_start = self.cpi_value(curves, as_of, start)?;
            if cpi_start <= 0.0 {
                return Err(finstack_core::InputError::NonPositiveValue.into());
            }
            let cpi_end = self.cpi_value(curves, as_of, end)?;

            let inflation_leg = self.notional.amount() * (cpi_end / cpi_start - 1.0);
            let fixed_leg = self.notional.amount() * self.fixed_rate * accrual;

            let net = match self.side {
                PayReceiveInflation::PayFixed => inflation_leg - fixed_leg,
                PayReceiveInflation::ReceiveFixed => fixed_leg - inflation_leg,
            };

            let t_discount = disc
                .day_count()
                .year_fraction(as_of, pay, DayCountCtx::default())?;
            let df = disc.df(t_discount);
            pv += net * df;
        }

        Ok(pv)
    }

    /// Fixed rate that sets the swap's present value to zero (par rate / breakeven).
    ///
    /// For a YoY inflation swap, the par rate K satisfies:
    /// ```text
    /// Sum_i [ DF_i × (CPI_i / CPI_{i-1} - 1) ] = Sum_i [ DF_i × K × accrual_i ]
    ///
    /// K = Sum_i [ DF_i × (CPI_i / CPI_{i-1} - 1) ] / Sum_i [ DF_i × accrual_i ]
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Discount curve is not found
    /// - Inflation curve/index is not found
    /// - Any CPI value is non-positive
    /// - The annuity (sum of discounted accruals) is zero
    pub fn par_rate(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        let disc = curves.get_discount(self.discount_curve_id.as_str())?;

        let mut sum_infl_pv = 0.0_f64;
        let mut sum_annuity = 0.0_f64;

        for (start, end, pay) in self.schedule()? {
            let accrual = self.dc.year_fraction(start, end, DayCountCtx::default())?;

            let cpi_start = self.cpi_value(curves, as_of, start)?;
            if cpi_start <= 0.0 {
                return Err(finstack_core::InputError::NonPositiveValue.into());
            }
            let cpi_end = self.cpi_value(curves, as_of, end)?;

            let t_discount = disc
                .day_count()
                .year_fraction(as_of, pay, DayCountCtx::default())?;
            let df = disc.df(t_discount);

            // Inflation leg contribution: DF × (CPI_end / CPI_start - 1)
            sum_infl_pv += df * (cpi_end / cpi_start - 1.0);

            // Annuity contribution: DF × accrual
            sum_annuity += df * accrual;
        }

        if sum_annuity.abs() < 1e-15 {
            // Degenerate case: no accrual periods
            return Ok(0.0);
        }

        Ok(sum_infl_pv / sum_annuity)
    }
}

impl YoYInflationSwapBuilder {
    /// Set the fixed rate using a typed rate.
    pub fn fixed_rate_rate(mut self, rate: Rate) -> Self {
        self.fixed_rate = Some(rate.as_decimal());
        self
    }
}

impl crate::instruments::common_impl::traits::Instrument for YoYInflationSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::YoYInflationSwap
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common_impl::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common_impl::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common_impl::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        let pv = self.npv_raw(curves, as_of)?;
        Ok(finstack_core::money::Money::new(
            pv,
            self.notional.currency(),
        ))
    }

    fn value_raw(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        self.npv_raw(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common_impl::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
            None,
            None,
        )
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        Some(self.start)
    }
}

impl crate::instruments::common_impl::traits::CurveDependencies for YoYInflationSwap {
    fn curve_dependencies(&self) -> crate::instruments::common_impl::traits::InstrumentCurves {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.inflation_index_id.clone())
            .build()
    }
}
