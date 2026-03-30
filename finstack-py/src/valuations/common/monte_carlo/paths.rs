//! Python bindings for Monte Carlo path data structures.

use finstack_core::HashMap;
use finstack_monte_carlo::paths::{CashflowType, PathDataset, PathPoint, SimulatedPath};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;

/// Helper function to convert CashflowType to string.
fn cashflow_type_to_string(cf_type: CashflowType) -> &'static str {
    match cf_type {
        CashflowType::Principal => "Principal",
        CashflowType::Interest => "Interest",
        CashflowType::CommitmentFee => "CommitmentFee",
        CashflowType::UsageFee => "UsageFee",
        CashflowType::FacilityFee => "FacilityFee",
        CashflowType::UpfrontFee => "UpfrontFee",
        CashflowType::Recovery => "Recovery",
        CashflowType::MarkToMarket => "MarkToMarket",
        CashflowType::Other => "Other",
    }
}

/// Type of cashflow for categorization.
#[pyclass(
    module = "finstack.valuations.common.monte_carlo",
    name = "CashflowType",
    from_py_object
)]
#[derive(Clone, Copy)]
pub enum PyCashflowType {
    /// Principal deployment or repayment
    Principal,
    /// Interest payment
    Interest,
    /// Commitment fee
    CommitmentFee,
    /// Usage fee
    UsageFee,
    /// Facility fee
    FacilityFee,
    /// Upfront fee
    UpfrontFee,
    /// Recovery proceeds
    Recovery,
    /// Mark-to-market P&L
    MarkToMarket,
    /// Other/generic cashflow
    Other,
}

impl From<CashflowType> for PyCashflowType {
    fn from(ct: CashflowType) -> Self {
        match ct {
            CashflowType::Principal => PyCashflowType::Principal,
            CashflowType::Interest => PyCashflowType::Interest,
            CashflowType::CommitmentFee => PyCashflowType::CommitmentFee,
            CashflowType::UsageFee => PyCashflowType::UsageFee,
            CashflowType::FacilityFee => PyCashflowType::FacilityFee,
            CashflowType::UpfrontFee => PyCashflowType::UpfrontFee,
            CashflowType::Recovery => PyCashflowType::Recovery,
            CashflowType::MarkToMarket => PyCashflowType::MarkToMarket,
            CashflowType::Other => PyCashflowType::Other,
        }
    }
}

#[pymethods]
impl PyCashflowType {
    fn __repr__(&self) -> &'static str {
        match self {
            PyCashflowType::Principal => "CashflowType.Principal",
            PyCashflowType::Interest => "CashflowType.Interest",
            PyCashflowType::CommitmentFee => "CashflowType.CommitmentFee",
            PyCashflowType::UsageFee => "CashflowType.UsageFee",
            PyCashflowType::FacilityFee => "CashflowType.FacilityFee",
            PyCashflowType::UpfrontFee => "CashflowType.UpfrontFee",
            PyCashflowType::Recovery => "CashflowType.Recovery",
            PyCashflowType::MarkToMarket => "CashflowType.MarkToMarket",
            PyCashflowType::Other => "CashflowType.Other",
        }
    }
}

/// A single point along a Monte Carlo path.
#[pyclass(
    module = "finstack.valuations.common.monte_carlo",
    name = "PathPoint",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPathPoint {
    pub(crate) inner: PathPoint,
}

#[pymethods]
impl PyPathPoint {
    /// Get the step index.
    #[getter]
    fn step(&self) -> usize {
        self.inner.step
    }

    /// Get the time in years.
    #[getter]
    fn time(&self) -> f64 {
        self.inner.time
    }

    /// Get state variables as a dictionary.
    #[getter]
    fn state_vars(&self, py: Python) -> PyResult<Py<PyDict>> {
        use finstack_monte_carlo::paths::state_indices;
        let dict = PyDict::new(py);

        // Map state vector indices to named keys
        if let Some(spot) = self.inner.state.get(state_indices::IDX_SPOT) {
            dict.set_item("spot", spot)?;
        }
        if let Some(variance) = self.inner.state.get(state_indices::IDX_VARIANCE) {
            dict.set_item("variance", variance)?;
        }
        if let Some(credit_spread) = self.inner.state.get(state_indices::IDX_CREDIT_SPREAD) {
            dict.set_item("credit_spread", credit_spread)?;
        }

        Ok(dict.into())
    }

