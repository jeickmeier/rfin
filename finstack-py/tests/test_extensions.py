"""Tests for statement extensions framework."""

from finstack.statements.extensions import (
    AccountType,
    CorkscrewAccount,
    CorkscrewConfig,
    CorkscrewExtension,
    CreditScorecardExtension,
    ExtensionMetadata,
    ExtensionRegistry,
    ExtensionResult,
    ExtensionStatus,
    ScorecardConfig,
    ScorecardMetric,
)


class TestAccountType:
    """Test AccountType enum."""

    def test_constants_exist(self) -> None:
        """Test that all account type constants exist."""
        assert hasattr(AccountType, "ASSET")
        assert hasattr(AccountType, "LIABILITY")
        assert hasattr(AccountType, "EQUITY")

    def test_repr(self) -> None:
        """Test string representation."""
        assert "AccountType" in repr(AccountType.ASSET)
        assert "Asset" in repr(AccountType.ASSET)


class TestCorkscrewAccount:
    """Test CorkscrewAccount configuration."""

    def test_create_minimal(self) -> None:
        """Test creating account with minimal parameters."""
        account = CorkscrewAccount("cash", AccountType.ASSET)
        assert account.node_id == "cash"
        assert account.account_type == AccountType.ASSET
        assert account.changes == []
        assert account.beginning_balance_node is None

    def test_create_with_changes(self) -> None:
        """Test creating account with changes."""
        account = CorkscrewAccount("cash", AccountType.ASSET, changes=["cash_inflows", "cash_outflows"])
        assert account.changes == ["cash_inflows", "cash_outflows"]

    def test_create_with_beginning_balance(self) -> None:
        """Test creating account with beginning balance override."""
        account = CorkscrewAccount("cash", AccountType.ASSET, beginning_balance_node="opening_cash")
        assert account.beginning_balance_node == "opening_cash"

    def test_repr(self) -> None:
        """Test string representation."""
        account = CorkscrewAccount("cash", AccountType.ASSET)
        repr_str = repr(account)
        assert "CorkscrewAccount" in repr_str
        assert "cash" in repr_str


class TestCorkscrewConfig:
    """Test CorkscrewConfig configuration."""

    def test_create_default(self) -> None:
        """Test creating config with defaults."""
        config = CorkscrewConfig()
        assert config.accounts == []
        assert config.tolerance == 0.01
        assert config.fail_on_error is False

    def test_create_with_accounts(self) -> None:
        """Test creating config with accounts."""
        accounts = [
            CorkscrewAccount("cash", AccountType.ASSET, changes=["cash_inflows"]),
            CorkscrewAccount("debt", AccountType.LIABILITY, changes=["debt_issuance"]),
        ]
        config = CorkscrewConfig(accounts=accounts)
        assert len(config.accounts) == 2
        assert config.accounts[0].node_id == "cash"
        assert config.accounts[1].node_id == "debt"

    def test_create_with_tolerance(self) -> None:
        """Test creating config with custom tolerance."""
        config = CorkscrewConfig(tolerance=0.001)
        assert config.tolerance == 0.001

    def test_create_with_fail_on_error(self) -> None:
        """Test creating config with fail_on_error."""
        config = CorkscrewConfig(fail_on_error=True)
        assert config.fail_on_error is True

    def test_repr(self) -> None:
        """Test string representation."""
        config = CorkscrewConfig(tolerance=0.01)
        repr_str = repr(config)
        assert "CorkscrewConfig" in repr_str
        assert "0.01" in repr_str

    def test_to_json(self) -> None:
        """Test JSON serialization."""
        accounts = [CorkscrewAccount("cash", AccountType.ASSET, changes=["cash_inflows"])]
        config = CorkscrewConfig(accounts=accounts, tolerance=0.01)
        json_str = config.to_json()
        assert isinstance(json_str, str)
        assert "cash" in json_str
        assert "0.01" in json_str

    def test_from_json(self) -> None:
        """Test JSON deserialization."""
        json_str = """
        {
            "accounts": [
                {
                    "node_id": "cash",
                    "account_type": "asset",
                    "changes": ["cash_inflows", "cash_outflows"]
                }
            ],
            "tolerance": 0.01,
            "fail_on_error": false
        }
        """
        config = CorkscrewConfig.from_json(json_str)
        assert len(config.accounts) == 1
        assert config.accounts[0].node_id == "cash"
        assert config.tolerance == 0.01

    def test_json_roundtrip(self) -> None:
        """Test JSON serialization roundtrip."""
        accounts = [
            CorkscrewAccount("cash", AccountType.ASSET, changes=["cash_inflows"]),
            CorkscrewAccount("debt", AccountType.LIABILITY, changes=["debt_issuance"]),
        ]
        config1 = CorkscrewConfig(accounts=accounts, tolerance=0.001, fail_on_error=True)
        json_str = config1.to_json()
        config2 = CorkscrewConfig.from_json(json_str)

        assert len(config2.accounts) == 2
        assert config2.tolerance == 0.001
        assert config2.fail_on_error is True


