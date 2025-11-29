//! Equity price shock adapter.
//!
//! This module supports equity price shocks through `OperationSpec::EquityPricePct`.
//! The engine applies equity shocks via `MarketBump::Curve` with `BumpUnits::Percent`,
//! which modifies the price stored in market data scalars.
