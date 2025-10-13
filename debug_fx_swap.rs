// Debug script to understand FX swap pricing
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::fx::providers::SimpleFxProvider;
use finstack_core::money::fx::FxMatrix;
use finstack_core::prelude::*;
use finstack_valuations::instruments::fx_swap::FxSwap;
use std::sync::Arc;
use time::macros::date;

fn main() -> finstack_core::Result<()> {
    println!("=== FX Swap Debug ===\n");

    let as_of = date!(2024 - 01 - 01);
    let market = build_market_data(as_of);

    // Create FX swap
    let fx_swap = FxSwap::builder()
        .id("FX_SWAP_EURUSD".into())
        .base_notional(Money::new(50_000_000.0, Currency::EUR))
        .base_currency(Currency::EUR)
        .quote_currency(Currency::USD)
        .near_date(date!(2024 - 02 - 01))  // 1 month from as_of
        .far_date(date!(2024 - 08 - 01))   // 7 months from as_of
        .domestic_disc_id("USD".into())
        .foreign_disc_id("EUR".into())
        .build()
        .unwrap();

    println!("FX Swap:");
    println!("  Base notional: {}", fx_swap.base_notional);
    println!("  Near date: {}", fx_swap.near_date);
    println!("  Far date: {}", fx_swap.far_date);
    println!("  Base currency: {}", fx_swap.base_currency);
    println!("  Quote currency: {}", fx_swap.quote_currency);
    println!("  Domestic disc ID: {}", fx_swap.domestic_disc_id);
    println!("  Foreign disc ID: {}", fx_swap.foreign_disc_id);
    println!();

    // Get curves
    let domestic_disc = market.get_discount_ref("USD")?;
    let foreign_disc = market.get_discount_ref("EUR")?;

    println!("Curves:");
    println!("  USD base date: {}", domestic_disc.base_date());
    println!("  EUR base date: {}", foreign_disc.base_date());
    println!();

    // Calculate discount factors manually
    let dom_dc = domestic_disc.day_count();
    let for_dc = foreign_disc.day_count();
    
    let t_as_of_dom = dom_dc.year_fraction(domestic_disc.base_date(), as_of, finstack_core::dates::DayCountCtx::default()).unwrap_or(0.0);
    let t_as_of_for = for_dc.year_fraction(foreign_disc.base_date(), as_of, finstack_core::dates::DayCountCtx::default()).unwrap_or(0.0);
    
    let df_as_of_dom = domestic_disc.df(t_as_of_dom);
    let df_as_of_for = foreign_disc.df(t_as_of_for);
    
    let t_near_dom = dom_dc.year_fraction(domestic_disc.base_date(), fx_swap.near_date, finstack_core::dates::DayCountCtx::default()).unwrap_or(0.0);
    let t_far_dom = dom_dc.year_fraction(domestic_disc.base_date(), fx_swap.far_date, finstack_core::dates::DayCountCtx::default()).unwrap_or(0.0);
    let t_far_for = for_dc.year_fraction(foreign_disc.base_date(), fx_swap.far_date, finstack_core::dates::DayCountCtx::default()).unwrap_or(0.0);
    
    let df_dom_near = if df_as_of_dom != 0.0 { domestic_disc.df(t_near_dom) / df_as_of_dom } else { 1.0 };
    let df_dom_far = if df_as_of_dom != 0.0 { domestic_disc.df(t_far_dom) / df_as_of_dom } else { 1.0 };
    let df_for_far = if df_as_of_for != 0.0 { foreign_disc.df(t_far_for) / df_as_of_for } else { 1.0 };

    println!("Time calculations:");
    println!("  t_as_of_dom: {:.6}", t_as_of_dom);
    println!("  t_as_of_for: {:.6}", t_as_of_for);
    println!("  t_near_dom: {:.6}", t_near_dom);
    println!("  t_far_dom: {:.6}", t_far_dom);
    println!("  t_far_for: {:.6}", t_far_for);
    println!();

    println!("Discount factors:");
    println!("  df_as_of_dom: {:.6}", df_as_of_dom);
    println!("  df_as_of_for: {:.6}", df_as_of_for);
    println!("  df_dom_near: {:.6}", df_dom_near);
    println!("  df_dom_far: {:.6}", df_dom_far);
    println!("  df_for_far: {:.6}", df_for_far);
    println!();

    // Get FX rates
    let fx_matrix = market.fx.as_ref().unwrap();
    let model_spot = (**fx_matrix)
        .rate(finstack_core::money::fx::FxQuery::new(fx_swap.base_currency, fx_swap.quote_currency, as_of))?
        .rate;

    let model_fwd = model_spot * df_for_far / df_dom_far;

    println!("FX rates:");
    println!("  Model spot: {:.6}", model_spot);
    println!("  Model forward: {:.6}", model_fwd);
    println!();

    // Calculate PV components
    let n_base = fx_swap.base_notional.amount();
    let pv_foreign_dom = n_base * model_spot * df_dom_near - n_base * model_fwd * df_dom_far;
    let pv_dom_leg = -n_base * model_spot * df_dom_near + n_base * model_fwd * df_dom_far;
    let total_pv = pv_foreign_dom + pv_dom_leg;

    println!("PV calculation:");
    println!("  n_base: {:.2}", n_base);
    println!("  pv_foreign_dom: {:.2}", pv_foreign_dom);
    println!("  pv_dom_leg: {:.2}", pv_dom_leg);
    println!("  total_pv: {:.2}", total_pv);
    println!();

    // Test the actual npv method
    let actual_pv = fx_swap.npv(&market, as_of)?;
    println!("Actual NPV: {}", actual_pv);

    Ok(())
}

fn build_market_data(as_of: Date) -> MarketContext {
    // Create USD discount curve with realistic rates (~5% rate)
    let usd_curve = DiscountCurve::builder("USD")
        .base_date(as_of)
        .knots(vec![
            (0.0, 1.0),
            (1.0/365.0, 0.999863),  // 1 day: ~5% rate
            (7.0/365.0, 0.999042),  // 1 week
            (30.0/365.0, 0.995890), // 1 month
            (0.25, 0.9875),         // 3 months: ~5% rate
            (0.5, 0.975),           // 6 months
            (1.0, 0.95),            // 1 year: ~5.13% rate
            (2.0, 0.90),            // 2 years
            (5.0, 0.80),            // 5 years
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    // Create EUR discount curve with realistic rates (~3% rate)
    let eur_curve = DiscountCurve::builder("EUR")
        .base_date(as_of)
        .knots(vec![
            (0.0, 1.0),
            (1.0/365.0, 0.999918),  // 1 day: ~3% rate
            (7.0/365.0, 0.999426),  // 1 week
            (30.0/365.0, 0.997534), // 1 month
            (0.25, 0.9925),         // 3 months: ~3% rate
            (0.5, 0.985),           // 6 months
            (1.0, 0.97),            // 1 year: ~3.05% rate
            (2.0, 0.94),            // 2 years
            (5.0, 0.86),            // 5 years
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    // Create FX matrix
    let fx_provider = SimpleFxProvider::new();
    fx_provider.set_quote(Currency::EUR, Currency::USD, 1.1);
    fx_provider.set_quote(Currency::GBP, Currency::USD, 1.25);
    let fx_matrix = FxMatrix::new(Arc::new(fx_provider));

    MarketContext::new()
        .insert_discount(usd_curve)
        .insert_discount(eur_curve)
        .insert_fx(fx_matrix)
}
