//! Python bindings for the valuation result cache.
//!
//! This binding exposes a simplified, numeric-keyed cache that demonstrates
//! the behavioural properties of [`finstack_valuations::cache::ValuationCache`]
//! — LRU eviction by entry count and estimated memory, hit/miss accounting,
//! market-version invalidation, and targeted per-instrument invalidation —
//! without requiring a full [`ValuationResult`](finstack_valuations::results::ValuationResult)
//! round-trip between Python and Rust.
//!
//! The key is the tuple ``(instrument_id: u64, market_version: u64)``; the
//! cached value is a single ``f64`` NPV. This mirrors the semantics of the
//! real cache: lookups at a new ``market_version`` miss, bumping the version
//! transparently invalidates the whole market state, and
//! :py:meth:`PyValuationCache.invalidate_instrument` drops every entry for a
//! given instrument id.
//!
//! LRU eviction is approximate and runs inline on insert, matching the
//! 10%-per-pass behaviour of the Rust implementation.
//!
//! This is a didactic surface aimed at the companion example
//! ``examples/16_valuation_caching.py``. Production pricing pipelines will
//! keep using the Rust cache directly through the pricer integration.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::sync::Mutex;

/// Fixed per-entry memory estimate (bytes) used for the memory cap.
///
/// Matches the `estimate_result_size` constant-ish floor in the real cache
/// closely enough for didactic purposes: a `(u64, u64)` key, an `f64` value,
/// a `u64` LRU stamp, and a rough HashMap bucket overhead.
const ESTIMATED_ENTRY_BYTES: usize = 96;

/// Internal per-entry bookkeeping.
struct CacheEntry {
    npv: f64,
    last_access: u64,
}

/// Internal state protected by a single `Mutex`. Contention is fine here —
/// this cache is a demonstrator, not a parallel pricing backbone.
struct Inner {
    map: HashMap<(u64, u64), CacheEntry>,
    max_entries: usize,
    max_memory_bytes: usize,
    // LRU clock.
    access_counter: u64,
    // Cumulative counters (monotonic over the lifetime of the cache).
    hits: u64,
    misses: u64,
    evictions: u64,
    inserts: u64,
}

impl Inner {
    fn new(max_entries: usize, max_memory_bytes: usize) -> Self {
        Self {
            map: HashMap::with_capacity(max_entries.min(1 << 20)),
            max_entries,
            max_memory_bytes,
            access_counter: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
            inserts: 0,
        }
    }

    fn memory_bytes(&self) -> usize {
        self.map.len() * ESTIMATED_ENTRY_BYTES
    }

    fn tick(&mut self) -> u64 {
        let t = self.access_counter;
        self.access_counter = self.access_counter.wrapping_add(1);
        t
    }

    /// Evict ~10% of the oldest-accessed entries if either cap is exceeded.
    fn maybe_evict(&mut self) {
        let over_entries = self.map.len() > self.max_entries;
        let over_memory = self.memory_bytes() > self.max_memory_bytes;
        if !over_entries && !over_memory {
            return;
        }

        let evict_count = (self.map.len() / 10).max(1);

        // Collect access times, sort, drop the oldest `evict_count`.
        let mut stamps: Vec<((u64, u64), u64)> = self
            .map
            .iter()
            .map(|(k, e)| (*k, e.last_access))
            .collect();
        stamps.sort_unstable_by_key(|(_, t)| *t);

        for (key, _) in stamps.iter().take(evict_count) {
            if self.map.remove(key).is_some() {
                self.evictions += 1;
            }
        }
    }
}

/// A memory-bounded LRU cache for valuation NPVs, keyed by
/// ``(instrument_id, market_version)``.
///
/// Mirrors the Rust :rust:struct:`finstack_valuations::cache::ValuationCache`
/// in behaviour: lookups at a fresh ``market_version`` miss, eviction is
/// approximate LRU bounded by entry count and estimated memory, and per
/// instrument invalidation is supported via
/// :py:meth:`invalidate_instrument`.
///
/// Parameters
/// ----------
/// max_entries : int, default 10_000
///     Soft cap on the number of cached entries.
/// max_memory_bytes : int, default 256_000_000
///     Soft cap on estimated memory usage (each entry is ~96 B).
#[pyclass(name = "ValuationCache", module = "finstack.valuations")]
pub(crate) struct PyValuationCache {
    inner: Mutex<Inner>,
}

