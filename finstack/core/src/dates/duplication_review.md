# Rust Duplication Review: finstack/core/src/dates

## 1. The "Kill" List
Code that appears redundant, deprecated, or essentially dead and should be removed or replaced.

*   **`finstack/core/src/dates/schedule_iter.rs`: `Step` enum**
    *   **Reason**: This private enum (`Step::Months(i32)`, `Step::Days(i32)`) and its `add` method duplicate the logic found in `Tenor::add_to_date` and `DateExt::add_months`. It exists solely as an internal helper for `schedule_iter` but offers no unique value over the core types.
    *   **Recommendation**: Replace `Step` usage with `Tenor` (adding sign/direction support if needed, or just using `Tenor` for the magnitude and controlling direction at call site).

*   **`finstack/core/src/dates/periods.rs`: `parse_id` helper**
    *   **Reason**: This private function has significant logic overlap with `Tenor::parse` (parsing "Q", "M", etc.), although it parses specific period IDs ("2025Q1") rather than just durations. However, the internal matching on characters (`Q`, `M`, `W`) duplicates the knowledge of period types found in `PeriodKind`.
    *   **Recommendation**: While not a strict "kill", this parsing logic is brittle. It should be consolidated into `PeriodId::from_str` and use more robust parsing shared with `TenorUnit` where possible (e.g. sharing the mapping of 'Q' -> Quarterly etc).

*   **`finstack/core/src/dates/schedule_iter.rs`: `is_cds_roll_date`**
    *   **Reason**: This local helper checks if a date is the 20th of Mar/Jun/Sep/Dec. `imm.rs` contains `next_cds_date` with the same implicit knowledge (20th of quarterly months).
    *   **Recommendation**: Centralize CDS date logic in `imm.rs`. Expose a public `is_cds_date(date)` in `imm.rs` and use it in `schedule_iter.rs`.

## 2. The "Merge" List
Groups of functions or types that should be combined into a more robust abstraction.

*   **`Frequency` (schedule_iter.rs) + `Tenor` (tenor.rs)**
    *   **Analysis**: This is the most significant duplication.
        *   `Tenor` (`count: u32`, `unit: TenorUnit`) represents a time duration (e.g., "3M", "1Y").
        *   `Frequency` (`Months(u8)`, `Days(u16)`) also represents a time duration/interval for schedules.
        *   `Frequency` contains logic like `from_payments_per_year` and convenience constructors (`quarterly()`, `monthly()`) that are essentially `Tenor` factories.
    *   **Proposal**: Deprecate `Frequency` in favor of `Tenor`.
        *   Move `from_payments_per_year` to `Tenor`.
        *   Add `Tenor::quarterly()`, `Tenor::semi_annual()` etc.
        *   Update `ScheduleBuilder` to accept `Tenor` instead of `Frequency`.
        *   This unifies "Time Interval" representation across the library.

*   **`PeriodKind` (periods.rs) + `TenorUnit` (tenor.rs)**
    *   **Analysis**:
        *   `PeriodKind` has: `Quarterly`, `Monthly`, `Weekly`, `SemiAnnual`, `Annual`.
        *   `TenorUnit` has: `Days`, `Weeks`, `Months`, `Years`.
    *   **Proposal**: While they serve slightly different purposes (Reporting Period vs Time Unit), they are deeply related. `PeriodKind` is effectively a standardized `Tenor` (e.g. `Quarterly` = `3 Months`).
    *   **Recommendation**: Implement `From<PeriodKind> for Tenor`. This allows converting a reporting period type immediately into a useful calculation unit.

## 3. Refactoring Plan
**Target**: Consolidate `Frequency` into `Tenor`.

`Tenor` is the stronger, more general abstraction. `Frequency` is a restrictive subset used only for schedules. Merging them simplifies the API (one way to describe "time interval") and reduces conversion friction.

### Before: Split definitions

**`src/dates/tenor.rs`**
```rust
pub struct Tenor {
    pub count: u32,
    pub unit: TenorUnit,
}
// ... has parsing logic, to_years etc.
```

**`src/dates/schedule_iter.rs`**
```rust
pub enum Frequency {
    Months(u8),
    Days(u16),
}
impl Frequency {
    pub const fn quarterly() -> Self { Self::Months(3) }
    // ... duplicated logic
}
```

### After: Unified `Tenor`

**`src/dates/tenor.rs`**
```rust
impl Tenor {
    /// Returns a tenor representing 3 months (Quarterly frequency).
    pub const fn quarterly() -> Self {
        Self::new(3, TenorUnit::Months)
    }

    /// Create a Tenor from payments per year (e.g. 4 -> 3M).
    pub fn from_payments_per_year(payments: u32) -> crate::Result<Self> {
        if payments == 0 { return Err(...); }
        let months = 12 / payments; // Simplified check
        Ok(Self::new(months, TenorUnit::Months))
    }
}
```

**`src/dates/schedule_iter.rs`**
```rust
// Frequency is removed. ScheduleBuilder uses Tenor.

pub struct ScheduleBuilder<'a> {
    freq: Tenor, // Was Frequency
    // ...
}

impl<'a> ScheduleBuilder<'a> {
    pub fn frequency(mut self, freq: Tenor) -> Self {
        self.freq = freq;
        self
    }
}
```

### Benefits
1.  **Single Source of Truth**: "3 Months" is always `Tenor { 3, Months }`, never `Frequency::Months(3)`.
2.  **Rich Parsing**: `ScheduleBuilder` now implicitly supports string parsing via `Tenor::parse("3M")`.
3.  **Flexibility**: Schedules can now be "30 Years" or "1 Week" without arbitrary u8 limits found in `Frequency`.
