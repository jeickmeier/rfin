"""Correlation infrastructure: copulas, factor models, recovery models.

Bindings for the ``finstack-correlation`` Rust crate.
"""

from __future__ import annotations

from finstack.finstack import correlation as _corr

CopulaSpec = _corr.CopulaSpec
Copula = _corr.Copula
RecoverySpec = _corr.RecoverySpec
RecoveryModel = _corr.RecoveryModel
FactorSpec = _corr.FactorSpec
FactorModel = _corr.FactorModel
SingleFactorModel = _corr.SingleFactorModel
TwoFactorModel = _corr.TwoFactorModel
MultiFactorModel = _corr.MultiFactorModel
CorrelatedBernoulli = _corr.CorrelatedBernoulli
correlation_bounds = _corr.correlation_bounds
joint_probabilities = _corr.joint_probabilities
validate_correlation_matrix = _corr.validate_correlation_matrix
cholesky_decompose = _corr.cholesky_decompose

__all__: list[str] = [
    "Copula",
    "CopulaSpec",
    "CorrelatedBernoulli",
    "FactorModel",
    "FactorSpec",
    "MultiFactorModel",
    "RecoveryModel",
    "RecoverySpec",
    "SingleFactorModel",
    "TwoFactorModel",
    "cholesky_decompose",
    "correlation_bounds",
    "joint_probabilities",
    "validate_correlation_matrix",
]
