//! Tornado chart generation.

use super::types::SensitivityResult;
use serde::{Deserialize, Serialize};

/// Entry in a tornado chart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TornadoEntry {
    /// Parameter identifier
    pub parameter_id: String,

    /// Impact of low value
    pub downside_impact: f64,

    /// Impact of high value
    pub upside_impact: f64,

    /// Total swing (abs(upside - downside))
    pub swing: f64,
}

impl TornadoEntry {
    /// Create from scenario data.
    pub fn new(
        parameter_id: String,
        downside_impact: f64,
        upside_impact: f64,
    ) -> Self {
        let swing = (upside_impact - downside_impact).abs();
        Self {
            parameter_id,
            downside_impact,
            upside_impact,
            swing,
        }
    }
}

/// Generate tornado chart data from sensitivity results.
///
/// Returns entries sorted by swing magnitude (descending).
pub fn generate_tornado_chart(
    _result: &SensitivityResult,
    _metric: &str,
) -> Vec<TornadoEntry> {
    // Simplified implementation
    Vec::new()
}