class TestScorecardMetric:
    """Test ScorecardMetric configuration."""

    def test_create_minimal(self) -> None:
        """Test creating metric with minimal parameters."""
        metric = ScorecardMetric("debt_to_ebitda", "total_debt / ebitda")
        assert metric.name == "debt_to_ebitda"
        assert metric.formula == "total_debt / ebitda"
        assert metric.weight == 1.0
        assert metric.description is None

    def test_create_with_weight(self) -> None:
        """Test creating metric with custom weight."""
        metric = ScorecardMetric("debt_to_ebitda", "total_debt / ebitda", weight=0.3)
        assert metric.weight == 0.3

    def test_create_with_thresholds(self) -> None:
        """Test creating metric with rating thresholds."""
        thresholds = {
            "AAA": (0.0, 1.0),
            "AA": (1.0, 2.0),
            "A": (2.0, 3.0),
        }
        metric = ScorecardMetric("debt_to_ebitda", "total_debt / ebitda", thresholds=thresholds)
        retrieved_thresholds = metric.thresholds
        assert retrieved_thresholds["AAA"] == (0.0, 1.0)
        assert retrieved_thresholds["AA"] == (1.0, 2.0)
        assert retrieved_thresholds["A"] == (2.0, 3.0)

    def test_create_with_description(self) -> None:
        """Test creating metric with description."""
        metric = ScorecardMetric(
            "debt_to_ebitda",
            "total_debt / ebitda",
            description="Measures leverage",
        )
        assert metric.description == "Measures leverage"

    def test_repr(self) -> None:
        """Test string representation."""
        metric = ScorecardMetric("debt_to_ebitda", "total_debt / ebitda", weight=0.3)
        repr_str = repr(metric)
        assert "ScorecardMetric" in repr_str
        assert "debt_to_ebitda" in repr_str
        assert "0.3" in repr_str


