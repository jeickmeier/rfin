# Calendars

Finstack ships 19 production calendars compiled from JSON sources at build time (NYSE, TARGET2, JPX, etc.). Calendars are addressed by a stable `CalendarId` string (e.g., `"target2"`, `"nyse"`). Two helper DTOs keep calendar usage deterministic and database-friendly:

- `CalendarMetadata`: exposes the compiled name, region, timezone, and supported business-day conventions. These records are `Serialize`/`Deserialize` so bindings can snapshot the registry.
- `ScheduleSpec`: the canonical wire format for schedule builders. It carries `calendar_id`, `BusinessDayConvention`, `Frequency`, `StubKind`, IMM/CDS toggles, and deterministically recreates a `Schedule`.

```json
{
  "start": "2025-01-15",
  "end": "2026-01-15",
  "frequency": "Quarterly",
  "stub": "None",
  "business_day_convention": "ModifiedFollowing",
  "calendar_id": "target2",
  "end_of_month": false,
  "cds_imm_mode": false,
  "graceful": true
}
```

For day-count calculations, `DayCountCtxState` captures the optional `calendar_id`, compounding `frequency`, and BUS/252 basis. Python and WASM bindings round-trip this DTO verbatim so analysts can persist context alongside results. Always add new calendar-aware runtime types with a corresponding `*State` structure and a JSON round-trip test.
