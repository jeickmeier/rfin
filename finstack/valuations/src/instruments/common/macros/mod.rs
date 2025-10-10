//! Instrument implementation utilities.
//!
//! Previously contained macros for generating instrument boilerplate.
//! Now all instruments use explicit `impl Instrument` trait implementations
//! for better IDE support, clearer stack traces, and easier debugging.

/// Macro for generating convenience constructor methods.
///
/// Creates static methods for common instrument patterns, reducing the need
/// for builder pattern in simple cases.
#[macro_export]
macro_rules! impl_convenience_constructors {
    (
        $type:ident {
            $(
                $method:ident($($param:ident: $param_type:ty),* $(,)?) => $constructor:expr
            ),* $(,)?
        }
    ) => {
        impl $type {
            $(
                /// Convenience constructor
                pub fn $method($($param: $param_type),*) -> Self {
                    $constructor
                }
            )*
        }
    };
}
