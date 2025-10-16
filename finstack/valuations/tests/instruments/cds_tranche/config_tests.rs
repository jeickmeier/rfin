//! Configuration and parameter validation tests for CDS Tranche.
//!
//! Tests cover:
//! - Default configuration values
//! - Custom configuration parameters
//! - CS01 bump units
//! - Heterogeneous calculation methods
//! - Configuration parameter bounds

use finstack_valuations::instruments::cds_tranche::pricer::{
    CDSTranchePricerConfig, Cs01BumpUnits, HeteroMethod,
};

// ==================== Default Configuration Tests ====================

#[test]
fn test_default_config_quadrature_order() {
    // Arrange & Act
    let config = CDSTranchePricerConfig::default();

    // Assert
    assert_eq!(
        config.quadrature_order, 7,
        "Default quadrature order should be 7"
    );
}

#[test]
fn test_default_config_uses_issuer_curves() {
    // Arrange & Act
    let config = CDSTranchePricerConfig::default();

    // Assert
    assert!(
        config.use_issuer_curves,
        "Default should use issuer curves when available"
    );
}

#[test]
fn test_default_config_correlation_bounds() {
    // Arrange & Act
    let config = CDSTranchePricerConfig::default();

    // Assert
    assert_eq!(
        config.min_correlation, 0.01,
        "Default min correlation should be 0.01"
    );
    assert_eq!(
        config.max_correlation, 0.99,
        "Default max correlation should be 0.99"
    );
    assert!(
        config.min_correlation < config.max_correlation,
        "Min correlation must be less than max correlation"
    );
}

#[test]
fn test_default_config_cs01_parameters() {
    // Arrange & Act
    let config = CDSTranchePricerConfig::default();

    // Assert
    assert_eq!(
        config.cs01_bump_size, 1.0,
        "Default CS01 bump size should be 1bp"
    );
    assert!(
        matches!(config.cs01_bump_units, Cs01BumpUnits::HazardRateBp),
        "Default CS01 units should be hazard rate basis points"
    );
}

#[test]
fn test_default_config_correlation_bump() {
    // Arrange & Act
    let config = CDSTranchePricerConfig::default();

    // Assert
    assert_eq!(
        config.corr_bump_abs, 0.01,
        "Default correlation bump should be 1%"
    );
}

#[test]
fn test_default_config_accrual_on_default() {
    // Arrange & Act
    let config = CDSTranchePricerConfig::default();

    // Assert
    assert!(
        config.accrual_on_default_enabled,
        "Accrual-on-default should be enabled by default"
    );
    assert_eq!(
        config.aod_allocation_fraction, 0.5,
        "Default AoD allocation should be 50%"
    );
}

#[test]
fn test_default_config_numerical_stability() {
    // Arrange & Act
    let config = CDSTranchePricerConfig::default();

    // Assert
    assert_eq!(config.numerical_tolerance, 1e-10);
    assert_eq!(config.cdf_clip, 10.0);
    assert_eq!(config.spa_variance_floor, 1e-14);
    assert_eq!(config.probability_clip, 1e-12);
}

#[test]
fn test_default_config_hetero_method() {
    // Arrange & Act
    let config = CDSTranchePricerConfig::default();

    // Assert
    assert!(
        matches!(config.hetero_method, HeteroMethod::Spa),
        "Default heterogeneous method should be SPA"
    );
}

// ==================== CS01 Bump Units Tests ====================

#[test]
fn test_cs01_bump_units_hazard_rate() {
    // Arrange & Act
    let units = Cs01BumpUnits::HazardRateBp;

    // Assert
    assert!(matches!(units, Cs01BumpUnits::HazardRateBp));
}

#[test]
fn test_cs01_bump_units_spread_additive() {
    // Arrange & Act
    let units = Cs01BumpUnits::SpreadBpAdditive;

    // Assert
    assert!(matches!(units, Cs01BumpUnits::SpreadBpAdditive));
}

