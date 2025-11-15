//! Interest Rate Future types and implementation.
use crate::cashflow::traits::CashflowProvider;
use crate::constants::PERCENT_TO_DECIMAL;
// Params-based constructor removed; build via builder instead.
use crate::instruments::common::traits::Attributes;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Interest Rate Future instrument.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct InterestRateFuture {
    /// Unique identifier
    pub id: InstrumentId,
    /// Exposure size expressed in currency units. PV is scaled by
    /// `notional.amount() / contract_specs.face_value` to support
    /// multiples of the standard contract.
    pub notional: Money,
    /// Future expiry/delivery date
    pub expiry_date: Date,
    /// Underlying rate fixing date
    pub fixing_date: Date,
    /// Rate period start date
    pub period_start: Date,
    /// Rate period end date
    pub period_end: Date,
    /// Quoted future price (e.g., 99.25)
    pub quoted_price: f64,
    /// Day count convention
    pub day_count: DayCount,
    /// Position side (Long or Short)
    pub position: Position,
    /// Contract specifications
    pub contract_specs: FutureContractSpecs,
    /// Discount curve identifier
    pub discount_curve_id: CurveId,
    /// Forward curve identifier
    pub forward_id: CurveId,
    /// Attributes
    pub attributes: Attributes,
}

/// Contract specifications for interest rate futures.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FutureContractSpecs {
    /// Face value of contract
    pub face_value: f64,
    /// Tick size
    pub tick_size: f64,
    /// Tick value in currency units
    pub tick_value: f64,
    /// Number of delivery months
    pub delivery_months: u8,
    /// Convexity adjustment (for long-dated contracts)
    pub convexity_adjustment: Option<f64>,
}

impl Default for FutureContractSpecs {
    fn default() -> Self {
        Self {
            face_value: 1_000_000.0,
            tick_size: 0.0025, // 0.25 bp (in price points)
            // Default tick value for a 3M contract on a $1MM face: $6.25 per tick
            // (Face × 0.25y × 1bp × 0.25bp-per-tick / 1bp = $6.25). For
            // other accrual lengths, prefer `InterestRateFuture::derived_tick_value`.
            tick_value: 6.25,
            delivery_months: 3,
            convexity_adjustment: None,
        }
    }
}

/// Position side for futures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Position {
    /// Long position (buyer of futures contract)
    Long,
    /// Short position (seller of futures contract)
    Short,
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Position::Long => write!(f, "long"),
            Position::Short => write!(f, "short"),
        }
    }
}

impl std::str::FromStr for Position {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "long" => Ok(Position::Long),
            "short" => Ok(Position::Short),
            other => Err(format!("Unknown position: {}", other)),
        }
    }
}

impl InterestRateFuture {
    // Note: use the builder (FinancialBuilder) for construction.

