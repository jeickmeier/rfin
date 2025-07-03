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

use crate::dates::calendar::HolidayCalendar;

// Static instances for &'static dyn HolidayCalendar references
static GBLO_INSTANCE: Gblo = Gblo;
static TARGET2_INSTANCE: Target2 = Target2;
static ASX_INSTANCE: Asx = Asx;
static AUCE_INSTANCE: Auce = Auce;
static CATO_INSTANCE: Cato = Cato;
static DEFR_INSTANCE: Defr = Defr;
static NYSE_INSTANCE: Nyse = Nyse;
static USNY_INSTANCE: Usny = Usny;
static SIFMA_INSTANCE: Sifma = Sifma;
static BRBD_INSTANCE: Brbd = Brbd;
static CHZH_INSTANCE: Chzh = Chzh;
static CNBE_INSTANCE: Cnbe = Cnbe;
static SGSI_INSTANCE: Sgsi = Sgsi;
static SSE_INSTANCE: Sse = Sse;
static HKHK_INSTANCE: Hkhk = Hkhk;
static HKEX_INSTANCE: Hkex = Hkex;
static JPTO_INSTANCE: Jpto = Jpto;
static JPX_INSTANCE: Jpx = Jpx;
static CME_INSTANCE: Cme = Cme;

/// Lookup a built-in calendar by its lowercase identifier (e.g. "gblo", "target2").
/// Returns `None` if the id is unrecognised.
pub fn calendar_by_id(id: &str) -> Option<&'static dyn HolidayCalendar> {
    match id {
        "gblo" => Some(&GBLO_INSTANCE as &dyn HolidayCalendar),
        "target2" => Some(&TARGET2_INSTANCE as &dyn HolidayCalendar),
        "asx" => Some(&ASX_INSTANCE as &dyn HolidayCalendar),
        "auce" => Some(&AUCE_INSTANCE as &dyn HolidayCalendar),
        "cato" => Some(&CATO_INSTANCE as &dyn HolidayCalendar),
        "defr" => Some(&DEFR_INSTANCE as &dyn HolidayCalendar),
        "nyse" => Some(&NYSE_INSTANCE as &dyn HolidayCalendar),
        "usny" => Some(&USNY_INSTANCE as &dyn HolidayCalendar),
        "sifma" => Some(&SIFMA_INSTANCE as &dyn HolidayCalendar),
        "brbd" => Some(&BRBD_INSTANCE as &dyn HolidayCalendar),
        "chzh" => Some(&CHZH_INSTANCE as &dyn HolidayCalendar),
        "cnbe" => Some(&CNBE_INSTANCE as &dyn HolidayCalendar),
        "sgsi" => Some(&SGSI_INSTANCE as &dyn HolidayCalendar),
        "sse" => Some(&SSE_INSTANCE as &dyn HolidayCalendar),
        "hkhk" => Some(&HKHK_INSTANCE as &dyn HolidayCalendar),
        "hkex" => Some(&HKEX_INSTANCE as &dyn HolidayCalendar),
        "jpto" => Some(&JPTO_INSTANCE as &dyn HolidayCalendar),
        "jpx" => Some(&JPX_INSTANCE as &dyn HolidayCalendar),
        "cme" => Some(&CME_INSTANCE as &dyn HolidayCalendar),
        _ => None,
    }
}

pub const ALL_IDS: &[&str] = &[
    "gblo", "target2", "asx", "auce", "cato", "defr", "nyse", "usny", "sifma", "brbd", "chzh",
    "cnbe", "sgsi", "sse", "hkhk", "hkex", "jpto", "jpx", "cme",
];

macro_rules! impl_new {
    ( $( $t:ty ),* $(,)? ) => {
        $( impl $t { #[inline] pub const fn new() -> Self { Self } } )*
    };
}

impl_new!(
    Gblo, Target2, Asx, Auce, Cato, Defr, Nyse, Usny, Sifma, Brbd, Chzh, Cnbe, Sgsi, Sse, Hkhk,
    Hkex, Jpto, Jpx, Cme
);
