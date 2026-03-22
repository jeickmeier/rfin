//! Corkscrew analysis extension.
//!
//! This extension provides roll-forward validation for balance sheet accounts, ensuring
//! that opening balances + changes = closing balances across periods.
//!
//! **Status:** ✅ Fully implemented with comprehensive validation logic.
//!
//! # Features
//!
//! - ✅ Validate balance sheet articulation (Assets = Liabilities + Equity)
//! - ✅ Track roll-forward schedules (beginning balance → changes → ending balance)
//! - ✅ Detect inconsistencies in period-to-period transitions
//! - ✅ Support for multiple balance sheet sections (assets, liabilities, equity)
//! - ✅ Configurable tolerance for rounding differences
//! - ✅ Optional fail-on-error mode for strict validation
//!
//! # Configuration Schema
//!
//! ```json
//! {
//!   "accounts": [
//!     {
//!       "node_id": "cash",
//!       "account_type": "asset",
//!       "changes": ["cash_inflows", "cash_outflows"]
//!     },
//!     {
//!       "node_id": "debt",
//!       "account_type": "liability",
//!       "changes": ["debt_issuance", "debt_repayment"]
//!     }
//!   ],
//!   "tolerance": 0.01
//! }
//! ```
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use finstack_statements_analytics::extensions::CorkscrewExtension;
//! use finstack_statements::extensions::{ExtensionRegistry, ExtensionContext};
//!
//! # fn main() -> finstack_statements::Result<()> {
//! # let context: ExtensionContext = unimplemented!("build ExtensionContext from a model and StatementResult");
//! let config = serde_json::json!({
//!   "accounts": [{"node_id": "cash", "account_type": "asset"}]
//! });
//! let mut registry = ExtensionRegistry::new();
//! registry.register(Box::new(CorkscrewExtension::new()))?;
//! let results = registry.execute("corkscrew", &context.with_config(&config))?;
//! # let _ = results;
//! # Ok(())
//! # }
//! ```

use finstack_statements::extensions::{Extension, ExtensionContext, ExtensionMetadata, ExtensionResult};
use finstack_statements::Result;
use serde::{Deserialize, Serialize};

/// Corkscrew analysis extension for balance sheet roll-forward validation.
///
/// **Features:**
/// - Validates period-to-period balance roll-forwards
/// - Checks balance sheet articulation (Assets = Liabilities + Equity)
/// - Configurable tolerance for rounding differences
/// - Detailed validation reports with errors and warnings
pub struct CorkscrewExtension {
    /// Extension configuration
    config: Option<CorkscrewConfig>,
}

/// Configuration for corkscrew analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorkscrewConfig {
    /// List of balance sheet accounts to validate
    #[serde(default)]
    pub accounts: Vec<CorkscrewAccount>,

    /// Tolerance for rounding differences (default: 0.01)
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,

    /// Whether to fail on inconsistencies (default: false)
    #[serde(default)]
    pub fail_on_error: bool,
}

/// Configuration for a single corkscrew account.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorkscrewAccount {
    /// Node ID for the balance account
    pub node_id: String,

    /// Account type (asset, liability, equity)
    pub account_type: AccountType,

    /// Node IDs representing changes to the balance
    #[serde(default)]
    pub changes: Vec<String>,

    /// Optional: Node ID for beginning balance override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beginning_balance_node: Option<String>,
}

/// Type of balance sheet account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    /// Asset account
    Asset,
    /// Liability account
    Liability,
    /// Equity account
    Equity,
}

/// Default tolerance for corkscrew validation (basis points).
///
/// Set to 0.01 (1 cent or 1 basis point) to accommodate normal rounding differences
/// in financial calculations while catching meaningful discrepancies.
const DEFAULT_CORKSCREW_TOLERANCE: f64 = 0.01;

fn default_tolerance() -> f64 {
    DEFAULT_CORKSCREW_TOLERANCE
}

impl CorkscrewExtension {
    /// Create a new corkscrew extension with default configuration.
    ///
    /// # Example
    /// ```rust
    /// # use finstack_statements_analytics::extensions::CorkscrewExtension;
    /// let extension = CorkscrewExtension::new();
    /// assert!(extension.config().is_none());
    /// ```
    pub fn new() -> Self {
        Self { config: None }
    }

