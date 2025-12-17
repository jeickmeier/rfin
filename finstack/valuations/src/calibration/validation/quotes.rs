//! Quote validation utilities for `CalibrationPricer`.

use crate::calibration::domain::pricing::{CalibrationPricer, RatesQuoteUseCase};
use crate::calibration::domain::quotes::rate_index::RateIndexConventions;
use crate::calibration::domain::quotes::RatesQuote;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::prelude::*;

impl CalibrationPricer {
    /// Extract rate from quote.
    pub fn get_rate(quote: &RatesQuote) -> f64 {
        match quote {
            RatesQuote::Deposit { rate, .. } => *rate,
            RatesQuote::FRA { rate, .. } => *rate,
            RatesQuote::Future { price, .. } => (100.0 - price) / 100.0,
            RatesQuote::Swap { rate, .. } => *rate,
            RatesQuote::BasisSwap { spread_bp, .. } => *spread_bp / 10_000.0,
        }
    }

    /// Build a duplicate-detection key tailored to the quote type.
    fn dedupe_key(quote: &RatesQuote) -> String {
        match quote {
            RatesQuote::Deposit { maturity, .. } => format!("DEP|{}", maturity),
            RatesQuote::FRA { start, end, .. } => format!("FRA|{}|{}", start, end),
            RatesQuote::Future {
                expiry,
                period_start,
                period_end,
                ..
            } => format!("FUT|{}|{}|{}", expiry, period_start, period_end),
            RatesQuote::Swap {
                maturity,
                float_leg_conventions,
                ..
            } => {
                let index = float_leg_conventions
                    .index
                    .as_ref()
                    .map(|i| i.as_str())
                    .unwrap_or("UNKNOWN");
                let is_ois = quote.is_ois_suitable();
                format!("SWAP|{}|{}|{}", maturity, index, is_ois)
            }
            RatesQuote::BasisSwap {
                maturity,
                primary_leg_conventions,
                reference_leg_conventions,
                ..
            } => {
                let primary_idx = primary_leg_conventions
                    .index
                    .as_ref()
                    .map(|i| i.as_str())
                    .unwrap_or("PRIMARY");
                let ref_idx = reference_leg_conventions
                    .index
                    .as_ref()
                    .map(|i| i.as_str())
                    .unwrap_or("REFERENCE");
                format!("BASIS|{}|{}|{}", maturity, primary_idx, ref_idx)
            }
        }
    }

