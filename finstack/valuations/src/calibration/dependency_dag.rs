//! Dependency DAG (Directed Acyclic Graph) for calibration sequencing.
//!
//! This module provides a flexible dependency resolution system that replaces
//! fixed sequential stages with dynamic dependency analysis, enabling:
//! - Parallel calibration of independent curves
//! - Flexible handling of complex cross-dependencies
//! - Optimal execution order based on actual dependencies

use crate::calibration::primitives::{HashableFloat, InstrumentQuote};
use finstack_core::market_data::term_structures::hazard_curve::Seniority;
use finstack_core::prelude::*;
use finstack_core::F;
use std::collections::{HashMap, HashSet, VecDeque};

/// Types of market data structures that can be calibrated.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CalibrationTarget {
    /// Discount curve (OIS/collateral)
    DiscountCurve { currency: Currency },
    /// Forward curve (IBOR/RFR)
    ForwardCurve { currency: Currency, tenor: String },
    /// Credit hazard curve
    HazardCurve { entity: String, seniority: Seniority },
    /// Inflation curve
    InflationCurve { index: String },
    /// Volatility surface
    VolatilitySurface { underlying: String },
    /// Base correlation curve
    BaseCorrelationCurve { index: String, maturity_years: HashableFloat },
}

impl CalibrationTarget {
    /// Get a string identifier for this target.
    pub fn id(&self) -> String {
        match self {
            CalibrationTarget::DiscountCurve { currency } => format!("{}-OIS", currency),
            CalibrationTarget::ForwardCurve { currency, tenor } => format!("{}-{}", currency, tenor),
            CalibrationTarget::HazardCurve { entity, seniority } => format!("{}-{}", entity, seniority),
            CalibrationTarget::InflationCurve { index } => index.clone(),
            CalibrationTarget::VolatilitySurface { underlying } => format!("{}-VOL", underlying),
            CalibrationTarget::BaseCorrelationCurve { index, maturity_years } => {
                format!("{}-CORR-{}Y", index, maturity_years.value())
            }
        }
    }

    /// Get calibration priority (lower = earlier).
    /// Used as a tie-breaker when dependencies don't determine order.
    pub fn priority(&self) -> u8 {
        match self {
            CalibrationTarget::DiscountCurve { .. } => 0,        // Always first
            CalibrationTarget::ForwardCurve { .. } => 1,         // After discount
            CalibrationTarget::HazardCurve { .. } => 2,          // After rates
            CalibrationTarget::InflationCurve { .. } => 2,       // After rates (parallel with credit)
            CalibrationTarget::VolatilitySurface { .. } => 3,    // After underlying curves
            CalibrationTarget::BaseCorrelationCurve { .. } => 4, // After hazard curves
        }
    }
}

/// Dependency relationship between calibration targets.
#[derive(Clone, Debug)]
pub struct CalibrationDependency {
    /// Target that depends on the prerequisite
    pub target: CalibrationTarget,
    /// Prerequisite that must be calibrated first
    pub prerequisite: CalibrationTarget,
    /// Dependency type (soft/hard)
    pub dependency_type: DependencyType,
}

/// Type of dependency relationship.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DependencyType {
    /// Hard dependency - target cannot be calibrated without prerequisite
    Required,
    /// Soft dependency - target can be calibrated but benefits from prerequisite
    Optional,
}

/// DAG-based calibration scheduler.
#[derive(Clone, Debug)]
pub struct CalibrationDAG {
    /// All targets to be calibrated
    targets: HashSet<CalibrationTarget>,
    /// Dependency edges
    dependencies: Vec<CalibrationDependency>,
    /// Quotes grouped by target
    quotes_by_target: HashMap<CalibrationTarget, Vec<InstrumentQuote>>,
}

impl CalibrationDAG {
    /// Create a new calibration DAG from instrument quotes.
    pub fn from_quotes(quotes: &[InstrumentQuote], base_currency: Currency) -> crate::Result<Self> {
        let mut dag = Self {
            targets: HashSet::new(),
            dependencies: Vec::new(),
            quotes_by_target: HashMap::new(),
        };

        // Analyze quotes to determine targets and dependencies
        dag.analyze_quotes(quotes, base_currency)?;
        dag.build_dependencies();

        Ok(dag)
    }

