//! Currency types and operations.
//!
//! This module provides the [`Currency`] enum based on ISO 4217 standard,
//! with numeric discriminants matching the official ISO currency codes.

use core::fmt;
use core::str::FromStr;

/// Error type for currency operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurrencyError {
    /// Invalid currency code provided
    InvalidCode,
}

impl fmt::Display for CurrencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CurrencyError::InvalidCode => write!(f, "Invalid currency code"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CurrencyError {}

// Include the generated Currency enum and implementations
include!(concat!(env!("OUT_DIR"), "/currency_generated.rs"));

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            Currency::AED => "AED",
            Currency::AFN => "AFN",
            Currency::ALL => "ALL",
            Currency::AMD => "AMD",
            Currency::ANG => "ANG",
            Currency::AOA => "AOA",
            Currency::ARS => "ARS",
            Currency::AUD => "AUD",
            Currency::AWG => "AWG",
            Currency::AZN => "AZN",
            Currency::BAM => "BAM",
            Currency::BBD => "BBD",
            Currency::BDT => "BDT",
            Currency::BGN => "BGN",
            Currency::BHD => "BHD",
            Currency::BIF => "BIF",
            Currency::BMD => "BMD",
            Currency::BND => "BND",
            Currency::BOB => "BOB",
            Currency::BRL => "BRL",
            Currency::BSD => "BSD",
            Currency::BTN => "BTN",
            Currency::BWP => "BWP",
            Currency::BYN => "BYN",
            Currency::BZD => "BZD",
            Currency::CAD => "CAD",
            Currency::CDF => "CDF",
            Currency::CHF => "CHF",
            Currency::CLP => "CLP",
            Currency::CNY => "CNY",
            Currency::COP => "COP",
            Currency::CRC => "CRC",
            Currency::CUC => "CUC",
            Currency::CUP => "CUP",
            Currency::CVE => "CVE",
            Currency::CZK => "CZK",
            Currency::DJF => "DJF",
            Currency::DKK => "DKK",
            Currency::DOP => "DOP",
            Currency::DZD => "DZD",
            Currency::EGP => "EGP",
            Currency::ERN => "ERN",
            Currency::ETB => "ETB",
            Currency::EUR => "EUR",
            Currency::FJD => "FJD",
            Currency::FKP => "FKP",
            Currency::GBP => "GBP",
            Currency::GEL => "GEL",
            Currency::GHS => "GHS",
            Currency::GIP => "GIP",
            Currency::GMD => "GMD",
            Currency::GNF => "GNF",
            Currency::GTQ => "GTQ",
            Currency::GYD => "GYD",
            Currency::HKD => "HKD",
            Currency::HNL => "HNL",
            Currency::HRK => "HRK",
            Currency::HTG => "HTG",
            Currency::HUF => "HUF",
            Currency::IDR => "IDR",
            Currency::ILS => "ILS",
            Currency::INR => "INR",
            Currency::IQD => "IQD",
            Currency::IRR => "IRR",
            Currency::ISK => "ISK",
            Currency::JMD => "JMD",
            Currency::JOD => "JOD",
            Currency::JPY => "JPY",
            Currency::KES => "KES",
            Currency::KGS => "KGS",
            Currency::KHR => "KHR",
            Currency::KMF => "KMF",
            Currency::KPW => "KPW",
            Currency::KRW => "KRW",
            Currency::KWD => "KWD",
            Currency::KYD => "KYD",
            Currency::KZT => "KZT",
            Currency::LAK => "LAK",
            Currency::LBP => "LBP",
            Currency::LKR => "LKR",
            Currency::LRD => "LRD",
            Currency::LSL => "LSL",
            Currency::LYD => "LYD",
            Currency::MAD => "MAD",
            Currency::MDL => "MDL",
            Currency::MGA => "MGA",
            Currency::MKD => "MKD",
            Currency::MMK => "MMK",
            Currency::MNT => "MNT",
            Currency::MOP => "MOP",
            Currency::MRU => "MRU",
            Currency::MUR => "MUR",
            Currency::MVR => "MVR",
            Currency::MWK => "MWK",
            Currency::MXN => "MXN",
            Currency::MYR => "MYR",
            Currency::MZN => "MZN",
            Currency::NAD => "NAD",
            Currency::NGN => "NGN",
            Currency::NIO => "NIO",
            Currency::NOK => "NOK",
            Currency::NPR => "NPR",
            Currency::NZD => "NZD",
            Currency::OMR => "OMR",
            Currency::PAB => "PAB",
            Currency::PEN => "PEN",
            Currency::PGK => "PGK",
            Currency::PHP => "PHP",
            Currency::PKR => "PKR",
            Currency::PLN => "PLN",
            Currency::PYG => "PYG",
            Currency::QAR => "QAR",
            Currency::RON => "RON",
            Currency::RSD => "RSD",
            Currency::RUB => "RUB",
            Currency::RWF => "RWF",
            Currency::SAR => "SAR",
            Currency::SBD => "SBD",
            Currency::SCR => "SCR",
            Currency::SDG => "SDG",
            Currency::SEK => "SEK",
            Currency::SGD => "SGD",
            Currency::SHP => "SHP",
            Currency::SLE => "SLE",
            Currency::SLL => "SLL",
            Currency::SOS => "SOS",
            Currency::SRD => "SRD",
            Currency::STN => "STN",
            Currency::SYP => "SYP",
            Currency::SZL => "SZL",
            Currency::THB => "THB",
            Currency::TJS => "TJS",
            Currency::TMT => "TMT",
            Currency::TND => "TND",
            Currency::TOP => "TOP",
            Currency::TRY => "TRY",
            Currency::TTD => "TTD",
            Currency::TWD => "TWD",
            Currency::TZS => "TZS",
            Currency::UAH => "UAH",
            Currency::UGX => "UGX",
            Currency::USD => "USD",
            Currency::UYU => "UYU",
            Currency::UZS => "UZS",
            Currency::VED => "VED",
            Currency::VES => "VES",
            Currency::VND => "VND",
            Currency::VUV => "VUV",
            Currency::WST => "WST",
            Currency::XAF => "XAF",
            Currency::XCD => "XCD",
            Currency::XOF => "XOF",
            Currency::XPF => "XPF",
            Currency::YER => "YER",
            Currency::ZAR => "ZAR",
            Currency::ZMW => "ZMW",
            Currency::ZWL => "ZWL",
        };
        write!(f, "{}", code)
    }
}

