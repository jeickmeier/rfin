//! Price, yield, and spread metrics for bonds.

/// Asset swap spread calculators (par, market, forward)
pub mod asw;
/// Discount margin calculator
pub mod dm;
/// I-spread (interpolated spread) calculator
pub mod i_spread;
/// Option-adjusted spread (OAS) calculator
pub mod oas;
/// Price calculators (clean and dirty)
pub mod prices;
/// Yield-to-maturity (YTM) calculator
pub mod ytm;
/// Yield-to-worst (YTW) calculator
pub mod ytw;
/// Z-spread (zero-volatility spread) calculator
pub mod z_spread;

pub use asw::{
    AssetSwapMarketCalculator, AssetSwapMarketFwdCalculator, AssetSwapParCalculator,
    AssetSwapParFwdCalculator,
};
pub use dm::{DiscountMarginCalculator, DiscountMarginSolverConfig};
pub use i_spread::ISpreadCalculator;
pub use oas::OasCalculator;
pub use prices::{CleanPriceCalculator, DirtyPriceCalculator};
pub use ytm::YtmCalculator;
pub use ytw::YtwCalculator;
pub use z_spread::{ZSpreadCalculator, ZSpreadSolverConfig};

