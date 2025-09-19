//! FX Swap metrics (forward points, PV01).
use crate::instruments::fx_swap::FxSwap;
use crate::instruments::traits::Priceable;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::money::fx::FxConversionPolicy;
use finstack_core::F;

/// Forward points (far rate - near rate).
pub struct ForwardPoints;

impl MetricCalculator for ForwardPoints {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fx_swap: &FxSwap = context.instrument_as()?;
        let curves = context.curves.clone();
        let as_of = context.as_of;

        let dc = finstack_core::dates::DayCount::Act365F;
        let t_far = dc
            .year_fraction(
                as_of,
                fx_swap.far_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        let domestic_disc = curves
            .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                fx_swap.domestic_disc_id,
            )?;
        let foreign_disc = curves
            .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                fx_swap.foreign_disc_id,
            )?;

        let df_dom_far = domestic_disc.df(t_far);
        let df_for_far = foreign_disc.df(t_far);

        let fx_matrix = curves.fx.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            })
        })?;
        let near_rate = match fx_swap.near_rate {
            Some(rate) => rate,
            None => {
                (**fx_matrix)
                    .rate(finstack_core::money::fx::FxQuery {
                        from: fx_swap.base_currency,
                        to: fx_swap.quote_currency,
                        on: as_of,
                        policy: FxConversionPolicy::CashflowDate,
                        closure_check: None,
                        want_meta: false,
                    })?
                    .rate
            }
        };

        let far_rate = match fx_swap.far_rate {
            Some(rate) => rate,
            None => near_rate * df_for_far / df_dom_far,
        };

        Ok(far_rate - near_rate)
    }
}

/// Domestic IR01 (sensitivity to 1bp parallel shift in domestic curve).
pub struct DomesticIR01;

impl MetricCalculator for DomesticIR01 {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fx_swap: &FxSwap = context.instrument_as()?;
        let curves = context.curves.clone();
        let as_of = context.as_of;
        let original_pv = fx_swap.value(&curves, as_of)?;

        // Manually re-calculate PV with bumped domestic curve
        let domestic_disc = curves
            .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                fx_swap.domestic_disc_id,
            )?;
        let foreign_disc = curves
            .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                fx_swap.foreign_disc_id,
            )?;

        let dc = finstack_core::dates::DayCount::Act365F;
        let t_near = dc
            .year_fraction(
                as_of,
                fx_swap.near_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let t_far = dc
            .year_fraction(
                as_of,
                fx_swap.far_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        let bump = 0.0001;
        let bumped_df_dom_near = domestic_disc.df(t_near) * (-bump * t_near).exp();
        let bumped_df_dom_far = domestic_disc.df(t_far) * (-bump * t_far).exp();

        let df_for_near = foreign_disc.df(t_near);
        let df_for_far = foreign_disc.df(t_far);

        let fx_matrix = curves.fx.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            })
        })?;
        let near_rate = match fx_swap.near_rate {
            Some(rate) => rate,
            None => {
                (**fx_matrix)
                    .rate(finstack_core::money::fx::FxQuery {
                        from: fx_swap.base_currency,
                        to: fx_swap.quote_currency,
                        on: as_of,
                        policy: FxConversionPolicy::CashflowDate,
                        closure_check: None,
                        want_meta: false,
                    })?
                    .rate
            }
        };

        let far_rate = match fx_swap.far_rate {
            Some(rate) => rate,
            None => near_rate * df_for_far / bumped_df_dom_far,
        };

        let base_amt = fx_swap.base_notional.amount();
        let pv_for_leg = base_amt * df_for_near - base_amt * df_for_far;
        let pv_dom_leg =
            -base_amt * near_rate * bumped_df_dom_near + base_amt * far_rate * bumped_df_dom_far;

        let spot_rate_val = (**fx_matrix)
            .rate(finstack_core::money::fx::FxQuery {
                from: fx_swap.base_currency,
                to: fx_swap.quote_currency,
                on: as_of,
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })?
            .rate;
        let spot_rate = spot_rate_val;

        let bumped_pv = pv_for_leg * spot_rate + pv_dom_leg;

        Ok(bumped_pv - original_pv.amount())
    }
}

/// Foreign IR01 (sensitivity to 1bp parallel shift in foreign curve).
pub struct ForeignIR01;

impl MetricCalculator for ForeignIR01 {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fx_swap: &FxSwap = context.instrument_as()?;
        let curves = context.curves.clone();
        let as_of = context.as_of;
        let original_pv = fx_swap.value(&curves, as_of)?;

