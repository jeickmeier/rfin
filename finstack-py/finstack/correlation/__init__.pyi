"""Correlation infrastructure for credit portfolio modeling.

Copula models, factor models, recovery models, and linear-algebra utilities
used across CDS tranche pricing, structured credit, and portfolio credit risk.
"""

from __future__ import annotations

from .copulas import (
    GaussianCopula,
    StudentTCopula,
    MultiFactorCopula,
    RandomFactorLoadingCopula,
    CopulaSpec,
)
from .factor_models import (
    SingleFactorModel,
    TwoFactorModel,
    MultiFactorModel,
    FactorSpec,
)
from .recovery import (
    ConstantRecovery,
    CorrelatedRecovery,
    RecoverySpec,
)
from .utils import (
    CorrelatedBernoulli,
    validate_correlation_matrix,
    cholesky_decompose,
    correlation_bounds,
    joint_probabilities,
)

__all__ = [
    "GaussianCopula",
    "StudentTCopula",
    "MultiFactorCopula",
    "RandomFactorLoadingCopula",
    "CopulaSpec",
    "SingleFactorModel",
    "TwoFactorModel",
    "MultiFactorModel",
    "FactorSpec",
    "ConstantRecovery",
    "CorrelatedRecovery",
    "RecoverySpec",
    "CorrelatedBernoulli",
    "validate_correlation_matrix",
    "cholesky_decompose",
    "correlation_bounds",
    "joint_probabilities",
]
