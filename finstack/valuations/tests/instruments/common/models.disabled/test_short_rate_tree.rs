//! Tests for short-rate tree models.

use finstack_valuations::instruments::common::models::{
    short_rate_keys, ShortRateModel, ShortRateTree,
};

use super::super::test_helpers::*;

#[test]
fn test_ho_lee_tree_creation() {
    // Arrange & Act
    let tree = ShortRateTree::ho_lee(50, 0.015);

    // Assert
    assert_eq!(tree.config.steps, 50);
    assert_eq!(tree.config.model, ShortRateModel::HoLee);
    assert_approx_eq(tree.config.volatility, 0.015, TIGHT_TOLERANCE, "Volatility");
}

#[test]
fn test_bdt_tree_creation() {
    // Arrange & Act
    let tree = ShortRateTree::black_derman_toy(25, 0.02, 0.1);

    // Assert
    assert_eq!(tree.config.model, ShortRateModel::BlackDermanToy);
    assert_eq!(tree.config.mean_reversion, Some(0.1));
}

#[test]
fn test_tree_calibration() {
    // Arrange
    let mut tree = ShortRateTree::ho_lee(10, 0.015);
    let curve = flat_curve(0.05, "TEST");

    // Act
    let result = tree.calibrate(&curve, 2.0);

    // Assert
    assert!(result.is_ok());
    assert_eq!(tree.rates.len(), 11); // 0 to 10 steps
}

#[test]
fn test_rate_access() {
    // Arrange
    let mut tree = ShortRateTree::ho_lee(5, 0.01);
    let curve = upward_curve("TEST");
    tree.calibrate(&curve, 1.0).unwrap();

    // Act & Assert: Valid access
    let r0 = tree.rate_at_node(0, 0);
    assert!(r0.is_ok());
    assert!(r0.unwrap() > 0.0);

    // Act & Assert: Invalid access
    assert!(tree.rate_at_node(10, 0).is_err());
}