        // Manually re-calculate PV with bumped foreign curve
        let domestic_disc = curves
            .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                fx_swap.domestic_disc_id,
            )?;
        let foreign_disc = curves
            .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                fx_swap.foreign_disc_id,
            )?;

        let dc = finstack_core::dates::DayCount::Act365F;
        let t_near = dc
            .year_fraction(
                as_of,
                fx_swap.near_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let t_far = dc
            .year_fraction(
                as_of,
                fx_swap.far_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        let bump = 0.0001;
        let df_dom_near = domestic_disc.df(t_near);
        let df_dom_far = domestic_disc.df(t_far);

        let bumped_df_for_near = foreign_disc.df(t_near) * (-bump * t_near).exp();
        let bumped_df_for_far = foreign_disc.df(t_far) * (-bump * t_far).exp();

        let fx_matrix = curves.fx.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            })
        })?;
        let near_rate = match fx_swap.near_rate {
            Some(rate) => rate,
            None => {
                (**fx_matrix)
                    .rate(finstack_core::money::fx::FxQuery {
                        from: fx_swap.base_currency,
                        to: fx_swap.quote_currency,
                        on: as_of,
                        policy: FxConversionPolicy::CashflowDate,
                        closure_check: None,
                        want_meta: false,
                    })?
                    .rate
            }
        };

        let far_rate = match fx_swap.far_rate {
            Some(rate) => rate,
            None => near_rate * bumped_df_for_far / df_dom_far,
        };

        let base_amt = fx_swap.base_notional.amount();
        let pv_for_leg = base_amt * bumped_df_for_near - base_amt * bumped_df_for_far;
        let pv_dom_leg = -base_amt * near_rate * df_dom_near + base_amt * far_rate * df_dom_far;

        let spot_rate_val = (**fx_matrix)
            .rate(finstack_core::money::fx::FxQuery {
                from: fx_swap.base_currency,
                to: fx_swap.quote_currency,
                on: as_of,
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })?
            .rate;
        let spot_rate = spot_rate_val;

        let bumped_pv = pv_for_leg * spot_rate + pv_dom_leg;

        Ok(bumped_pv - original_pv.amount())
    }
}

/// FX01 (sensitivity to 1bp shift in spot rate).
pub struct FX01;

impl MetricCalculator for FX01 {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fx_swap: &FxSwap = context.instrument_as()?;
        let curves = context.curves.clone();
        let as_of = context.as_of;

        // Calculate original PV
        let original_pv = fx_swap.value(&curves, as_of)?;

        // Manually recalculate PV with bumped spot rate
        let domestic_disc = curves
            .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                fx_swap.domestic_disc_id,
            )?;
        let foreign_disc = curves
            .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                fx_swap.foreign_disc_id,
            )?;

        let dc = finstack_core::dates::DayCount::Act365F;
        let t_near = dc
            .year_fraction(
                as_of,
                fx_swap.near_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let t_far = dc
            .year_fraction(
                as_of,
                fx_swap.far_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        let df_dom_near = domestic_disc.df(t_near);
        let df_dom_far = domestic_disc.df(t_far);
        let df_for_near = foreign_disc.df(t_near);
        let df_for_far = foreign_disc.df(t_far);

        let fx_matrix = curves.fx.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            })
        })?;

        // Get original spot rate
        let spot_rate_val = (**fx_matrix)
            .rate(finstack_core::money::fx::FxQuery {
                from: fx_swap.base_currency,
                to: fx_swap.quote_currency,
                on: as_of,
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })?
            .rate;
        let original_spot = spot_rate_val;

        // Apply 1bp bump to spot rate
        let bump = 0.0001;
        let bumped_spot = original_spot + bump;

        // Recalculate near rate (use bumped spot if not fixed)
        let near_rate = match fx_swap.near_rate {
            Some(rate) => rate,
            None => bumped_spot,
        };

        // Recalculate far rate using bumped spot in the forward calculation
        let far_rate = match fx_swap.far_rate {
            Some(rate) => rate,
            None => bumped_spot * df_for_far / df_dom_far,
        };

        // Calculate PV with bumped rates
        let base_amt = fx_swap.base_notional.amount();
        let pv_for_leg = base_amt * df_for_near - base_amt * df_for_far;
        let pv_dom_leg = -base_amt * near_rate * df_dom_near + base_amt * far_rate * df_dom_far;

        // Convert foreign leg to domestic using bumped spot
        let bumped_pv = pv_for_leg * bumped_spot + pv_dom_leg;

        // Return the difference
        Ok(bumped_pv - original_pv.amount())
    }
}

/// Registers FX Swap metrics
pub fn register_fx_swap_metrics(registry: &mut MetricRegistry) {
    use std::sync::Arc;

    registry
        .register_metric(
            MetricId::custom("forward_points"),
            Arc::new(ForwardPoints),
            &["FxSwap"],
        )
        .register_metric(MetricId::custom("fx01"), Arc::new(FX01), &["FxSwap"])
        .register_metric(
            MetricId::custom("ir01_domestic"),
            Arc::new(DomesticIR01),
            &["FxSwap"],
        )
        .register_metric(
            MetricId::custom("ir01_foreign"),
            Arc::new(ForeignIR01),
            &["FxSwap"],
        );
}
