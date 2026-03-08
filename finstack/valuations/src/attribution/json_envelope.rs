//! Generic JSON envelope serialization trait.
//!
//! Provides a trait for types that serialize to/from JSON with domain-specific
//! error conversion. Used by attribution envelope types but applicable to any
//! serde-compatible type.

use finstack_core::Result;
use serde::Serialize;

/// Trait for types that can be serialized to/from JSON envelopes.
///
/// Provides default implementations for common JSON I/O operations with
/// consistent error handling. Types implementing this trait must provide
/// error conversion methods to map `serde_json` errors to domain-specific
/// error types.
///
/// # Type Requirements
///
/// Implementors must:
/// - Implement `serde::Serialize` for JSON output
/// - Implement `serde::de::DeserializeOwned` for JSON input
/// - Provide `parse_error` to convert deserialization errors
/// - Provide `serialize_error` to convert serialization errors
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::attribution::JsonEnvelope;
/// use serde::{Deserialize, Serialize};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #[derive(Serialize, Deserialize)]
/// struct MyEnvelope {
///     schema: String,
///     data: String,
/// }
///
/// impl JsonEnvelope for MyEnvelope {
///     fn parse_error(e: serde_json::Error) -> finstack_core::Error {
///         finstack_core::Error::Calibration {
///             message: format!("Failed to parse envelope: {}", e),
///             category: "json_parse".to_string(),
///         }
///     }
///
///     fn serialize_error(e: serde_json::Error) -> finstack_core::Error {
///         finstack_core::Error::Calibration {
///             message: format!("Failed to serialize envelope: {}", e),
///             category: "json_serialize".to_string(),
///         }
///     }
/// }
///
/// let envelope = MyEnvelope {
///     schema: "v1".to_string(),
///     data: "test".to_string(),
/// };
///
/// let json = envelope.to_json()?;
/// let parsed = MyEnvelope::from_json(&json)?;
/// let cursor = std::io::Cursor::new(json.as_bytes());
/// let from_reader = MyEnvelope::from_reader(cursor)?;
/// # let _ = (parsed, from_reader);
/// # Ok(())
/// # }
/// ```
pub trait JsonEnvelope: Sized + Serialize + serde::de::DeserializeOwned {
    /// Convert a JSON parsing error to the domain error type.
    fn parse_error(e: serde_json::Error) -> finstack_core::Error;

    /// Convert a JSON serialization error to the domain error type.
    fn serialize_error(e: serde_json::Error) -> finstack_core::Error;

    /// Parse from a JSON string.
    fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(Self::parse_error)
    }

    /// Parse from a reader (file, socket, buffer, etc.).
    fn from_reader<R: std::io::Read>(reader: R) -> Result<Self> {
        serde_json::from_reader(reader).map_err(Self::parse_error)
    }

    /// Serialize to a pretty-printed JSON string.
    fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(Self::serialize_error)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestEnvelope {
        schema: String,
        data: String,
        number: i32,
    }

    impl JsonEnvelope for TestEnvelope {
        fn parse_error(e: serde_json::Error) -> finstack_core::Error {
            finstack_core::Error::Calibration {
                message: format!("Failed to parse test envelope: {}", e),
                category: "test_parse".to_string(),
            }
        }

        fn serialize_error(e: serde_json::Error) -> finstack_core::Error {
            finstack_core::Error::Calibration {
                message: format!("Failed to serialize test envelope: {}", e),
                category: "test_serialize".to_string(),
            }
        }
    }

    #[test]
    fn test_json_envelope_roundtrip() {
        let envelope = TestEnvelope {
            schema: "test/v1".to_string(),
            data: "test data".to_string(),
            number: 42,
        };

        let json = envelope.to_json().expect("Serialization should succeed");
        assert!(json.contains("\"schema\""));
        assert!(json.contains("\"test/v1\""));
        assert!(json.contains("42"));

        let parsed = TestEnvelope::from_json(&json).expect("Deserialization should succeed");
        assert_eq!(parsed, envelope);
    }

    #[test]
    fn test_json_envelope_from_reader() {
        let envelope = TestEnvelope {
            schema: "test/v1".to_string(),
            data: "reader test".to_string(),
            number: 123,
        };

        let json = envelope.to_json().expect("Serialization should succeed");
        let cursor = std::io::Cursor::new(json.as_bytes());
        let parsed =
            TestEnvelope::from_reader(cursor).expect("Deserialization from reader should succeed");
        assert_eq!(parsed, envelope);
    }

    #[test]
    fn test_json_envelope_parse_error() {
        let invalid_json = r#"{"schema": "test/v1", "data": "test", "number": "not a number"}"#;
        let result = TestEnvelope::from_json(invalid_json);
        assert!(result.is_err());

        let err = result.expect_err("Expected error from invalid JSON");
        if let finstack_core::Error::Calibration { message, category } = err {
            assert!(message.contains("Failed to parse test envelope"));
            assert_eq!(category, "test_parse");
        } else {
            panic!("Expected Calibration error, got: {:?}", err);
        }
    }

    #[test]
    fn test_json_envelope_missing_fields() {
        let incomplete_json = r#"{"schema": "test/v1"}"#;
        let result = TestEnvelope::from_json(incomplete_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_envelope_malformed_json() {
        let malformed_json = r#"{"schema": "test/v1", "data": "test", "number": 42"#;
        let result = TestEnvelope::from_json(malformed_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_envelope_reader_io_error() {
        struct FailingReader;
        impl std::io::Read for FailingReader {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("Simulated I/O error"))
            }
        }

        let result = TestEnvelope::from_reader(FailingReader);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_envelope_pretty_printing() {
        let envelope = TestEnvelope {
            schema: "test/v1".to_string(),
            data: "test".to_string(),
            number: 42,
        };

        let json = envelope.to_json().expect("Serialization should succeed");
        assert!(json.contains('\n'));
        assert!(json.lines().count() > 1);

        let parsed = TestEnvelope::from_json(&json).expect("Parsing pretty JSON should succeed");
        assert_eq!(parsed, envelope);
    }

    #[test]
    fn test_json_envelope_equivalence() {
        let envelope1 = TestEnvelope {
            schema: "test/v1".to_string(),
            data: "data1".to_string(),
            number: 100,
        };

        let envelope2 = envelope1.clone();

        let json1 = envelope1.to_json().expect("Serialization should succeed");
        let json2 = envelope2.to_json().expect("Serialization should succeed");
        assert_eq!(json1, json2);

        let parsed1 = TestEnvelope::from_json(&json1).expect("Parse should succeed");
        assert_eq!(parsed1, envelope1);
    }
}
