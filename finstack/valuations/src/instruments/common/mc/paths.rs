//! Path data structures for Monte Carlo simulation capture and visualization.
//!
//! This module provides data structures to capture and store individual Monte Carlo
//! paths for visualization, debugging, and price explanation. Paths can be captured
//! in full or sampled for efficiency.

use finstack_core::collections::HashMap;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Default state variable indices for standard single-asset models.
///
/// These constants define the expected layout for common cases (GBM, Heston, etc.).
/// Multi-asset models should use `PathDataset::process_params.factor_names` to interpret indices.
pub mod state_indices {
    /// Spot price (equity/FX) - index 0
    pub const IDX_SPOT: usize = 0;
    /// Stochastic variance (Heston, etc.) - index 1
    pub const IDX_VARIANCE: usize = 1;
    /// Short rate (Hull-White, etc.) - index 1 (aliases variance in current engine)
    pub const IDX_SHORT_RATE: usize = 1;
    /// Credit spread - index 2
    pub const IDX_CREDIT_SPREAD: usize = 2;
}

/// Type of cashflow for categorization and analysis.
///
/// Used to distinguish different cashflow categories in Monte Carlo simulations,
/// particularly for complex instruments like revolving credit facilities.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CashflowType {
    /// Principal deployment (draws) or repayment
    Principal,
    /// Interest payment on drawn amounts
    Interest,
    /// Commitment fee on undrawn amounts
    CommitmentFee,
    /// Usage fee on drawn amounts
    UsageFee,
    /// Facility fee on total commitment
    FacilityFee,
    /// One-time upfront fee
    UpfrontFee,
    /// Recovery proceeds on default
    Recovery,
    /// Mark-to-market P&L at timestep
    MarkToMarket,
    /// Other/generic cashflow
    Other,
}

/// A single point along a Monte Carlo path.
///
/// Captures the state at a specific time step, including state variables
/// and optionally the payoff value at that point.
///
/// # State Vector Layout
///
/// The `state` vector contains all state variable values in a compact, fixed-size allocation.
/// For standard single-asset models, see `state_indices` for the expected layout.
/// For multi-asset models, consult `PathDataset::process_params.factor_names` to interpret indices.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathPoint {
    /// Time step index (0 = initial, N = final)
    pub step: usize,
    /// Time in years from valuation date
    pub time: f64,
    /// State variables at this point (spot, variance, rate, etc.)
    /// Indexed by position - see `state_indices` for standard layout
    pub state: SmallVec<[f64; 8]>,
    /// Optional payoff value at this point (if capture_payoffs is enabled)
    pub payoff_value: Option<f64>,
    /// Typed cashflows generated at this timestep (time, amount, type) tuples
    /// For instruments like revolving credit: interest, fees, principal changes
    #[serde(default)]
    pub cashflows: Vec<(f64, f64, CashflowType)>,
}

impl PathPoint {
    /// Create a new path point with empty state.
    pub fn new(step: usize, time: f64) -> Self {
        Self {
            step,
            time,
            state: SmallVec::new(),
            payoff_value: None,
            cashflows: Vec::new(),
        }
    }

    /// Create a path point with the given state vector.
    pub fn with_state(step: usize, time: f64, state: SmallVec<[f64; 8]>) -> Self {
        Self {
            step,
            time,
            state,
            payoff_value: None,
            cashflows: Vec::new(),
        }
    }

    /// Set payoff value.
    pub fn set_payoff(&mut self, value: f64) {
        self.payoff_value = Some(value);
    }

    /// Get spot price (convenience method for standard single-asset models).
    ///
    /// Returns the value at `state_indices::IDX_SPOT` if it exists.
    /// For multi-asset models, use `state` directly with the schema from `PathDataset`.
    pub fn spot(&self) -> Option<f64> {
        self.state.get(state_indices::IDX_SPOT).copied()
    }

    /// Get variance (convenience method for standard stochastic volatility models).
    ///
    /// Returns the value at `state_indices::IDX_VARIANCE` if it exists.
    /// For multi-asset models, use `state` directly with the schema from `PathDataset`.
    pub fn variance(&self) -> Option<f64> {
        self.state.get(state_indices::IDX_VARIANCE).copied()
    }

