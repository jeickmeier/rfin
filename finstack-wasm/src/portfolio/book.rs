//! Portfolio book hierarchy bindings for WASM.
//!
//! Wraps `finstack_portfolio::Book` and `BookId` for optional hierarchical
//! organization of portfolio positions.

use crate::utils::json::{from_js_value, to_js_value};
use finstack_portfolio::types::PositionId;
use finstack_portfolio::{Book, BookId};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// BookId
// ---------------------------------------------------------------------------

/// Strongly-typed book identifier.
///
/// @example
/// ```javascript
/// const id = new BookId("americas");
/// console.log(id.value);  // "americas"
/// ```
#[wasm_bindgen(js_name = BookId)]
#[derive(Clone)]
pub struct JsBookId {
    inner: BookId,
}

#[allow(dead_code)]
impl JsBookId {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: BookId) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &BookId {
        &self.inner
    }
}

#[wasm_bindgen(js_class = BookId)]
impl JsBookId {
    /// Create a new book identifier.
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str) -> Self {
        Self {
            inner: BookId::new(id),
        }
    }

    /// Get the identifier string value.
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> String {
        self.inner.as_str().to_string()
    }

    /// String representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_js_string(&self) -> String {
        self.inner.to_string()
    }
}

// ---------------------------------------------------------------------------
// Book
// ---------------------------------------------------------------------------

/// A book in the portfolio hierarchy.
///
/// Books provide an optional hierarchical organization structure for portfolios,
/// allowing positions to be grouped into folders/books with parent-child
/// relationships.
///
/// @example
/// ```javascript
/// const book = new Book("credit", "Credit");
/// book.addPosition("pos-1");
/// console.log(book.isRoot);  // true
/// console.log(book.containsPosition("pos-1"));  // true
/// ```
#[wasm_bindgen(js_name = Book)]
#[derive(Clone)]
pub struct JsBook {
    inner: Book,
}

#[allow(dead_code)]
impl JsBook {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: Book) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &Book {
        &self.inner
    }
}

#[wasm_bindgen(js_class = Book)]
impl JsBook {
    /// Create a new root book.
    ///
    /// @param id - Unique book identifier
    /// @param name - Optional human-readable name
    /// @param parentId - Optional parent book identifier
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str, name: Option<String>, parent_id: Option<String>) -> Self {
        let book = Book::new(id, name);
        let book = match parent_id {
            Some(pid) => book.with_parent(pid),
            None => book,
        };
        Self { inner: book }
    }

    /// Book identifier.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.to_string()
    }

    /// Human-readable name.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    /// Parent book identifier (undefined for root books).
    #[wasm_bindgen(getter, js_name = parentId)]
    pub fn parent_id(&self) -> Option<String> {
        self.inner.parent_id.as_ref().map(|id| id.to_string())
    }

    /// Whether this is a root book (no parent).
    #[wasm_bindgen(getter, js_name = isRoot)]
    pub fn is_root(&self) -> bool {
        self.inner.is_root()
    }

    /// Position IDs directly assigned to this book.
    #[wasm_bindgen(getter, js_name = positionIds)]
    pub fn position_ids(&self) -> Vec<String> {
        self.inner
            .position_ids
            .iter()
            .map(|id| id.to_string())
            .collect()
    }

    /// Child book IDs.
    #[wasm_bindgen(getter, js_name = childBookIds)]
    pub fn child_book_ids(&self) -> Vec<String> {
        self.inner
            .child_book_ids
            .iter()
            .map(|id| id.to_string())
            .collect()
    }

    /// Check if this book directly contains a position.
    #[wasm_bindgen(js_name = containsPosition)]
    pub fn contains_position(&self, position_id: &str) -> bool {
        self.inner.contains_position(&PositionId::new(position_id))
    }

    /// Check if this book contains a specific child book.
    #[wasm_bindgen(js_name = containsChild)]
    pub fn contains_child(&self, child_id: &str) -> bool {
        self.inner.contains_child(&BookId::new(child_id))
    }

    /// Add a position to this book.
    #[wasm_bindgen(js_name = addPosition)]
    pub fn add_position(&mut self, position_id: &str) {
        self.inner.add_position(PositionId::new(position_id));
    }

    /// Add a child book to this book.
    #[wasm_bindgen(js_name = addChild)]
    pub fn add_child(&mut self, child_id: &str) {
        self.inner.add_child(BookId::new(child_id));
    }

    /// Remove a position from this book.
    #[wasm_bindgen(js_name = removePosition)]
    pub fn remove_position(&mut self, position_id: &str) {
        self.inner.remove_position(&PositionId::new(position_id));
    }

    /// Remove a child book from this book.
    #[wasm_bindgen(js_name = removeChild)]
    pub fn remove_child(&mut self, child_id: &str) {
        self.inner.remove_child(&BookId::new(child_id));
    }

    /// Serialize to a JSON object.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Deserialize from a JSON object.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsBook, JsValue> {
        from_js_value(value).map(|inner| Self { inner })
    }
}