    /// Create a new corkscrew extension with the given configuration.
    ///
    /// # Arguments
    /// * `config` - Pre-built [`CorkscrewConfig`] describing the accounts to validate
    pub fn with_config(config: CorkscrewConfig) -> Self {
        Self {
            config: Some(config),
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> Option<&CorkscrewConfig> {
        self.config.as_ref()
    }

    /// Set the configuration.
    ///
    /// # Arguments
    /// * `config` - New configuration to assign
    pub fn set_config(&mut self, config: CorkscrewConfig) {
        self.config = Some(config);
    }

    fn resolve_config<'a>(&'a self, context: &'a ExtensionContext) -> Result<CorkscrewConfig> {
        if let Some(config) = context.config {
            serde_json::from_value(config.clone()).map_err(|e| {
                finstack_statements::error::Error::invalid_input(format!(
                    "Invalid corkscrew configuration: {}",
                    e
                ))
            })
        } else {
            self.config.clone().ok_or_else(|| {
                finstack_statements::error::Error::registry("Corkscrew extension requires configuration")
            })
        }
    }

    /// Validate a single account's roll-forward schedule.
    fn validate_account(
        &self,
        account: &CorkscrewAccount,
        context: &ExtensionContext,
        tolerance: f64,
    ) -> Result<AccountValidation> {
        let mut validation = AccountValidation {
            account_id: account.node_id.clone(),
            account_type: format!("{:?}", account.account_type),
            periods_validated: 0,
            max_error: 0.0,
            is_valid: true,
        };

        // Get balance values from results
        let balance_values = context.results.nodes.get(&account.node_id).ok_or_else(|| {
            finstack_statements::error::Error::registry(format!(
                "Balance account '{}' not found in results",
                account.node_id
            ))
        })?;

        // Get change values and validate roll-forward
        let periods: Vec<_> = context.model.periods.iter().collect();

        for i in 1..periods.len() {
            let prev_period = &periods[i - 1].id;
            let curr_period = &periods[i].id;

            // Get previous and current balance
            let prev_balance = balance_values.get(prev_period).copied().unwrap_or(0.0);
            let curr_balance = balance_values.get(curr_period).copied().unwrap_or(0.0);

            // Calculate expected balance from changes
            let mut expected_balance = prev_balance;

            // Add changes for this period
            for change_node_id in &account.changes {
                if let Some(change_values) = context.results.nodes.get(change_node_id) {
                    if let Some(change) = change_values.get(curr_period) {
                        expected_balance += change;
                    }
                }
            }

            // Check if beginning balance override is used
            if let Some(beginning_node) = &account.beginning_balance_node {
                if let Some(beginning_values) = context.results.nodes.get(beginning_node) {
                    if let Some(beginning) = beginning_values.get(curr_period) {
                        expected_balance = beginning + expected_balance - prev_balance;
                    }
                }
            }

            // Validate the roll-forward using an absolute tolerance.
            let error = (curr_balance - expected_balance).abs();
            validation.max_error = validation.max_error.max(error);
            validation.periods_validated += 1;

            if error > tolerance {
                validation.is_valid = false;
            }
        }

        Ok(validation)
    }

    /// Check balance sheet articulation (A = L + E) using actual balances.
    ///
    /// Sums the most recent period's balance for each configured account,
    /// grouped by account type, and checks that Assets = Liabilities + Equity.
    /// Uses an absolute tolerance matching the configured rounding threshold.
    fn check_articulation(
        &self,
        context: &ExtensionContext,
        config: &CorkscrewConfig,
        tolerance: f64,
    ) -> Option<ArticulationResult> {
        let last_period = context.model.periods.last()?;
        let period_id = &last_period.id;

        let mut assets = 0.0;
        let mut liabilities = 0.0;
        let mut equity = 0.0;
        let mut has_balance_sheet = false;

        for account in &config.accounts {
            if let Some(node_values) = context.results.nodes.get(&account.node_id) {
                if let Some(balance) = node_values.get(period_id) {
                    has_balance_sheet = true;
                    match account.account_type {
                        AccountType::Asset => assets += balance,
                        AccountType::Liability => liabilities += balance,
                        AccountType::Equity => equity += balance,
                    }
                }
            }
        }

        if !has_balance_sheet {
            return None;
        }

        let imbalance = assets - (liabilities + equity);
        let is_balanced = imbalance.abs() <= tolerance;

        Some(ArticulationResult {
            total_imbalance: imbalance.abs(),
            is_balanced,
        })
    }
}

/// Result of validating a single account.
struct AccountValidation {
    account_id: String,
    account_type: String,
    periods_validated: usize,
    max_error: f64,
    is_valid: bool,
}

/// Result of checking balance sheet articulation.
struct ArticulationResult {
    total_imbalance: f64,
    is_balanced: bool,
}

impl Default for CorkscrewExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl Extension for CorkscrewExtension {
    fn metadata(&self) -> ExtensionMetadata {
        ExtensionMetadata {
            name: "corkscrew".into(),
            version: "0.1.0".into(),
            description: Some("Balance sheet roll-forward validation (corkscrew analysis)".into()),
            author: Some("Finstack Team".into()),
        }
    }

