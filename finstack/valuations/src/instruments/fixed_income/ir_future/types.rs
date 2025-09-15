//! Interest Rate Future types and implementation.
use finstack_core::market_data::traits::Forward;
use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common::{DateRange, IRFutureParams, MarketRefs};
use crate::instruments::traits::Attributes;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::traits::{Discounting};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::F;

/// Interest Rate Future instrument.
#[derive(Clone, Debug)]
pub struct InterestRateFuture {
    /// Unique identifier
    pub id: String,
    /// Contract notional amount
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
    pub quoted_price: F,
    /// Day count convention
    pub day_count: DayCount,
    /// Contract specifications
    pub contract_specs: FutureContractSpecs,
    /// Discount curve identifier
    pub disc_id: &'static str,
    /// Forward curve identifier
    pub forward_id: &'static str,
    /// Attributes
    pub attributes: Attributes,
}

/// Contract specifications for interest rate futures.
#[derive(Clone, Debug)]
pub struct FutureContractSpecs {
    /// Face value of contract
    pub face_value: F,
    /// Tick size
    pub tick_size: F,
    /// Tick value in currency units
    pub tick_value: F,
    /// Number of delivery months
    pub delivery_months: u8,
    /// Convexity adjustment (for long-dated contracts)
    pub convexity_adjustment: Option<F>,
}

impl Default for FutureContractSpecs {
    fn default() -> Self {
        Self {
            face_value: 1_000_000.0,
            tick_size: 0.0025, // 0.25 bp
            tick_value: 25.0,  // $25 per tick for $1MM
            delivery_months: 3,
            convexity_adjustment: None,
        }
    }
}

impl InterestRateFuture {
    /// Create a new interest rate future using parameter structs.
    pub fn new(
        id: impl Into<String>,
        future_params: &IRFutureParams,
        period_range: &DateRange,
        market_refs: &MarketRefs,
    ) -> Self {
        let forward_id = market_refs
            .fwd_id
            .as_ref()
            .expect("Forward curve required for IR futures");

        Self {
            id: id.into(),
            notional: future_params.notional,
            expiry_date: future_params.expiry_date,
            fixing_date: future_params.fixing_date,
            period_start: period_range.start,
            period_end: period_range.end,
            quoted_price: future_params.quoted_price,
            day_count: future_params.day_count,
            contract_specs: FutureContractSpecs::default(),
            disc_id: Box::leak(market_refs.disc_id.to_string().into_boxed_str()),
            forward_id: Box::leak(forward_id.to_string().into_boxed_str()),
            attributes: Attributes::new(),
        }
    }

    /// Set contract specifications.
    pub fn with_contract_specs(mut self, specs: FutureContractSpecs) -> Self {
        self.contract_specs = specs;
        self
    }

    /// Get implied rate from quoted price.
    pub fn implied_rate(&self) -> F {
        (100.0 - self.quoted_price) / 100.0
    }

    /// Calculate future value with convexity adjustment.
    pub fn future_value(
        &self,
        discount_curve: &dyn Discounting,
        forward_curve: &dyn Forward,
        _as_of: Date,
    ) -> finstack_core::Result<Money> {
        let base_date = discount_curve.base_date();

        // Time to fixing and rate period
        let t_fixing = self
            .day_count
            .year_fraction(
                base_date,
                self.fixing_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let t_start = self
            .day_count
            .year_fraction(
                base_date,
                self.period_start,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let t_end = self
            .day_count
            .year_fraction(
                base_date,
                self.period_end,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        // Get forward rate for the underlying period
        let forward_rate = forward_curve.rate_period(t_start, t_end);

        // Apply convexity adjustment - market practice applies to all futures
        let adjusted_rate = if let Some(convexity_adj) = self.contract_specs.convexity_adjustment {
            forward_rate + convexity_adj
        } else {
            // Calculate convexity adjustment for all futures
            // Use more sophisticated volatility estimate based on time to expiry
            let vol_estimate = if t_fixing <= 0.25 {
                0.008 // 80bp for very short-dated futures
            } else if t_fixing <= 0.5 {
                0.0085 // 85bp for 3-6 month futures
            } else if t_fixing <= 1.0 {
                0.009 // 90bp for 6-12 month futures
            } else if t_fixing <= 2.0 {
                0.0095 // 95bp for 1-2 year futures
            } else {
                0.01 // 100bp for longer-dated futures
            };

            // Hull-White approximation: CA = 0.5 * σ² * T₁ * T₂
            // where T₁ is time to expiry and T₂ is typically close to T₁ for futures
            let tau = t_end - t_start; // Length of underlying rate period
            let convexity = 0.5 * vol_estimate * vol_estimate * t_fixing * (t_fixing + tau);
            forward_rate + convexity
        };

        // Future value = (Model Rate - Implied Rate) × Face Value × Period Length
        let implied_rate = self.implied_rate();
        let tau = self
            .day_count
            .year_fraction(
                self.period_start,
                self.period_end,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let rate_diff = adjusted_rate - implied_rate;

        let pv = rate_diff * self.contract_specs.face_value * tau;

        Ok(Money::new(pv, self.notional.currency()))
    }
}

impl_instrument!(
    InterestRateFuture,
    "InterestRateFuture",
    pv = |s, curves, as_of| {
        let discount_curve = curves.discount_ref(s.disc_id)?;
        let forward_curve = curves.forward_ref(s.forward_id)?;
        s.future_value(discount_curve, forward_curve, as_of)
    }
);

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

        let settlement_pv = self.future_value(
            curves.discount_ref(self.disc_id)?,
            curves.forward_ref(self.forward_id)?,
            as_of,
        )?;

        Ok(vec![(self.expiry_date, settlement_pv)])
    }
}
