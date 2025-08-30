//! Built-in holiday calendars using the unified `Rule` DSL.
#![allow(missing_docs)]
//!
//! Each market calendar is expressed as a `const &[Rule]` slice plus a zero-sized
//! marker struct exposing an `id()` helper.

pub mod asx;
pub mod auce;
pub mod brbd;
pub mod cato;
pub mod chzh;
pub mod cme;
pub mod cnbe;
pub mod defr;
pub mod gblo;
pub mod hkex;
pub mod hkhk;
pub mod jpto;
pub mod jpx;
pub mod nyse;
pub mod sgsi;
pub mod sifma;
pub mod sse;
pub mod target2;
pub mod usny;

pub use asx::Asx;
pub use auce::Auce;
pub use brbd::Brbd;
pub use cato::Cato;
pub use chzh::Chzh;
pub use cme::Cme;
pub use cnbe::Cnbe;
pub use defr::Defr;
pub use gblo::Gblo;
pub use hkex::Hkex;
pub use hkhk::Hkhk;
pub use jpto::Jpto;
pub use jpx::Jpx;
pub use nyse::Nyse;
pub use sgsi::Sgsi;
pub use sifma::Sifma;
pub use sse::Sse;
pub use target2::Target2;
pub use usny::Usny;

// Include generated registry: ALL_IDS, calendar_by_id, and helpers
include!(concat!(env!("OUT_DIR"), "/generated_calendars.rs"));