    /// Analyze quotes to determine calibration targets.
    fn analyze_quotes(&mut self, quotes: &[InstrumentQuote], base_currency: Currency) -> crate::Result<()> {
        for quote in quotes {
            let targets = self.extract_targets_from_quote(quote, base_currency);
            for target in targets {
                self.targets.insert(target.clone());
                self.quotes_by_target
                    .entry(target)
                    .or_default()
                    .push(quote.clone());
            }
        }
        Ok(())
    }

    /// Extract calibration targets from a single quote.
    fn extract_targets_from_quote(&self, quote: &InstrumentQuote, base_currency: Currency) -> Vec<CalibrationTarget> {
        let mut targets = Vec::new();

        match quote {
            InstrumentQuote::Deposit { .. } => {
                targets.push(CalibrationTarget::DiscountCurve { currency: base_currency });
            }
            InstrumentQuote::Swap { index, .. } => {
                targets.push(CalibrationTarget::DiscountCurve { currency: base_currency });
                if !index.contains("OIS") {
                    // Extract tenor from index name
                    let tenor = self.extract_tenor_from_index(index);
                    targets.push(CalibrationTarget::ForwardCurve { 
                        currency: base_currency, 
                        tenor 
                    });
                }
            }
            InstrumentQuote::FRA { .. } | InstrumentQuote::Future { .. } => {
                targets.push(CalibrationTarget::DiscountCurve { currency: base_currency });
                targets.push(CalibrationTarget::ForwardCurve { 
                    currency: base_currency, 
                    tenor: "3M".to_string() // Default tenor for FRA/Futures
                });
            }
            InstrumentQuote::CDS { entity, .. } | InstrumentQuote::CDSUpfront { entity, .. } => {
                targets.push(CalibrationTarget::DiscountCurve { currency: base_currency });
                targets.push(CalibrationTarget::HazardCurve { 
                    entity: entity.clone(), 
                    seniority: Seniority::Senior // Will be configurable
                });
            }
            InstrumentQuote::InflationSwap { index, .. } => {
                targets.push(CalibrationTarget::DiscountCurve { currency: base_currency });
                targets.push(CalibrationTarget::InflationCurve { index: index.clone() });
            }
            InstrumentQuote::OptionVol { underlying, .. } => {
                targets.push(CalibrationTarget::DiscountCurve { currency: base_currency });
                targets.push(CalibrationTarget::VolatilitySurface { underlying: underlying.clone() });
            }
            InstrumentQuote::CDSTranche { index, maturity, .. } => {
                // Calculate maturity in years for base correlation
                let maturity_years_f = (*maturity - finstack_core::dates::Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
                    .whole_days() as F / 365.25;
                let maturity_years = HashableFloat::new(maturity_years_f);
                targets.push(CalibrationTarget::DiscountCurve { currency: base_currency });
                targets.push(CalibrationTarget::HazardCurve { 
                    entity: index.clone(), 
                    seniority: Seniority::Senior 
                });
                targets.push(CalibrationTarget::BaseCorrelationCurve { 
                    index: index.clone(), 
                    maturity_years 
                });
            }
            InstrumentQuote::BasisSwap { primary_index, reference_index, currency, .. } => {
                targets.push(CalibrationTarget::DiscountCurve { currency: *currency });
                let primary_tenor = self.extract_tenor_from_index(primary_index);
                let reference_tenor = self.extract_tenor_from_index(reference_index);
                targets.push(CalibrationTarget::ForwardCurve { 
                    currency: *currency, 
                    tenor: primary_tenor 
                });
                targets.push(CalibrationTarget::ForwardCurve { 
                    currency: *currency, 
                    tenor: reference_tenor 
                });
            }
        }

        targets
    }

    /// Extract tenor from index name (e.g., "USD-SOFR-3M" -> "3M").
    fn extract_tenor_from_index(&self, index: &str) -> String {
        if index.contains("3M") {
            "3M".to_string()
        } else if index.contains("6M") {
            "6M".to_string()
        } else if index.contains("1M") {
            "1M".to_string()
        } else if index.contains("12M") {
            "12M".to_string()
        } else {
            "3M".to_string() // Default
        }
    }

    /// Build dependency relationships between targets.
    fn build_dependencies(&mut self) {
        // Clear existing dependencies
        self.dependencies.clear();

        for target in &self.targets {
            match target {
                CalibrationTarget::DiscountCurve { .. } => {
                    // Discount curves have no dependencies (base case)
                }
                CalibrationTarget::ForwardCurve { currency, .. } => {
                    // Forward curves depend on discount curves
                    self.dependencies.push(CalibrationDependency {
                        target: target.clone(),
                        prerequisite: CalibrationTarget::DiscountCurve { currency: *currency },
                        dependency_type: DependencyType::Required,
                    });
                }
                CalibrationTarget::HazardCurve { .. } => {
                    // Hazard curves depend on discount curves for PV calculations
                    let currency = self.get_currency_for_target(target);
                    self.dependencies.push(CalibrationDependency {
                        target: target.clone(),
                        prerequisite: CalibrationTarget::DiscountCurve { currency },
                        dependency_type: DependencyType::Required,
                    });
                }
                CalibrationTarget::InflationCurve { .. } => {
                    // Inflation curves depend on discount curves
                    let currency = self.get_currency_for_target(target);
                    self.dependencies.push(CalibrationDependency {
                        target: target.clone(),
                        prerequisite: CalibrationTarget::DiscountCurve { currency },
                        dependency_type: DependencyType::Required,
                    });
                }
                CalibrationTarget::VolatilitySurface { underlying } => {
                    // Vol surfaces depend on underlying forward curves/discount curves
                    let currency = self.get_currency_for_target(target);
                    self.dependencies.push(CalibrationDependency {
                        target: target.clone(),
                        prerequisite: CalibrationTarget::DiscountCurve { currency },
                        dependency_type: DependencyType::Required,
                    });

                    // If this is an equity/FX vol surface, also depend on forward curve
                    if self.is_rates_underlying(underlying) {
                        let tenor = self.extract_tenor_from_underlying(underlying);
                        self.dependencies.push(CalibrationDependency {
                            target: target.clone(),
                            prerequisite: CalibrationTarget::ForwardCurve { currency, tenor },
                            dependency_type: DependencyType::Optional,
                        });
                    }
                }
                CalibrationTarget::BaseCorrelationCurve { index, .. } => {
                    // Base correlation depends on hazard curves for the index
                    let currency = self.get_currency_for_target(target);
                    self.dependencies.push(CalibrationDependency {
                        target: target.clone(),
                        prerequisite: CalibrationTarget::DiscountCurve { currency },
                        dependency_type: DependencyType::Required,
                    });
                    self.dependencies.push(CalibrationDependency {
                        target: target.clone(),
                        prerequisite: CalibrationTarget::HazardCurve { 
                            entity: index.clone(), 
                            seniority: Seniority::Senior 
                        },
                        dependency_type: DependencyType::Required,
                    });
                }
            }
        }
    }

    /// Get the primary currency associated with a target.
    fn get_currency_for_target(&self, target: &CalibrationTarget) -> Currency {
        match target {
            CalibrationTarget::DiscountCurve { currency } => *currency,
            CalibrationTarget::ForwardCurve { currency, .. } => *currency,
            CalibrationTarget::HazardCurve { .. } => Currency::USD, // Default, should be configurable
            CalibrationTarget::InflationCurve { .. } => Currency::USD, // Default, should be configurable
            CalibrationTarget::VolatilitySurface { .. } => Currency::USD, // Default, should be configurable
            CalibrationTarget::BaseCorrelationCurve { .. } => Currency::USD, // Default, should be configurable
        }
    }

    /// Check if an underlying represents rates (vs equity/FX).
    fn is_rates_underlying(&self, underlying: &str) -> bool {
        underlying.contains("SOFR") || underlying.contains("EURIBOR") || underlying.contains("SONIA") || underlying.contains("OIS")
    }

    /// Extract tenor from underlying name.
    fn extract_tenor_from_underlying(&self, underlying: &str) -> String {
        self.extract_tenor_from_index(underlying)
    }

    /// Perform topological sort to determine calibration order.
    ///
    /// Returns a sequence of calibration batches where each batch contains
    /// targets that can be calibrated in parallel.
    pub fn topological_sort(&self) -> crate::Result<Vec<Vec<CalibrationTarget>>> {
        let mut in_degree: HashMap<CalibrationTarget, usize> = HashMap::new();
        let mut adj_list: HashMap<CalibrationTarget, Vec<CalibrationTarget>> = HashMap::new();

        // Initialize in-degree and adjacency list
        for target in &self.targets {
            in_degree.insert(target.clone(), 0);
            adj_list.insert(target.clone(), Vec::new());
        }

        // Build adjacency list and calculate in-degrees
        for dep in &self.dependencies {
            // Only consider required dependencies for ordering
            if dep.dependency_type == DependencyType::Required {
                if let Some(dependents) = adj_list.get_mut(&dep.prerequisite) {
                    dependents.push(dep.target.clone());
                }
                if let Some(degree) = in_degree.get_mut(&dep.target) {
                    *degree += 1;
                }
            }
        }

        let mut result = Vec::new();
        let mut queue: VecDeque<CalibrationTarget> = VecDeque::new();

        // Find all targets with no incoming edges (can be started immediately)
        for (target, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(target.clone());
            }
        }

        while !queue.is_empty() {
            // Current batch - all targets with no remaining dependencies
            let mut current_batch = Vec::new();
            let batch_size = queue.len();
            
            for _ in 0..batch_size {
                if let Some(target) = queue.pop_front() {
                    current_batch.push(target.clone());

                    // Reduce in-degree for all dependents
                    if let Some(dependents) = adj_list.get(&target) {
                        for dependent in dependents {
                            if let Some(degree) = in_degree.get_mut(dependent) {
                                *degree -= 1;
                                if *degree == 0 {
                                    queue.push_back(dependent.clone());
                                }
                            }
                        }
                    }
                }
            }

            if !current_batch.is_empty() {
                // Sort batch by priority for deterministic ordering
                current_batch.sort_by_key(|t| (t.priority(), t.id()));
                result.push(current_batch);
            }
        }

        // Check for cycles
        let total_processed: usize = result.iter().map(|batch| batch.len()).sum();
        if total_processed != self.targets.len() {
            return Err(finstack_core::Error::Internal); // Cycle detected
        }

        Ok(result)
    }

