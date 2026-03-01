"""Extension system for statements crate."""

from typing import Optional, Dict, Any, List, Tuple
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

class AccountType:
    """Balance sheet account type for corkscrew analysis."""

    ASSET: AccountType
    LIABILITY: AccountType
    EQUITY: AccountType

    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...

class CorkscrewAccount:
    """Configuration for a single corkscrew account.

    Defines balance sheet account to validate roll-forward.

    Parameters
    ----------
    node_id : str
        Node ID for the balance account.
    account_type : AccountType
        Account type (Asset, Liability, or Equity).
    changes : list[str], optional
        Node IDs representing changes to the balance.
    beginning_balance_node : str, optional
        Node ID for beginning balance override.
    """

    def __init__(
        self,
        node_id: str,
        account_type: AccountType,
        changes: Optional[List[str]] = None,
        beginning_balance_node: Optional[str] = None,
    ) -> None: ...

    @property
    def node_id(self) -> str: ...
    @property
    def account_type(self) -> AccountType: ...
    @property
    def changes(self) -> List[str]: ...
    @property
    def beginning_balance_node(self) -> Optional[str]: ...
    def __repr__(self) -> str: ...

class CorkscrewConfig:
    """Configuration for corkscrew analysis.

    Defines accounts and validation parameters for balance sheet roll-forward.

    Parameters
    ----------
    accounts : list[CorkscrewAccount], optional
        List of balance sheet accounts to validate.
    tolerance : float, optional
        Tolerance for rounding differences (default: 0.01).
    fail_on_error : bool, optional
        Whether to fail on inconsistencies (default: False).
    """

    def __init__(
        self,
        accounts: Optional[List[CorkscrewAccount]] = None,
        tolerance: Optional[float] = None,
        fail_on_error: Optional[bool] = None,
    ) -> None: ...

    @property
    def accounts(self) -> List[CorkscrewAccount]: ...
    @property
    def tolerance(self) -> float: ...
    @property
    def fail_on_error(self) -> bool: ...
    def to_json(self) -> str: ...
    @classmethod
    def from_json(cls, json_str: str) -> CorkscrewConfig: ...
    def __repr__(self) -> str: ...

class ScorecardMetric:
    """Definition of a scorecard metric.

    Defines metric calculation formula, weight, and rating thresholds.

    Parameters
    ----------
    name : str
        Metric name.
    formula : str
        Formula to calculate the metric (DSL syntax).
    weight : float, optional
        Weight in overall score (0.0 to 1.0, default: 1.0).
    thresholds : dict[str, tuple[float, float]], optional
        Rating thresholds: rating -> (min, max).
    description : str, optional
        Metric description.
    """

    def __init__(
        self,
        name: str,
        formula: str,
        weight: Optional[float] = None,
        thresholds: Optional[Dict[str, Tuple[float, float]]] = None,
        description: Optional[str] = None,
    ) -> None: ...

    @property
    def name(self) -> str: ...
    @property
    def formula(self) -> str: ...
    @property
    def weight(self) -> float: ...
    @property
    def thresholds(self) -> Dict[str, Tuple[float, float]]: ...
    @property
    def description(self) -> Optional[str]: ...
    def __repr__(self) -> str: ...

class ScorecardConfig:
    """Configuration for credit scorecard analysis.

    Defines rating scale, metrics, and thresholds for credit rating assignment.

    Parameters
    ----------
    rating_scale : str, optional
        Rating scale to use (default: "S&P").
    metrics : list[ScorecardMetric], optional
        List of metrics to evaluate.
    min_rating : str, optional
        Minimum acceptable rating.
    """

    def __init__(
        self,
        rating_scale: Optional[str] = None,
        metrics: Optional[List[ScorecardMetric]] = None,
        min_rating: Optional[str] = None,
    ) -> None: ...

    @property
    def rating_scale(self) -> str: ...
    @property
    def metrics(self) -> List[ScorecardMetric]: ...
    @property
    def min_rating(self) -> Optional[str]: ...
    def to_json(self) -> str: ...
    @classmethod
    def from_json(cls, json_str: str) -> ScorecardConfig: ...
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

    @classmethod
    def with_config(cls, config: CorkscrewConfig) -> CorkscrewExtension:
        """Create a corkscrew extension with the given configuration.

        Parameters
        ----------
        config : CorkscrewConfig
            Extension configuration.

        Returns
        -------
        CorkscrewExtension
            Configured extension instance.
        """
        ...

    def set_config(self, config: CorkscrewConfig) -> None:
        """Set the extension configuration.

        Parameters
        ----------
        config : CorkscrewConfig
            New configuration to assign.
        """
        ...

    def config(self) -> Optional[CorkscrewConfig]:
        """Get the current configuration.

        Returns
        -------
        CorkscrewConfig or None
            Current configuration if set, None otherwise.
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

    @classmethod
    def with_config(cls, config: ScorecardConfig) -> CreditScorecardExtension:
        """Create a credit scorecard extension with the given configuration.

        Parameters
        ----------
        config : ScorecardConfig
            Extension configuration.

        Returns
        -------
        CreditScorecardExtension
            Configured extension instance.
        """
        ...

    def set_config(self, config: ScorecardConfig) -> None:
        """Set the extension configuration.

        Parameters
        ----------
        config : ScorecardConfig
            New configuration to assign.
        """
        ...

    def config(self) -> Optional[ScorecardConfig]:
        """Get the current configuration.

        Returns
        -------
        ScorecardConfig or None
            Current configuration if set, None otherwise.
        """
        ...

    def __repr__(self) -> str: ...