class TestScorecardConfig:
    """Test ScorecardConfig configuration."""

    def test_create_default(self) -> None:
        """Test creating config with defaults."""
        config = ScorecardConfig()
        assert config.rating_scale == "S&P"
        assert config.metrics == []
        assert config.min_rating is None

    def test_create_with_rating_scale(self) -> None:
        """Test creating config with custom rating scale."""
        config = ScorecardConfig(rating_scale="Moody's")
        assert config.rating_scale == "Moody's"

    def test_create_with_metrics(self) -> None:
        """Test creating config with metrics."""
        metrics = [
            ScorecardMetric("debt_to_ebitda", "total_debt / ttm(ebitda)", weight=0.3),
            ScorecardMetric("interest_coverage", "ebitda / interest_expense", weight=0.25),
        ]
        config = ScorecardConfig(metrics=metrics)
        assert len(config.metrics) == 2
        assert config.metrics[0].name == "debt_to_ebitda"
        assert config.metrics[1].name == "interest_coverage"

    def test_create_with_min_rating(self) -> None:
        """Test creating config with minimum rating."""
        config = ScorecardConfig(min_rating="BB")
        assert config.min_rating == "BB"

    def test_repr(self) -> None:
        """Test string representation."""
        config = ScorecardConfig(rating_scale="S&P")
        repr_str = repr(config)
        assert "ScorecardConfig" in repr_str
        assert "S&P" in repr_str

    def test_to_json(self) -> None:
        """Test JSON serialization."""
        metrics = [ScorecardMetric("debt_to_ebitda", "total_debt / ebitda", weight=0.3)]
        config = ScorecardConfig(rating_scale="S&P", metrics=metrics)
        json_str = config.to_json()
        assert isinstance(json_str, str)
        assert "S&P" in json_str
        assert "debt_to_ebitda" in json_str

    def test_from_json(self) -> None:
        """Test JSON deserialization."""
        json_str = """
        {
            "rating_scale": "S&P",
            "metrics": [
                {
                    "name": "debt_to_ebitda",
                    "formula": "total_debt / ttm(ebitda)",
                    "weight": 0.3,
                    "thresholds": {
                        "AAA": [0.0, 1.0],
                        "AA": [1.0, 2.0]
                    }
                }
            ],
            "min_rating": "BB"
        }
        """
        config = ScorecardConfig.from_json(json_str)
        assert config.rating_scale == "S&P"
        assert len(config.metrics) == 1
        assert config.metrics[0].name == "debt_to_ebitda"
        assert config.min_rating == "BB"

    def test_json_roundtrip(self) -> None:
        """Test JSON serialization roundtrip."""
        metrics = [
            ScorecardMetric("debt_to_ebitda", "total_debt / ebitda", weight=0.3),
            ScorecardMetric("interest_coverage", "ebitda / interest", weight=0.25),
        ]
        config1 = ScorecardConfig(rating_scale="Moody's", metrics=metrics, min_rating="Baa")
        json_str = config1.to_json()
        config2 = ScorecardConfig.from_json(json_str)

        assert config2.rating_scale == "Moody's"
        assert len(config2.metrics) == 2
        assert config2.min_rating == "Baa"


class TestCorkscrewExtension:
    """Test CorkscrewExtension."""

    def test_create_default(self) -> None:
        """Test creating extension with default config."""
        extension = CorkscrewExtension.new()
        assert extension.config() is None

    def test_create_with_config(self) -> None:
        """Test creating extension with configuration."""
        config = CorkscrewConfig(tolerance=0.001)
        extension = CorkscrewExtension.with_config(config)
        retrieved_config = extension.config()
        assert retrieved_config is not None
        assert retrieved_config.tolerance == 0.001

    def test_set_config(self) -> None:
        """Test setting extension configuration."""
        extension = CorkscrewExtension.new()
        assert extension.config() is None

        config = CorkscrewConfig(tolerance=0.001)
        extension.set_config(config)
        retrieved_config = extension.config()
        assert retrieved_config is not None
        assert retrieved_config.tolerance == 0.001

    def test_repr_no_config(self) -> None:
        """Test string representation without config."""
        extension = CorkscrewExtension.new()
        repr_str = repr(extension)
        assert "CorkscrewExtension" in repr_str

    def test_repr_with_config(self) -> None:
        """Test string representation with config."""
        accounts = [CorkscrewAccount("cash", AccountType.ASSET, changes=["cash_inflows"])]
        config = CorkscrewConfig(accounts=accounts, tolerance=0.01)
        extension = CorkscrewExtension.with_config(config)
        repr_str = repr(extension)
        assert "CorkscrewExtension" in repr_str
        assert "0.01" in repr_str


