//! Test serialization of interpolation types

#[cfg(feature = "serde")]
#[test]
fn test_interpolation_serialization() {
    use finstack_core::math::interp::{
        CubicHermite, ExtrapolationPolicy, FlatFwd, InterpFn, LinearDf, LogLinearDf, MonotoneConvex,
    };

    // Test data
    let knots = vec![0.0, 1.0, 2.0, 3.0, 5.0].into_boxed_slice();
    let dfs = vec![1.0, 0.98, 0.95, 0.92, 0.87].into_boxed_slice();
    let extrapolation = ExtrapolationPolicy::FlatZero;

    // Test LinearDf
    {
        let linear = LinearDf::new(knots.clone(), dfs.clone(), extrapolation).unwrap();
        let json = serde_json::to_string_pretty(&linear).unwrap();
        let deserialized: LinearDf = serde_json::from_str(&json).unwrap();
        assert_eq!(linear.interp(1.5), deserialized.interp(1.5));
    }

    // Test LogLinearDf
    {
        let log_linear = LogLinearDf::new(knots.clone(), dfs.clone(), extrapolation).unwrap();
        let json = serde_json::to_string_pretty(&log_linear).unwrap();
        let deserialized: LogLinearDf = serde_json::from_str(&json).unwrap();
        // Use tolerance for floating point comparison
        assert!((log_linear.interp(1.5) - deserialized.interp(1.5)).abs() < 1e-10);
    }

    // Test MonotoneConvex
    {
        let monotone = MonotoneConvex::new(knots.clone(), dfs.clone(), extrapolation).unwrap();
        let json = serde_json::to_string_pretty(&monotone).unwrap();
        let deserialized: MonotoneConvex = serde_json::from_str(&json).unwrap();
        assert!((monotone.interp(1.5) - deserialized.interp(1.5)).abs() < 1e-10);
    }

    // Test CubicHermite
    {
        let cubic = CubicHermite::new(knots.clone(), dfs.clone(), extrapolation).unwrap();
        let json = serde_json::to_string_pretty(&cubic).unwrap();
        let deserialized: CubicHermite = serde_json::from_str(&json).unwrap();
        assert!((cubic.interp(1.5) - deserialized.interp(1.5)).abs() < 1e-10);
    }

    // Test FlatFwd
    {
        let flat_fwd = FlatFwd::new(knots.clone(), dfs.clone(), extrapolation).unwrap();
        let json = serde_json::to_string_pretty(&flat_fwd).unwrap();
        let deserialized: FlatFwd = serde_json::from_str(&json).unwrap();
        assert!((flat_fwd.interp(1.5) - deserialized.interp(1.5)).abs() < 1e-10);
    }
}
