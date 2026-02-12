//! End-to-end example: statements model -> NOI/CapEx -> RealEstateAsset valuation.
//!
//! This demonstrates how to:
//! - Build a simple property operating statement (rent roll + expenses + capex)
//! - Evaluate the statement model
//! - Export a node series into dated cashflows
//! - Feed those cashflows into `RealEstateAsset` for DCF valuation and metrics

use finstack_core::currency::Currency;
use finstack_core::dates::{DayCount, PeriodId};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_statements::builder::ModelBuilder;
use finstack_statements::evaluator::{node_to_dated_schedule, Evaluator, PeriodDateConvention};
use finstack_statements::templates::real_estate::LeaseSpec;
use finstack_statements::templates::RealEstateExtension;
use finstack_statements::types::AmountOrScalar;
use finstack_valuations::instruments::equity::real_estate::{
    RealEstateAsset, RealEstateValuationMethod,
};
use finstack_valuations::instruments::{Attributes, Instrument};
use finstack_valuations::metrics::MetricId;

fn main() -> finstack_core::Result<()> {
    // -----------------------------
    // 1) Build a statement model
    // -----------------------------
    let leases = vec![
        LeaseSpec {
            node_id: "lease_a_rent".into(),
            start: PeriodId::quarter(2025, 1),
            end: Some(PeriodId::quarter(2026, 4)),
            base_rent: 1_000_000.0,
            growth_rate: 0.01, // +1% per quarter
            free_rent_periods: 0,
            occupancy: 1.0,
        },
        LeaseSpec {
            node_id: "lease_b_rent".into(),
            start: PeriodId::quarter(2025, 3),
            end: Some(PeriodId::quarter(2026, 4)),
            base_rent: 600_000.0,
            growth_rate: 0.005,
            free_rent_periods: 1, // free rent first active quarter
            occupancy: 0.95,
        },
    ];

    let builder = ModelBuilder::new("re_operating_model")
        .periods("2025Q1..2026Q4", None)
        .expect("periods should parse")
        .add_rent_roll_rental_revenue(&leases, "rent_total")
        .expect("rent roll template")
        // A couple of simple expense / capex assumptions.
        .value(
            "opex",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(350_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(360_000.0),
                ),
            ],
        )
        .forecast(
            "opex",
            finstack_statements::types::ForecastSpec::growth(0.01),
        )
        .value(
            "capex",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(50_000.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(50_000.0)),
            ],
        )
        .forecast(
            "capex",
            finstack_statements::types::ForecastSpec::forward_fill(),
        )
        .add_noi_buildup("total_rev", &["rent_total"], "total_exp", &["opex"], "noi")
        .expect("noi template")
        .add_ncf_buildup("noi", &["capex"], "ncf")
        .expect("ncf template")
        .build()
        .expect("build");

    let mut eval = Evaluator::new();
    let results = eval.evaluate(&builder).expect("evaluate");

    // -----------------------------
    // 2) Export nodes into dated cashflows
    // -----------------------------
    let noi_schedule = node_to_dated_schedule(&builder, &results, "noi", PeriodDateConvention::End)
        .expect("noi export");

    let capex_schedule =
        node_to_dated_schedule(&builder, &results, "capex", PeriodDateConvention::End)
            .expect("capex export");

    // -----------------------------
    // 3) Build and value RealEstateAsset
    // -----------------------------
    let as_of = builder.periods.first().expect("periods").start;

    let asset = RealEstateAsset::builder()
        .id(InstrumentId::new("RE-ASSET-STATEMENTS"))
        .currency(Currency::USD)
        .valuation_date(as_of)
        .valuation_method(RealEstateValuationMethod::Dcf)
        .noi_schedule(noi_schedule)
        .capex_schedule_opt(Some(capex_schedule))
        .discount_rate_opt(Some(0.09))
        .terminal_cap_rate_opt(Some(0.065))
        .purchase_price_opt(Some(Money::new(20_000_000.0, Currency::USD)))
        .acquisition_cost_opt(Some(250_000.0))
        .disposition_cost_pct_opt(Some(0.02))
        .day_count(DayCount::Act365F)
        .discount_curve_id(CurveId::new("USD-OIS"))
        .attributes(Attributes::new())
        .build()?;

    let market = MarketContext::new(); // curve-free DCF (discount_rate used)
    let pv = asset.value(&market, as_of)?;

    let metrics = [
        MetricId::custom("real_estate::going_in_cap_rate"),
        MetricId::custom("real_estate::unlevered_irr"),
        MetricId::custom("real_estate::cap_rate_sensitivity"),
        MetricId::custom("real_estate::discount_rate_sensitivity"),
    ];
    let priced = asset.price_with_metrics(&market, as_of, &metrics)?;

    println!("=== Real Estate (Statements -> Valuation) ===");
    println!("PV: {}", pv);
    for m in metrics {
        println!(
            "{} = {}",
            m.as_str(),
            priced.measures.get(&m).copied().unwrap_or(f64::NAN)
        );
    }

    Ok(())
}