    fn execute(&mut self, context: &ExtensionContext) -> Result<ExtensionResult> {
        // Validate balance sheet roll-forward schedules
        let config = self.resolve_config(context)?;

        let mut validations = Vec::new();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Process each configured account
        for account in &config.accounts {
            match self.validate_account(account, context, config.tolerance) {
                Ok(validation) => validations.push(validation),
                Err(e) => {
                    if config.fail_on_error {
                        return Err(e);
                    } else {
                        errors.push(format!("Account '{}': {}", account.node_id, e));
                    }
                }
            }
        }

        // Check for balance sheet articulation using actual balances
        if let Some(articulation_result) =
            self.check_articulation(context, &config, config.tolerance)
        {
            if !articulation_result.is_balanced {
                let msg = format!(
                    "Balance sheet not articulated. Total imbalance: {:.2}",
                    articulation_result.total_imbalance
                );
                if config.fail_on_error {
                    errors.push(msg);
                } else {
                    warnings.push(msg);
                }
            }
        }

        // Build result
        let mut result = if errors.is_empty() {
            ExtensionResult::success(format!(
                "Corkscrew validation complete. {} accounts validated.",
                validations.len()
            ))
        } else {
            ExtensionResult::failure(format!(
                "Corkscrew validation failed with {} errors",
                errors.len()
            ))
        };

        // Add validation data
        result = result.with_data(
            "validations",
            serde_json::json!(validations
                .iter()
                .map(|v| {
                    serde_json::json!({
                        "account": v.account_id,
                        "type": v.account_type,
                        "periods_validated": v.periods_validated,
                        "max_error": v.max_error,
                        "is_valid": v.is_valid,
                    })
                })
                .collect::<Vec<_>>()),
        );

        // Add warnings and errors
        for warning in warnings {
            result = result.with_warning(warning);
        }
        for error in errors {
            result = result.with_error(error);
        }

        Ok(result)
    }

    fn is_enabled(&self) -> bool {
        // Extension is always available but returns NotImplemented
        true
    }