    /// Get short rate (convenience method for interest rate models).
    ///
    /// Returns the value at `state_indices::IDX_SHORT_RATE` if it exists.
    /// For multi-asset models, use `state` directly with the schema from `PathDataset`.
    pub fn short_rate(&self) -> Option<f64> {
        self.state.get(state_indices::IDX_SHORT_RATE).copied()
    }

    /// Add a cashflow to this point (uses Other type).
    ///
    /// # Arguments
    /// * `time` - Time in years when the cashflow occurs
    /// * `amount` - Cashflow amount (positive = inflow, negative = outflow)
    pub fn add_cashflow(&mut self, time: f64, amount: f64) {
        self.cashflows.push((time, amount, CashflowType::Other));
    }

    /// Add a typed cashflow to this point.
    ///
    /// # Arguments
    /// * `time` - Time in years when the cashflow occurs
    /// * `amount` - Cashflow amount (positive = inflow, negative = outflow)
    /// * `cf_type` - Type of cashflow
    pub fn add_typed_cashflow(&mut self, time: f64, amount: f64, cf_type: CashflowType) {
        self.cashflows.push((time, amount, cf_type));
    }

    /// Get all cashflows at this point.
    pub fn get_cashflows(&self) -> &[(f64, f64, CashflowType)] {
        &self.cashflows
    }

    /// Get cashflows by type.
    ///
    /// Returns (time, amount) pairs for all cashflows matching the given type.
    pub fn get_cashflows_by_type(&self, cf_type: CashflowType) -> Vec<(f64, f64)> {
        self.cashflows
            .iter()
            .filter(|(_, _, t)| *t == cf_type)
            .map(|(time, amount, _)| (*time, *amount))
            .collect()
    }

    /// Get principal flows (convenience method).
    pub fn principal_flows(&self) -> Vec<(f64, f64)> {
        self.get_cashflows_by_type(CashflowType::Principal)
    }

    /// Get interest flows (convenience method).
    pub fn interest_flows(&self) -> Vec<(f64, f64)> {
        self.get_cashflows_by_type(CashflowType::Interest)
    }

    /// Get total cashflow amount at this timestep.
    pub fn total_cashflow(&self) -> f64 {
        self.cashflows.iter().map(|(_, amt, _)| amt).sum()
    }

    /// Get total cashflow amount by type.
    pub fn total_cashflow_by_type(&self, cf_type: CashflowType) -> f64 {
        self.cashflows
            .iter()
            .filter(|(_, _, t)| *t == cf_type)
            .map(|(_, amt, _)| amt)
            .sum()
    }
}

/// A complete simulated path through time.
///
/// Contains all time steps for a single Monte Carlo path, along with
/// metadata and the final payoff value.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulatedPath {
    /// Path identifier (0-indexed)
    pub path_id: usize,
    /// Time points along the path
    pub points: Vec<PathPoint>,
    /// Final discounted payoff value for this path
    pub final_value: f64,
    /// Internal Rate of Return for this path (if calculable)
    #[serde(default)]
    pub irr: Option<f64>,
}

impl SimulatedPath {
    /// Create a new simulated path.
    pub fn new(path_id: usize) -> Self {
        Self {
            path_id,
            points: Vec::new(),
            final_value: 0.0,
            irr: None,
        }
    }

    /// Create a path with preallocated capacity.
    pub fn with_capacity(path_id: usize, capacity: usize) -> Self {
        Self {
            path_id,
            points: Vec::with_capacity(capacity),
            final_value: 0.0,
            irr: None,
        }
    }

    /// Add a point to the path.
    pub fn add_point(&mut self, point: PathPoint) {
        self.points.push(point);
    }

    /// Set the final payoff value.
    pub fn set_final_value(&mut self, value: f64) {
        self.final_value = value;
    }

    /// Set the IRR for this path.
    pub fn set_irr(&mut self, irr: f64) {
        self.irr = Some(irr);
    }

    /// Get the number of time steps.
    pub fn num_steps(&self) -> usize {
        self.points.len()
    }

    /// Get a point by step index.
    pub fn point(&self, step: usize) -> Option<&PathPoint> {
        self.points.get(step)
    }

    /// Get the initial point.
    pub fn initial_point(&self) -> Option<&PathPoint> {
        self.points.first()
    }

