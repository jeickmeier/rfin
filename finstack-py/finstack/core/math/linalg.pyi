"""Linear algebra utilities."""

from __future__ import annotations
from typing import List, Sequence, Tuple

from finstack import ParameterError

class CholeskyError(ParameterError): ...

SINGULAR_THRESHOLD: float
DIAGONAL_TOLERANCE: float
SYMMETRY_TOLERANCE: float

def cholesky_decomposition(matrix: List[List[float]]) -> List[List[float]]: ...
def cholesky_solve(cholesky: List[List[float]], b: List[float]) -> List[float]: ...
def validate_correlation_matrix(matrix: List[List[float]]) -> bool: ...
def apply_correlation(cholesky: List[List[float]], independent: List[float]) -> List[float]: ...
def build_correlation_matrix(
    n: int,
    correlations: Sequence[Tuple[int, int, float]],
) -> List[List[float]]: ...
