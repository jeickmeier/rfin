//! Accrual factor cache – stores day-count year fractions between two dates.
//!
//! Implements a simple global LRU (≈2 k entries) keyed by `(start, end, DayCount)`.
//! The cache uses `hashbrown::HashMap` guarded by a `Mutex`.  When the capacity
//! is exceeded we evict the **oldest** entry via a ring-buffer index.
#![allow(clippy::module_name_repetitions)]

use core::hash::{Hash, Hasher};
use hashbrown::HashMap;
use once_cell::sync::Lazy;
use std::sync::Mutex;

use finstack_core::dates::{Date, DayCount};
use finstack_core::Error;

// -----------------------------------------------------------------------------
// Key type – customise Hash to avoid collisions on DayCount enum alone.
// -----------------------------------------------------------------------------
#[derive(Clone, Copy, Eq)]
struct Key {
    start: Date,
    end: Date,
    dc: DayCount,
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end && self.dc == other.dc
    }
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.start.hash(state);
        self.end.hash(state);
        (self.dc as u8).hash(state);
    }
}

/// Maximum number of cached entries.
const MAX_ENTRIES: usize = 2_000;

/// Global cache guarded by a mutex (low contention – computation is cheap).
static CACHE: Lazy<Mutex<Cache>> = Lazy::new(|| Mutex::new(Cache::new()));

struct Cache {
    map: HashMap<Key, f64>,
    order: Vec<Key>, // insertion order for naive LRU
}

impl Cache {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: Vec::with_capacity(MAX_ENTRIES),
        }
    }

    fn get_or_insert(
        &mut self,
        key: Key,
        compute: impl FnOnce() -> Result<f64, Error>,
    ) -> Result<f64, Error> {
        if let Some(&val) = self.map.get(&key) {
            return Ok(val);
        }
        let val = compute()?;
        if self.map.len() >= MAX_ENTRIES {
            // Evict oldest (FIFO) – sufficient for small cache size.
            if let Some(old) = self.order.first().copied() {
                self.order.remove(0);
                self.map.remove(&old);
            }
        }
        self.map.insert(key, val);
        self.order.push(key);
        Ok(val)
    }
}

/// Return the year fraction between `start` and `end` using an internal cache.
pub fn year_fraction_cached(start: Date, end: Date, dc: DayCount) -> Result<f64, Error> {
    let key = Key { start, end, dc };
    let mut cache = CACHE.lock().unwrap();
    cache.get_or_insert(key, || dc.year_fraction(start, end))
}