    /// Get the final point.
    pub fn terminal_point(&self) -> Option<&PathPoint> {
        self.points.last()
    }

    /// Extract all cashflows from the path.
    ///
    /// Returns all (time, amount) cashflow pairs across all timesteps.
    pub fn extract_cashflows(&self) -> Vec<(f64, f64)> {
        let mut all_cashflows = Vec::new();
        for point in &self.points {
            for (time, amount, _) in &point.cashflows {
                all_cashflows.push((*time, *amount));
            }
        }
        all_cashflows
    }

    /// Extract typed cashflows from the path.
    ///
    /// Returns all (time, amount, type) tuples across all timesteps.
    pub fn extract_typed_cashflows(&self) -> Vec<(f64, f64, CashflowType)> {
        let mut all_cashflows = Vec::new();
        for point in &self.points {
            all_cashflows.extend_from_slice(&point.cashflows);
        }
        all_cashflows
    }

    /// Extract cashflows by type.
    ///
    /// Returns all (time, amount) pairs for cashflows of the specified type.
    pub fn extract_cashflows_by_type(&self, cf_type: CashflowType) -> Vec<(f64, f64)> {
        let mut all_cashflows = Vec::new();
        for point in &self.points {
            all_cashflows.extend(point.get_cashflows_by_type(cf_type));
        }
        all_cashflows
    }
}

/// Method used to sample paths from the full simulation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PathSamplingMethod {
    /// All paths were captured
    All,
    /// Random sample of N paths
    RandomSample {
        /// Number of paths to sample
        count: usize,
        /// Random seed for sampling
        seed: u64,
    },
}

impl std::fmt::Display for PathSamplingMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => write!(f, "all"),
            Self::RandomSample { count, seed } => {
                write!(f, "random_sample(n={}, seed={})", count, seed)
            }
        }
    }
}

/// Process parameters for metadata.
///
/// This will be populated by the ProcessMetadata trait implementations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessParams {
    /// Process type name (e.g., "GBM", "Heston", "MultiGBM")
    pub process_type: String,
    /// Key-value parameters (e.g., r, q, sigma, kappa, theta)
    pub parameters: HashMap<String, f64>,
    /// Optional correlation matrix (row-major, n×n)
    pub correlation: Option<Vec<f64>>,
    /// Factor names (e.g., ["spot"], ["spot", "variance"])
    pub factor_names: Vec<String>,
}

impl ProcessParams {
    /// Create new process parameters.
    pub fn new(process_type: impl Into<String>) -> Self {
        Self {
            process_type: process_type.into(),
            parameters: HashMap::default(),
            correlation: None,
            factor_names: Vec::new(),
        }
    }

    /// Add a parameter.
    pub fn add_param(&mut self, key: impl Into<String>, value: f64) {
        self.parameters.insert(key.into(), value);
    }

    /// Set correlation matrix.
    pub fn with_correlation(mut self, correlation: Vec<f64>) -> Self {
        self.correlation = Some(correlation);
        self
    }

    /// Set factor names.
    pub fn with_factors(mut self, names: Vec<String>) -> Self {
        self.factor_names = names;
        self
    }

    /// Get the dimension (number of factors) from correlation matrix.
    pub fn dim(&self) -> Option<usize> {
        self.correlation
            .as_ref()
            .map(|corr| (corr.len() as f64).sqrt() as usize)
    }
}

/// Collection of simulated paths with metadata.
///
/// This structure holds captured paths along with information about
/// the simulation parameters and sampling method used.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathDataset {
    /// Captured paths
    pub paths: Vec<SimulatedPath>,
    /// Total number of paths in the full simulation
    pub num_paths_total: usize,
    /// Sampling method used
    pub sampling_method: PathSamplingMethod,
    /// Process parameters and metadata
    pub process_params: ProcessParams,
}

impl PathDataset {
    /// Create a new path dataset.
    pub fn new(
        num_paths_total: usize,
        sampling_method: PathSamplingMethod,
        process_params: ProcessParams,
    ) -> Self {
        // Pre-allocate based on sampling method
        let estimated_capacity = match sampling_method {
            PathSamplingMethod::All => num_paths_total,
            PathSamplingMethod::RandomSample { count, .. } => count,
        };
        Self {
            paths: Vec::with_capacity(estimated_capacity),
            num_paths_total,
            sampling_method,
            process_params,
        }
    }