    /// Get quotes for a specific target.
    pub fn quotes_for_target(&self, target: &CalibrationTarget) -> &[InstrumentQuote] {
        self.quotes_by_target
            .get(target)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all calibration targets.
    pub fn targets(&self) -> &HashSet<CalibrationTarget> {
        &self.targets
    }

    /// Get dependency edges.
    pub fn dependencies(&self) -> &[CalibrationDependency] {
        &self.dependencies
    }

    /// Check if target A depends on target B (directly or transitively).
    pub fn depends_on(&self, target_a: &CalibrationTarget, target_b: &CalibrationTarget) -> bool {
        let mut visited = HashSet::new();
        self.depends_on_recursive(target_a, target_b, &mut visited)
    }

    /// Recursive helper for transitive dependency checking.
    fn depends_on_recursive(
        &self,
        target_a: &CalibrationTarget,
        target_b: &CalibrationTarget,
        visited: &mut HashSet<CalibrationTarget>,
    ) -> bool {
        if visited.contains(target_a) {
            return false; // Avoid infinite recursion
        }
        visited.insert(target_a.clone());

        // Check direct dependencies
        for dep in &self.dependencies {
            if dep.target == *target_a {
                if dep.prerequisite == *target_b {
                    return true; // Direct dependency
                }
                // Check transitive dependencies
                if self.depends_on_recursive(&dep.prerequisite, target_b, visited) {
                    return true;
                }
            }
        }

        false
    }

    /// Get calibration statistics.
    pub fn statistics(&self) -> CalibrationDAGStats {
        let batches = self.topological_sort().unwrap_or_default();
        let max_parallelism = batches.iter().map(|b| b.len()).max().unwrap_or(0);
        let total_dependencies = self.dependencies.len();
        let required_dependencies = self.dependencies
            .iter()
            .filter(|d| d.dependency_type == DependencyType::Required)
            .count();

        CalibrationDAGStats {
            total_targets: self.targets.len(),
            total_dependencies,
            required_dependencies,
            optional_dependencies: total_dependencies - required_dependencies,
            calibration_batches: batches.len(),
            max_parallelism,
            estimated_speedup: if max_parallelism > 1 { 
                (self.targets.len() as f64 / batches.len() as f64).min(max_parallelism as f64)
            } else { 
                1.0 
            },
        }
    }
}

/// Statistics about the calibration DAG.
#[derive(Clone, Debug)]
pub struct CalibrationDAGStats {
    /// Total number of targets to calibrate
    pub total_targets: usize,
    /// Total dependency edges
    pub total_dependencies: usize,
    /// Required dependency edges
    pub required_dependencies: usize,
    /// Optional dependency edges
    pub optional_dependencies: usize,
    /// Number of calibration batches (sequential steps)
    pub calibration_batches: usize,
    /// Maximum parallelism within a batch
    pub max_parallelism: usize,
    /// Estimated speedup vs sequential calibration
    pub estimated_speedup: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::{Date, DayCount, Frequency};
    use time::Month;

