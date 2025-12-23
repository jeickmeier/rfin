//! Equity instruments and equity derivatives.

/// Autocallable module - Autocallable structured notes.
pub mod autocallable;
/// Cliquet option module - Cliquet/ratchet options.
pub mod cliquet_option;
/// DCF equity module - Discounted cash flow for equity (renamed from dcf).
pub mod dcf_equity;
/// Equity index future module.
pub mod equity_index_future;
/// Equity option module - Vanilla equity options.
pub mod equity_option;
/// Equity TRS module - Equity total return swaps.
pub mod equity_trs;
/// PE fund module - Private equity/markets funds (renamed from private_markets_fund).
pub mod pe_fund;
/// Equity spot module - Equity spot positions.
pub mod spot;
/// Variance swap module - Variance and volatility swaps.
pub mod variance_swap;
/// Volatility index future module.
pub mod vol_index_future;
/// Volatility index option module.
pub mod vol_index_option;

// Re-export primary types
pub use autocallable::{Autocallable, FinalPayoffType};
pub use cliquet_option::CliquetOption;
pub use dcf_equity::{DiscountedCashFlow, TerminalValueSpec};
pub use equity_index_future::{EquityFutureSpecs, EquityIndexFuture};
pub use equity_option::EquityOption;
pub use equity_trs::EquityTotalReturnSwap;
pub use pe_fund::PrivateMarketsFund;
pub use spot::Equity;
pub use variance_swap::VarianceSwap;
pub use vol_index_future::{VolIndexContractSpecs, VolatilityIndexFuture};
pub use vol_index_option::{VolIndexOptionSpecs, VolatilityIndexOption};

// Preserve public path for equity metrics
pub use spot::metrics as equity_metrics;
