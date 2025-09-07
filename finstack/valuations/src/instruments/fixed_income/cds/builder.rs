use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::F;

use super::types::{CDSConvention, CreditDefaultSwap, PayReceive};

/// Builder pattern for CDS instruments
#[derive(Default)]
pub struct CDSBuilder {
    id: Option<String>,
    notional: Option<Money>,
    reference_entity: Option<String>,
    side: Option<PayReceive>,
    convention: Option<CDSConvention>,
    start: Option<Date>,
    end: Option<Date>,
    spread_bp: Option<F>,
    credit_id: Option<&'static str>,
    recovery_rate: Option<F>,
    disc_id: Option<&'static str>,
    upfront: Option<Money>,
}

impl CDSBuilder {
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

    pub fn reference_entity(mut self, value: impl Into<String>) -> Self {
        self.reference_entity = Some(value.into());
        self
    }

    pub fn side(mut self, value: PayReceive) -> Self {
        self.side = Some(value);
        self
    }

    pub fn convention(mut self, value: CDSConvention) -> Self {
        self.convention = Some(value);
        self
    }

    pub fn start(mut self, value: Date) -> Self {
        self.start = Some(value);
        self
    }

    pub fn end(mut self, value: Date) -> Self {
        self.end = Some(value);
        self
    }

    pub fn spread_bp(mut self, value: F) -> Self {
        self.spread_bp = Some(value);
        self
    }

    pub fn credit_id(mut self, value: &'static str) -> Self {
        self.credit_id = Some(value);
        self
    }

    pub fn recovery_rate(mut self, value: F) -> Self {
        self.recovery_rate = Some(value);
        self
    }

    pub fn disc_id(mut self, value: &'static str) -> Self {
        self.disc_id = Some(value);
        self
    }

    pub fn upfront(mut self, value: Money) -> Self {
        self.upfront = Some(value);
        self
    }

    pub fn build(self) -> finstack_core::Result<CreditDefaultSwap> {
        let id = self
            .id
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let notional = self
            .notional
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let reference_entity = self
            .reference_entity
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let side = self
            .side
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let convention = self
            .convention
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let start = self
            .start
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let end = self
            .end
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let spread_bp = self
            .spread_bp
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let credit_id = self
            .credit_id
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let recovery_rate = self
            .recovery_rate
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let disc_id = self
            .disc_id
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;

        let mut cds = CreditDefaultSwap::new_isda(
            id,
            notional,
            reference_entity,
            side,
            convention,
            start,
            end,
            spread_bp,
            credit_id,
            recovery_rate,
            disc_id,
        );

        // Set optional upfront payment
        cds.upfront = self.upfront;

        Ok(cds)
    }
}