#[test]
fn test_cs01_bump_units_equality() {
    // Arrange
    let hazard1 = Cs01BumpUnits::HazardRateBp;
    let hazard2 = Cs01BumpUnits::HazardRateBp;
    let spread = Cs01BumpUnits::SpreadBpAdditive;

    // Assert
    assert_eq!(hazard1, hazard2, "Same enum variants should be equal");
    assert_ne!(
        hazard1, spread,
        "Different enum variants should not be equal"
    );
}

// ==================== Heterogeneous Method Tests ====================

#[test]
fn test_hetero_method_spa() {
    // Arrange & Act
    let method = HeteroMethod::Spa;

    // Assert
    assert!(matches!(method, HeteroMethod::Spa));
}

#[test]
fn test_hetero_method_exact_convolution() {
    // Arrange & Act
    let method = HeteroMethod::ExactConvolution;

    // Assert
    assert!(matches!(method, HeteroMethod::ExactConvolution));
}

#[test]
fn test_hetero_method_equality() {
    // Arrange
    let spa1 = HeteroMethod::Spa;
    let spa2 = HeteroMethod::Spa;
    let exact = HeteroMethod::ExactConvolution;

    // Assert
    assert_eq!(spa1, spa2, "Same enum variants should be equal");
    assert_ne!(spa1, exact, "Different enum variants should not be equal");
}

// ==================== Custom Configuration Tests ====================

#[test]
fn test_custom_config_quadrature_orders() {
    // Test different valid quadrature orders
    for order in [5u8, 7, 10] {
        // Arrange
        let config = CDSTranchePricerConfig {
            quadrature_order: order,
            ..Default::default()
        };

        // Assert
        assert_eq!(config.quadrature_order, order);
    }
}

#[test]
fn test_custom_config_correlation_bounds_modification() {
    // Arrange & Act
    let config = CDSTranchePricerConfig {
        min_correlation: 0.05,
        max_correlation: 0.95,
        ..Default::default()
    };

    // Assert
    assert_eq!(config.min_correlation, 0.05);
    assert_eq!(config.max_correlation, 0.95);
}

#[test]
fn test_custom_config_cs01_bump_size() {
    // Arrange & Act
    let config = CDSTranchePricerConfig {
        cs01_bump_size: 0.5,
        ..Default::default()
    };

    // Assert
    assert_eq!(config.cs01_bump_size, 0.5);
}

#[test]
fn test_custom_config_disable_accrual_on_default() {
    // Arrange & Act
    let config = CDSTranchePricerConfig {
        accrual_on_default_enabled: false,
        ..Default::default()
    };

    // Assert
    assert!(!config.accrual_on_default_enabled);
}

#[test]
fn test_custom_config_hetero_method_exact() {
    // Arrange & Act
    let config = CDSTranchePricerConfig {
        hetero_method: HeteroMethod::ExactConvolution,
        ..Default::default()
    };

    // Assert
    assert!(matches!(
        config.hetero_method,
        HeteroMethod::ExactConvolution
    ));
}

#[test]
fn test_custom_config_grid_step() {
    // Arrange & Act
    let config = CDSTranchePricerConfig {
        grid_step: 0.0005,
        ..Default::default()
    };

    // Assert
    assert_eq!(config.grid_step, 0.0005);
}

// ==================== Configuration Clone Tests ====================

#[test]
fn test_config_cloneable() {
    // Arrange
    let config1 = CDSTranchePricerConfig::default();

    // Act
    let config2 = config1.clone();

    // Assert
    assert_eq!(config2.quadrature_order, config1.quadrature_order);
    assert_eq!(config2.min_correlation, config1.min_correlation);
    assert_eq!(config2.cs01_bump_size, config1.cs01_bump_size);
}

#[test]
fn test_config_independent_after_clone() {
    // Arrange
    let mut config1 = CDSTranchePricerConfig::default();
    let config2 = config1.clone();

    // Act
    config1.quadrature_order = 10;

    // Assert
    assert_eq!(config1.quadrature_order, 10);
    assert_eq!(
        config2.quadrature_order, 7,
        "Cloned config should not be affected"
    );
}
