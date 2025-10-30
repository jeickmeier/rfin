//! Generic basket instrument for ETFs and multi-asset baskets.
//!
//! This module provides a unified basket instrument that can handle various asset types
//! including equities, bonds, ETFs, and other instruments. It leverages existing
//! pricing infrastructure to avoid code duplication and ensure consistency.
//!
//! # Features
//!
//! - **Multi-asset support**: Equity, bond, and other instrument constituents
//! - **Flexible weighting**: Weight-based or unit-based position sizing
//! - **Existing pricing**: Uses existing Bond and Equity instrument pricing
//! - **NAV calculation**: Real-time net asset value calculation
//!
//! # Examples
//!
//! ## Simple Basket
//! ```no_run
//! use finstack_valuations::instruments::basket::*;
//! use finstack_core::prelude::*;
//!
//! let basket = Basket::builder()
//!     .id("EQUITY_BASKET".into())
//!     .currency(Currency::USD)
//!     .discount_curve_id("USD-OIS".into())
//!     .expense_ratio(0.0025) // 25 basis points
//!     .constituents(vec![]) // Add constituents as needed
//!     .build()
//!     .map_err(|e| format!("Failed to build basket: {}", e))?;
//! ```
//!
//! ## Basket with Market Data Constituents
//! ```no_run
//! use finstack_valuations::instruments::basket::*;
//! use finstack_core::prelude::*;
//!
//! let constituent = BasketConstituent {
//!     id: "AAPL".to_string(),
//!     reference: ConstituentReference::MarketData {
//!         price_id: "AAPL".to_string().into(),
//!         asset_type: AssetType::Equity,
//!     },
//!     weight: 1.0,
//!     units: None,
//!     ticker: None,
//! };
//!
//! let basket = Basket::builder()
//!     .id("EQUITY_BASKET".into())
//!     .currency(Currency::USD)
//!     .discount_curve_id("USD-OIS".into())
//!     .constituents(vec![constituent])
//!     .build()
//!     .map_err(|e| format!("Failed to build basket: {}", e))?;
//! ```

pub mod metrics;
pub mod pricer;
pub mod types;

// Re-export main types for convenience
// Builder is generated via derive on `Basket`.
pub use metrics::register_basket_metrics;
pub use pricer::BasketCalculator;
pub use types::{AssetType, Basket, BasketConstituent, ConstituentReference};

// Use the generic discounting pricer for registry integration
pub use crate::instruments::common::GenericDiscountingPricer;
pub type SimpleBasketDiscountingPricer = GenericDiscountingPricer<Basket>;

impl Default for SimpleBasketDiscountingPricer {
    fn default() -> Self {
        Self::new(crate::pricer::InstrumentType::Basket)
    }
}
