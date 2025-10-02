/// Trait for WASM instrument wrappers that delegate to finstack-valuations types.
///
/// This trait provides a consistent interface for wrapping Rust core instruments
/// in JavaScript-compatible types. It eliminates boilerplate by standardizing
/// the conversion pattern across all 25+ instruments.
///
/// # Usage
///
/// ```rust
/// use crate::valuations::instruments::InstrumentWrapper;
/// use finstack_valuations::instruments::bond::Bond;
///
/// #[wasm_bindgen(js_name = Bond)]
/// #[derive(Clone, Debug)]
/// pub struct JsBond(Bond);
///
/// impl InstrumentWrapper for JsBond {
///     type Inner = Bond;
///     fn from_inner(inner: Bond) -> Self { JsBond(inner) }
///     fn inner(&self) -> Bond { self.0.clone() }
/// }
/// ```
///
/// # Benefits
///
/// - **Consistency**: All instruments follow the same wrapper pattern
/// - **Maintainability**: Changes to the pattern affect all instruments uniformly
/// - **Clarity**: The trait makes it obvious which types are wrappers
/// - **Reduced LOC**: 30 lines of boilerplate → 3 lines per instrument
///
/// # Pattern
///
/// Each instrument wrapper is a tuple struct containing the inner type:
/// - Use `from_inner()` to construct from Rust core types
/// - Use `inner()` to extract for passing to Rust core functions
/// - Access fields via `self.0.field_name`
pub(crate) trait InstrumentWrapper: Sized + Clone {
    /// The wrapped Rust core instrument type
    type Inner: Clone;

    /// Construct the wrapper from an inner instrument.
    ///
    /// This is the canonical way to create a wrapper from a core type.
    fn from_inner(inner: Self::Inner) -> Self;

    /// Extract a clone of the inner instrument.
    ///
    /// This is used to pass the instrument to Rust core functions that
    /// require owned values.
    fn inner(&self) -> Self::Inner;
}
