use crate::instruments::common::MarketRefs;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;

use super::types::ForwardRateAgreement;

/// Builder for Forward Rate Agreement using MarketRefs for market links.
#[derive(Default)]
#[allow(dead_code)]
pub struct FraBuilder {
    id: Option<String>,
    notional: Option<Money>,
    fixing_date: Option<finstack_core::dates::Date>,
    start_date: Option<finstack_core::dates::Date>,
    end_date: Option<finstack_core::dates::Date>,
    fixed_rate: Option<finstack_core::F>,
    day_count: Option<DayCount>,
    market_refs: Option<MarketRefs>,
    pay_fixed: Option<bool>,
    reset_lag: Option<i32>,
}

#[allow(dead_code)]
impl FraBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }

    pub fn notional(mut self, value: Money) -> Self {
        self.notional = Some(value);
        self
    }

    pub fn fixing_date(mut self, value: finstack_core::dates::Date) -> Self {
        self.fixing_date = Some(value);
        self
    }

    pub fn start_date(mut self, value: finstack_core::dates::Date) -> Self {
        self.start_date = Some(value);
        self
    }

    pub fn end_date(mut self, value: finstack_core::dates::Date) -> Self {
        self.end_date = Some(value);
        self
    }

    pub fn fixed_rate(mut self, value: finstack_core::F) -> Self {
        self.fixed_rate = Some(value);
        self
    }

    pub fn day_count(mut self, value: DayCount) -> Self {
        self.day_count = Some(value);
        self
    }

    /// Set discount and forward ids via MarketRefs (requires both disc and fwd)
    pub fn market_refs(mut self, refs: MarketRefs) -> Self {
        self.market_refs = Some(refs);
        self
    }

    pub fn pay_fixed(mut self, value: bool) -> Self {
        self.pay_fixed = Some(value);
        self
    }

    pub fn reset_lag(mut self, value: i32) -> Self {
        self.reset_lag = Some(value);
        self
    }

    pub fn build(self) -> finstack_core::Result<ForwardRateAgreement> {
        let id = self.id.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "fra_id".to_string(),
            })
        })?;
        let notional = self.notional.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "fra_notional".to_string(),
            })
        })?;
        let fixing_date = self.fixing_date.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "fra_fixing_date".to_string(),
            })
        })?;
        let start_date = self.start_date.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "fra_start_date".to_string(),
            })
        })?;
        let end_date = self.end_date.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "fra_end_date".to_string(),
            })
        })?;
        let fixed_rate = self.fixed_rate.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "fra_fixed_rate".to_string(),
            })
        })?;
        let refs = self.market_refs.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "market_refs".to_string(),
            })
        })?;
        let fwd_id = refs.fwd_id.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "forward_curve_id".to_string(),
            })
        })?;

        let disc_id: &'static str = Box::leak(refs.disc_id.into_string().into_boxed_str());
        let forward_id: &'static str = Box::leak(fwd_id.into_string().into_boxed_str());

        let mut fra = ForwardRateAgreement::new(
            id,
            notional,
            fixing_date,
            start_date,
            end_date,
            fixed_rate,
            self.day_count.unwrap_or(DayCount::Act360),
            disc_id,
            forward_id,
        );

        if let Some(pf) = self.pay_fixed {
            fra = fra.with_pay_fixed(pf);
        }
        if let Some(rl) = self.reset_lag {
            fra = fra.with_reset_lag(rl);
        }

        Ok(fra)
    }
}
