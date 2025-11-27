//! Instrument integration tests - comprehensive test runner for all instruments
//!
//! This module organizes instrument tests by asset class and product type.

// ============================================================================
// Common Test Infrastructure
// ============================================================================

/// Common test utilities, helpers, models, and shared functionality
#[path = "instruments/common/mod.rs"]
mod common;

/// QuantLib parity testing framework
#[macro_use]
#[path = "quantlib_parity_helpers.rs"]
pub mod quantlib_parity_helpers;

// ============================================================================
// Fixed Income Instruments
// ============================================================================

/// Bond tests - Fixed coupon bonds, zero-coupon bonds, floating rate notes
#[path = "instruments/bond/mod.rs"]
mod bond;

/// Deposit tests - Money market deposits and term deposits
#[path = "instruments/deposit/mod.rs"]
mod deposit;

/// Inflation-linked bond tests - Index-linked bonds (e.g., TIPS, linkers)
#[path = "instruments/inflation_linked_bond/mod.rs"]
mod inflation_linked_bond;

// ============================================================================
// Interest Rate Derivatives
// ============================================================================

/// Basis swap tests - Multi-curve basis swaps (e.g., 3m vs 6m LIBOR)
#[path = "instruments/basis_swap/mod.rs"]
mod basis_swap;

/// Cap/Floor tests - Interest rate caps and floors
#[path = "instruments/cap_floor/mod.rs"]
mod cap_floor;

/// Forward Rate Agreement (FRA) tests - Single period rate agreements
#[path = "instruments/fra/mod.rs"]
mod fra;

/// CMS Option tests - Constant Maturity Swap caps and floors
#[path = "instruments/cms_option/mod.rs"]
mod cms_option;

/// Interest rate future tests - Exchange-traded IR futures
#[path = "instruments/ir_future/mod.rs"]
mod ir_future;

/// Inflation swap tests - Zero-coupon and year-on-year inflation swaps
#[path = "instruments/inflation_swap/mod.rs"]
mod inflation_swap;

/// Interest Rate Swap (IRS) tests - Fixed-for-floating swaps
#[path = "instruments/irs/mod.rs"]
mod irs;

/// Swaption tests - Options on interest rate swaps
#[path = "instruments/swaption/mod.rs"]
mod swaption;

// ============================================================================
// Credit Derivatives
// ============================================================================

/// Credit Default Swap (CDS) tests - Single-name protection
#[path = "instruments/cds/mod.rs"]
mod cds;

/// CDS index tests - Index CDS (e.g., CDX, iTraxx)
#[path = "instruments/cds_index/mod.rs"]
mod cds_index;

/// CDS option tests - Options on credit default swaps
#[path = "instruments/cds_option/mod.rs"]
mod cds_option;

/// CDS tranche tests - Structured credit tranches
#[path = "instruments/cds_tranche/mod.rs"]
mod cds_tranche;

// ============================================================================
// Equity Derivatives
// ============================================================================

/// Equity tests - Equity spot instruments
#[path = "instruments/equity/mod.rs"]
mod equity;

/// Basket option tests - Multi-underlying options
#[path = "instruments/basket/mod.rs"]
mod basket;

/// Equity option tests - Vanilla and exotic equity options
#[path = "instruments/equity_option/mod.rs"]
mod equity_option;

/// Lookback option tests
#[path = "instruments/lookback_option/mod.rs"]
mod lookback_option;

/// Total Return Swap (TRS) tests - Equity and asset swaps
#[path = "instruments/trs/mod.rs"]
mod trs;

/// Variance swap tests - Volatility derivatives
#[path = "instruments/variance_swap/mod.rs"]
mod variance_swap;

// ============================================================================
// FX Derivatives
// ============================================================================

/// FX option tests - Currency options (vanilla and exotic)
#[path = "instruments/fx_option/mod.rs"]
mod fx_option;

/// FX spot tests - Spot foreign exchange transactions
#[path = "instruments/fx_spot/mod.rs"]
mod fx_spot;

/// FX swap tests - FX forward and swap transactions
#[path = "instruments/fx_swap/mod.rs"]
mod fx_swap;

// ============================================================================
// Structured Products
// ============================================================================

/// Convertible bond tests - Bonds with embedded conversion options
#[path = "instruments/convertible/mod.rs"]
mod convertible;

/// Private market fund tests - Private equity and credit fund structures
#[path = "instruments/private_market_fund/mod.rs"]
mod private_market_fund;

/// Structured credit tests - CLOs, CDOs, and synthetic structures
#[path = "instruments/structured_credit/mod.rs"]
mod structured_credit;

// ============================================================================
// Repo and Financing
// ============================================================================

/// Repo tests - Repurchase agreements
#[path = "instruments/repo/mod.rs"]
mod repo;

/// Revolving credit tests - Revolving credit facilities
#[path = "instruments/revolving_credit/mod.rs"]
mod revolving_credit;

/// Term loan tests - Institutional term loans and DDTL facilities
#[path = "instruments/term_loan/mod.rs"]
mod term_loan;
