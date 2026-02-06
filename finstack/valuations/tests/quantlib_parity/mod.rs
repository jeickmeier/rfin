//! QuantLib parity test modules.
//!
//! Each module validates a specific instrument class against known analytical
//! reference values matching QuantLib conventions.

mod test_bond_pricing_parity;
mod test_cds_parity;
mod test_equity_option_parity;
mod test_fx_option_parity;
mod test_irs_parity;