    fn config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "CorkscrewConfig",
            "type": "object",
            "properties": {
                "accounts": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["node_id", "account_type"],
                        "properties": {
                            "node_id": {
                                "type": "string",
                                "description": "Node ID for the balance account"
                            },
                            "account_type": {
                                "type": "string",
                                "enum": ["asset", "liability", "equity"],
                                "description": "Type of balance sheet account"
                            },
                            "changes": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Node IDs representing changes to the balance"
                            },
                            "beginning_balance_node": {
                                "type": "string",
                                "description": "Optional node ID for beginning balance override"
                            }
                        }
                    }
                },
                "tolerance": {
                    "type": "number",
                    "default": 0.01,
                    "description": "Tolerance for rounding differences"
                },
                "fail_on_error": {
                    "type": "boolean",
                    "default": false,
                    "description": "Whether to fail on inconsistencies"
                }
            }
        }))
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        // Validate configuration structure
        let _: CorkscrewConfig = serde_json::from_value(config.clone()).map_err(|e| {
            finstack_statements::error::Error::invalid_input(format!("Invalid corkscrew configuration: {}", e))
        })?;

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use finstack_statements::builder::ModelBuilder;
    use finstack_statements::evaluator::Evaluator;
    use finstack_statements::extensions::ExtensionStatus;
    use finstack_statements::types::AmountOrScalar;
    use finstack_core::dates::PeriodId;

    #[test]
    fn test_corkscrew_extension_creation() {
        let extension = CorkscrewExtension::new();
        let metadata = extension.metadata();

        assert_eq!(metadata.name, "corkscrew");
        assert_eq!(metadata.version, "0.1.0");
        assert!(extension.is_enabled());
    }

    #[test]
    fn test_corkscrew_extension_with_config() {
        let config = CorkscrewConfig {
            accounts: vec![CorkscrewAccount {
                node_id: "cash".into(),
                account_type: AccountType::Asset,
                changes: vec!["cash_inflows".into(), "cash_outflows".into()],
                beginning_balance_node: None,
            }],
            tolerance: 0.01,
            fail_on_error: false,
        };

        let extension = CorkscrewExtension::with_config(config);
        assert!(extension.config().is_some());
        assert_eq!(
            extension
                .config()
                .expect("test should succeed")
                .accounts
                .len(),
            1
        );
    }

    #[test]
    fn test_corkscrew_execute_requires_config() {
        use finstack_statements::evaluator::StatementResult;
        use finstack_statements::types::FinancialModelSpec;

        let model = FinancialModelSpec::new("test", Vec::new());
        let results = StatementResult::new();
        let context = ExtensionContext::new(&model, &results);

        let mut extension = CorkscrewExtension::new();
        // Without config, should return an error
        let result = extension.execute(&context);

        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("requires configuration"));
    }

    #[test]
    fn test_corkscrew_execute_accepts_runtime_context_config() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q1", None)
            .expect("valid periods")
            .value(
                "cash",
                &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0))],
            )
            .build()
            .expect("model should build");
        let mut evaluator = Evaluator::new();
        let results = evaluator
            .evaluate(&model)
            .expect("evaluation should succeed");

        let config = serde_json::json!({
            "accounts": [],
            "fail_on_error": false
        });
        let context = ExtensionContext::new(&model, &results).with_config(&config);

        let mut extension = CorkscrewExtension::new();
        let result = extension
            .execute(&context)
            .expect("runtime config should be accepted");
        assert_eq!(result.status, ExtensionStatus::Success);
    }

    #[test]
    fn test_corkscrew_config_schema() {
        let extension = CorkscrewExtension::new();
        let schema = extension.config_schema();

        assert!(schema.is_some());
        let schema_obj = schema.expect("test should succeed");
        assert!(schema_obj.get("properties").is_some());
    }

    #[test]
    fn test_corkscrew_config_validation() {
        let extension = CorkscrewExtension::new();

        let valid_config = serde_json::json!({
            "accounts": [
                {
                    "node_id": "cash",
                    "account_type": "asset",
                    "changes": ["inflows", "outflows"]
                }
            ],
            "tolerance": 0.01,
            "fail_on_error": false
        });

        assert!(extension.validate_config(&valid_config).is_ok());
    }

    #[test]
    fn test_corkscrew_config_validation_invalid() {
        let extension = CorkscrewExtension::new();

        let invalid_config = serde_json::json!({
            "accounts": "not_an_array"
        });

        assert!(extension.validate_config(&invalid_config).is_err());
    }

    #[test]
    fn test_account_type_serialization() {
        let account_type = AccountType::Asset;
        let json = serde_json::to_string(&account_type).expect("test should succeed");
        assert_eq!(json, r#""asset""#);

        let deserialized: AccountType = serde_json::from_str(&json).expect("test should succeed");
        assert_eq!(deserialized, AccountType::Asset);
    }

    #[test]
    fn test_corkscrew_uses_absolute_tolerance_for_articulation() {
        let model = ModelBuilder::new("articulation_tolerance")
            .periods("2025Q1..Q1", None)
            .expect("valid periods")
            .value(
                "assets",
                &[(
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(1_000_000.00),
                )],
            )
            .value(
                "liabilities",
                &[(
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(999_999.98),
                )],
            )
            .value(
                "equity",
                &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(0.0))],
            )
            .build()
            .expect("model should build");

        let mut evaluator = Evaluator::new();
        let results = evaluator
            .evaluate(&model)
            .expect("evaluation should succeed");

        let config = CorkscrewConfig {
            accounts: vec![
                CorkscrewAccount {
                    node_id: "assets".into(),
                    account_type: AccountType::Asset,
                    changes: vec![],
                    beginning_balance_node: None,
                },
                CorkscrewAccount {
                    node_id: "liabilities".into(),
                    account_type: AccountType::Liability,
                    changes: vec![],
                    beginning_balance_node: None,
                },
                CorkscrewAccount {
                    node_id: "equity".into(),
                    account_type: AccountType::Equity,
                    changes: vec![],
                    beginning_balance_node: None,
                },
            ],
            tolerance: 0.01,
            fail_on_error: true,
        };

        let mut extension = CorkscrewExtension::with_config(config);
        let context = ExtensionContext::new(&model, &results);
        let result = extension
            .execute(&context)
            .expect("extension should execute");

        assert_eq!(result.status, ExtensionStatus::Failed);
        assert!(
            result
                .errors
                .iter()
                .any(|msg| msg.contains("Balance sheet not articulated")),
            "expected articulation failure, got {:?}",
            result.errors
        );
    }
}
