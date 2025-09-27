use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyDate, PyDateAccess, PyDateTime};
use pyo3::Bound;
use time::{Date, Month};

pub(crate) fn py_to_date(value: &Bound<'_, PyAny>) -> PyResult<Date> {
    if let Ok(date) = value.downcast::<PyDate>() {
        return build_date(
            date.get_year(),
            date.get_month() as u8,
            date.get_day() as u8,
        );
    }

    if let Ok(dt) = value.downcast::<PyDateTime>() {
        return build_date(dt.get_year(), dt.get_month() as u8, dt.get_day() as u8);
    }

    Err(PyTypeError::new_err(
        "Expected datetime.date or datetime.datetime",
    ))
}

pub(crate) fn date_to_py(py: Python<'_>, date: Date) -> PyResult<PyObject> {
    PyDate::new(py, date.year(), u8::from(date.month()), date.day())
        .map(|obj| obj.into())
        .map_err(|err| PyValueError::new_err(err.to_string()))
}

fn build_date(year: i32, month: u8, day: u8) -> PyResult<Date> {
    let month = Month::try_from(month)
        .map_err(|_| PyValueError::new_err(format!("Month out of range: {month}")))?;
    Date::from_calendar_date(year, month, day).map_err(|err| PyValueError::new_err(err.to_string()))
}
