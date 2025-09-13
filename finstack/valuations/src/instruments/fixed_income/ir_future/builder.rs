use super::types::{FutureContractSpecs, InterestRateFuture};
use crate::instruments::common::MarketRefs;
use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;
use finstack_core::F;

/// Builder for Interest Rate Future instruments.
#[derive(Default)]
pub struct IRFutureBuilder {
    id: Option<String>,
    notional: Option<Money>,
    expiry_date: Option<Date>,
    fixing_date: Option<Date>,
    period_start: Option<Date>,
    period_end: Option<Date>,
    quoted_price: Option<F>,
    day_count: Option<DayCount>,
    contract_specs: Option<FutureContractSpecs>,
    market_refs: Option<MarketRefs>,
}

impl IRFutureBuilder {
    /// Create a new IR future builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the instrument identifier.
    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }

    /// Set the notional amount.
    pub fn notional(mut self, value: Money) -> Self {
        self.notional = Some(value);
        self
    }

    /// Set the expiry/delivery date.
    pub fn expiry_date(mut self, value: Date) -> Self {
        self.expiry_date = Some(value);
        self
    }

    /// Set the fixing date for the underlying rate.
    pub fn fixing_date(mut self, value: Date) -> Self {
        self.fixing_date = Some(value);
        self
    }

    /// Set the rate period start date.
    pub fn period_start(mut self, value: Date) -> Self {
        self.period_start = Some(value);
        self
    }

    /// Set the rate period end date.
    pub fn period_end(mut self, value: Date) -> Self {
        self.period_end = Some(value);
        self
    }

    /// Set the quoted future price (e.g., 99.25 for 0.75% implied rate).
    pub fn quoted_price(mut self, value: F) -> Self {
        self.quoted_price = Some(value);
        self
    }

    /// Set the day count convention.
    pub fn day_count(mut self, value: DayCount) -> Self {
        self.day_count = Some(value);
        self
    }

    /// Set contract specifications.
    pub fn contract_specs(mut self, value: FutureContractSpecs) -> Self {
        self.contract_specs = Some(value);
        self
    }

    /// Set market references (discount and forward curve IDs).
    pub fn market_refs(mut self, refs: MarketRefs) -> Self {
        self.market_refs = Some(refs);
        self
    }

    /// Convenience method to set standard 3-month SOFR future specs.
    pub fn sofr_3m_standard(mut self) -> Self {
        self.contract_specs = Some(FutureContractSpecs {
            face_value: 2_500_000.0, // CME SOFR future notional
            tick_size: 0.0025,       // 0.25 bp
            tick_value: 62.50,       // $62.50 per tick for $2.5MM
            delivery_months: 3,
            convexity_adjustment: None, // Let the model calculate
        });
        self.day_count = Some(DayCount::Act360);
        self
    }

    /// Convenience method to set standard Eurodollar future specs.
    pub fn eurodollar_standard(mut self) -> Self {
        self.contract_specs = Some(FutureContractSpecs {
            face_value: 1_000_000.0, // Standard Eurodollar future
            tick_size: 0.0025,       // 0.25 bp  
            tick_value: 25.0,        // $25 per tick for $1MM
            delivery_months: 3,
            convexity_adjustment: None,
        });
        self.day_count = Some(DayCount::Act360);
        self
    }

    /// Build the interest rate future instrument.
    pub fn build(self) -> finstack_core::Result<InterestRateFuture> {
        let id = self
            .id
            .ok_or(finstack_core::error::InputError::Invalid)?;
        let notional = self
            .notional
            .ok_or(finstack_core::error::InputError::Invalid)?;
        let expiry_date = self
            .expiry_date
            .ok_or(finstack_core::error::InputError::Invalid)?;
        let fixing_date = self
            .fixing_date
            .ok_or(finstack_core::error::InputError::Invalid)?;
        let period_start = self
            .period_start
            .ok_or(finstack_core::error::InputError::Invalid)?;
        let period_end = self
            .period_end
            .ok_or(finstack_core::error::InputError::Invalid)?;
        let quoted_price = self
            .quoted_price
            .ok_or(finstack_core::error::InputError::Invalid)?;
        let day_count = self.day_count.unwrap_or(DayCount::Act360);
        let refs = self
            .market_refs
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Extract curve IDs from market refs
        let disc_id: &'static str = Box::leak(refs.disc_id.into_string().into_boxed_str());
        let forward_id: &'static str = if let Some(fwd_id) = refs.fwd_id {
            Box::leak(fwd_id.into_string().into_boxed_str())
        } else {
            return Err(finstack_core::error::InputError::Invalid.into());
        };

        let mut future = InterestRateFuture::new(
            id,
            notional,
            expiry_date,
            fixing_date,
            period_start,
            period_end,
            quoted_price,
            day_count,
            disc_id,
            forward_id,
        );

        if let Some(specs) = self.contract_specs {
            future = future.with_contract_specs(specs);
        }

        Ok(future)
    }
}

impl InterestRateFuture {
    /// Create a new IR future builder.
    pub fn builder() -> IRFutureBuilder {
        IRFutureBuilder::new()
    }
}