    /// Get the payoff value at this point (if captured).
    #[getter]
    fn payoff_value(&self) -> Option<f64> {
        self.inner.payoff_value
    }

    /// Get a specific state variable by name.
    fn get_var(&self, key: &str) -> Option<f64> {
        use finstack_monte_carlo::paths::state_indices;
        match key {
            "spot" => self.inner.state.get(state_indices::IDX_SPOT).copied(),
            "variance" => self.inner.state.get(state_indices::IDX_VARIANCE).copied(),
            "credit_spread" => self
                .inner
                .state
                .get(state_indices::IDX_CREDIT_SPREAD)
                .copied(),
            _ => None,
        }
    }

    /// Get the spot price (convenience method).
    fn spot(&self) -> Option<f64> {
        self.inner.spot()
    }

    /// Get the variance (convenience method).
    fn variance(&self) -> Option<f64> {
        self.inner.variance()
    }

    /// Get the short rate (convenience method).
    fn short_rate(&self) -> Option<f64> {
        self.inner.short_rate()
    }

    /// Get cashflows generated at this timestep.
    ///
    /// Returns:
    ///     list[tuple[float, float, CashflowType]]: List of (time, amount, type) tuples.
    ///     For revolving credit: interest, fees, principal changes.
    #[getter]
    fn cashflows(&self) -> Vec<(f64, f64, PyCashflowType)> {
        self.inner
            .cashflows
            .iter()
            .map(|(time, amount, cf_type)| (*time, *amount, PyCashflowType::from(*cf_type)))
            .collect()
    }

    /// Get cashflows by type.
    ///
    /// Args:
    ///     cf_type: CashflowType to filter by
    ///
    /// Returns:
    ///     list[tuple[float, float]]: List of (time, amount) pairs matching the type
    fn get_cashflows_by_type(&self, cf_type: PyCashflowType) -> Vec<(f64, f64)> {
        let rust_type = match cf_type {
            PyCashflowType::Principal => CashflowType::Principal,
            PyCashflowType::Interest => CashflowType::Interest,
            PyCashflowType::CommitmentFee => CashflowType::CommitmentFee,
            PyCashflowType::UsageFee => CashflowType::UsageFee,
            PyCashflowType::FacilityFee => CashflowType::FacilityFee,
            PyCashflowType::UpfrontFee => CashflowType::UpfrontFee,
            PyCashflowType::Recovery => CashflowType::Recovery,
            PyCashflowType::MarkToMarket => CashflowType::MarkToMarket,
            PyCashflowType::Other => CashflowType::Other,
        };
        self.inner.get_cashflows_by_type(rust_type)
    }

    /// Get principal flows (convenience method).
    fn principal_flows(&self) -> Vec<(f64, f64)> {
        self.inner.principal_flows()
    }

    /// Get interest flows (convenience method).
    fn interest_flows(&self) -> Vec<(f64, f64)> {
        self.inner.interest_flows()
    }

    /// Get total cashflow amount at this timestep.
    ///
    /// Returns:
    ///     float: Sum of all cashflows at this timestep
    fn total_cashflow(&self) -> f64 {
        self.inner.total_cashflow()
    }

    /// Get total cashflow by type.
    fn total_cashflow_by_type(&self, cf_type: PyCashflowType) -> f64 {
        let rust_type = match cf_type {
            PyCashflowType::Principal => CashflowType::Principal,
            PyCashflowType::Interest => CashflowType::Interest,
            PyCashflowType::CommitmentFee => CashflowType::CommitmentFee,
            PyCashflowType::UsageFee => CashflowType::UsageFee,
            PyCashflowType::FacilityFee => CashflowType::FacilityFee,
            PyCashflowType::UpfrontFee => CashflowType::UpfrontFee,
            PyCashflowType::Recovery => CashflowType::Recovery,
            PyCashflowType::MarkToMarket => CashflowType::MarkToMarket,
            PyCashflowType::Other => CashflowType::Other,
        };
        self.inner.total_cashflow_by_type(rust_type)
    }

