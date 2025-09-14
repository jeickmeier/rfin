use crate::instruments::common::{CreditParams, DateRange, MarketRefs, PricingOverrides};
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::F;

use super::types::{CDSConvention, CreditDefaultSwap, PayReceive};

/// Enhanced CDS builder using parameter groups.
///
/// Reduces 11 optional fields to 4 required parameter groups.
#[derive(Default)]
pub struct CDSBuilder {
    // Core required parameters
    id: Option<String>,
    notional: Option<Money>,
    side: Option<PayReceive>,
    spread_bp: Option<F>,

    // Parameter groups (required)
    credit_params: Option<CreditParams>,
    date_range: Option<DateRange>,
    market_refs: Option<MarketRefs>,

    // Optional parameters
    convention: Option<CDSConvention>,
    pricing_overrides: Option<PricingOverrides>,
}

impl CDSBuilder {
    /// Create a new CDS builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set instrument ID (required)
    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }

    /// Set notional amount (required)
    pub fn notional(mut self, value: Money) -> Self {
        self.notional = Some(value);
        self
    }

    /// Set side (PayProtection or ReceiveProtection) (required)
    pub fn side(mut self, value: PayReceive) -> Self {
        self.side = Some(value);
        self
    }

    /// Set CDS spread in basis points (required)
    pub fn spread_bp(mut self, value: F) -> Self {
        self.spread_bp = Some(value);
        self
    }

    /// Set credit parameters (required)
    pub fn credit_params(mut self, value: CreditParams) -> Self {
        self.credit_params = Some(value);
        self
    }

    /// Set date range (required)
    pub fn date_range(mut self, value: DateRange) -> Self {
        self.date_range = Some(value);
        self
    }

    /// Set dates directly
    pub fn dates(mut self, start: Date, end: Date) -> Self {
        self.date_range = Some(DateRange::new(start, end));
        self
    }

    /// Set date range from tenor
    pub fn tenor(mut self, start: Date, tenor_years: F) -> Self {
        self.date_range = Some(DateRange::from_tenor(start, tenor_years));
        self
    }

    /// Set market data references (required)
    pub fn market_refs(mut self, value: MarketRefs) -> Self {
        self.market_refs = Some(value);
        self
    }

    /// Set ISDA convention (optional, defaults to IsdaNa)
    pub fn convention(mut self, value: CDSConvention) -> Self {
        self.convention = Some(value);
        self
    }

    /// Set pricing overrides (optional)
    pub fn pricing_overrides(mut self, value: PricingOverrides) -> Self {
        self.pricing_overrides = Some(value);
        self
    }

    /// Convenience: Set upfront payment
    pub fn upfront(mut self, value: Money) -> Self {
        self.pricing_overrides = Some(
            self.pricing_overrides
                .unwrap_or_default()
                .with_upfront(value),
        );
        self
    }

    /// Build the Credit Default Swap
    pub fn build(self) -> finstack_core::Result<CreditDefaultSwap> {
        // Validate required fields
        let id = self.id.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "cds_id".to_string(),
            })
        })?;
        let notional = self.notional.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "cds_notional".to_string(),
            })
        })?;
        let side = self.side.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "cds_side".to_string(),
            })
        })?;
        let spread_bp = self.spread_bp.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "cds_spread".to_string(),
            })
        })?;
        let credit_params = self.credit_params.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "credit_params".to_string(),
            })
        })?;
        let date_range = self.date_range.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "cds_dates".to_string(),
            })
        })?;
        let market_refs = self.market_refs.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "market_refs".to_string(),
            })
        })?;

        // Use provided convention or default to ISDA NA
        let convention = self.convention.unwrap_or(CDSConvention::IsdaNa);
        let pricing = self.pricing_overrides.unwrap_or_default();

        let construction_params = crate::instruments::common::CDSConstructionParams::new(
            notional,
            side,
            convention,
            spread_bp,
        );
        let mut cds = CreditDefaultSwap::new_isda(
            id,
            &construction_params,
            &date_range,
            &credit_params,
            &market_refs,
        );

        // Set pricing overrides
        cds.pricing_overrides = pricing;

        Ok(cds)
    }
}