impl FromStr for Currency {
    type Err = CurrencyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lookup_currency(s).ok_or(CurrencyError::InvalidCode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem;

    #[cfg(feature = "std")]
    use std::format;

    #[test]
    fn test_currency_size() {
        assert_eq!(mem::size_of::<Currency>(), 2);
    }

    #[test]
    fn test_currency_numeric_values() {
        assert_eq!(Currency::USD as u16, 840);
        assert_eq!(Currency::EUR as u16, 978);
        assert_eq!(Currency::GBP as u16, 826);
        assert_eq!(Currency::JPY as u16, 392);
        assert_eq!(Currency::CHF as u16, 756);
        assert_eq!(Currency::AUD as u16, 36);
        assert_eq!(Currency::CAD as u16, 124);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_currency_display() {
        assert_eq!(format!("{}", Currency::USD), "USD");
        assert_eq!(format!("{}", Currency::EUR), "EUR");
        assert_eq!(format!("{}", Currency::GBP), "GBP");
        assert_eq!(format!("{}", Currency::JPY), "JPY");
    }

    #[test]
    fn test_currency_from_str() {
        assert_eq!("USD".parse::<Currency>().unwrap(), Currency::USD);
        assert_eq!("EUR".parse::<Currency>().unwrap(), Currency::EUR);
        assert_eq!("GBP".parse::<Currency>().unwrap(), Currency::GBP);
        assert_eq!("JPY".parse::<Currency>().unwrap(), Currency::JPY);

        // Test case insensitive
        assert_eq!("usd".parse::<Currency>().unwrap(), Currency::USD);
        assert_eq!("eur".parse::<Currency>().unwrap(), Currency::EUR);
        assert_eq!("gbp".parse::<Currency>().unwrap(), Currency::GBP);
    }

    #[test]
    fn test_currency_from_str_invalid() {
        assert_eq!(
            "INVALID".parse::<Currency>(),
            Err(CurrencyError::InvalidCode)
        );
        assert_eq!("XXX".parse::<Currency>(), Err(CurrencyError::InvalidCode));
        assert_eq!("".parse::<Currency>(), Err(CurrencyError::InvalidCode));
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_currency_round_trip() {
        let currencies = [
            Currency::USD,
            Currency::EUR,
            Currency::GBP,
            Currency::JPY,
            Currency::CHF,
            Currency::AUD,
            Currency::CAD,
            Currency::CNY,
            Currency::SEK,
            Currency::NOK,
            Currency::DKK,
            Currency::PLN,
        ];

        for currency in &currencies {
            let formatted = format!("{}", currency);
            let parsed: Currency = formatted.parse().unwrap();
            assert_eq!(*currency, parsed);
        }
    }

    #[test]
    fn test_currency_minor_units() {
        assert_eq!(Currency::USD.minor_units(), 2);
        assert_eq!(Currency::EUR.minor_units(), 2);
        assert_eq!(Currency::JPY.minor_units(), 0);
        assert_eq!(Currency::BHD.minor_units(), 3);
        assert_eq!(Currency::KWD.minor_units(), 3);
        assert_eq!(Currency::JOD.minor_units(), 3);
        // Most currencies have 2 minor units

        // Currencies with 0 minor units
        assert_eq!(Currency::BIF.minor_units(), 0);
        assert_eq!(Currency::CLP.minor_units(), 0);
        assert_eq!(Currency::DJF.minor_units(), 0);
        assert_eq!(Currency::GNF.minor_units(), 0);
        assert_eq!(Currency::ISK.minor_units(), 0);
        assert_eq!(Currency::KMF.minor_units(), 0);
        assert_eq!(Currency::KRW.minor_units(), 0);
        assert_eq!(Currency::PYG.minor_units(), 0);
        assert_eq!(Currency::RWF.minor_units(), 0);
        assert_eq!(Currency::UGX.minor_units(), 0);
        assert_eq!(Currency::VND.minor_units(), 0);
        assert_eq!(Currency::VUV.minor_units(), 0);
        assert_eq!(Currency::XAF.minor_units(), 0);
        assert_eq!(Currency::XOF.minor_units(), 0);
        assert_eq!(Currency::XPF.minor_units(), 0);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_currency_error_display() {
        let error = CurrencyError::InvalidCode;
        assert_eq!(format!("{}", error), "Invalid currency code");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_currency_serde() {
        let currency = Currency::USD;
        let serialized = serde_json::to_string(&currency).unwrap();
        let deserialized: Currency = serde_json::from_str(&serialized).unwrap();
        assert_eq!(currency, deserialized);
    }
}