    /// Convert cashflows to a pandas DataFrame.
    ///
    /// Returns:
    ///     pd.DataFrame: DataFrame with columns:
    ///         - step: timestep index
    ///         - time_years: time in years
    ///         - amount: cashflow amount
    ///         - cashflow_type: type of cashflow as string
    fn to_dataframe(&self, py: Python) -> PyResult<Py<PyAny>> {
        let pd = py.import("pandas")?;
        let dict = PyDict::new(py);

        let mut steps = Vec::new();
        let mut times = Vec::new();
        let mut amounts = Vec::new();
        let mut types = Vec::new();

        for (time, amount, cf_type) in &self.inner.cashflows {
            steps.push(self.inner.step);
            times.push(*time);
            amounts.push(*amount);
            types.push(cashflow_type_to_string(*cf_type));
        }

        dict.set_item("step", steps)?;
        dict.set_item("time_years", times)?;
        dict.set_item("amount", amounts)?;
        dict.set_item("cashflow_type", types)?;

        pd.call_method1("DataFrame", (dict,))?
            .extract()
            .map_err(Into::into)
    }

    fn __repr__(&self) -> String {
        format!(
            "PathPoint(step={}, time={:.4}, vars={})",
            self.inner.step,
            self.inner.time,
            self.inner.state.len()
        )
    }
}

/// A complete simulated Monte Carlo path.
#[pyclass(
    module = "finstack.valuations.common.monte_carlo",
    name = "SimulatedPath",
    from_py_object
)]
#[derive(Clone)]
pub struct PySimulatedPath {
    pub(crate) inner: SimulatedPath,
}

#[pymethods]
impl PySimulatedPath {
    /// Get the path ID.
    #[getter]
    fn path_id(&self) -> usize {
        self.inner.path_id
    }

    /// Get all points along the path.
    #[getter]
    fn points(&self) -> Vec<PyPathPoint> {
        self.inner
            .points
            .iter()
            .map(|p| PyPathPoint { inner: p.clone() })
            .collect()
    }

    /// Get the final discounted payoff value.
    #[getter]
    fn final_value(&self) -> f64 {
        self.inner.final_value
    }

    /// Get the IRR for this path (if calculated).
    ///
    /// Returns:
    ///     float | None: Internal Rate of Return as annualized decimal (e.g., 0.08 for 8%),
    ///                   or None if IRR wasn't calculated or doesn't exist
    #[getter]
    fn irr(&self) -> Option<f64> {
        self.inner.irr
    }

    /// Get the number of time steps.
    fn num_steps(&self) -> usize {
        self.inner.num_steps()
    }

    /// Get a specific point by step index.
    fn point(&self, step: usize) -> Option<PyPathPoint> {
        self.inner
            .point(step)
            .map(|p| PyPathPoint { inner: p.clone() })
    }

    /// Get the initial point.
    fn initial_point(&self) -> Option<PyPathPoint> {
        self.inner
            .initial_point()
            .map(|p| PyPathPoint { inner: p.clone() })
    }

    /// Get the terminal point.
    fn terminal_point(&self) -> Option<PyPathPoint> {
        self.inner
            .terminal_point()
            .map(|p| PyPathPoint { inner: p.clone() })
    }

    /// Extract all cashflows from the path.
    ///
    /// Returns:
    ///     list[tuple[float, float]]: All (time_years, amount) cashflow pairs across all timesteps
    fn extract_cashflows(&self) -> Vec<(f64, f64)> {
        self.inner.extract_cashflows()
    }

    /// Extract typed cashflows from the path.
    ///
    /// Returns:
    ///     list[tuple[float, float, CashflowType]]: All (time, amount, type) tuples across all timesteps
    fn extract_typed_cashflows(&self) -> Vec<(f64, f64, PyCashflowType)> {
        self.inner
            .extract_typed_cashflows()
            .iter()
            .map(|(time, amount, cf_type)| (*time, *amount, PyCashflowType::from(*cf_type)))
            .collect()
    }

