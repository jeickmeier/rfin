//! Foreign exchange shock adapter.
//!
//! This module supports FX shocks through the `OperationSpec::MarketFxPct` variant.
//! The engine applies FX shocks via `MarketBump::FxPct` which wraps the existing
//! provider behind a [`BumpedFxProvider`](finstack_core::money::fx::BumpedFxProvider)
//! so the operation remains deterministic and easy to audit.

use crate::adapters::traits::{ScenarioAdapter, ScenarioEffect};
use crate::engine::ExecutionContext;
use crate::error::Result;
use crate::spec::OperationSpec;
use finstack_core::market_data::bumps::MarketBump;

/// Adapter for FX operations.
pub struct FxAdapter;

fn post_shock_triangulation_warnings(
    market: &finstack_core::market_data::context::MarketContext,
    base: finstack_core::currency::Currency,
    quote: finstack_core::currency::Currency,
    pct: f64,
    as_of: finstack_core::dates::Date,
) -> Result<Vec<String>> {
    let Some(fx) = market.fx() else {
        return Ok(Vec::new());
    };
    let state = fx.get_serializable_state();
    if !state.config.enable_triangulation {
        return Ok(Vec::new());
    }

    let pivot = state.config.pivot_currency;
    let bumped = fx.with_bumped_rate(base, quote, pct / 100.0, as_of)?;
    let mut warnings = Vec::new();

    for &(from, to, _) in &state.quotes {
        if from == to || from == pivot || to == pivot {
            continue;
        }
        if from != base && from != quote && to != base && to != quote {
            continue;
        }

        let direct = bumped
            .rate(finstack_core::money::fx::FxQuery::new(from, to, as_of))
            .map(|r| r.rate);
        let via_pivot = bumped
            .rate(finstack_core::money::fx::FxQuery::new(from, pivot, as_of))
            .and_then(|lhs| {
                bumped
                    .rate(finstack_core::money::fx::FxQuery::new(pivot, to, as_of))
                    .map(|rhs| lhs.rate * rhs.rate)
            });

        let (Ok(direct), Ok(implied)) = (direct, via_pivot) else {
            continue;
        };

        let tolerance = 1.0e-8 * direct.abs().max(implied.abs()).max(1.0);
        if (direct - implied).abs() > tolerance {
            warnings.push(format!(
                "FX shock on {base}/{quote} leaves direct quote {from}/{to} inconsistent with {pivot}-triangulated cross (direct={direct:.10}, implied={implied:.10})"
            ));
        }
    }

    Ok(warnings)
}

impl ScenarioAdapter for FxAdapter {
    fn try_generate_effects(
        &self,
        op: &OperationSpec,
        ctx: &ExecutionContext,
    ) -> Result<Option<Vec<ScenarioEffect>>> {
        match op {
            OperationSpec::MarketFxPct { base, quote, pct } => {
                let bump = MarketBump::FxPct {
                    base: *base,
                    quote: *quote,
                    pct: *pct,
                    as_of: ctx.as_of,
                };
                let mut effects = vec![ScenarioEffect::MarketBump(bump)];
                effects.extend(
                    post_shock_triangulation_warnings(ctx.market, *base, *quote, *pct, ctx.as_of)?
                        .into_iter()
                        .map(ScenarioEffect::Warning),
                );
                Ok(Some(effects))
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::money::fx::{FxConfig, FxConversionPolicy, FxMatrix, FxProvider};
    use std::sync::Arc;
    use time::macros::date;

    struct NullFx;

    impl FxProvider for NullFx {
        fn rate(
            &self,
            from: Currency,
            to: Currency,
            _on: finstack_core::dates::Date,
            _policy: FxConversionPolicy,
        ) -> finstack_core::Result<f64> {
            Err(finstack_core::error::InputError::NotFound {
                id: format!("FX:{from}->{to}"),
            }
            .into())
        }
    }

    #[test]
    fn post_shock_triangulation_check_warns_on_inconsistent_cross() {
        let as_of = date!(2025 - 01 - 01);
        let fx = FxMatrix::try_with_config(
            Arc::new(NullFx),
            FxConfig {
                pivot_currency: Currency::USD,
                enable_triangulation: true,
                cache_capacity: 32,
            },
        )
        .expect("fx config should be valid");
        fx.set_quotes(&[
            (Currency::EUR, Currency::USD, 1.20),
            (Currency::USD, Currency::JPY, 150.0),
            (Currency::EUR, Currency::JPY, 180.0),
        ])
        .expect("quotes should seed");
        let market = MarketContext::new().insert_fx(fx);

        let warnings =
            post_shock_triangulation_warnings(&market, Currency::EUR, Currency::USD, 10.0, as_of)
                .expect("warning generation should succeed");

        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("EUR/JPY")
                    && warning.contains("implied=198.0000000000")),
            "expected direct cross inconsistency warning, got {warnings:?}"
        );
    }

    #[test]
    fn post_shock_triangulation_check_accepts_valid_negative_percent_shock() {
        let as_of = date!(2025 - 01 - 01);
        let fx = FxMatrix::try_with_config(
            Arc::new(NullFx),
            FxConfig {
                pivot_currency: Currency::USD,
                enable_triangulation: true,
                cache_capacity: 32,
            },
        )
        .expect("fx config should be valid");
        fx.set_quotes(&[
            (Currency::EUR, Currency::USD, 1.20),
            (Currency::USD, Currency::JPY, 150.0),
            (Currency::EUR, Currency::JPY, 180.0),
        ])
        .expect("quotes should seed");
        let market = MarketContext::new().insert_fx(fx);

        let warnings =
            post_shock_triangulation_warnings(&market, Currency::EUR, Currency::USD, -5.0, as_of)
                .expect("valid -5% shock should not fail preview");

        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("EUR/JPY")
                    && warning.contains("implied=171.0000000000")),
            "expected post-shock state from a -5% bump, got {warnings:?}"
        );
    }
}
