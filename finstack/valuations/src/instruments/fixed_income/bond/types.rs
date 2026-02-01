//! Bond instrument types and implementations.

use crate::cashflow::builder::CashFlowSchedule;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;
use finstack_core::types::{Bps, CurveId, InstrumentId, Rate};
use finstack_core::Result;
use rust_decimal::prelude::ToPrimitive;
use time::macros::date;

use crate::instruments::common::validation;
// Re-export for compatibility in tests and external users referencing bond::AmortizationSpec
pub use super::cashflow_spec::CashflowSpec;
pub use crate::cashflow::builder::AmortizationSpec;

/// Bond instrument with fixed, floating, or amortizing cashflows.
///
/// Cashflow sign convention (holder view):
/// - All contractual cashflows **received by a long holder** (coupons,
///   amortization, final redemption) are represented as **positive** amounts.
/// - Cash outflows for the holder (e.g., purchase price, funding, short
///   positions) are represented as **negative** amounts and are handled at
///   trade level rather than in the bond's contractual schedule.
///
/// Supports call/put schedules, quoted prices for yield-to-maturity calculations,
/// and custom cashflow schedule overrides. Uses a clean `CashflowSpec` that wraps
/// the canonical builder coupon specs for maximum flexibility and parity.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
// Note: JsonSchema derive requires finstack-core types to implement JsonSchema
// #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Bond {
    /// Unique identifier for the bond.
    pub id: InstrumentId,
    /// Principal amount of the bond.
    pub notional: Money,
    /// Issue date of the bond.
    pub issue: Date,
    /// Maturity date of the bond.
    pub maturity: Date,
    /// Cashflow specification (fixed, floating, or amortizing).
    pub cashflow_spec: CashflowSpec,
    /// Discount curve identifier for pricing.
    pub discount_curve_id: CurveId,
    /// Optional credit curve identifier (default intensity). When present,
    /// credit-rate pricing is enabled.
    pub credit_curve_id: Option<CurveId>,
    /// Pricing overrides (including quoted clean price)
    #[cfg_attr(feature = "serde", serde(default))]
    #[builder(default)]
    pub pricing_overrides: PricingOverrides,
    /// Optional call/put schedule (dates and redemption prices as % of par amount).
    pub call_put: Option<CallPutSchedule>,
    /// Optional pre-built cashflow schedule. If provided, this will be used instead of
    /// generating cashflows from the cashflow_spec.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub custom_cashflows: Option<CashFlowSchedule>,
    /// Accrual method for interest calculation between coupon dates.
    ///
    /// Determines how accrued interest is calculated:
    /// - `Linear` (default): Simple interest interpolation (most bonds)
    /// - `Compounded`: Actuarial accrual per ICMA Rule 251 (some European bonds)
    /// - `Indexed`: Index ratio interpolation (inflation-linked bonds like TIPS)
    #[cfg_attr(feature = "serde", serde(default))]
    #[builder(default)]
    pub accrual_method: crate::cashflow::accrual::AccrualMethod,
    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
    /// Settlement convention: number of settlement days after trade date.
    pub settlement_days: Option<u32>,
    /// Ex-coupon convention: number of days before coupon date that go ex.
    pub ex_coupon_days: Option<u32>,
    /// Ex-coupon calendar identifier.
    ///
    /// If provided, ex-coupon days are treated as business days according to this calendar.
    /// If None, ex-coupon days are treated as calendar days (default).
    pub ex_coupon_calendar_id: Option<String>,
}

/// Call or put option on a bond.
///
/// Represents a single call or put option with an exercise date and redemption price.
/// Call options allow the issuer to redeem early; put options allow the holder to redeem early.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::fixed_income::bond::CallPut;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// // Call option: issuer can redeem at 102% of par on Jan 1, 2027
/// let call = CallPut {
///     date: Date::from_calendar_date(2027, Month::January, 1).unwrap(),
///     price_pct_of_par: 102.0,
/// };
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CallPut {
    /// Exercise date of the option.
    pub date: Date,
    /// Redemption price as percentage of par amount.
    pub price_pct_of_par: f64,
}

/// Schedule of call and put options for a bond.
///
/// Contains lists of call and put options that can be exercised during the bond's life.
/// Used for pricing callable/putable bonds and calculating yield-to-worst.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::fixed_income::bond::{CallPut, CallPutSchedule};
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// let mut schedule = CallPutSchedule::default();
/// schedule.calls.push(CallPut {
///     date: Date::from_calendar_date(2027, Month::January, 1).unwrap(),
///     price_pct_of_par: 102.0,
/// });
/// ```
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CallPutSchedule {
    /// Call options (issuer can redeem early).
    pub calls: Vec<CallPut>,
    /// Put options (holder can redeem early).
    pub puts: Vec<CallPut>,
}

impl CallPutSchedule {
    /// Check if this schedule has any active call or put options.
    ///
    /// # Returns
    ///
    /// `true` if the schedule contains at least one call or put option, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::fixed_income::bond::CallPutSchedule;
    ///
    /// let schedule = CallPutSchedule::default();
    /// assert!(!schedule.has_options());
    /// ```
    pub fn has_options(&self) -> bool {
        !self.calls.is_empty() || !self.puts.is_empty()
    }
}

impl Bond {
    /// Create a canonical example bond for testing and documentation.
    ///
    /// Returns a 10-year USD Treasury-style bond with realistic parameters.
    pub fn example() -> Self {
        // SAFETY: All inputs are compile-time validated constants
        Self::fixed(
            "US912828XG33",
            Money::new(1_000_000.0, Currency::USD),
            Rate::from_decimal(0.0425),
            date!(2024 - 01 - 15),
            date!(2034 - 01 - 15),
            "USD-TREASURY",
        )
        .unwrap_or_else(|_| unreachable!("Example bond with valid constants should never fail"))
    }