    /// Extract cashflows by type.
    ///
    /// Args:
    ///     cf_type: CashflowType to filter by
    ///
    /// Returns:
    ///     list[tuple[float, float]]: All (time, amount) pairs matching the type
    fn extract_cashflows_by_type(&self, cf_type: PyCashflowType) -> Vec<(f64, f64)> {
        let rust_type = match cf_type {
            PyCashflowType::Principal => CashflowType::Principal,
            PyCashflowType::Interest => CashflowType::Interest,
            PyCashflowType::CommitmentFee => CashflowType::CommitmentFee,
            PyCashflowType::UsageFee => CashflowType::UsageFee,
            PyCashflowType::FacilityFee => CashflowType::FacilityFee,
            PyCashflowType::UpfrontFee => CashflowType::UpfrontFee,
            PyCashflowType::Recovery => CashflowType::Recovery,
            PyCashflowType::MarkToMarket => CashflowType::MarkToMarket,
            PyCashflowType::Other => CashflowType::Other,
        };
        self.inner.extract_cashflows_by_type(rust_type)
    }

    /// Get cashflows with calendar dates.
    ///
    /// Args:
    ///     base_date: Commitment/start date of the facility
    ///
    /// Returns:
    ///     list[tuple[datetime.date, float]]: Cashflows with calendar dates suitable for XIRR
    fn get_cashflows_with_dates(
        &self,
        py: Python,
        base_date: Bound<'_, PyAny>,
    ) -> PyResult<Vec<(Py<PyAny>, f64)>> {
        use crate::core::dates::utils::{date_to_py, py_to_date};

        let base = py_to_date(&base_date)?;
        let mut result = Vec::new();

        for point in &self.inner.points {
            for (time_years, amount, _cf_type) in &point.cashflows {
                // Convert year fraction to date (approximate using 365.25 days/year)
                let days_offset = (time_years * 365.25) as i64;
                let cf_date = base + time::Duration::days(days_offset);
                let py_date = date_to_py(py, cf_date)?;
                result.push((py_date, *amount));
            }
        }

        Ok(result)
    }

    /// Convert all cashflows from the path to a pandas DataFrame.
    ///
    /// Returns:
    ///     pd.DataFrame: DataFrame with columns:
    ///         - path_id: path identifier
    ///         - step: timestep index
    ///         - time_years: time in years
    ///         - amount: cashflow amount
    ///         - cashflow_type: type of cashflow as string
    fn to_dataframe(&self, py: Python) -> PyResult<Py<PyAny>> {
        let pd = py.import("pandas")?;
        let dict = PyDict::new(py);

        let mut path_ids = Vec::new();
        let mut steps = Vec::new();
        let mut times = Vec::new();
        let mut amounts = Vec::new();
        let mut types = Vec::new();

        for point in &self.inner.points {
            for (time, amount, cf_type) in &point.cashflows {
                path_ids.push(self.inner.path_id);
                steps.push(point.step);
                times.push(*time);
                amounts.push(*amount);
                types.push(cashflow_type_to_string(*cf_type));
            }
        }

        dict.set_item("path_id", path_ids)?;
        dict.set_item("step", steps)?;
        dict.set_item("time_years", times)?;
        dict.set_item("amount", amounts)?;
        dict.set_item("cashflow_type", types)?;

        pd.call_method1("DataFrame", (dict,))?
            .extract()
            .map_err(Into::into)
    }

    fn __repr__(&self) -> String {
        format!(
            "SimulatedPath(id={}, steps={}, final_value={:.4})",
            self.inner.path_id,
            self.inner.num_steps(),
            self.inner.final_value
        )
    }

    fn __len__(&self) -> usize {
        self.inner.num_steps()
    }
}

/// Collection of simulated paths with metadata.
#[pyclass(
    module = "finstack.valuations.common.monte_carlo",
    name = "PathDataset",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPathDataset {
    pub(crate) inner: Arc<PathDataset>,
}

