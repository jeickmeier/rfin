"""Valuation result caching example.

Demonstrates the behaviour of :class:`finstack.valuations.ValuationCache` —
a memory-bounded LRU cache keyed by ``(instrument_id, market_version)``:

1. Build a cache with a capped number of entries.
2. Price a 100-instrument portfolio ten times per name (1,000 operations).
   The second pass at the same ``market_version`` is a pure hit stream.
3. Bump ``market_version`` — every entry becomes stale; all 100 lookups
   miss until the cache is repopulated.
4. Force eviction by filling beyond capacity.
5. Print the cumulative statistics (hits/misses/evictions/memory).

The cache exposed here is intentionally minimal: it caches an NPV
(``f64``) per key so the example can focus on cache mechanics without
reconstructing full :class:`ValuationResult` objects. The underlying
Rust :rust:struct:`finstack_valuations::cache::ValuationCache` follows
the same eviction policy with richer content-addressed keys and stores
whole ``ValuationResult`` values — this Python surface is the didactic
sibling.

Run standalone:

    python finstack-py/examples/16_valuation_caching.py
"""

from __future__ import annotations

from finstack.valuations import ValuationCache


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _fake_npv(instrument_id: int, market_version: int) -> float:
    """Deterministic pseudo-NPV so we can verify cache hits return the same value."""
    # Notional ~ 1M, coupon drift, small market-version tilt.
    return 1_000_000.0 + instrument_id * 137.0 - market_version * 53.0


def _print_stats(label: str, cache: ValuationCache) -> None:
    stats = cache.stats()
    print(f"[{label}]")
    print(f"  entries       = {stats['entries']}")
    print(f"  lookups       = {stats['lookups']}")
    print(f"  hits          = {stats['hits']}")
    print(f"  misses        = {stats['misses']}")
    print(f"  hit_rate      = {stats['hit_rate']:.4f}")
    print(f"  inserts       = {stats['inserts']}")
    print(f"  evictions     = {stats['evictions']}")
    print(f"  memory_bytes  = {stats['memory_bytes']:,}")
    print(f"  memory_mb     = {stats['memory_mb']:.4f}")
    print()


# ---------------------------------------------------------------------------
# Scenarios
# ---------------------------------------------------------------------------


def portfolio_repricing(cache: ValuationCache) -> None:
    """Simulate a 100-instrument portfolio priced 10 times at market_version=1.

    Pass 1 misses everywhere; passes 2-10 are hits.
    """
    n_instruments = 100
    n_passes = 10
    market_version = 1

    for pass_idx in range(n_passes):
        for inst_id in range(n_instruments):
            hit = cache.get(inst_id, market_version)
            if hit is None:
                npv = _fake_npv(inst_id, market_version)
                cache.insert(inst_id, npv, market_version)
            else:
                # Hit — value must match what we stored.
                expected = _fake_npv(inst_id, market_version)
                assert hit == expected, f"stale npv for {inst_id}: {hit} != {expected}"

        # Snapshot after each pass for visibility on the hit/miss ratio curve.
        if pass_idx in (0, 1, 9):
            _print_stats(f"after pass {pass_idx + 1}", cache)


def market_version_bump(cache: ValuationCache) -> None:
    """Bump market_version - every previous entry becomes stale."""
    n_instruments = 100
    new_version = 2

    misses_before = cache.stats()["misses"]
    for inst_id in range(n_instruments):
        hit = cache.get(inst_id, new_version)
        assert hit is None, f"unexpected hit at new market_version for {inst_id}"
    misses_after = cache.stats()["misses"]
    print(
        f"[market_version bump 1 -> {new_version}] "
        f"added {misses_after - misses_before} misses across {n_instruments} instruments"
    )
    _print_stats("after version bump", cache)


def force_eviction() -> None:
    """Fill a small cache beyond capacity to observe LRU eviction."""
    cap = 50
    cache = ValuationCache(max_entries=cap, max_memory_bytes=256_000_000)

    # Insert 2 * cap distinct entries; ~half of them should survive.
    total = cap * 2
    for inst_id in range(total):
        cache.insert(inst_id, _fake_npv(inst_id, 1), market_version=1)

    stats = cache.stats()
    print(
        f"[eviction demo] inserted {total} into cap={cap}; "
        f"final entries={stats['entries']}, evictions={stats['evictions']}"
    )
    assert stats["evictions"] > 0, "expected at least one eviction"
    assert stats["entries"] <= cap + cap // 10, (
        f"entries {stats['entries']} should be near cap {cap}"
    )
    _print_stats("after eviction", cache)


def targeted_invalidation() -> None:
    """Demonstrate invalidate_instrument dropping all entries for an id."""
    cache = ValuationCache(max_entries=1_000, max_memory_bytes=256_000_000)

    # Insert the same instrument under three market versions.
    target_id = 42
    for mv in range(1, 4):
        cache.insert(target_id, _fake_npv(target_id, mv), market_version=mv)
    # A few other instruments as noise.
    for inst_id in (1, 2, 3):
        cache.insert(inst_id, _fake_npv(inst_id, 1), market_version=1)

    before = cache.stats()["entries"]
    cache.invalidate_instrument(target_id)
    after = cache.stats()["entries"]
    print(
        f"[invalidate_instrument({target_id})] entries {before} -> {after} "
        f"(dropped {before - after})"
    )
    assert after == before - 3, "all three versions for target should be dropped"
    assert cache.get(target_id, 1) is None
    assert cache.get(1, 1) is not None, "unrelated instrument must survive"
    print()


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------


def main() -> None:
    print("=" * 72)
    print("finstack.valuations ValuationCache demo")
    print("=" * 72)
    print()

    cache = ValuationCache(max_entries=1_000, max_memory_bytes=256_000_000)

    print("1) Pricing 100 instruments x 10 passes at market_version=1")
    print("-" * 72)
    portfolio_repricing(cache)

    print("2) Bumping market_version -> every entry is stale")
    print("-" * 72)
    market_version_bump(cache)

    print("3) Forcing LRU eviction with a small capacity")
    print("-" * 72)
    force_eviction()

    print("4) Targeted per-instrument invalidation")
    print("-" * 72)
    targeted_invalidation()

    print("done.")


if __name__ == "__main__":
    main()