    /// Create a canonical example 3M Eurodollar-style interest rate future.
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use time::Month;
        InterestRateFutureBuilder::new()
            .id(InstrumentId::new("IRF-ED-3M-MAR25"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .expiry_date(Date::from_calendar_date(2025, Month::March, 17).expect("Valid example date"))
            .fixing_date(Date::from_calendar_date(2025, Month::March, 17).expect("Valid example date"))
            .period_start(Date::from_calendar_date(2025, Month::March, 19).expect("Valid example date"))
            .period_end(Date::from_calendar_date(2025, Month::June, 18).expect("Valid example date"))
            .quoted_price(95.50)
            .day_count(finstack_core::dates::DayCount::Act360)
            .position(Position::Long)
            .contract_specs(FutureContractSpecs::default())
            .discount_curve_id(CurveId::new("USD-OIS"))
            .forward_id(CurveId::new("USD-SOFR-3M"))
            .attributes(Attributes::new())
            .build()
            .expect("Example InterestRateFuture construction should not fail")
    }

    /// Set contract specifications.
    pub fn with_contract_specs(mut self, specs: FutureContractSpecs) -> Self {
        self.contract_specs = specs;
        self
    }

    /// Get implied rate from quoted price.
    /// Interest rate futures quote as 100 minus the rate.
    pub fn implied_rate(&self) -> f64 {
        (100.0 - self.quoted_price) * PERCENT_TO_DECIMAL
    }

    /// Calculates the present value of the interest rate future.
    ///
    /// PV = (R_implied - R_model_adj) × FaceValue × tau(period_start, period_end) × contracts × position_sign
    ///
    /// Uses discount/forward curves from the MarketContext and applies convexity adjustments.
    pub fn npv(&self, context: &MarketContext) -> finstack_core::Result<Money> {
        use finstack_core::dates::DayCountCtx;

        let disc = context.get_discount_ref(&self.discount_curve_id)?;
        let fwd = context.get_forward_ref(&self.forward_id)?;

        // Base date for mapping to curve time
        let _base_date = disc.base_date();

        // Time to fixing and rate period for forward rate calculation should use
        // the forward curve's day-count basis to avoid basis mismatches.
        let fwd_dc = fwd.day_count();
        let fwd_base = fwd.base_date();
        let t_fixing = fwd_dc
            .year_fraction(fwd_base, self.fixing_date, DayCountCtx::default())?
            .max(0.0);
        let t_start = fwd_dc
            .year_fraction(fwd_base, self.period_start, DayCountCtx::default())?
            .max(0.0);
        let t_end = fwd_dc
            .year_fraction(fwd_base, self.period_end, DayCountCtx::default())?
            .max(t_start);

        // Forward rate over the period
        let forward_rate = fwd.rate_period(t_start, t_end);

        // Apply convexity adjustment policy
        let adjusted_rate = if let Some(ca) = self.contract_specs.convexity_adjustment {
            forward_rate + ca
        } else {
            // Estimate convexity using a Hull-White style approximation
            let vol_estimate = if t_fixing <= 0.25 {
                0.008
            } else if t_fixing <= 0.5 {
                0.0085
            } else if t_fixing <= 1.0 {
                0.009
            } else if t_fixing <= 2.0 {
                0.0095
            } else {
                0.01
            };
            let tau_len = t_end - t_start;
            let convexity = 0.5 * vol_estimate * vol_estimate * t_fixing * (t_fixing + tau_len);
            forward_rate + convexity
        };

        // Implied rate from price and accrual over the underlying period
        let implied_rate = self.implied_rate();
        let tau = self
            .day_count
            .year_fraction(self.period_start, self.period_end, DayCountCtx::default())?
            .max(0.0);
        if tau == 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        // Position sign: Long benefits when implied > model (rates down → price up)
        let sign = match self.position {
            Position::Long => 1.0,
            Position::Short => -1.0,
        };

        // Scale by contracts: notional may represent multiples of face value
        let contracts_scale = if self.contract_specs.face_value != 0.0 {
            self.notional.amount() / self.contract_specs.face_value
        } else {
            1.0
        };

        let pv_per_contract = (implied_rate - adjusted_rate) * self.contract_specs.face_value * tau;
        let pv_total = sign * contracts_scale * pv_per_contract;
        Ok(Money::new(pv_total, self.notional.currency()))
    }

    /// Derive contract tick value for the instrument accrual.
    ///
    /// tick_value ≈ Face × tau(period_start, period_end) × 1bp × (tick_size / 1bp)
    pub fn derived_tick_value(&self) -> finstack_core::Result<f64> {
        let tau = self
            .day_count
            .year_fraction(
                self.period_start,
                self.period_end,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        Ok(self.contract_specs.face_value * tau * 1e-4 * (self.contract_specs.tick_size / 1e-4))
    }
}

impl crate::instruments::common::traits::Instrument for InterestRateFuture {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::InterestRateFuture
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
        _as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.npv(curves)
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

impl CashflowProvider for InterestRateFuture {
    fn build_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Vec<(Date, Money)>> {
        // Futures settle daily (mark-to-market), but for simplicity
        // we'll return the final settlement at expiry
        if self.expiry_date <= as_of {
            return Ok(vec![]); // Already expired
        }

        let settlement_pv = self.npv(curves)?;

        Ok(vec![(self.expiry_date, settlement_pv)])
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for InterestRateFuture {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for InterestRateFuture {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.forward_id.clone())
            .build()
    }
}

impl crate::instruments::common::pricing::HasForwardCurves for InterestRateFuture {
    fn forward_curve_ids(&self) -> Vec<finstack_core::types::CurveId> {
        vec![self.forward_id.clone()]
    }
}