    /// Create a standard fixed-rate bond (most common use case).
    ///
    /// Creates a bond with semi-annual frequency and 30/360 day count following
    /// **US Corporate bond conventions**. For US Treasuries or other regional
    /// conventions, use `::with_convention()` or `::builder()` for full customization.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the bond
    /// * `notional` - Principal amount of the bond
    /// * `coupon_rate` - Annual coupon rate as a typed `Rate`
    /// * `issue` - Issue date of the bond
    /// * `maturity` - Maturity date of the bond
    /// * `discount_curve_id` - Discount curve identifier for pricing
    ///
    /// # Returns
    ///
    /// A `Bond` instance with US Corporate conventions (semi-annual, 30/360).
    ///
    /// # Panics
    ///
    /// Panics if bond construction fails (should not occur with valid inputs).
    ///
    /// # Regional Bond Conventions
    ///
    /// ## US Corporate (Default for this method)
    /// - **Day Count:** 30/360 (US Bond Basis)
    /// - **Tenor:** Semi-annual
    /// - **Settlement:** T+2
    /// - **Calendar:** US (NYSE holidays)
    ///
    /// ## US Treasury (use `BondConvention::USTreasury`)
    /// - **Day Count:** ACT/ACT ICMA
    /// - **Tenor:** Semi-annual
    /// - **Settlement:** T+1
    /// - **Calendar:** US (Federal Reserve holidays)
    ///
    /// ## US Agency (use `BondConvention::USAgency`)
    /// - **Day Count:** 30/360 (US Bond Basis)
    /// - **Tenor:** Semi-annual
    /// - **Settlement:** T+1
    /// - **Calendar:** US (NYSE holidays)
    ///
    /// ## United Kingdom (use `BondConvention::UKGilt`)
    /// - **Day Count:** ACT/ACT ICMA
    /// - **Tenor:** Semi-annual
    /// - **Settlement:** T+1
    /// - **Ex-coupon:** 7 business days
    /// - **Calendar:** UK (London holidays)
    ///
    /// ## Europe (use `BondConvention::GermanBund` or `FrenchOAT`)
    /// - **Day Count:** ACT/ACT ICMA
    /// - **Tenor:** Annual
    /// - **Settlement:** T+2
    /// - **Calendar:** TARGET2
    ///
    /// ## Japan (use `BondConvention::JGB`)
    /// - **Day Count:** ACT/365 Fixed
    /// - **Tenor:** Semi-annual
    /// - **Settlement:** T+2
    /// - **Calendar:** Japan holidays
    ///
    /// # Example
    /// ```rust,no_run
    /// use finstack_valuations::instruments::fixed_income::bond::Bond;
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    /// use time::macros::date;
    ///
    /// // US Corporate bond (default)
    /// let notional = Money::new(1_000_000.0, Currency::USD);
    /// let issue = date!(2025-01-15);
    /// let maturity = date!(2030-01-15);
    /// let corp_bond = Bond::fixed(
    ///     "CORP-001",
    ///     notional,
    ///     finstack_core::types::Rate::from_decimal(0.05),
    ///     issue,
    ///     maturity,
    ///     "USD-OIS",
    /// )
    /// .unwrap();
    /// # let _ = corp_bond;
    ///
    /// // For US Treasury, use with_convention:
    /// // let treasury = Bond::with_convention("UST-001", notional, 0.04, issue, maturity,
    /// //                                       BondConvention::USTreasury, "USD-TREASURY").unwrap();
    /// ```
    /// # Errors
    ///
    /// Returns an error if the builder fails validation.
    pub fn fixed(
        id: impl Into<InstrumentId>,
        notional: Money,
        coupon_rate: impl Into<Rate>,
        issue: Date,
        maturity: Date,
        discount_curve_id: impl Into<CurveId>,
    ) -> finstack_core::Result<Self> {
        let coupon_rate = coupon_rate.into();
        let bond = Self::builder()
            .id(id.into())
            .notional(notional)
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed_rate(
                coupon_rate,
                finstack_core::dates::Tenor::semi_annual(),
                DayCount::Thirty360,
            ))
            .discount_curve_id(discount_curve_id.into())
            .credit_curve_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .ex_coupon_calendar_id_opt(None)
            .build()?;