class TestCreditScorecardExtension:
    """Test CreditScorecardExtension."""

    def test_create_default(self) -> None:
        """Test creating extension with default config."""
        extension = CreditScorecardExtension.new()
        assert extension.config() is None

    def test_create_with_config(self) -> None:
        """Test creating extension with configuration."""
        config = ScorecardConfig(rating_scale="Moody's")
        extension = CreditScorecardExtension.with_config(config)
        retrieved_config = extension.config()
        assert retrieved_config is not None
        assert retrieved_config.rating_scale == "Moody's"

    def test_set_config(self) -> None:
        """Test setting extension configuration."""
        extension = CreditScorecardExtension.new()
        assert extension.config() is None

        config = ScorecardConfig(rating_scale="Fitch")
        extension.set_config(config)
        retrieved_config = extension.config()
        assert retrieved_config is not None
        assert retrieved_config.rating_scale == "Fitch"

    def test_repr_no_config(self) -> None:
        """Test string representation without config."""
        extension = CreditScorecardExtension.new()
        repr_str = repr(extension)
        assert "CreditScorecardExtension" in repr_str

    def test_repr_with_config(self) -> None:
        """Test string representation with config."""
        metrics = [ScorecardMetric("debt_to_ebitda", "total_debt / ebitda", weight=0.3)]
        config = ScorecardConfig(rating_scale="S&P", metrics=metrics)
        extension = CreditScorecardExtension.with_config(config)
        repr_str = repr(extension)
        assert "CreditScorecardExtension" in repr_str
        assert "S&P" in repr_str


class TestExtensionRegistry:
    """Test ExtensionRegistry."""

    def test_create_registry(self) -> None:
        """Test creating extension registry."""
        registry = ExtensionRegistry.new()
        assert registry is not None

    # Note: We can't test register() or execute_all() here because they require
    # actual model and results objects. Those tests should be in integration tests.


class TestExtensionStatus:
    """Test ExtensionStatus enum."""

    def test_constants_exist(self) -> None:
        """Test that all status constants exist."""
        assert hasattr(ExtensionStatus, "SUCCESS")
        assert hasattr(ExtensionStatus, "FAILED")
        assert hasattr(ExtensionStatus, "NOT_IMPLEMENTED")
        assert hasattr(ExtensionStatus, "SKIPPED")

    def test_repr(self) -> None:
        """Test string representation."""
        assert "ExtensionStatus" in repr(ExtensionStatus.SUCCESS)


class TestExtensionResult:
    """Test ExtensionResult."""

    def test_create_success(self) -> None:
        """Test creating success result."""
        result = ExtensionResult.success("Validation passed")
        assert result.status == ExtensionStatus.SUCCESS
        assert result.message == "Validation passed"

    def test_create_failure(self) -> None:
        """Test creating failure result."""
        result = ExtensionResult.failure("Validation failed")
        assert result.status == ExtensionStatus.FAILED
        assert result.message == "Validation failed"

    def test_create_skipped(self) -> None:
        """Test creating skipped result."""
        result = ExtensionResult.skipped("Extension disabled")
        assert result.status == ExtensionStatus.SKIPPED
        assert result.message == "Extension disabled"

    def test_repr(self) -> None:
        """Test string representation."""
        result = ExtensionResult.success("Test")
        repr_str = repr(result)
        assert "ExtensionResult" in repr_str


class TestExtensionMetadata:
    """Test ExtensionMetadata."""

    def test_create_minimal(self) -> None:
        """Test creating metadata with minimal parameters."""
        metadata = ExtensionMetadata("test_extension", "1.0.0")
        assert metadata.name == "test_extension"
        assert metadata.version == "1.0.0"
        assert metadata.description is None
        assert metadata.author is None

    def test_create_with_description(self) -> None:
        """Test creating metadata with description."""
        metadata = ExtensionMetadata("test_extension", "1.0.0", description="Test extension")
        assert metadata.description == "Test extension"

    def test_create_with_author(self) -> None:
        """Test creating metadata with author."""
        metadata = ExtensionMetadata("test_extension", "1.0.0", author="Test Author")
        assert metadata.author == "Test Author"

    def test_repr(self) -> None:
        """Test string representation."""
        metadata = ExtensionMetadata("test_extension", "1.0.0")
        repr_str = repr(metadata)
        assert "ExtensionMetadata" in repr_str
        assert "test_extension" in repr_str
        assert "1.0.0" in repr_str


# Integration tests would go here, but they require actual models and results
# which would make the test suite too complex for unit tests.
# See examples/statements/extensions_example.py for working integration examples.
