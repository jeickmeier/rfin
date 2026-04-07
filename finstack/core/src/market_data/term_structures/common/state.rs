use crate::math::interp::{ExtrapolationPolicy, InterpStyle};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct StateId {
    /// Curve identifier
    pub id: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct StateKnotPoints {
    /// Time/value pairs used to construct the curve
    pub knot_points: Vec<(f64, f64)>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct StateInterp {
    /// Interpolation style
    pub interp_style: InterpStyle,
    /// Extrapolation policy
    pub extrapolation: ExtrapolationPolicy,
}
