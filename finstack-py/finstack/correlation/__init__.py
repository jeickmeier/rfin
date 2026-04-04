"""Correlation infrastructure for credit portfolio modeling (Rust bindings).

This package re-exports the Rust extension module types for copula models,
factor models, recovery models, and correlation utilities.
"""

from __future__ import annotations

from finstack import finstack as _finstack

_rust = _finstack.correlation

# Copulas
GaussianCopula = _rust.GaussianCopula
StudentTCopula = _rust.StudentTCopula
MultiFactorCopula = _rust.MultiFactorCopula
RandomFactorLoadingCopula = _rust.RandomFactorLoadingCopula
CopulaSpec = _rust.CopulaSpec

# Factor models
SingleFactorModel = _rust.SingleFactorModel
TwoFactorModel = _rust.TwoFactorModel
MultiFactorModel = _rust.MultiFactorModel
FactorSpec = _rust.FactorSpec

# Recovery models
ConstantRecovery = _rust.ConstantRecovery
CorrelatedRecovery = _rust.CorrelatedRecovery
RecoverySpec = _rust.RecoverySpec

# Utilities
CorrelatedBernoulli = _rust.CorrelatedBernoulli
validate_correlation_matrix = _rust.validate_correlation_matrix
cholesky_decompose = _rust.cholesky_decompose
correlation_bounds = _rust.correlation_bounds
joint_probabilities = _rust.joint_probabilities

__all__ = [
    # Recovery models
    "ConstantRecovery",
    "CopulaSpec",
    # Utilities
    "CorrelatedBernoulli",
    "CorrelatedRecovery",
    "FactorSpec",
    # Copulas
    "GaussianCopula",
    "MultiFactorCopula",
    "MultiFactorModel",
    "RandomFactorLoadingCopula",
    "RecoverySpec",
    # Factor models
    "SingleFactorModel",
    "StudentTCopula",
    "TwoFactorModel",
    "cholesky_decompose",
    "correlation_bounds",
    "joint_probabilities",
    "validate_correlation_matrix",
]
