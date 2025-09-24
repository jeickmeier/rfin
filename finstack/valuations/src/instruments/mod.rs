//! Financial instruments module: imports and re-exports only.

// Common functionality (traits, macros, models, helpers)
#[macro_use]
pub mod common;

// Flattened instrument modules
pub mod basis_swap;
pub mod basket;
pub mod bond;
pub mod cap_floor;
pub mod cds;
pub mod cds_index;
pub mod cds_option;
pub mod cds_tranche;
pub mod convertible;
pub mod deposit;
pub mod equity;
pub mod equity_option;
pub mod fra;
pub mod fx_option;
pub mod fx_spot;
pub mod fx_swap;
pub mod inflation_linked_bond;
pub mod inflation_swap;
pub mod ir_future;
pub mod irs;
pub mod pricing_overrides;
pub mod private_markets_fund;
pub mod repo;
// Preserve public path for equity underlying params after move
pub use equity::underlying;
// Preserve public path for equity metrics after move
pub use equity::metrics as equity_metrics;
pub mod structured_credit;
pub mod swaption;
pub mod trs;
pub mod variance_swap;

// Re-export common types for convenience (avoid glob re-exports to keep API unambiguous)
pub use basis_swap::BasisSwap;
pub use basket::Basket;
pub use bond::Bond;
pub use cds::CreditDefaultSwap;
pub use cds_index::CDSIndex;
pub use cds_option::CdsOption;
pub use cds_tranche::CdsTranche;
pub use convertible::ConvertibleBond;
pub use deposit::Deposit;
pub use equity::Equity;
pub use equity_option::EquityOption;
pub use fra::ForwardRateAgreement;
pub use fx_option::FxOption;
pub use fx_spot::FxSpot;
pub use fx_swap::FxSwap;
pub use inflation_linked_bond::InflationLinkedBond;
pub use inflation_swap::InflationSwap;
pub use ir_future::InterestRateFuture;
pub use irs::InterestRateSwap;
pub use pricing_overrides::PricingOverrides;
pub use private_markets_fund::PrivateMarketsFund;
pub use repo::{CollateralSpec, CollateralType, Repo, RepoType};
pub use structured_credit::{Abs, Clo, StructuredCredit};
pub use swaption::Swaption;
pub use trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};

// Re-export option-related enums and models at top-level after flattening
pub use cap_floor::RateOptionType;
pub use common::{BinomialTree, TreeType};

pub use common::build_with_metrics_dyn;

// Backward compatibility surface (deprecated/legacy re-exports)
pub mod compat;
pub use compat::*;

// Pricer registrations for instruments
#[allow(dead_code)]
pub(crate) mod registry {
    // Register pricers here
    crate::pricers! {
        Bond / Discounting => crate::instruments::bond::pricing::pricer::DiscountingPricer::new,
        Bond / Tree         => crate::instruments::bond::pricing::pricer::OasPricer::new,
        IRS  / Discounting  => crate::instruments::irs::pricing::pricer::DiscountingPricer::new,
        CapFloor / Black76  => crate::instruments::cap_floor::pricing::pricer::BlackPricer::new,
        Swaption / Black76  => crate::instruments::swaption::pricing::pricer::BlackPricer::new,
        CDS / HazardRate   => crate::instruments::cds::pricing::pricer::DiscountingPricer::new,
        CDSIndex / HazardRate => crate::instruments::cds_index::pricing::pricer::DiscountingPricer::new,
        CDSTranche / HazardRate => crate::instruments::cds_tranche::pricing::pricer::DiscountingPricer::new,
        CDSOption / Black76 => crate::instruments::cds_option::pricing::pricer::BlackPricer::new,
        BasisSwap / Discounting => crate::instruments::basis_swap::pricing::pricer::DiscountingPricer::new,
        Basket / Discounting => crate::instruments::basket::pricing::pricer::DiscountingPricer::new,
        Deposit / Discounting => crate::instruments::deposit::pricing::pricer::DiscountingPricer::new,
        Equity / Discounting => crate::instruments::equity::pricing::pricer::DiscountingPricer::new,
        EquityOption / Black76 => crate::instruments::equity_option::pricing::pricer::BlackPricer::new,
        FxOption / Black76 => crate::instruments::fx_option::pricing::pricer::BlackPricer::new,
        FxSpot / Discounting => crate::instruments::fx_spot::pricing::pricer::DiscountingPricer::new,
        TRS / Discounting => crate::instruments::trs::pricing::pricer::DiscountingPricer::new,
        InterestRateFuture / Discounting => crate::instruments::ir_future::pricing::pricer::DiscountingPricer::new,
        InflationLinkedBond / Discounting => crate::instruments::inflation_linked_bond::pricing::pricer::DiscountingPricer::new,
        VarianceSwap / Discounting => crate::instruments::variance_swap::pricing::pricer::DiscountingPricer::new,
    }
}
