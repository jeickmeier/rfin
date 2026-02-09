"""Extension system for statements crate."""

from typing import Optional, Dict, Any
from ..types.model import FinancialModelSpec
from ..evaluator import StatementResult

class ExtensionMetadata:
    """Extension metadata."""

    def __init__(
        self, name: str, version: str, description: Optional[str] = None, author: Optional[str] = None
    ) -> None:
        """Create extension metadata.

        Args:
            name: Unique extension name
            version: Semantic version
            description: Human-readable description
            author: Extension author

        Returns:
            ExtensionMetadata: Metadata instance
        """
        ...

    @property
    def name(self) -> str: ...
    @property
    def version(self) -> str: ...
    @property
    def description(self) -> Optional[str]: ...
    @property
    def author(self) -> Optional[str]: ...
    def __repr__(self) -> str: ...

class ExtensionStatus:
    """Extension execution status."""

    # Class attributes
    SUCCESS: ExtensionStatus
    FAILED: ExtensionStatus
    NOT_IMPLEMENTED: ExtensionStatus
    SKIPPED: ExtensionStatus

    def __repr__(self) -> str: ...

class ExtensionResult:
    """Extension execution result."""

    @staticmethod
    def success(message: str) -> ExtensionResult:
        """Create a success result.

        Args:
            message: Success message

        Returns:
            ExtensionResult: Success result
        """
        ...

    @staticmethod
    def failure(message: str) -> ExtensionResult:
        """Create a failure result.

        Args:
            message: Failure message

        Returns:
            ExtensionResult: Failure result
        """
        ...

    @staticmethod
    def skipped(message: str) -> ExtensionResult:
        """Create a skipped result.

        Args:
            message: Skip reason

        Returns:
            ExtensionResult: Skipped result
        """
        ...

    @property
    def status(self) -> ExtensionStatus: ...
    @property
    def message(self) -> str: ...
    @property
    def data(self) -> Dict[str, Any]: ...
    def __repr__(self) -> str: ...

class ExtensionContext:
    """Extension context.

    Context passed to extensions during execution.
    """

    @property
    def model(self) -> FinancialModelSpec: ...
    @property
    def results(self) -> StatementResult: ...
    @property
    def config(self) -> Any: ...

class ExtensionRegistry:
    """Extension registry.

    Manages and executes extensions for financial models.
    """

    @classmethod
    def new(cls) -> ExtensionRegistry:
        """Create a new extension registry.

        Returns:
            ExtensionRegistry: Registry instance
        """
        ...

    def execute_all(self, model: FinancialModelSpec, results: StatementResult) -> Dict[str, ExtensionResult]:
        """Execute all registered extensions.

        Args:
            model: Financial model
            results: Evaluation results

        Returns:
            dict[str, ExtensionResult]: Map of extension name to result
        """
        ...

    def __repr__(self) -> str: ...

class CorkscrewExtension:
    """Corkscrew extension for balance sheet roll-forward validation.

    CorkscrewExtension validates that balance sheet accounts properly roll
    forward according to the accounting identity:
    Ending Balance = Beginning Balance + Additions - Reductions

    This extension is used to ensure balance sheet articulation and detect
    modeling errors in financial statement models.

    Examples
    --------
    Instantiate the extension:

        >>> from finstack.statements.extensions import CorkscrewExtension
        >>> corkscrew = CorkscrewExtension.new()
        >>> print(repr(corkscrew))
        CorkscrewExtension()

    Notes
    -----
    - Validates balance sheet roll-forward logic
    - Detects modeling errors and inconsistencies
    - Works with balance sheet nodes (assets, liabilities, equity)
    - Returns ExtensionResult with validation status

    See Also
    --------
    :class:`ExtensionRegistry`: Extension management
    :class:`CreditScorecardExtension`: Credit rating extension
    """

    @classmethod
    def new(cls) -> CorkscrewExtension:
        """Create a corkscrew extension with default configuration.

        Returns:
            CorkscrewExtension: Extension instance
        """
        ...

    def __repr__(self) -> str: ...

class CreditScorecardExtension:
    """Credit scorecard extension for rating assignment.

    CreditScorecardExtension assigns credit ratings to entities based on
    financial ratios and predefined thresholds. It evaluates financial
    metrics (leverage, coverage, profitability) and maps them to credit
    ratings using configurable scorecard logic.

    Examples
    --------
    Instantiate the extension:

        >>> from finstack.statements.extensions import CreditScorecardExtension
        >>> scorecard = CreditScorecardExtension.new()
        >>> print(repr(scorecard))
        CreditScorecardExtension()

    Notes
    -----
    - Assigns credit ratings based on financial ratios
    - Uses configurable thresholds and scorecard logic
    - Evaluates leverage, coverage, profitability metrics
    - Returns ExtensionResult with assigned rating

    See Also
    --------
    :class:`ExtensionRegistry`: Extension management
    :class:`CorkscrewExtension`: Balance sheet validation
    """

    @classmethod
    def new(cls) -> CreditScorecardExtension:
        """Create a credit scorecard extension with default configuration.

        Returns:
            CreditScorecardExtension: Extension instance
        """
        ...

    def __repr__(self) -> str: ...
