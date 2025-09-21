//! Interest Rate Future types and implementation.
use crate::cashflow::traits::CashflowProvider;
// Params-based constructor removed; build via builder instead.
use crate::instruments::common::traits::Attributes;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::F;

/// Interest Rate Future instrument.
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
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
    pub quoted_price: F,
    /// Day count convention
    pub day_count: DayCount,
    /// Position side (Long or Short)
    pub position: Position,
    /// Contract specifications
    pub contract_specs: FutureContractSpecs,
    /// Discount curve identifier
    pub disc_id: CurveId,
    /// Forward curve identifier
    pub forward_id: CurveId,
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
pub enum Position {
    Long,
    Short,
}

impl InterestRateFuture {
    // Note: use the builder (FinancialBuilder) for construction.

    /// Set contract specifications.
    pub fn with_contract_specs(mut self, specs: FutureContractSpecs) -> Self {
        self.contract_specs = specs;
        self
    }

    /// Get implied rate from quoted price.
    pub fn implied_rate(&self) -> F {
        (100.0 - self.quoted_price) / 100.0
    }

    // Pricing moved to `pricing::engine::IrFutureEngine`.

    /// Derive contract tick value for the instrument accrual.
    ///
    /// tick_value ≈ Face × tau(period_start, period_end) × 1bp × (tick_size / 1bp)
    pub fn derived_tick_value(&self) -> finstack_core::Result<F> {
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

impl_instrument!(
    InterestRateFuture,
    "InterestRateFuture",
    pv = |s, curves, as_of| {
        let _ = as_of; // PV does not depend on `as_of`; uses curve base dates
        crate::instruments::ir_future::pricing::engine::IrFutureEngine::pv(s, curves)
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

        let settlement_pv =
            crate::instruments::ir_future::pricing::engine::IrFutureEngine::pv(self, curves)?;

        Ok(vec![(self.expiry_date, settlement_pv)])
    }
}