        // Validate all parameters before returning
        bond.validate()?;
        Ok(bond)
    }

    /// Create a bond with standard market conventions.
    ///
    /// Prefer typed rates for clarity and unit safety.
    ///
    /// Applies region-specific conventions for day count, frequency, and
    /// calendar adjustments. For full customization, use `::builder()`.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the bond
    /// * `notional` - Principal amount of the bond
    /// * `coupon_rate` - Annual coupon rate as a typed `Rate`
    /// * `issue` - Issue date of the bond
    /// * `maturity` - Maturity date of the bond
    /// * `convention` - Regional bond convention (e.g., `BondConvention::USTreasury`)
    /// * `discount_curve_id` - Discount curve identifier for pricing
    ///
    /// # Returns
    ///
    /// A `Bond` instance with conventions matching the specified regional standard.
    ///
    /// # Panics
    ///
    /// Panics if bond construction fails (should not occur with valid inputs).
    ///
    /// # Example
    /// ```rust,no_run
    /// use finstack_valuations::instruments::fixed_income::bond::Bond;
    /// use finstack_valuations::instruments::BondConvention;
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    /// use time::macros::date;
    ///
    /// let notional = Money::new(1_000_000.0, Currency::USD);
    /// let issue = date!(2025-01-15);
    /// let maturity = date!(2030-01-15);
    /// let treasury = Bond::with_convention(
    ///     "UST-5Y",
    ///     notional,
    ///     finstack_core::types::Rate::from_decimal(0.03),
    ///     issue,
    ///     maturity,
    ///     BondConvention::USTreasury,
    ///     "USD-TREASURY"
    /// );
    /// # let _ = treasury;
    /// ```
    /// # Errors
    ///
    /// Returns an error if the builder fails validation.
    pub fn with_convention(
        id: impl Into<InstrumentId>,
        notional: Money,
        coupon_rate: impl Into<Rate>,
        issue: Date,
        maturity: Date,
        convention: crate::instruments::common::parameters::BondConvention,
        discount_curve_id: impl Into<CurveId>,
    ) -> finstack_core::Result<Self> {
        let coupon_rate = coupon_rate.into();
        let bond = Self::builder()
            .id(id.into())
            .notional(notional)
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed_rate(
                coupon_rate,
                convention.frequency(),
                convention.day_count(),
            ))
            .discount_curve_id(discount_curve_id.into())
            .credit_curve_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .ex_coupon_calendar_id_opt(None)
            .build()?;

        // Validate all parameters before returning
        bond.validate()?;
        Ok(bond)
    }

    /// Create a floating-rate bond (FRN).
    ///
    /// Creates a bond with floating-rate coupons linked to a forward index
    /// (e.g., SOFR, EURIBOR) plus a margin.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the bond
    /// * `notional` - Principal amount of the bond
    /// * `index_id` - Forward curve identifier (e.g., "USD-SOFR-3M")
    /// * `margin_bp` - Spread over index in typed basis points (e.g., `Bps::new(200)`)
    /// * `issue` - Issue date of the bond
    /// * `maturity` - Maturity date of the bond
    /// * `freq` - Payment frequency (e.g., `Tenor::quarterly()`)
    /// * `dc` - Day count convention (e.g., `DayCount::Act360`)
    /// * `discount_curve_id` - Discount curve identifier for pricing
    ///
    /// # Returns
    ///
    /// A `Bond` instance configured as a floating-rate note.
    ///
    /// # Panics
    ///
    /// Panics if bond construction fails (should not occur with valid inputs).
    ///
    /// # Example
    /// ```rust,no_run
    /// use finstack_valuations::instruments::fixed_income::bond::Bond;
    /// use finstack_core::dates::{Tenor, DayCount};
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    /// use time::macros::date;
    ///
    /// // 3M SOFR + 200bps, quarterly payments
    /// let notional = Money::new(1_000_000.0, Currency::USD);
    /// let issue = date!(2025-01-15);
    /// let maturity = date!(2030-01-15);
    /// let frn = Bond::floating(
    ///     "FRN-001",
    ///     notional,
    ///     "USD-SOFR-3M",
    ///     finstack_core::types::Bps::new(200),
    ///     issue,
    ///     maturity,
    ///     Tenor::quarterly(),
    ///     DayCount::Act360,
    ///     "USD-OIS"
    /// );
    /// # let _ = frn;
    /// ```
    /// # Errors
    ///
    /// Returns an error if the builder fails validation.
    #[allow(clippy::too_many_arguments)]
    pub fn floating(
        id: impl Into<InstrumentId>,
        notional: Money,
        index_id: impl Into<CurveId>,
        margin_bp: impl Into<Bps>,
        issue: Date,
        maturity: Date,
        freq: finstack_core::dates::Tenor,
        dc: DayCount,
        discount_curve_id: impl Into<CurveId>,
    ) -> finstack_core::Result<Self> {
        let margin_bp = margin_bp.into();
        let bond = Self::builder()
            .id(id.into())
            .notional(notional)
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::floating_bps(
                index_id.into(),
                margin_bp,
                freq,
                dc,
            ))
            .discount_curve_id(discount_curve_id.into())
            .credit_curve_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .ex_coupon_calendar_id_opt(None)
            .build()?;

        // Validate all parameters before returning
        bond.validate()?;
        Ok(bond)
    }

    /// Create a bond from a pre-built cashflow schedule.
    ///
    /// This extracts key bond parameters from the cashflow schedule and creates
    /// a bond that will use these custom cashflows for all calculations.
    /// Use this for complex structures like PIK bonds, fixed-to-floating, or
    /// custom amortization schedules.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the bond
    /// * `schedule` - Pre-built cashflow schedule
    /// * `discount_curve_id` - Discount curve identifier for pricing
    /// * `quoted_clean` - Optional quoted clean price as percentage of par
    ///
    /// # Returns
    ///
    /// A `Bond` instance configured to use the provided custom cashflow schedule.
    ///
    /// # Errors
    ///
    /// Returns `Err` when:
    /// - Schedule has fewer than 2 dates
    /// - Bond construction fails
    ///
    /// # Example
    /// ```rust,no_run
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
    /// use finstack_core::money::Money;
    /// use finstack_valuations::cashflow::builder::{CashFlowSchedule, CouponType, FixedCouponSpec};
    /// use finstack_valuations::instruments::Bond;
    /// use rust_decimal_macros::dec;
    /// use time::macros::date;
    ///
    /// # fn main() -> finstack_core::Result<()> {
    /// // Build a custom schedule (PIK, step-ups, amortization, etc.) with `CashFlowSchedule::builder()`.
    /// let issue = date!(2025-01-15);
    /// let maturity = date!(2027-01-15);
    /// let fixed_spec = FixedCouponSpec {
    ///     coupon_type: CouponType::Cash,
    ///     rate: dec!(0.06),
    ///     freq: Tenor::semi_annual(),
    ///     dc: DayCount::Act365F,
    ///     bdc: BusinessDayConvention::Following,
    ///     calendar_id: None,
    ///     stub: StubKind::None,
    /// };
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    ///     .fixed_cf(fixed_spec)
    ///     .build_with_curves(None)?;
    ///
    /// let bond = Bond::from_cashflows("PIK-001", schedule, "USD-HY", Some(95.0))?;
    /// # let _ = bond;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_cashflows(
        id: impl Into<InstrumentId>,
        schedule: CashFlowSchedule,
        discount_curve_id: impl Into<CurveId>,
        quoted_clean: Option<f64>,
    ) -> finstack_core::Result<Self> {
        // Extract parameters from the schedule
        let notional = schedule.notional.initial;

        // Find issue and maturity from the cashflow dates
        let dates = schedule.dates();
        if dates.len() < 2 {
            return Err(finstack_core::InputError::TooFewPoints.into());
        }
        let issue = dates[0];
        let maturity = dates
            .last()
            .copied()
            .ok_or(finstack_core::InputError::TooFewPoints)?;

        // Infer a representative coupon frequency from the schedule's coupon dates.
        //
        // This is used only for yield/duration conventions; the actual cashflows
        // always come from `custom_cashflows`.
        //
        // We compute the mode (most frequent) interval across all consecutive
        // coupon dates, ignoring potential stubs at the front or back. This is
        // more robust than using just the first interval, which may be a stub.
        use crate::cashflow::primitives::CFKind;
        use finstack_core::dates::Tenor;
        use std::collections::HashMap;

        let mut coupon_dates: Vec<Date> = schedule
            .flows
            .iter()
            .filter(|cf| matches!(cf.kind, CFKind::Fixed | CFKind::Stub | CFKind::FloatReset))
            .map(|cf| cf.date)
            .collect();
        coupon_dates.sort();
        coupon_dates.dedup();

        let inferred_freq = if coupon_dates.len() < 2 {
            // Fallback to annual if we cannot infer a pattern
            Tenor::annual()
        } else {
            // Compute all interval lengths between consecutive coupon dates
            let mut interval_counts: HashMap<i64, usize> = HashMap::new();
            for window in coupon_dates.windows(2) {
                let d0 = window[0];
                let d1 = window[1];
                let days = (d1 - d0).whole_days().abs();
                // Bucket into standard frequency ranges for robust mode detection
                let bucket = match days {
                    360..=370 => 365, // Annual
                    178..=187 => 182, // Semi-annual
                    88..=95 => 91,    // Quarterly
                    27..=35 => 30,    // Monthly
                    6..=8 => 7,       // Weekly
                    _ => days,        // Non-standard interval
                };
                *interval_counts.entry(bucket).or_insert(0) += 1;
            }

            // Find the mode (most frequent interval)
            let (mode_days, _mode_count) = interval_counts
                .iter()
                .max_by_key(|(_, count)| *count)
                .map(|(days, count)| (*days, *count))
                .unwrap_or((365, 1));

            // Map bucketed mode to standard Tenor
            match mode_days {
                365 => Tenor::annual(),
                182 => Tenor::semi_annual(),
                91 => Tenor::quarterly(),
                30 => Tenor::monthly(),
                7 => Tenor::weekly(),
                _ => finstack_core::dates::Tenor::new(
                    mode_days as u32,
                    finstack_core::dates::TenorUnit::Days,
                ),
            }
        };

        // Use schedule day count and inferred frequency in the spec so that
        // YTM/YTW/duration/convexity use correct conventions for custom bonds.
        let cashflow_spec = CashflowSpec::fixed(0.0, inferred_freq, schedule.day_count);

        let pricing_overrides = if let Some(price) = quoted_clean {
            PricingOverrides::default().with_clean_price(price)
        } else {
            PricingOverrides::default()
        };

        Self::builder()
            .id(id.into())
            .notional(notional)
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(cashflow_spec)
            .discount_curve_id(discount_curve_id.into())
            .credit_curve_id_opt(None)
            .pricing_overrides(pricing_overrides)
            .custom_cashflows_opt(Some(schedule))
            .attributes(Attributes::new())
            .ex_coupon_calendar_id_opt(None)
            .build()
    }

    /// Set custom cashflows for this bond.
    ///
    /// When custom cashflows are set, they will be used instead of generating
    /// cashflows from the bond's coupon and amortization specifications.
    ///
    /// # Arguments
    ///
    /// * `schedule` - Custom cashflow schedule to use for pricing and metrics
    ///
    /// # Returns
    ///
    /// The bond instance with custom cashflows set.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::fixed_income::bond::Bond;
    /// use finstack_valuations::cashflow::builder::CashFlowSchedule;
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    ///
    /// # let bond = Bond::example();
    /// # let schedule = CashFlowSchedule::builder()
    /// #     .principal(Money::new(1_000_000.0, Currency::USD), Date::from_calendar_date(2024, time::Month::January, 1).unwrap(), Date::from_calendar_date(2034, time::Month::January, 1).unwrap())
    /// #     .build_with_curves(None).unwrap();
    /// let bond_with_custom = bond.with_cashflows(schedule);
    /// ```
    pub fn with_cashflows(mut self, schedule: CashFlowSchedule) -> Self {
        self.custom_cashflows = Some(schedule);
        self
    }

    /// Build accrual configuration from bond's accrual method and ex-coupon convention.
    ///
    /// This helper creates the generic `AccrualConfig` needed by the cashflow
    /// accrual engine, incorporating both the accrual method and ex-coupon rules.
    ///
    /// # Returns
    ///
    /// An `AccrualConfig` combining the bond's accrual method and ex-coupon convention.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::fixed_income::bond::Bond;
    ///
    /// # let bond = Bond::example();
    /// let accrual_config = bond.accrual_config();
    /// // Use with accrual engine
    /// ```
    pub fn accrual_config(&self) -> crate::cashflow::accrual::AccrualConfig {
        crate::cashflow::accrual::AccrualConfig {
            method: self.accrual_method.clone(),
            ex_coupon: self
                .ex_coupon_days
                .map(|d| crate::cashflow::accrual::ExCouponRule {
                    days_before_coupon: d,
                    calendar_id: self.ex_coupon_calendar_id.clone(),
                }),
            include_pik: true,
        }
    }

    /// Get the full cashflow schedule with kinds for this bond.
    ///
    /// This returns the complete `CashFlowSchedule` including all cashflow types
    /// (Fixed, Float, PIK, Amortization, Notional, etc.) and metadata in the
    /// builder's native convention (typically issuer view).
    ///
    /// For floating rate bonds, requires market curves to properly compute floating
    /// coupon amounts (forward rate + discount margin).
    ///
    /// Note: Amortization amounts are stored as POSITIVE values in the schedule.
    ///
    /// # Arguments
    ///
    /// * `curves` - Market context containing discount and forward curves
    ///
    /// # Returns
    ///
    /// The complete `CashFlowSchedule` with all cashflow types and metadata.
    ///
    /// # Errors
    ///
    /// Returns `Err` when:
    /// - Market curves are missing (for floating rate bonds)
    /// - Cashflow schedule building fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::fixed_income::bond::Bond;
    /// use finstack_core::market_data::context::MarketContext;
    ///
    /// # let bond = Bond::example();
    /// # let curves = MarketContext::new();
    /// let schedule = bond.get_full_schedule(&curves)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_full_schedule(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
    ) -> Result<CashFlowSchedule> {
        use crate::cashflow::builder::CashFlowSchedule;

        // If custom cashflows are set, return them directly
        if let Some(ref custom) = self.custom_cashflows {
            return Ok(custom.clone());
        }

        // Build the schedule using the cashflow builder and cashflow_spec
        let mut b = CashFlowSchedule::builder();
        let _ = b.principal(self.notional, self.issue, self.maturity);

        // Match on the cashflow spec variant
        match &self.cashflow_spec {
            CashflowSpec::Fixed(spec) => {
                let _ = b.fixed_cf(spec.clone());
            }
            CashflowSpec::Floating(spec) => {
                let _ = b.floating_cf(spec.clone());
            }
            CashflowSpec::Amortizing { base, schedule } => {
                let _ = b.amortization(schedule.clone());
                match &**base {
                    CashflowSpec::Fixed(spec) => {
                        let _ = b.fixed_cf(spec.clone());
                    }
                    CashflowSpec::Floating(spec) => {
                        let _ = b.floating_cf(spec.clone());
                    }
                    CashflowSpec::Amortizing { .. } => {
                        return Err(finstack_core::InputError::Invalid.into());
                    }
                }
            }
        }

        // Build the schedule with market curves for floating rate computation
        b.build_with_curves(Some(curves))
    }

    /// Price bond using tree-based pricing for embedded options (calls/puts).
    ///
    /// This method is automatically called by `value()` when the bond has a non-empty
    /// call/put schedule. It uses a short-rate tree model to properly value the
    /// embedded optionality via backward induction.
    ///
    /// # Arguments
    /// * `market` - Market context with discount curve (and optionally hazard curve)
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    /// Option-adjusted present value of the bond
    fn value_with_tree(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::common::models::{
            short_rate_keys, state_keys, ShortRateTree, ShortRateTreeConfig, StateVariables,
            TreeModel,
        };
        use crate::instruments::fixed_income::bond::pricing::tree_engine::{
            bond_tree_config, BondValuator,
        };

        // Calculate time to maturity from the valuation date (as_of) using the
        // discount curve's day-count convention to ensure consistency with tree calibration.
        let discount_curve = market.get_discount(&self.discount_curve_id)?;
        let time_to_maturity = discount_curve.day_count().year_fraction(
            as_of,
            self.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if time_to_maturity <= 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        // Use centralized tree config from pricing_overrides (or defaults)
        let config = bond_tree_config(self);
        let tree_steps = config.tree_steps;
        let volatility = config.volatility;

        let tree_config = ShortRateTreeConfig {
            steps: tree_steps,
            volatility,
            ..Default::default()
        };

        // Initialize and calibrate short-rate tree to match discount curve
        let mut tree = ShortRateTree::new(tree_config);
        tree.calibrate(discount_curve.as_ref(), time_to_maturity)?;

        // Create bond valuator with call/put schedule mapped to tree steps
        let valuator =
            BondValuator::new(self.clone(), market, as_of, time_to_maturity, tree_steps)?;

        // Set up initial state variables (no OAS for vanilla pricing)
        let initial_rate = tree
            .rate_at_node(0, 0)
            .unwrap_or_else(|_| discount_curve.zero(0.0));
        let mut vars = StateVariables::default();
        vars.insert(state_keys::INTEREST_RATE, initial_rate);
        vars.insert(short_rate_keys::OAS, 0.0);

        // Price via tree with backward induction applying call/put constraints
        let price_amount = tree.price(vars, time_to_maturity, market, &valuator)?;

        Ok(Money::new(price_amount, self.notional.currency()))
    }

    /// Validate all bond parameters.
    ///
    /// Performs comprehensive validation of the bond instrument:
    /// - Issue date must be before maturity date
    /// - Notional must be positive
    /// - Coupon rate must be non-negative (for fixed-rate bonds)
    /// - Call/put prices must be positive
    ///
    /// # Errors
    ///
    /// Returns `Error::Validation` with a descriptive message if any validation fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let bond = Bond::fixed(...)?;
    /// bond.validate()?; // Validates all parameters
    /// ```
    pub fn validate(&self) -> Result<()> {
        // Validate date ordering
        validation::validate_date_range_strict_with(self.issue, self.maturity, |start, end| {
            format!(
                "Bond issue date ({}) must be before maturity date ({})",
                start, end
            )
        })?;

        // Validate notional is positive
        validation::validate_money_gt_with(self.notional, 0.0, |amount| {
            format!("Bond notional must be positive, got {}", amount)
        })?;

        // Validate coupon rate for fixed-rate bonds
        if let CashflowSpec::Fixed(spec) = &self.cashflow_spec {
            // Convert Decimal to f64 for comparison
            let rate = spec.rate.to_f64().unwrap_or(0.0);
            if rate < 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "Bond fixed coupon rate must be non-negative, got {}",
                    rate
                )));
            }
        }

        // Validate call/put prices
        if let Some(ref call_put) = self.call_put {
            for call in &call_put.calls {
                if call.price_pct_of_par <= 0.0 {
                    return Err(finstack_core::Error::Validation(format!(
                        "Bond call price must be positive, got {} on {}",
                        call.price_pct_of_par, call.date
                    )));
                }
            }
            for put in &call_put.puts {
                if put.price_pct_of_par <= 0.0 {
                    return Err(finstack_core::Error::Validation(format!(
                        "Bond put price must be positive, got {} on {}",
                        put.price_pct_of_par, put.date
                    )));
                }
            }
        }

        Ok(())
    }
}

