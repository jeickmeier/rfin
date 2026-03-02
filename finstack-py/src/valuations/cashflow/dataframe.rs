use finstack_valuations::cashflow::builder::PeriodDataFrame;
use polars::prelude::*;
use pyo3::prelude::*;

/// Convert a Rust `PeriodDataFrame` into a Polars `PyDataFrame`.
///
/// This is the single source of truth for the PeriodDataFrame -> Polars conversion,
/// shared by `CashFlowSchedule.to_dataframe()` and `Bond.pricing_cashflows()`.
pub(crate) fn period_dataframe_to_polars(
    frame: PeriodDataFrame,
) -> PyResult<pyo3_polars::PyDataFrame> {
    let epoch = time::Date::from_calendar_date(1970, time::Month::January, 1).map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to construct epoch date: {e}"))
    })?;

    let start_dates: Vec<i32> = frame
        .start_dates
        .iter()
        .map(|d| (*d - epoch).whole_days() as i32)
        .collect();
    let end_dates: Vec<i32> = frame
        .end_dates
        .iter()
        .map(|d| (*d - epoch).whole_days() as i32)
        .collect();
    let pay_dates: Vec<i32> = frame
        .pay_dates
        .iter()
        .map(|d| (*d - epoch).whole_days() as i32)
        .collect();

    let kinds: Vec<String> = frame
        .cf_types
        .iter()
        .map(|k| {
            crate::core::cashflow::primitives::PyCFKind::new(*k)
                .name()
                .to_string()
        })
        .collect();

    let reset_dates: Vec<Option<i32>> = frame
        .reset_dates
        .iter()
        .map(|opt_d| opt_d.map(|d| (d - epoch).whole_days() as i32))
        .collect();

    let notionals_f64: Vec<f64> = frame
        .notionals
        .iter()
        .map(|opt| opt.unwrap_or(f64::NAN))
        .collect();

    let polars_err = |e: PolarsError| pyo3::exceptions::PyRuntimeError::new_err(e.to_string());

    let start_dates = Series::new("start_date".into(), start_dates)
        .cast(&DataType::Date)
        .map_err(polars_err)?;
    let end_dates = Series::new("end_date".into(), end_dates)
        .cast(&DataType::Date)
        .map_err(polars_err)?;
    let pay_dates = Series::new("pay_date".into(), pay_dates)
        .cast(&DataType::Date)
        .map_err(polars_err)?;
    let reset_dates = Series::new("reset_date".into(), reset_dates)
        .cast(&DataType::Date)
        .map_err(polars_err)?;

    let mut df = DataFrame::new_infer_height(vec![
        start_dates.into(),
        end_dates.into(),
        pay_dates.into(),
        reset_dates.into(),
        Series::new("kind".into(), kinds).into(),
        Series::new("amount".into(), frame.amounts).into(),
        Series::new("accrual_factor".into(), frame.accrual_factors).into(),
        Series::new("rate".into(), frame.rates).into(),
        Series::new("notional".into(), notionals_f64).into(),
        Series::new("yr_fraq".into(), frame.yr_fraqs).into(),
        Series::new("discount_factor".into(), frame.discount_factors).into(),
        Series::new("pv".into(), frame.pvs).into(),
    ])
    .map_err(polars_err)?;

    if let Some(undrawn) = frame.undrawn_notionals {
        let undrawn_f64: Vec<f64> = undrawn.iter().map(|opt| opt.unwrap_or(f64::NAN)).collect();
        let col = Series::new("undrawn_notional".into(), undrawn_f64);
        df.with_column(col.into()).map_err(polars_err)?;
    }

    if let Some(sp) = frame.survival_probs {
        let sp_f64: Vec<f64> = sp.iter().map(|opt| opt.unwrap_or(f64::NAN)).collect();
        let col = Series::new("survival_prob".into(), sp_f64);
        df.with_column(col.into()).map_err(polars_err)?;
    }

    Ok(pyo3_polars::PyDataFrame(df))
}