#[pymethods]
impl PyPathDataset {
    /// Get all captured paths.
    #[getter]
    fn paths(&self) -> Vec<PySimulatedPath> {
        self.inner
            .paths
            .iter()
            .map(|p| PySimulatedPath { inner: p.clone() })
            .collect()
    }

    /// Get the total number of paths in the simulation.
    #[getter]
    fn num_paths_total(&self) -> usize {
        self.inner.num_paths_total
    }

    /// Get the sampling method used.
    #[getter]
    fn sampling_method(&self) -> String {
        self.inner.sampling_method.to_string()
    }

    /// Get the number of captured paths.
    fn num_captured(&self) -> usize {
        self.inner.num_captured()
    }

    /// Get a specific path by index.
    fn path(&self, index: usize) -> Option<PySimulatedPath> {
        self.inner
            .path(index)
            .map(|p| PySimulatedPath { inner: p.clone() })
    }

    /// Check if all paths were captured.
    fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }

    /// Get the sampling ratio (captured / total).
    fn sampling_ratio(&self) -> f64 {
        self.inner.sampling_ratio()
    }

    /// Get all state variable keys present in the dataset.
    fn state_var_keys(&self) -> Vec<String> {
        self.inner.state_var_keys()
    }

    /// Convert to a long-format dictionary suitable for pandas DataFrame.
    ///
    /// Returns a dictionary with columns:
    /// - path_id: Path identifier
    /// - step: Time step index
    /// - time: Time in years
    /// - final_value: Final discounted payoff for this path
    /// - One column per state variable (e.g., 'spot', 'variance')
    /// - payoff_value: Optional payoff at each step (if captured)
    fn to_dict(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);

        // Pre-allocate vectors
        let total_points: usize = self.inner.paths.iter().map(|p| p.points.len()).sum();

        let mut path_ids = Vec::with_capacity(total_points);
        let mut steps = Vec::with_capacity(total_points);
        let mut times = Vec::with_capacity(total_points);
        let mut final_values = Vec::with_capacity(total_points);
        let mut payoff_values = Vec::with_capacity(total_points);

        // Collect all state variable keys
        let state_keys = self.inner.state_var_keys();
        let mut state_columns: HashMap<String, Vec<Option<f64>>> = state_keys
            .iter()
            .map(|k| (k.clone(), Vec::with_capacity(total_points)))
            .collect();

        // Iterate through all paths and points
        for path in &self.inner.paths {
            for point in &path.points {
                path_ids.push(path.path_id);
                steps.push(point.step);
                times.push(point.time);
                final_values.push(path.final_value);
                payoff_values.push(point.payoff_value);

                // Add state variables
                for (idx, key) in state_keys.iter().enumerate() {
                    if let Some(col) = state_columns.get_mut(key) {
                        col.push(point.state.get(idx).copied());
                    }
                }
            }
        }

        // Add to dictionary
        dict.set_item("path_id", path_ids)?;
        dict.set_item("step", steps)?;
        dict.set_item("time", times)?;
        dict.set_item("final_value", final_values)?;
        dict.set_item("payoff_value", payoff_values)?;

        for (key, values) in state_columns {
            dict.set_item(key, values)?;
        }

        Ok(dict.into())
    }

    /// Convert to a wide-format dictionary (paths as columns).
    ///
    /// Returns a dictionary with:
    /// - time: Time points (shared across all paths)
    /// - step: Step indices
    /// - path_0, path_1, ...: State variable values for each path
    ///
    /// Args:
    ///     state_var: Name of the state variable to extract (e.g., 'spot')
    fn to_wide_dict(&self, py: Python, state_var: &str) -> PyResult<Py<PyDict>> {
        if self.inner.paths.is_empty() {
            return Err(PyValueError::new_err("No paths in dataset"));
        }

        let dict = PyDict::new(py);

        // Use first path to get time points
        let first_path = &self.inner.paths[0];
        let times: Vec<f64> = first_path.points.iter().map(|p| p.time).collect();
        let steps: Vec<usize> = first_path.points.iter().map(|p| p.step).collect();

        dict.set_item("time", times)?;
        dict.set_item("step", steps)?;

        // Add each path as a column
        // First, find the index of the requested state variable
        let state_keys = self.inner.state_var_keys();
        let state_idx = state_keys.iter().position(|k| k == state_var);

        for (idx, path) in self.inner.paths.iter().enumerate() {
            let values: Vec<Option<f64>> = path
                .points
                .iter()
                .map(|p| {
                    if let Some(idx) = state_idx {
                        p.state.get(idx).copied()
                    } else {
                        None
                    }
                })
                .collect();
            dict.set_item(format!("path_{}", idx), values)?;
        }

        Ok(dict.into())
    }

    /// Convert to pandas DataFrame.
    ///
    /// Returns:
    ///     pd.DataFrame: Long-format DataFrame with all paths and state variables
    fn to_dataframe(&self, py: Python) -> PyResult<Py<PyAny>> {
        let pd = py.import("pandas")?;
        let dict = self.to_dict(py)?;
        pd.call_method1("DataFrame", (dict,))?
            .extract()
            .map_err(Into::into)
    }

    /// Convert all cashflows from all paths to a pandas DataFrame.
    ///
    /// Returns:
    ///     pd.DataFrame: DataFrame with columns:
    ///         - path_id: path identifier
    ///         - step: timestep index
    ///         - time_years: time in years
    ///         - amount: cashflow amount
    ///         - cashflow_type: type of cashflow as string
    fn cashflows_to_dataframe(&self, py: Python) -> PyResult<Py<PyAny>> {
        let pd = py.import("pandas")?;
        let dict = PyDict::new(py);

        let mut path_ids = Vec::new();
        let mut steps = Vec::new();
        let mut times = Vec::new();
        let mut amounts = Vec::new();
        let mut types = Vec::new();

        // Iterate through all paths and their cashflows
        for path in &self.inner.paths {
            for point in &path.points {
                for (time, amount, cf_type) in &point.cashflows {
                    path_ids.push(path.path_id);
                    steps.push(point.step);
                    times.push(*time);
                    amounts.push(*amount);
                    types.push(cashflow_type_to_string(*cf_type));
                }
            }
        }

        dict.set_item("path_id", path_ids)?;
        dict.set_item("step", steps)?;
        dict.set_item("time_years", times)?;
        dict.set_item("amount", amounts)?;
        dict.set_item("cashflow_type", types)?;

        pd.call_method1("DataFrame", (dict,))?
            .extract()
            .map_err(Into::into)
    }

    fn __repr__(&self) -> String {
        format!(
            "PathDataset(captured={}, total={}, sampling={})",
            self.inner.num_captured(),
            self.inner.num_paths_total,
            self.inner.sampling_method
        )
    }

    fn __len__(&self) -> usize {
        self.inner.num_captured()
    }

    /// Get a path by index.
    fn __getitem__(&self, index: isize) -> PyResult<PySimulatedPath> {
        let len = self.inner.paths.len() as isize;
        let actual = if index < 0 { len + index } else { index };
        if actual < 0 || actual >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                "path index out of range: {}",
                index
            )));
        }
        Ok(PySimulatedPath {
            inner: self.inner.paths[actual as usize].clone(),
        })
    }

    /// Return an iterator over the paths in this dataset.
    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<PyPathDatasetIterator>> {
        Py::new(
            slf.py(),
            PyPathDatasetIterator {
                dataset: Arc::clone(&slf.inner),
                index: 0,
            },
        )
    }
}

/// Iterator over simulated paths in a dataset.
#[pyclass(
    module = "finstack.valuations.common.monte_carlo",
    name = "PathDatasetIterator"
)]
pub struct PyPathDatasetIterator {
    dataset: Arc<PathDataset>,
    index: usize,
}

#[pymethods]
impl PyPathDatasetIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<PySimulatedPath> {
        if slf.index < slf.dataset.paths.len() {
            let path = PySimulatedPath {
                inner: slf.dataset.paths[slf.index].clone(),
            };
            slf.index += 1;
            Some(path)
        } else {
            None
        }
    }
}
