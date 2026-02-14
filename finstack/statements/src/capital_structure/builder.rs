//! Builder API Extensions for Capital Structure
//!
//! This module provides fluent builder methods for adding capital structure
//! instruments to a financial model.

use crate::builder::ModelBuilder;
use crate::error::Result;
use crate::types::{CapitalStructureSpec, DebtInstrumentSpec};
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::irs::FloatingLegCompounding;
use finstack_valuations::instruments::{Bond, FixedLegSpec, FloatLegSpec, InterestRateSwap};
use rust_decimal::Decimal;

/// Helper to ensure capital structure exists and return mutable reference.
///
/// Returns a mutable reference to the capital structure spec, creating an empty
/// instance if one is not already present.
fn ensure_capital_structure<State>(builder: &mut ModelBuilder<State>) -> &mut CapitalStructureSpec {
    builder
        .capital_structure
        .get_or_insert_with(|| CapitalStructureSpec {
            debt_instruments: vec![],
            equity_instruments: vec![],
            meta: indexmap::IndexMap::new(),
            reporting_currency: None,
            fx_policy: None,
            waterfall: None,
        })
}

impl<State> ModelBuilder<State> {
    /// Add a bond instrument to the capital structure specification.
    ///
    /// Uses `Bond::fixed()` with default conventions (semi-annual, 30/360).
    /// For non-standard conventions (e.g., EUR bonds with ACT/ACT), use
    /// [`add_custom_debt`](Self::add_custom_debt) with a pre-built `Bond`.
    ///
    /// # Arguments
    /// * `id` - Unique instrument identifier
    /// * `notional` - Principal amount
    /// * `coupon_rate` - Annual coupon rate (e.g., 0.05 for 5%)
    /// * `issue_date` - Bond issue date
    /// * `maturity_date` - Bond maturity date
    /// * `discount_curve_id` - Discount curve ID for pricing
    ///
    /// # Returns
    /// Updated builder with the bond appended to the capital-structure spec.
    ///
    /// # Example
    /// ```rust,no_run
    /// use finstack_statements::builder::ModelBuilder;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::money::Money;
    /// use time::macros::date;
    ///
    /// # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    /// let issue_date = date!(2025-01-15);
    /// let maturity_date = date!(2030-01-15);
    ///
    /// let builder = ModelBuilder::new("cs-model")
    ///     .add_bond(
    ///         "BOND-001",
    ///         Money::new(10_000_000.0, Currency::USD),
    ///         0.05, // 5% coupon
    ///         issue_date,
    ///         maturity_date,
    ///         "USD-OIS",
    ///     )?;
    /// # let _ = builder;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_bond(
        mut self,
        id: impl Into<String>,
        notional: Money,
        coupon_rate: f64,
        issue_date: Date,
        maturity_date: Date,
        discount_curve_id: impl Into<String>,
    ) -> Result<Self> {
        let id_str: String = id.into();

        // Create bond using valuations crate
        let bond = Bond::fixed(
            InstrumentId::new(&id_str),
            notional,
            coupon_rate,
            issue_date,
            maturity_date,
            CurveId::new(discount_curve_id),
        )
        .map_err(|e| {
            crate::error::Error::build(format!("Failed to create bond '{}': {}", id_str, e))
        })?;

        // Serialize to JSON
        let spec_json = serde_json::to_value(&bond).map_err(|e| {
            crate::error::Error::build(format!("Failed to serialize bond '{}': {}", id_str, e))
        })?;

        // Add to capital structure
        ensure_capital_structure(&mut self)
            .debt_instruments
            .push(DebtInstrumentSpec::Bond {
                id: id_str,
                spec: spec_json,
            });

        Ok(self)
    }

    /// Add a bond instrument with a market convention preset.
    ///
    /// This overload applies regional day count, coupon frequency, and calendar
    /// conventions automatically. The default `add_bond` uses US corporate bond
    /// conventions (30/360, semi-annual).
    ///
    /// # Arguments
    /// * `id` - Unique instrument identifier
    /// * `notional` - Principal amount
    /// * `coupon_rate` - Annual coupon rate as typed `Rate`
    /// * `issue_date` - Bond issue date
    /// * `maturity_date` - Bond maturity date
    /// * `convention` - Regional convention preset (e.g., `BondConvention::EurGovernment`)
    /// * `discount_curve_id` - Discount curve ID for pricing
    ///
    /// # Example
    /// ```rust,no_run
    /// use finstack_statements::builder::ModelBuilder;
    /// use finstack_valuations::instruments::BondConvention;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::money::Money;
    /// use finstack_core::types::Rate;
    /// use time::macros::date;
    ///
    /// # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    /// let builder = ModelBuilder::new("cs-model")
    ///     .add_bond_with_convention(
    ///         "BUND-001",
    ///         Money::new(10_000_000.0, Currency::EUR),
    ///         Rate::from_decimal(0.03),
    ///         date!(2025-01-15),
    ///         date!(2030-01-15),
    ///         BondConvention::GermanBund,
    ///         "EUR-OIS",
    ///     )?;
    /// # let _ = builder;
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn add_bond_with_convention(
        mut self,
        id: impl Into<String>,
        notional: Money,
        coupon_rate: finstack_core::types::Rate,
        issue_date: Date,
        maturity_date: Date,
        convention: finstack_valuations::instruments::BondConvention,
        discount_curve_id: impl Into<String>,
    ) -> Result<Self> {
        let id_str: String = id.into();

        let bond = Bond::with_convention(
            InstrumentId::new(&id_str),
            notional,
            coupon_rate,
            issue_date,
            maturity_date,
            convention,
            CurveId::new(discount_curve_id),
        )
        .map_err(|e| {
            crate::error::Error::build(format!("Failed to create bond '{}': {}", id_str, e))
        })?;

        let spec_json = serde_json::to_value(&bond).map_err(|e| {
            crate::error::Error::build(format!("Failed to serialize bond '{}': {}", id_str, e))
        })?;

        ensure_capital_structure(&mut self)
            .debt_instruments
            .push(DebtInstrumentSpec::Bond {
                id: id_str,
                spec: spec_json,
            });

        Ok(self)
    }

    /// Add an interest rate swap to the capital structure.
    ///
    /// # Arguments
    /// * `id` - Unique instrument identifier
    /// * `notional` - Notional amount
    /// * `fixed_rate` - Fixed rate (e.g., 0.04 for 4%)
    /// * `start_date` - Swap start date
    /// * `maturity_date` - Swap maturity date
    /// * `discount_curve_id` - Discount curve ID
    /// * `forward_curve_id` - Forward curve ID for floating leg
    ///
    /// # Example
    /// ```rust,no_run
    /// use finstack_statements::builder::ModelBuilder;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::money::Money;
    /// use time::macros::date;
    ///
    /// # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    /// let start_date = date!(2025-01-15);
    /// let maturity_date = date!(2030-01-15);
    ///
    /// let builder = ModelBuilder::new("cs-model")
    ///     .add_swap(
    ///         "SWAP-001",
    ///         Money::new(5_000_000.0, Currency::USD),
    ///         0.04, // 4% fixed rate
    ///         start_date,
    ///         maturity_date,
    ///         "USD-OIS",
    ///         "USD-SOFR-3M",
    ///     )?;
    /// # let _ = builder;
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn add_swap(
        mut self,
        id: impl Into<String>,
        notional: Money,
        fixed_rate: f64,
        start_date: Date,
        maturity_date: Date,
        discount_curve_id: impl Into<String>,
        forward_curve_id: impl Into<String>,
    ) -> Result<Self> {
        let id_str: String = id.into();

        use finstack_valuations::instruments::PayReceive;

        let rate_decimal = Decimal::try_from(fixed_rate).map_err(|_| {
            crate::error::Error::InvalidInput(format!(
                "Invalid fixed rate: {} cannot be converted to Decimal.",
                fixed_rate
            ))
        })?;

        let discount_curve_id = CurveId::new(discount_curve_id.into());
        let forward_curve_id = CurveId::new(forward_curve_id.into());

        let fixed = FixedLegSpec {
            discount_curve_id: discount_curve_id.clone(),
            rate: rate_decimal,
            frequency: Tenor::semi_annual(),
            dc: DayCount::Thirty360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("usny".to_string()),
            stub: StubKind::None,
            start: start_date,
            end: maturity_date,
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0,
            end_of_month: false,
        };

        let float = FloatLegSpec {
            discount_curve_id,
            forward_curve_id,
            spread_bp: Decimal::ZERO,
            frequency: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("usny".to_string()),
            stub: StubKind::None,
            reset_lag_days: 0,
            fixing_calendar_id: None,
            start: start_date,
            end: maturity_date,
            compounding: FloatingLegCompounding::Simple,
            payment_delay_days: 0,
            end_of_month: false,
        };

        let swap = InterestRateSwap::builder()
            .id(InstrumentId::new(&id_str))
            .notional(notional)
            .side(PayReceive::PayFixed) // Default to pay-fixed
            .fixed(fixed)
            .float(float)
            .build()?;

        // Serialize to JSON
        let spec_json = serde_json::to_value(&swap)
            .map_err(|e| crate::error::Error::build(format!("Failed to serialize swap: {}", e)))?;

        // Add to capital structure
        ensure_capital_structure(&mut self)
            .debt_instruments
            .push(DebtInstrumentSpec::Swap {
                id: id_str,
                spec: spec_json,
            });

        Ok(self)
    }

    /// Add an interest rate swap with custom conventions.
    ///
    /// This overload exposes day count, frequency, and business day convention
    /// parameters for non-USD swaps (e.g., EUR swaps with ACT/360 annual fixed,
    /// GBP swaps with ACT/365F semi-annual fixed).
    ///
    /// The default `add_swap` uses US conventions:
    /// - Fixed: Semi-annual, 30/360, Modified Following
    /// - Float: Quarterly, ACT/360, Modified Following
    #[allow(clippy::too_many_arguments)]
    pub fn add_swap_with_conventions(
        mut self,
        id: impl Into<String>,
        notional: Money,
        fixed_rate: f64,
        start_date: Date,
        maturity_date: Date,
        discount_curve_id: impl Into<String>,
        forward_curve_id: impl Into<String>,
        fixed_freq: Tenor,
        fixed_dc: DayCount,
        float_freq: Tenor,
        float_dc: DayCount,
        bdc: BusinessDayConvention,
    ) -> Result<Self> {
        let id_str: String = id.into();

        use finstack_valuations::instruments::PayReceive;

        let rate_decimal = Decimal::try_from(fixed_rate).map_err(|_| {
            crate::error::Error::InvalidInput(format!(
                "Invalid fixed rate: {} cannot be converted to Decimal.",
                fixed_rate
            ))
        })?;

        let discount_curve_id = CurveId::new(discount_curve_id.into());
        let forward_curve_id = CurveId::new(forward_curve_id.into());

        let fixed = FixedLegSpec {
            discount_curve_id: discount_curve_id.clone(),
            rate: rate_decimal,
            frequency: fixed_freq,
            dc: fixed_dc,
            bdc,
            calendar_id: Some("usny".to_string()),
            stub: StubKind::None,
            start: start_date,
            end: maturity_date,
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0,
            end_of_month: false,
        };

        let float = FloatLegSpec {
            discount_curve_id,
            forward_curve_id,
            spread_bp: Decimal::ZERO,
            frequency: float_freq,
            dc: float_dc,
            bdc,
            calendar_id: Some("usny".to_string()),
            stub: StubKind::None,
            reset_lag_days: 0,
            fixing_calendar_id: None,
            start: start_date,
            end: maturity_date,
            compounding: FloatingLegCompounding::Simple,
            payment_delay_days: 0,
            end_of_month: false,
        };

        let swap = InterestRateSwap::builder()
            .id(InstrumentId::new(&id_str))
            .notional(notional)
            .side(PayReceive::PayFixed)
            .fixed(fixed)
            .float(float)
            .build()?;

        let spec_json = serde_json::to_value(&swap)
            .map_err(|e| crate::error::Error::build(format!("Failed to serialize swap: {}", e)))?;

        ensure_capital_structure(&mut self)
            .debt_instruments
            .push(DebtInstrumentSpec::Swap {
                id: id_str,
                spec: spec_json,
            });

        Ok(self)
    }

    /// Add a generic debt instrument via JSON specification.
    ///
    /// This allows adding custom debt instruments not covered by the convenience
    /// methods (bonds, swaps).
    ///
    /// # Example
    /// ```rust,no_run
    /// use finstack_statements::builder::ModelBuilder;
    /// use serde_json::json;
    ///
    /// let builder = ModelBuilder::new("cs-model").add_custom_debt(
    ///     "TL-A",
    ///     json!({
    ///         "type": "amortizing_loan",
    ///         "notional": 25_000_000.0,
    ///         "currency": "USD",
    ///         "issue_date": "2025-01-15",
    ///         "maturity_date": "2030-01-15",
    ///         "coupon_rate": 0.06,
    ///         "frequency": "quarterly",
    ///         "amortization": { "type": "linear", "final_notional": 0.0 }
    ///     }),
    /// );
    /// # let _ = builder;
    /// ```
    pub fn add_custom_debt(mut self, id: impl Into<String>, spec: serde_json::Value) -> Self {
        // Add to capital structure
        ensure_capital_structure(&mut self)
            .debt_instruments
            .push(DebtInstrumentSpec::Generic {
                id: id.into(),
                spec,
            });

        self
    }

    /// Set an explicit reporting currency for capital-structure totals.
    pub fn reporting_currency(mut self, currency: finstack_core::currency::Currency) -> Self {
        ensure_capital_structure(&mut self).reporting_currency = Some(currency);
        self
    }

    /// Set the FX conversion policy used when converting capital-structure cashflows.
    pub fn fx_policy(mut self, policy: finstack_core::money::fx::FxConversionPolicy) -> Self {
        ensure_capital_structure(&mut self).fx_policy = Some(policy);
        self
    }

    /// Configure waterfall specification for dynamic cash flow allocation.
    ///
    /// # Arguments
    /// * `waterfall_spec` - Waterfall configuration with ECF sweep and PIK toggle settings
    ///
    /// # Example
    /// ```rust,no_run
    /// use finstack_statements::capital_structure::{WaterfallSpec, EcfSweepSpec};
    /// use finstack_statements::builder::ModelBuilder;
    ///
    /// let waterfall = WaterfallSpec {
    ///     ecf_sweep: Some(EcfSweepSpec {
    ///         ebitda_node: "ebitda".to_string(),
    ///         taxes_node: Some("taxes".to_string()),
    ///         capex_node: Some("capex".to_string()),
    ///         working_capital_node: Some("wc_change".to_string()),
    ///         cash_interest_node: None,
    ///         sweep_percentage: 0.5,  // 50% sweep
    ///         target_instrument_id: Some("TL-A".to_string()),
    ///     }),
    ///     ..WaterfallSpec::default()
    /// };
    ///
    /// let builder = ModelBuilder::new("cs-model").waterfall(waterfall);
    /// # let _ = builder;
    /// ```
    pub fn waterfall(mut self, waterfall_spec: crate::capital_structure::WaterfallSpec) -> Self {
        ensure_capital_structure(&mut self).waterfall = Some(waterfall_spec);
        self
    }

    // Add a revolving credit facility to the capital structure specification.
    // Commented out until revolving_credit module is implemented
    /*
    #[allow(clippy::too_many_arguments)]
    pub fn add_revolving_credit(
        mut self,
        id: impl Into<String>,
        credit_limit: Money,
        initial_drawn: Money,
        start_date: Date,
        maturity_date: Date,
        interest_spec: finstack_valuations::instruments::fixed_income::revolving_credit::InterestRateSpec,
        fees: finstack_valuations::instruments::fixed_income::revolving_credit::RcfFeeSpec,
        discount_curve_id: impl Into<String>,
    ) -> Result<Self> {
        let id_str = id.into();
        let facility = finstack_valuations::instruments::RevolvingCreditFacility::new(
            InstrumentId::new(&id_str),
            credit_limit,
            initial_drawn,
            start_date,
            maturity_date,
            interest_spec,
            fees,
            CurveId::new(discount_curve_id),
        );

        let spec_json = serde_json::to_value(&facility).map_err(|e| {
            crate::error::Error::build(format!(
                "Failed to serialize revolving credit facility '{}': {}",
                id_str, e
            ))
        })?;

        ensure_capital_structure(&mut self)
            .debt_instruments
            .push(DebtInstrumentSpec::Generic {
                id: id_str,
                spec: spec_json,
            });

        Ok(self)
    }
    */
}