    fn create_test_quotes() -> Vec<InstrumentQuote> {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        vec![
            InstrumentQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.045,
                day_count: DayCount::Act360,
            },
            InstrumentQuote::Swap {
                maturity: base_date + time::Duration::days(365),
                rate: 0.047,
                fixed_freq: Frequency::semi_annual(),
                float_freq: Frequency::quarterly(),
                fixed_dc: DayCount::Thirty360,
                float_dc: DayCount::Act360,
                index: "USD-SOFR-3M".to_string(),
            },
            InstrumentQuote::CDS {
                entity: "AAPL".to_string(),
                maturity: base_date + time::Duration::days(365 * 5),
                spread_bp: 75.0,
                recovery_rate: 0.4,
                currency: Currency::USD,
            },
            InstrumentQuote::InflationSwap {
                maturity: base_date + time::Duration::days(365 * 5),
                rate: 0.025,
                index: "US-CPI-U".to_string(),
            },
        ]
    }

    #[test]
    fn test_dag_creation_from_quotes() {
        let quotes = create_test_quotes();
        let dag = CalibrationDAG::from_quotes(&quotes, Currency::USD).unwrap();

        assert!(!dag.targets.is_empty());
        assert!(!dag.dependencies.is_empty());

        // Should have discount curve target
        assert!(dag.targets.contains(&CalibrationTarget::DiscountCurve { currency: Currency::USD }));
        
        // Should have hazard curve target
        assert!(dag.targets.iter().any(|t| matches!(t, CalibrationTarget::HazardCurve { .. })));
    }

