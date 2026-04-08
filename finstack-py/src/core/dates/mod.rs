pub mod calendar;
pub mod daycount;
pub mod fx;
pub mod imm;
pub mod periods;
pub mod rate_conversions;
pub mod schedule;
pub mod tenor;
pub mod utils;

#[allow(unused_imports)]
pub use calendar::{PyBusinessDayConvention, PyCalendar, PyCompositeCalendar};
#[allow(unused_imports)]
pub use daycount::{PyDayCount, PyDayCountContext, PyThirty360Convention};
#[allow(unused_imports)]
pub use periods::{PyFiscalConfig, PyPeriod, PyPeriodId, PyPeriodPlan};
#[allow(unused_imports)]
pub use schedule::{
    PyFrequency, PySchedule, PyScheduleBuilder, PyScheduleErrorPolicy, PyScheduleWarning,
    PyStubKind,
};
#[allow(unused_imports)]
pub use tenor::{PyTenor, PyTenorUnit};

use super::common::reexport::promote_exports;
use finstack_core::HashSet;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "dates")?;
    module.setattr(
        "__doc__",
        "Business-day calendars, day-counts, schedules, IMM helpers, periods, and utilities.",
    )?;

    let mut exports: Vec<&str> = Vec::new();

    let calendar_exports = calendar::register(py, &module)?;
    exports.extend(calendar_exports.iter().copied());

    let daycount_exports = daycount::register(py, &module)?;
    exports.extend(daycount_exports.iter().copied());
    promote_exports(&module, "daycount", &daycount_exports)?;

    let schedule_exports = schedule::register(py, &module)?;
    exports.extend(schedule_exports.iter().copied());
    promote_exports(&module, "schedule", &schedule_exports)?;

    let periods_exports = periods::register(py, &module)?;
    exports.extend(periods_exports.iter().copied());
    promote_exports(&module, "periods", &periods_exports)?;

    let imm_exports = imm::register(py, &module)?;
    exports.extend(imm_exports.iter().copied());
    promote_exports(&module, "imm", &imm_exports)?;

    let tenor_exports = tenor::register(py, &module)?;
    exports.extend(tenor_exports.iter().copied());
    promote_exports(&module, "tenor", &tenor_exports)?;

    let utils_exports = utils::register(py, &module)?;
    exports.extend(utils_exports.iter().copied());
    promote_exports(&module, "utils", &utils_exports)?;

    let rate_exports = rate_conversions::register(py, &module)?;
    exports.extend(rate_exports.iter().copied());
    promote_exports(&module, "rate_conversions", &rate_exports)?;

    let fx_exports = fx::register(py, &module)?;
    exports.extend(fx_exports.iter().copied());
    promote_exports(&module, "fx", &fx_exports)?;

    let mut uniq = HashSet::default();
    exports.retain(|item| uniq.insert(*item));
    exports.sort_unstable();
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;

    Ok(())
}