    /// Create with preallocated capacity.
    pub fn with_capacity(
        capacity: usize,
        num_paths_total: usize,
        sampling_method: PathSamplingMethod,
        process_params: ProcessParams,
    ) -> Self {
        Self {
            paths: Vec::with_capacity(capacity),
            num_paths_total,
            sampling_method,
            process_params,
        }
    }

    /// Add a simulated path.
    pub fn add_path(&mut self, path: SimulatedPath) {
        self.paths.push(path);
    }

    /// Get the number of captured paths.
    pub fn num_captured(&self) -> usize {
        self.paths.len()
    }

    /// Get a path by index.
    pub fn path(&self, index: usize) -> Option<&SimulatedPath> {
        self.paths.get(index)
    }

    /// Check if all paths were captured.
    pub fn is_complete(&self) -> bool {
        self.sampling_method == PathSamplingMethod::All && self.paths.len() == self.num_paths_total
    }

    /// Get the sampling ratio (captured / total).
    pub fn sampling_ratio(&self) -> f64 {
        if self.num_paths_total == 0 {
            0.0
        } else {
            self.paths.len() as f64 / self.num_paths_total as f64
        }
    }

    /// Get the state variable names from the process metadata.
    ///
    /// Returns the factor names if available, otherwise returns generic names
    /// based on the maximum state dimension found in the dataset.
    pub fn state_var_keys(&self) -> Vec<String> {
        // Use factor names from process metadata if available
        if !self.process_params.factor_names.is_empty() {
            return self.process_params.factor_names.clone();
        }

        // Otherwise, generate generic names based on max dimension
        let max_dim = self
            .paths
            .iter()
            .flat_map(|path| path.points.iter())
            .map(|point| point.state.len())
            .max()
            .unwrap_or(0);

        (0..max_dim).map(|i| format!("state_{}", i)).collect()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_path_point_creation() {
        let mut point = PathPoint::new(0, 0.0);
        assert_eq!(point.step, 0);
        assert_eq!(point.time, 0.0);
        assert!(point.state.is_empty());
        assert!(point.payoff_value.is_none());
        assert!(point.cashflows.is_empty());

        // Create a point with state
        let mut state = SmallVec::new();
        state.push(100.0); // spot
        let mut point_with_state = PathPoint::with_state(0, 0.0, state);
        assert_eq!(point_with_state.spot(), Some(100.0));

        point_with_state.set_payoff(42.5);
        assert_eq!(point_with_state.payoff_value, Some(42.5));

        // Test cashflows
        point.add_cashflow(0.25, 1000.0);
        point.add_cashflow(0.25, 500.0);
        assert_eq!(point.cashflows.len(), 2);
        assert_eq!(
            point.get_cashflows()[0],
            (0.25, 1000.0, CashflowType::Other)
        );
        assert_eq!(point.get_cashflows()[1], (0.25, 500.0, CashflowType::Other));
        assert_eq!(point.total_cashflow(), 1500.0);

        // Test typed cashflows
        let mut point2 = PathPoint::new(1, 0.5);
        point2.add_typed_cashflow(0.5, 100.0, CashflowType::Interest);
        point2.add_typed_cashflow(0.5, 50.0, CashflowType::Principal);
        assert_eq!(point2.principal_flows().len(), 1);
        assert_eq!(point2.interest_flows().len(), 1);
        assert_eq!(point2.total_cashflow_by_type(CashflowType::Interest), 100.0);
    }

    #[test]
    fn test_simulated_path() {
        let mut path = SimulatedPath::with_capacity(1, 10);
        assert_eq!(path.path_id, 1);
        assert_eq!(path.num_steps(), 0);

        let mut state1 = SmallVec::new();
        state1.push(100.0); // spot
        let point1 = PathPoint::with_state(0, 0.0, state1);
        path.add_point(point1);

        let mut state2 = SmallVec::new();
        state2.push(102.0); // spot
        let point2 = PathPoint::with_state(1, 0.1, state2);
        path.add_point(point2);

        assert_eq!(path.num_steps(), 2);
        assert_eq!(
            path.initial_point()
                .expect("Path should have initial point")
                .spot(),
            Some(100.0)
        );
        assert_eq!(
            path.terminal_point()
                .expect("Path should have terminal point")
                .spot(),
            Some(102.0)
        );

        path.set_final_value(5.0);
        assert_eq!(path.final_value, 5.0);
    }

    #[test]
    fn test_simulated_path_cashflows() {
        let mut path = SimulatedPath::with_capacity(1, 10);

        // Add points with cashflows
        let mut point1 = PathPoint::new(0, 0.0);
        point1.add_cashflow(0.0, -100.0); // Initial outflow
        path.add_point(point1);

        let mut point2 = PathPoint::new(1, 0.25);
        point2.add_cashflow(0.25, 5.0); // Interest payment
        point2.add_cashflow(0.25, 2.0); // Fee payment
        path.add_point(point2);

        let mut point3 = PathPoint::new(2, 0.50);
        point3.add_cashflow(0.50, 5.0); // Interest payment
        path.add_point(point3);

        // Extract all cashflows
        let all_cashflows = path.extract_cashflows();
        assert_eq!(all_cashflows.len(), 4);
        assert_eq!(all_cashflows[0], (0.0, -100.0));
        assert_eq!(all_cashflows[1], (0.25, 5.0));
        assert_eq!(all_cashflows[2], (0.25, 2.0));
        assert_eq!(all_cashflows[3], (0.50, 5.0));
    }

    #[test]
    fn test_process_params() {
        let mut params = ProcessParams::new("GBM");
        params.add_param("r", 0.05);
        params.add_param("q", 0.02);
        params.add_param("sigma", 0.2);

        assert_eq!(params.process_type, "GBM");
        assert_eq!(params.parameters.get("r"), Some(&0.05));
        assert_eq!(params.parameters.get("sigma"), Some(&0.2));
    }

    #[test]
    fn test_path_dataset() {
        let process_params = ProcessParams::new("GBM");
        let mut dataset = PathDataset::new(
            100,
            PathSamplingMethod::RandomSample {
                count: 10,
                seed: 42,
            },
            process_params,
        );

        assert_eq!(dataset.num_paths_total, 100);
        assert_eq!(dataset.num_captured(), 0);
        assert!(!dataset.is_complete());

        let path1 = SimulatedPath::new(0);
        let path2 = SimulatedPath::new(1);
        dataset.add_path(path1);
        dataset.add_path(path2);

        assert_eq!(dataset.num_captured(), 2);
        assert_eq!(dataset.sampling_ratio(), 0.02);
    }

    #[test]
    fn test_sampling_method_display() {
        let all = PathSamplingMethod::All;
        assert_eq!(all.to_string(), "all");

        let sample = PathSamplingMethod::RandomSample {
            count: 100,
            seed: 42,
        };
        assert_eq!(sample.to_string(), "random_sample(n=100, seed=42)");
    }

    #[test]
    fn test_state_var_keys_extraction() {
        // Test with factor names in metadata
        let mut process_params = ProcessParams::new("GBM");
        process_params.factor_names = vec!["spot".to_string(), "variance".to_string()];
        let mut dataset = PathDataset::new(10, PathSamplingMethod::All, process_params);

        let mut path1 = SimulatedPath::new(0);
        let mut state1 = SmallVec::new();
        state1.push(100.0); // spot
        state1.push(0.04); // variance
        let point1 = PathPoint::with_state(0, 0.0, state1);
        path1.add_point(point1);
        dataset.add_path(path1);

        let keys = dataset.state_var_keys();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0], "spot");
        assert_eq!(keys[1], "variance");

        // Test with no factor names (should generate generic names)
        let process_params2 = ProcessParams::new("GBM");
        let mut dataset2 = PathDataset::new(10, PathSamplingMethod::All, process_params2);

        let mut path2 = SimulatedPath::new(0);
        let mut state2 = SmallVec::new();
        state2.push(105.0); // state_0
        state2.push(0.03); // state_1
        let point2 = PathPoint::with_state(0, 0.0, state2);
        path2.add_point(point2);
        dataset2.add_path(path2);

        let keys2 = dataset2.state_var_keys();
        assert_eq!(keys2.len(), 2);
        assert_eq!(keys2[0], "state_0");
        assert_eq!(keys2[1], "state_1");
    }
}