    #[test]
    fn test_topological_sort() {
        let quotes = create_test_quotes();
        let dag = CalibrationDAG::from_quotes(&quotes, Currency::USD).unwrap();

        let batches = dag.topological_sort().unwrap();
        assert!(!batches.is_empty());

        // First batch should contain only discount curves (no dependencies)
        assert!(batches[0].iter().all(|t| matches!(t, CalibrationTarget::DiscountCurve { .. })));

        // Later batches should contain dependent targets
        let all_later_targets: Vec<_> = batches.iter().skip(1).flatten().collect();
        assert!(all_later_targets.iter().any(|t| matches!(t, CalibrationTarget::HazardCurve { .. })));
    }

    #[test]
    fn test_dependency_detection() {
        let quotes = create_test_quotes();
        let dag = CalibrationDAG::from_quotes(&quotes, Currency::USD).unwrap();

        let discount_target = CalibrationTarget::DiscountCurve { currency: Currency::USD };
        let hazard_target = CalibrationTarget::HazardCurve { 
            entity: "AAPL".to_string(), 
            seniority: Seniority::Senior 
        };

        // Hazard curve should depend on discount curve
        assert!(dag.depends_on(&hazard_target, &discount_target));
        
        // Discount curve should not depend on hazard curve
        assert!(!dag.depends_on(&discount_target, &hazard_target));
    }

    #[test]
    fn test_dag_statistics() {
        let quotes = create_test_quotes();
        let dag = CalibrationDAG::from_quotes(&quotes, Currency::USD).unwrap();

        let stats = dag.statistics();
        assert!(stats.total_targets > 0);
        assert!(stats.total_dependencies > 0);
        assert!(stats.calibration_batches > 0);
        assert!(stats.estimated_speedup >= 1.0);
    }
}