    /// Pre-validate that all required curves exist for the quote set.
    pub fn validate_curve_dependencies(
        &self,
        quotes: &[RatesQuote],
        context: &MarketContext,
    ) -> Result<()> {
        let calibrating_forward_id = self.forward_curve_id.as_str();
        let allow_missing_calibrated_curve = self.tenor_years.is_some();

        for quote in quotes {
            if let RatesQuote::BasisSwap {
                primary_leg_conventions,
                reference_leg_conventions,
                ..
            } = quote
            {
                // Get index names from conventions
                let primary_index = primary_leg_conventions.index.as_ref().ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "BasisSwap requires primary_leg_conventions.index".to_string(),
                    )
                })?;
                let reference_index =
                    reference_leg_conventions.index.as_ref().ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "BasisSwap requires reference_leg_conventions.index".to_string(),
                        )
                    })?;

                // Use resolver for consistent curve ID derivation
                let primary_fwd = self.resolve_forward_curve_id(primary_index.as_str());
                let ref_fwd = self.resolve_forward_curve_id(reference_index.as_str());

                let primary_missing = context.get_forward_ref(primary_fwd.as_str()).is_err();
                let reference_missing = context.get_forward_ref(ref_fwd.as_str()).is_err();

                if allow_missing_calibrated_curve {
                    // In forward calibration we allow the calibrated curve to be absent.
                    if primary_missing && primary_fwd.as_str() != calibrating_forward_id {
                        return Err(finstack_core::Error::Input(
                            finstack_core::error::InputError::NotFound {
                                id: format!(
                                    "Forward curve '{}' required for basis swap calibration. \
                                     Please calibrate the forward curve first.",
                                    primary_fwd
                                ),
                            },
                        ));
                    }

                    if reference_missing && ref_fwd.as_str() != calibrating_forward_id {
                        return Err(finstack_core::Error::Input(
                            finstack_core::error::InputError::NotFound {
                                id: format!(
                                    "Forward curve '{}' required for basis swap calibration. \
                                     Please calibrate the forward curve first.",
                                    ref_fwd
                                ),
                            },
                        ));
                    }
                } else {
                    if primary_missing {
                        return Err(finstack_core::Error::Input(
                            finstack_core::error::InputError::NotFound {
                                id: format!(
                                    "Forward curve '{}' required for basis swap calibration. \
                                     Please calibrate the forward curve first.",
                                    primary_fwd
                                ),
                            },
                        ));
                    }
                    if reference_missing {
                        return Err(finstack_core::Error::Input(
                            finstack_core::error::InputError::NotFound {
                                id: format!(
                                    "Forward curve '{}' required for basis swap calibration. \
                                     Please calibrate the forward curve first.",
                                    ref_fwd
                                ),
                            },
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Unified validation for rate quotes with use-case-specific rules.
    pub fn validate_rates_quotes(
        quotes: &[RatesQuote],
        rate_bounds: &crate::calibration::config::RateBounds,
        base_date: Date,
        use_case: RatesQuoteUseCase,
    ) -> Result<()> {
        // 1. Non-empty check
        if quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        // 2. Duplicate detection with instrument-specific keys
        let mut seen = std::collections::HashSet::new();
        for quote in quotes {
            let key = Self::dedupe_key(quote);
            if !seen.insert(key) {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }
        }

        // 3. Per-quote validation (rate bounds, finite check, maturity, use-case constraints)
        // Also accumulate discount-curve "separation" violations for a single warn/error.
        let mut separation_violations: Vec<&'static str> = Vec::new();

        for quote in quotes {
            // Use-case specific: Forward curve does not support Deposit
            if let RatesQuoteUseCase::ForwardCurve = use_case {
                if matches!(quote, RatesQuote::Deposit { .. }) {
                    return Err(finstack_core::Error::Validation(
                        "ForwardCurveCalibrator does not support Deposit quotes (use DiscountCurveCalibrator)".into(),
                    ));
                }
            }

            // Use-case specific: Discount curve checks non-OIS forward-dependent instruments
            // Collect violations here; decide warn vs error below based on enforce_separation
            if let RatesQuoteUseCase::DiscountCurve { .. } = use_case {
                if !quote.is_ois_suitable()
                    && quote.requires_forward_curve()
                    && separation_violations.len() < 5
                {
                    separation_violations.push(quote.get_type());
                }
            }

            // Instrument-specific date sanity
            match quote {
                RatesQuote::FRA { start, end, .. } => {
                    if *start <= base_date {
                        return Err(finstack_core::Error::Calibration {
                            message: format!(
                                "FRA start {} is on or before base date {}",
                                start, base_date
                            ),
                            category: "quote_validation".to_string(),
                        });
                    }
                    if *end <= *start {
                        return Err(finstack_core::Error::Calibration {
                            message: format!("FRA end {} is on or before start {}", end, start),
                            category: "quote_validation".to_string(),
                        });
                    }
                }
                RatesQuote::Future {
                    period_start,
                    period_end,
                    ..
                } => {
                    if *period_start <= base_date {
                        return Err(finstack_core::Error::Calibration {
                            message: format!(
                                "Future period_start {} is on or before base date {}",
                                period_start, base_date
                            ),
                            category: "quote_validation".to_string(),
                        });
                    }
                    if *period_end <= *period_start {
                        return Err(finstack_core::Error::Calibration {
                            message: format!(
                                "Future period_end {} is on or before period_start {}",
                                period_end, period_start
                            ),
                            category: "quote_validation".to_string(),
                        });
                    }
                }
                _ => {}
            }

            if let RatesQuote::Swap {
                is_ois: true,
                float_leg_conventions,
                ..
            } = quote
            {
                match float_leg_conventions.index.as_ref() {
                    Some(index_id) if RateIndexConventions::is_overnight_rfr_index(index_id) => {}
                    Some(index_id) => {
                        return Err(finstack_core::Error::Validation(format!(
                            "Swap flagged as OIS but float leg index '{}' is not a recognized overnight index",
                            index_id
                        )));
                    }
                    None => {
                        return Err(finstack_core::Error::Validation(
                            "Swap quote flagged as OIS requires float_leg_conventions.index".into(),
                        ));
                    }
                }
            }

            // Rate extraction and validation
            let rate = Self::get_rate(quote);
            if !rate.is_finite() {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }
            if !rate_bounds.contains(rate) {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Quote rate {:.4}% outside allowed bounds [{:.2}%, {:.2}%]. \
                        Use `with_rate_bounds()` to adjust bounds for this market regime.",
                        rate * 100.0,
                        rate_bounds.min_rate * 100.0,
                        rate_bounds.max_rate * 100.0
                    ),
                    category: "quote_validation".to_string(),
                });
            }

            // Maturity must be after base date
            let maturity = quote.maturity_date();
            if maturity <= base_date {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Quote maturity {} is on or before base date {}",
                        maturity, base_date
                    ),
                    category: "quote_validation".to_string(),
                });
            }
        }

        // 4. Use-case specific: Discount curve separation enforcement (warn vs error)
        if let RatesQuoteUseCase::DiscountCurve { enforce_separation } = use_case {
            if !separation_violations.is_empty() {
                let examples = separation_violations.join(", ");
                let msg = format!(
                    "Discount curve calibration received {} non-OIS forward-dependent quote(s) \
(e.g. {}). Best practice: use Deposits/OIS swaps for discount curves and calibrate forward curves separately.",
                    separation_violations.len(),
                    examples
                );

                if enforce_separation {
                    return Err(finstack_core::Error::Validation(msg));
                } else {
                    tracing::warn!("{msg}");
                }
            }
        }

        Ok(())
    }
}


