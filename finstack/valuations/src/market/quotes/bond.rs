//! Bond market quote schema.

use super::ids::QuoteId;
use crate::market::conventions::ids::BondConventionId;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Market quote for bond instruments.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum BondQuote {
    /// Fixed-rate bullet bond quoted in clean price (% of par).
    FixedRateBulletCleanPrice {
        /// Unique identifier for the quote.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        id: QuoteId,
        /// Settlement / pricing currency of the bond.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        currency: Currency,
        /// Bond issue date.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        issue_date: Date,
        /// Bond maturity date.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Annual coupon rate in decimal form.
        coupon_rate: f64,
        /// Bond convention identifier.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        convention: BondConventionId,
        /// Quoted clean price as percent of par.
        clean_price_pct: f64,
    },
    /// Fixed-rate bullet bond quoted in Z-spread over a discount curve (decimal).
    FixedRateBulletZSpread {
        /// Unique identifier for the quote.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        id: QuoteId,
        /// Settlement / pricing currency of the bond.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        currency: Currency,
        /// Bond issue date.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        issue_date: Date,
        /// Bond maturity date.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Annual coupon rate in decimal form.
        coupon_rate: f64,
        /// Bond convention identifier.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        convention: BondConventionId,
        /// Z-spread in decimal form (e.g. 0.01 = 100bp).
        z_spread: f64,
    },
    /// Fixed-rate bullet bond quoted in OAS (decimal).
    FixedRateBulletOas {
        /// Unique identifier for the quote.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        id: QuoteId,
        /// Settlement / pricing currency of the bond.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        currency: Currency,
        /// Bond issue date.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        issue_date: Date,
        /// Bond maturity date.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Annual coupon rate in decimal form.
        coupon_rate: f64,
        /// Bond convention identifier.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        convention: BondConventionId,
        /// Option-adjusted spread in decimal form (e.g. 0.005 = 50bp).
        oas: f64,
    },
    /// Fixed-rate bullet bond quoted in yield-to-maturity (decimal).
    FixedRateBulletYtm {
        /// Unique identifier for the quote.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        id: QuoteId,
        /// Settlement / pricing currency of the bond.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        currency: Currency,
        /// Bond issue date.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        issue_date: Date,
        /// Bond maturity date.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Annual coupon rate in decimal form.
        coupon_rate: f64,
        /// Bond convention identifier.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        convention: BondConventionId,
        /// Yield to maturity in decimal form.
        ytm: f64,
    },
}

impl BondQuote {
    /// Get the unique identifier of the quote.
    pub fn id(&self) -> &QuoteId {
        match self {
            BondQuote::FixedRateBulletCleanPrice { id, .. } => id,
            BondQuote::FixedRateBulletZSpread { id, .. } => id,
            BondQuote::FixedRateBulletOas { id, .. } => id,
            BondQuote::FixedRateBulletYtm { id, .. } => id,
        }
    }

    /// Get the primary market value of the quote.
    pub fn value(&self) -> f64 {
        match self {
            BondQuote::FixedRateBulletCleanPrice {
                clean_price_pct, ..
            } => *clean_price_pct,
            BondQuote::FixedRateBulletZSpread { z_spread, .. } => *z_spread,
            BondQuote::FixedRateBulletOas { oas, .. } => *oas,
            BondQuote::FixedRateBulletYtm { ytm, .. } => *ytm,
        }
    }

    /// Bump the primary quoted value by a decimal amount.
    ///
    /// - For clean-price quotes, bumps `clean_price_pct`.
    /// - For YTM quotes, bumps `ytm`.
    /// - For Z-spread quotes, bumps `z_spread`.
    /// - For OAS quotes, bumps `oas`.
    pub fn bump_value_decimal(&self, bump: f64) -> Self {
        match self {
            BondQuote::FixedRateBulletCleanPrice {
                id,
                currency,
                issue_date,
                maturity,
                coupon_rate,
                convention,
                clean_price_pct,
            } => BondQuote::FixedRateBulletCleanPrice {
                id: id.clone(),
                currency: *currency,
                issue_date: *issue_date,
                maturity: *maturity,
                coupon_rate: *coupon_rate,
                convention: convention.clone(),
                clean_price_pct: clean_price_pct + bump,
            },
            BondQuote::FixedRateBulletZSpread {
                id,
                currency,
                issue_date,
                maturity,
                coupon_rate,
                convention,
                z_spread,
            } => BondQuote::FixedRateBulletZSpread {
                id: id.clone(),
                currency: *currency,
                issue_date: *issue_date,
                maturity: *maturity,
                coupon_rate: *coupon_rate,
                convention: convention.clone(),
                z_spread: z_spread + bump,
            },
            BondQuote::FixedRateBulletOas {
                id,
                currency,
                issue_date,
                maturity,
                coupon_rate,
                convention,
                oas,
            } => BondQuote::FixedRateBulletOas {
                id: id.clone(),
                currency: *currency,
                issue_date: *issue_date,
                maturity: *maturity,
                coupon_rate: *coupon_rate,
                convention: convention.clone(),
                oas: oas + bump,
            },
            BondQuote::FixedRateBulletYtm {
                id,
                currency,
                issue_date,
                maturity,
                coupon_rate,
                convention,
                ytm,
            } => BondQuote::FixedRateBulletYtm {
                id: id.clone(),
                currency: *currency,
                issue_date: *issue_date,
                maturity: *maturity,
                coupon_rate: *coupon_rate,
                convention: convention.clone(),
                ytm: ytm + bump,
            },
        }
    }

    /// Bump the primary quoted value by basis points (e.g., `1.0` = 1bp).
    pub fn bump_value_bp(&self, bump_bp: f64) -> Self {
        self.bump_value_decimal(bump_bp / 10_000.0)
    }
}