// Explicit trait implementations for better IDE support and code clarity

// Explicit Instrument trait implementation (replaces macro for better IDE visibility)
impl crate::instruments::common::traits::Instrument for Bond {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::Bond
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
        // Check if bond has embedded options requiring tree-based pricing
        if let Some(ref cp) = self.call_put {
            if cp.has_options() {
                return self.value_with_tree(curves, as_of);
            }
        }

        // Standard cashflow discounting for straight bonds using bond cashflows
        // sized under the bond's own day-count and discount factors provided by
        // the assigned discount curve.
        crate::instruments::fixed_income::bond::pricing::discount_engine::BondEngine::price(
            self, curves, as_of,
        )
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
            None,
            None,
        )
    }

    fn market_dependencies(&self) -> crate::instruments::common::dependencies::MarketDependencies {
        crate::instruments::common::dependencies::MarketDependencies::from_curve_dependencies(self)
    }

    fn as_cashflow_provider(&self) -> Option<&dyn crate::cashflow::traits::CashflowProvider> {
        Some(self)
    }

    fn scenario_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&mut self.pricing_overrides)
    }

    fn scenario_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&self.pricing_overrides)
    }
}

// Implement CurveDependencies for DV01/CS01 calculators
impl crate::instruments::common::traits::CurveDependencies for Bond {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        let mut builder = crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone());

        // Add credit curve if present
        if let Some(ref credit_curve_id) = self.credit_curve_id {
            builder = builder.credit(credit_curve_id.clone());
        }

        // For floating rate bonds, add forward curve from the cashflow spec
        match &self.cashflow_spec {
            CashflowSpec::Floating(floating_spec) => {
                builder = builder.forward(floating_spec.rate_spec.index_id.clone());
            }
            CashflowSpec::Amortizing { base, .. } => {
                // Check if the base spec is floating
                if let CashflowSpec::Floating(floating_spec) = base.as_ref() {
                    builder = builder.forward(floating_spec.rate_spec.index_id.clone());
                }
            }
            _ => {}
        }

        builder.build()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::cashflow::builder::{CashFlowSchedule, CouponType, FixedCouponSpec, ScheduleParams};
    use crate::cashflow::traits::CashflowProvider;
    use crate::instruments::common::traits::Instrument;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::market_data::term_structures::ForwardCurve;
    use finstack_core::math::interp::InterpStyle;
    use rust_decimal::Decimal;
    use time::Month;

    #[test]
    fn test_bond_with_custom_cashflows() {
        // Setup dates
        let issue = Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");
        let maturity = Date::from_calendar_date(2027, Month::January, 15).expect("Valid test date");

        // Build a custom cashflow schedule with step-up coupons
        let schedule_params = ScheduleParams {
            freq: Tenor::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        };

        let step1_date =
            Date::from_calendar_date(2026, Month::January, 15).expect("Valid test date");

        let custom_schedule = CashFlowSchedule::builder()
            .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
            .fixed_stepup(
                &[(step1_date, 0.03), (maturity, 0.05)],
                schedule_params,
                CouponType::Cash,
            )
            .build_with_curves(None)
            .expect("CashFlowSchedule builder should succeed with valid test data");

        // Create bond from custom cashflows
        let bond = Bond::from_cashflows(
            "CUSTOM_STEPUP_BOND",
            custom_schedule.clone(),
            "USD-OIS",
            Some(98.5),
        )
        .expect("Bond::from_cashflows should succeed with valid test data");

        // Verify bond properties
        assert_eq!(bond.id.as_str(), "CUSTOM_STEPUP_BOND");
        assert_eq!(bond.discount_curve_id.as_str(), "USD-OIS");
        assert_eq!(bond.pricing_overrides.quoted_clean_price, Some(98.5));
        assert_eq!(bond.issue, issue);
        assert_eq!(bond.maturity, maturity);
        assert!(bond.custom_cashflows.is_some());

        // Create curves for pricing
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (3.0, 0.95)])
            .set_interp(finstack_core::math::interp::InterpStyle::Linear)
            .build()
            .expect("CashFlowSchedule builder should succeed with valid test data");
        let curves = MarketContext::new().insert_discount(disc_curve);

        // Build schedule and verify it uses custom cashflows
        let flows = bond
            .build_dated_flows(&curves, issue)
            .expect("Schedule building should succeed in test");
        assert!(!flows.is_empty());

        // The flows should match what we put in the custom schedule
        // (after conversion for holder perspective)
        let expected_flow_count = custom_schedule
            .flows
            .iter()
            .filter(|cf| {
                use crate::cashflow::primitives::CFKind;
                matches!(
                    cf.kind,
                    CFKind::Fixed | CFKind::Stub | CFKind::Amortization | CFKind::Notional
                ) && (cf.kind != CFKind::Notional || cf.amount.amount() > 0.0)
            })
            .count();
        assert_eq!(flows.len(), expected_flow_count);
    }

    #[test]
    fn test_bond_builder_with_custom_cashflows() {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2026, Month::January, 1).expect("Valid test date");

        // Build custom cashflow with PIK toggle
        let custom_schedule = CashFlowSchedule::builder()
            .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Split {
                    cash_pct: Decimal::try_from(0.5).expect("valid"),
                    pik_pct: Decimal::try_from(0.5).expect("valid"),
                },
                rate: Decimal::try_from(0.06).expect("valid"),
                freq: Tenor::quarterly(),
                dc: DayCount::Thirty360,
                bdc: BusinessDayConvention::Following,
                calendar_id: None,
                stub: StubKind::None,
            })
            .build_with_curves(None)
            .expect("CashFlowSchedule builder should succeed with valid test data");

        // Use builder pattern (default cashflow_spec since custom_cashflows overrides)
        let bond = Bond::builder()
            .id("PIK_TOGGLE_BOND".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::default())
            .custom_cashflows_opt(Some(custom_schedule))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .pricing_overrides(PricingOverrides::default().with_clean_price(99.0))
            .attributes(Attributes::new())
            .build()
            .expect("CashFlowSchedule builder should succeed with valid test data");

        assert_eq!(bond.id.as_str(), "PIK_TOGGLE_BOND");
        assert_eq!(bond.discount_curve_id.as_str(), "USD-OIS");
        assert_eq!(bond.pricing_overrides.quoted_clean_price, Some(99.0));
        assert!(bond.custom_cashflows.is_some());
        assert_eq!(bond.notional.currency(), Currency::USD);
    }

    #[test]
    fn test_bond_with_cashflows_method() {
        let issue = Date::from_calendar_date(2025, Month::March, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::March, 1).expect("Valid test date");

        // Create a traditional bond first (builder)
        let mut bond = Bond::builder()
            .id(InstrumentId::new("REGULAR_BOND"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed(
                0.04,
                Tenor::semi_annual(),
                DayCount::Act365F,
            ))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .credit_curve_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .call_put_opt(None)
            .custom_cashflows_opt(None)
            .attributes(Attributes::new())
            .settlement_days_opt(None)
            .ex_coupon_days_opt(None)
            .ex_coupon_calendar_id_opt(None)
            .build()
            .expect("CashFlowSchedule builder should succeed with valid test data");

        // Build a custom schedule separately
        let custom_schedule = CashFlowSchedule::builder()
            .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: Decimal::try_from(0.055).expect("valid"), // Different from default spec
                freq: Tenor::quarterly(),
                dc: DayCount::Act365F,
                bdc: BusinessDayConvention::Following,
                calendar_id: None,
                stub: StubKind::None,
            })
            .build_with_curves(None)
            .expect("CashFlowSchedule builder should succeed with valid test data");

        // Apply custom cashflows
        bond = bond.with_cashflows(custom_schedule);

        assert!(bond.custom_cashflows.is_some());
        // The original cashflow_spec is preserved but custom_cashflows takes precedence
    }

    #[test]
    fn test_custom_cashflows_override_regular_generation() {
        let issue = Date::from_calendar_date(2025, Month::June, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2026, Month::June, 1).expect("Valid test date");

        // Create bond with regular specs (builder)
        let regular_bond = Bond::builder()
            .id(InstrumentId::new("TEST"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed(
                0.03,
                Tenor::annual(),
                DayCount::Act365F,
            ))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .credit_curve_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .call_put_opt(None)
            .custom_cashflows_opt(None)
            .attributes(Attributes::new())
            .settlement_days_opt(None)
            .ex_coupon_days_opt(None)
            .ex_coupon_calendar_id_opt(None)
            .build()
            .expect("CashFlowSchedule builder should succeed with valid test data");

        // Same bond with custom cashflows
        let custom_schedule = CashFlowSchedule::builder()
            .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: Decimal::try_from(0.05).expect("valid"), // Different rate
                freq: Tenor::semi_annual(),                    // Different frequency
                dc: DayCount::Act365F,
                bdc: BusinessDayConvention::Following,
                calendar_id: None,
                stub: StubKind::None,
            })
            .build_with_curves(None)
            .expect("CashFlowSchedule builder should succeed with valid test data");

        let custom_bond = regular_bond.clone().with_cashflows(custom_schedule);

        // Create curves
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (2.0, 0.98)])
            .set_interp(finstack_core::math::interp::InterpStyle::Linear)
            .build()
            .expect("CashFlowSchedule builder should succeed with valid test data");
        let curves = MarketContext::new().insert_discount(disc_curve);

        // Build schedules
        let regular_flows = regular_bond
            .build_dated_flows(&curves, issue)
            .expect("Schedule building should succeed in test");
        let custom_flows = custom_bond
            .build_dated_flows(&curves, issue)
            .expect("Schedule building should succeed in test");

        // Should have different number of flows due to different frequency
        assert_ne!(regular_flows.len(), custom_flows.len());

        // Custom bond should have semi-annual flows (more flows)
        assert!(custom_flows.len() > regular_flows.len());
    }

    #[test]
    fn test_bond_floating_value() {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2027, Month::January, 1).expect("Valid test date");
        let notional = Money::new(1_000_000.0, Currency::USD);

        // Curves
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (2.0, 0.95)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("CashFlowSchedule builder should succeed with valid test data");
        let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
            .base_date(issue)
            .knots([(0.0, 0.05), (2.0, 0.055)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("CashFlowSchedule builder should succeed with valid test data");
        let ctx = MarketContext::new()
            .insert_discount(disc)
            .insert_forward(fwd);

        let bond = Bond::floating(
            "FRN-TEST",
            notional,
            "USD-SOFR-3M",
            150,
            issue,
            maturity,
            Tenor::quarterly(),
            DayCount::Act360,
            "USD-OIS",
        )
        .unwrap();

        // Price should be finite and positive under positive forwards
        let pv = bond
            .value(&ctx, issue)
            .expect("Bond valuation should succeed in test");
        assert!(pv.amount().is_finite());
    }

    #[test]
    fn test_bond_frn_ex_coupon_accrual_zero_in_window() {
        use crate::cashflow::primitives::CFKind;
        use time::Duration;

        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2027, Month::January, 1).expect("Valid test date");
        let notional = Money::new(1_000_000.0, Currency::USD);

        // Curves
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (2.0, 0.95)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed in test");
        let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
            .base_date(issue)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.05), (2.0, 0.055)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("ForwardCurve builder should succeed in test");
        let ctx = MarketContext::new()
            .insert_discount(disc)
            .insert_forward(fwd);

        let mut bond = Bond::floating(
            "FRN-EX-COUPON",
            notional,
            "USD-SOFR-3M",
            150,
            issue,
            maturity,
            Tenor::quarterly(),
            DayCount::Act360,
            "USD-OIS",
        )
        .unwrap();
        // Apply an ex-coupon convention of 5 days
        bond.ex_coupon_days = Some(5);

        // Use the full schedule to locate the first coupon end date
        let full_schedule = bond
            .get_full_schedule(&ctx)
            .expect("Full schedule retrieval should succeed in test");
        let first_coupon_date = full_schedule
            .flows
            .iter()
            .filter(|cf| matches!(cf.kind, CFKind::Fixed | CFKind::FloatReset | CFKind::Stub))
            .map(|cf| cf.date)
            .filter(|d| *d > issue)
            .min()
            .expect("FRN should have at least one coupon date in test");

        let ex_date = first_coupon_date - Duration::days(5);
        let day_before_ex = ex_date - Duration::days(1);

        // Before ex-date, accrued interest should be positive
        let schedule_before = bond
            .get_full_schedule(&ctx)
            .expect("Full schedule should build");
        let ai_before = crate::cashflow::accrual::accrued_interest_amount(
            &schedule_before,
            day_before_ex,
            &bond.accrual_config(),
        )
        .expect("Accrued interest calculation should succeed before ex-date");
        assert!(
            ai_before > 0.0,
            "Accrued interest should be positive before ex-date"
        );

        // On or inside the ex-coupon window, accrued interest should be zero
        let schedule_ex = bond
            .get_full_schedule(&ctx)
            .expect("Full schedule should build");
        let ai_ex = crate::cashflow::accrual::accrued_interest_amount(
            &schedule_ex,
            ex_date,
            &bond.accrual_config(),
        )
        .expect("Accrued interest calculation should succeed on ex-date");
        assert!(
            ai_ex == 0.0,
            "Accrued interest in ex-coupon window should be zero, got {}",
            ai_ex
        );
    }

    #[test]
    fn test_amortizing_bond_ex_coupon_accrual_zero_in_window() {
        use crate::cashflow::builder::AmortizationSpec;
        use crate::cashflow::primitives::CFKind;
        use time::Duration;

        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2028, Month::January, 1).expect("Valid test date");
        let notional = Money::new(1_000_000.0, Currency::USD);

        // Amortizing bond with annual 5% coupon, 1/3 principal returned each year.
        // StepRemaining schedule specifies remaining balance AFTER each date.
        // After step1: 2/3 remaining (paid 1/3), after step2: 1/3 remaining (paid 2/3),
        // after maturity: 0 remaining (all paid).
        let step1 = Date::from_calendar_date(2026, Month::January, 1).expect("Valid test date");
        let step2 = Date::from_calendar_date(2027, Month::January, 1).expect("Valid test date");
        let amort_spec = AmortizationSpec::StepRemaining {
            schedule: vec![
                (step1, Money::new(2.0 * 1_000_000.0 / 3.0, Currency::USD)), // 2/3 remaining
                (step2, Money::new(1_000_000.0 / 3.0, Currency::USD)),       // 1/3 remaining
                (maturity, Money::new(0.0, Currency::USD)),                  // 0 remaining
            ],
        };
        let base_spec = CashflowSpec::fixed(0.05, Tenor::annual(), DayCount::Act365F);
        let cashflow_spec = CashflowSpec::amortizing(base_spec, amort_spec);

        let mut bond = Bond::builder()
            .id("AMORT-EX-COUPON".into())
            .notional(notional)
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(cashflow_spec)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Amortizing bond construction should succeed in test");

        // Apply an ex-coupon convention of 7 days
        bond.ex_coupon_days = Some(7);

        // Curves for pricing (levels are not important for accrual)
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (3.0, 0.9)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed in test");
        let ctx = MarketContext::new().insert_discount(disc_curve);

        // Use the full schedule to locate the first coupon end date
        let full_schedule = bond
            .get_full_schedule(&ctx)
            .expect("Full schedule retrieval should succeed in test");
        let first_coupon_date = full_schedule
            .flows
            .iter()
            .filter(|cf| matches!(cf.kind, CFKind::Fixed | CFKind::Stub))
            .map(|cf| cf.date)
            .filter(|d| *d > issue)
            .min()
            .expect("Amortizing bond should have at least one coupon date in test");

        let ex_date = first_coupon_date - Duration::days(7);
        let day_before_ex = ex_date - Duration::days(1);

        // Before ex-date, accrued interest should be positive
        let schedule_before = bond
            .get_full_schedule(&ctx)
            .expect("Full schedule should build");
        let ai_before = crate::cashflow::accrual::accrued_interest_amount(
            &schedule_before,
            day_before_ex,
            &bond.accrual_config(),
        )
        .expect("Accrued interest calculation should succeed before ex-date");
        assert!(
            ai_before > 0.0,
            "Accrued interest should be positive before ex-date for amortizing bond"
        );

        // On or inside the ex-coupon window, accrued interest should be zero
        let schedule_ex = bond
            .get_full_schedule(&ctx)
            .expect("Full schedule should build");
        let ai_ex = crate::cashflow::accrual::accrued_interest_amount(
            &schedule_ex,
            ex_date,
            &bond.accrual_config(),
        )
        .expect("Accrued interest calculation should succeed on ex-date");
        assert!(
            ai_ex == 0.0,
            "Accrued interest in ex-coupon window should be zero for amortizing bond, got {}",
            ai_ex
        );
    }

    #[test]
    fn test_bond_frn_build_dated_flows_uses_builder() {
        use crate::cashflow::primitives::CFKind;
        use crate::cashflow::traits::CashflowProvider;

        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2026, Month::January, 1).expect("Valid test date");

        // Create FRN
        let frn = Bond::floating(
            "FRN-BUILDER-TEST",
            Money::new(1_000_000.0, Currency::USD),
            "USD-SOFR",
            100,
            issue,
            maturity,
            Tenor::quarterly(),
            DayCount::Act360,
            "USD-OIS",
        )
        .unwrap();

        // Create market with forward curve
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (1.0, 0.95)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("CashFlowSchedule builder should succeed with valid test data");

        let fwd_curve = ForwardCurve::builder("USD-SOFR", 0.25)
            .base_date(issue)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.03), (1.0, 0.035)])
            .build()
            .expect("CashFlowSchedule builder should succeed with valid test data");

        let market = MarketContext::new()
            .insert_discount(disc_curve)
            .insert_forward(fwd_curve);

        // Get full schedule to verify it includes FloatReset CFKind
        let full_schedule = frn
            .get_full_schedule(&market)
            .expect("Full schedule retrieval should succeed in test");
        let has_floating = full_schedule
            .flows
            .iter()
            .any(|cf| matches!(cf.kind, CFKind::FloatReset));
        assert!(
            has_floating,
            "Full schedule should include CFKind::FloatReset for FRN"
        );

        // Get simplified schedule via build_dated_flows
        let flows = frn
            .build_dated_flows(&market, issue)
            .expect("Schedule building should succeed in test");
        assert!(!flows.is_empty(), "FRN should have cashflows");

        // Verify flows include floating coupons (should be > just redemption)
        assert!(
            flows.len() > 1,
            "FRN should have coupon flows + redemption, got {} flows",
            flows.len()
        );
    }

    #[test]
    fn test_bond_amortization_holder_view_and_notional_exclusion() {
        use crate::cashflow::builder::AmortizationSpec;
        use crate::cashflow::primitives::CFKind;
        use crate::cashflow::traits::CashflowProvider;

        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2027, Month::January, 1).expect("Valid test date");

        // Create amortizing bond using CashflowSpec::Amortizing
        let step1 = Date::from_calendar_date(2026, Month::January, 1).expect("Valid test date");
        let amort_spec = AmortizationSpec::StepRemaining {
            schedule: vec![
                (step1, Money::new(500_000.0, Currency::USD)),
                (maturity, Money::new(0.0, Currency::USD)),
            ],
        };
        let base_spec = CashflowSpec::fixed(0.05, Tenor::semi_annual(), DayCount::Thirty360);
        let cashflow_spec = CashflowSpec::amortizing(base_spec, amort_spec);

        let bond = Bond::builder()
            .id("AMORT-TEST".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(cashflow_spec)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("CashFlowSchedule builder should succeed with valid test data");

        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (2.0, 0.95)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("CashFlowSchedule builder should succeed with valid test data");
        let market = MarketContext::new().insert_discount(disc_curve);

        // Get full schedule to check internal representation
        let full_schedule = bond
            .get_full_schedule(&market)
            .expect("Full schedule retrieval should succeed in test");

        // Find initial notional (should be negative - issuer receives)
        let initial_notional = full_schedule
            .flows
            .iter()
            .find(|cf| cf.date == issue && matches!(cf.kind, CFKind::Notional));
        assert!(
            initial_notional.is_some(),
            "Full schedule should have initial notional"
        );
        assert!(
            initial_notional
                .expect("Initial notional should exist")
                .amount
                .amount()
                < 0.0,
            "Initial notional should be negative (issuer receives)"
        );

        // Find amortization flows (should be positive in full schedule)
        let amort_flows: Vec<_> = full_schedule
            .flows
            .iter()
            .filter(|cf| matches!(cf.kind, CFKind::Amortization))
            .collect();
        assert!(!amort_flows.is_empty(), "Should have amortization flows");
        for cf in &amort_flows {
            assert!(
                cf.amount.amount() > 0.0,
                "Amortization in full schedule should be positive"
            );
        }

        // Get simplified schedule via build_dated_flows
        let flows = bond
            .build_dated_flows(&market, issue)
            .expect("Schedule building should succeed in test");

        // Initial draw should be excluded (negative notional)
        let has_negative_initial = flows.iter().any(|(d, m)| *d == issue && m.amount() < 0.0);
        assert!(
            !has_negative_initial,
            "Simplified schedule should exclude initial negative notional draw"
        );

        // Amortization should appear as positive holder receipts (principal repayments)
        let amort_in_simplified: Vec<_> = flows
            .iter()
            .filter(|(d, _)| *d == step1 || *d == maturity)
            .collect();
        // We expect at least one amortization payment
        let has_positive_amort = amort_in_simplified.iter().any(|(_, m)| m.amount() > 0.0);
        assert!(
            has_positive_amort,
            "Amortization in simplified schedule should be positive (holder-view principal receipt)"
        );

        // Final redemption at maturity: depending on amortization schedule the
        // maturity date can include coupon, amortization, and/or redemption
        // flows, all of which should be positive from the holder's perspective.
    }

    #[test]
    fn test_amortizing_bond_pv_greater_than_bullet_for_same_yield() {
        use crate::instruments::common::traits::Instrument;

        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2028, Month::January, 1).expect("Valid test date");

        let notional = Money::new(1_000_000.0, Currency::USD);

        // Common discount curve: flat-ish, just needs to be decreasing
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (1.0, 0.97), (2.0, 0.94), (3.0, 0.91)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed in test");
        let market = MarketContext::new().insert_discount(disc_curve);

        // Bullet bond: 3-year annual, 1% coupon, full principal at maturity
        let bullet_cashflow_spec = CashflowSpec::fixed(0.01, Tenor::annual(), DayCount::Act365F);
        let bullet_bond = Bond::builder()
            .id("BULLET-TEST".into())
            .notional(notional)
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(bullet_cashflow_spec)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Bullet bond construction should succeed in test");

        // Amortizing bond with same coupon but 1/3 principal returned each year.
        // StepRemaining schedule specifies remaining balance AFTER each date.
        // After step1: 2/3 remaining (paid 1/3), after step2: 1/3 remaining (paid 2/3).
        let amort_step1 =
            Date::from_calendar_date(2026, Month::January, 1).expect("Valid test date");
        let amort_step2 =
            Date::from_calendar_date(2027, Month::January, 1).expect("Valid test date");
        let amort_schedule = AmortizationSpec::StepRemaining {
            schedule: vec![
                (
                    amort_step1,
                    Money::new(2.0 * 1_000_000.0 / 3.0, Currency::USD), // 2/3 remaining
                ),
                (amort_step2, Money::new(1_000_000.0 / 3.0, Currency::USD)), // 1/3 remaining
                (maturity, Money::new(0.0, Currency::USD)),                  // 0 remaining
            ],
        };
        let amort_base_spec = CashflowSpec::fixed(0.01, Tenor::annual(), DayCount::Act365F);
        let amort_spec = CashflowSpec::amortizing(amort_base_spec, amort_schedule);
        let amort_bond = Bond::builder()
            .id("AMORT-TEST-PV".into())
            .notional(notional)
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(amort_spec)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Amortizing bond construction should succeed in test");

        let pv_bullet = bullet_bond
            .value(&market, issue)
            .expect("Bullet bond valuation should succeed in test")
            .amount();
        let pv_amort = amort_bond
            .value(&market, issue)
            .expect("Amortizing bond valuation should succeed in test")
            .amount();

        // With earlier principal repayments and a coupon below the curve's
        // effective yield, the amortizing bond should have a higher PV than
        // the bullet (principal is returned sooner and reinvested at higher
        // rates).
        assert!(
            pv_amort > pv_bullet,
            "Amortizing bond PV ({}) should be greater than bullet PV ({}) for the same yield curve",
            pv_amort,
            pv_bullet
        );
    }

    #[test]
    fn test_bond_build_dated_flows_includes_floating_cfkind() {
        use crate::cashflow::traits::CashflowProvider;

        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2026, Month::July, 1).expect("Valid test date");

        let frn = Bond::floating(
            "FRN-CFKIND-TEST",
            Money::new(1_000_000.0, Currency::USD),
            "USD-LIBOR-3M",
            200,
            issue,
            maturity,
            Tenor::quarterly(),
            DayCount::Act365F,
            "USD-OIS",
        )
        .unwrap();

        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (2.0, 0.90)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("CashFlowSchedule builder should succeed with valid test data");

        let fwd = ForwardCurve::builder("USD-LIBOR-3M", 0.25)
            .base_date(issue)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.04), (2.0, 0.045)])
            .build()
            .expect("CashFlowSchedule builder should succeed with valid test data");

        let market = MarketContext::new()
            .insert_discount(disc)
            .insert_forward(fwd);

        // Build simplified schedule
        let flows = frn
            .build_dated_flows(&market, issue)
            .expect("Schedule building should succeed in test");

        // Should have multiple flows (quarterly coupons + redemption)
        // Approximately 6 quarters over 18 months
        assert!(
            flows.len() >= 5,
            "FRN should have multiple quarterly flows, got {}",
            flows.len()
        );

        // All flows should have positive amounts (coupons and redemption are receipts)
        let all_positive = flows.iter().all(|(_, m)| m.amount() > 0.0);
        assert!(
            all_positive,
            "All simplified FRN flows should be positive (holder view)"
        );

        // Verify flows are sorted by date
        for i in 1..flows.len() {
            assert!(
                flows[i].0 >= flows[i - 1].0,
                "Flows should be sorted by date"
            );
        }
    }
}