#[pymethods]
impl PyValuationCache {
    #[new]
    #[pyo3(signature = (max_entries=10_000, max_memory_bytes=256_000_000))]
    fn new(max_entries: usize, max_memory_bytes: usize) -> Self {
        Self {
            inner: Mutex::new(Inner::new(max_entries, max_memory_bytes)),
        }
    }

    /// Insert ``npv`` under ``(key, market_version)``.
    ///
    /// Returns ``True`` if the entry was newly inserted (or replaced a stale
    /// entry from a different market version), ``False`` if an entry with the
    /// same key and market version already existed and matched.
    fn insert(&self, key: u64, npv: f64, market_version: u64) -> bool {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };

        let composite = (key, market_version);
        let already_present = inner.map.contains_key(&composite);
        if already_present {
            // Refresh LRU stamp but treat as a no-op for reporting purposes.
            let t = inner.tick();
            if let Some(e) = inner.map.get_mut(&composite) {
                e.last_access = t;
            }
            return false;
        }

        let t = inner.tick();
        inner
            .map
            .insert(composite, CacheEntry { npv, last_access: t });
        inner.inserts += 1;
        inner.maybe_evict();
        true
    }

    /// Look up the NPV for ``(key, market_version)``.
    ///
    /// Returns ``None`` on miss; the result otherwise. Updates the LRU stamp
    /// on hit and increments the hit/miss counters.
    fn get(&self, key: u64, market_version: u64) -> Option<f64> {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        let composite = (key, market_version);
        let t = inner.tick();
        match inner.map.get_mut(&composite) {
            Some(entry) => {
                entry.last_access = t;
                let npv = entry.npv;
                inner.hits += 1;
                Some(npv)
            }
            None => {
                inner.misses += 1;
                None
            }
        }
    }

    /// Current number of entries.
    fn len(&self) -> usize {
        self.inner.lock().map_or(0, |g| g.map.len())
    }

    /// Remove every entry for ``instrument_id`` across all market versions.
    fn invalidate_instrument(&self, instrument_id: u64) {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        inner.map.retain(|(id, _), _| *id != instrument_id);
    }

    /// Drop every entry and reset memory tracking (statistics are preserved).
    fn clear(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.map.clear();
        }
    }

    /// Snapshot the cumulative statistics as a plain dict.
    ///
    /// Keys: ``hits``, ``misses``, ``lookups``, ``hit_rate``, ``evictions``,
    /// ``inserts``, ``entries``, ``memory_bytes``, ``memory_mb``.
    fn stats<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let inner = self.inner.lock().map_err(|_| {
            pyo3::exceptions::PyRuntimeError::new_err("cache mutex poisoned")
        })?;

        let hits = inner.hits;
        let misses = inner.misses;
        let lookups = hits + misses;
        let hit_rate = if lookups == 0 {
            0.0
        } else {
            hits as f64 / lookups as f64
        };
        let memory_bytes = inner.memory_bytes();

        let out = PyDict::new(py);
        out.set_item("hits", hits)?;
        out.set_item("misses", misses)?;
        out.set_item("lookups", lookups)?;
        out.set_item("hit_rate", hit_rate)?;
        out.set_item("evictions", inner.evictions)?;
        out.set_item("inserts", inner.inserts)?;
        out.set_item("entries", inner.map.len())?;
        out.set_item("memory_bytes", memory_bytes)?;
        out.set_item("memory_mb", memory_bytes as f64 / (1024.0 * 1024.0))?;
        Ok(out)
    }

    fn __len__(&self) -> usize {
        self.len()
    }

    fn __repr__(&self) -> String {
        match self.inner.lock() {
            Ok(g) => format!(
                "ValuationCache(entries={}, max_entries={}, max_memory_bytes={})",
                g.map.len(),
                g.max_entries,
                g.max_memory_bytes
            ),
            Err(_) => "ValuationCache(<poisoned>)".to_string(),
        }
    }
}

/// Register the cache class on the valuations submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyValuationCache>()?;
    Ok(())
}
