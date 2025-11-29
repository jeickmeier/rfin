//! Foreign exchange shock adapter.
//!
//! This module supports FX shocks through the `OperationSpec::MarketFxPct` variant.
//! The engine applies FX shocks via `MarketBump::FxPct` which wraps the existing
//! provider behind a [`BumpedFxProvider`](finstack_core::money::fx::providers::BumpedFxProvider)
//! so the operation remains deterministic and easy to audit.
