# Market Conventions

Reference tables for day counts, business day rules, quote conventions,
and settlement practices.

## Day Count Conventions

| Convention | Formula | Standard For |
|-----------|---------|-------------|
| `Act360` | actual days / 360 | USD/EUR money markets, SOFR, €STR, FX swaps |
| `Act365F` | actual days / 365 | GBP money markets (SONIA), Commonwealth bonds |
| `Act365L` | actual days / (366 if Feb 29 in period else 365) | AFB method |
| `Thirty360` | 30/360 US (Bond Basis) | US corporate bonds |
| `ThirtyE360` | 30E/360 (Eurobond Basis) | Eurobonds |
| `ActAct` | actual/actual (ISDA) | US Treasuries, many swaps |
| `ActActIsma` | actual/actual (ICMA) | International bonds with regular coupons |
| `Bus252` | business days / 252 | Brazilian markets |

### Day Count Context

Some conventions require additional context:

```rust,no_run
DayCountCtx {
    calendar: Option<&dyn HolidayCalendar>,  // for Bus252
    frequency: Option<Tenor>,                 // for ActActIsma
    bus_basis: Option<u16>,                   // for Bus252 override
}
```

## Business Day Conventions

| Convention | Behavior | ISDA Reference |
|-----------|----------|---------------|
| `Unadjusted` | No adjustment | Section 4.12(a) |
| `Following` | Next business day (may cross month) | Section 4.12(b) |
| `ModifiedFollowing` | Next business day, unless crosses month → preceding | Section 4.12(c) |
| `Preceding` | Previous business day (may cross month) | Section 4.12(d) |
| `ModifiedPreceding` | Previous business day, unless crosses month → following | Section 4.12(e) |

## Quote Conventions

| Market | Quote Style | Example |
|--------|------------|--------|
| Bonds | Clean price (% of par) | 99.50 |
| Swaps | Fixed rate (decimal) | 0.0425 |
| CDS | Spread in bps | 200.0 |
| FX | Spot rate | 1.0850 |
| Caps/Floors | Vol (decimal) | 0.20 |
| Swaptions | Vol (decimal or bps, normal) | 0.0050 |

## Settlement Conventions

| Market | Settlement | Notes |
|--------|-----------|-------|
| US Treasuries | T+1 | Since May 2024 |
| US Corporates | T+2 | Standard |
| Interest Rate Swaps | T+2 | |
| CDS | T+1 (cash) | IMM dates for rolls |
| FX Spot | T+2 | T+1 for USD/CAD, USD/MXN |
| Equities (US) | T+1 | Since May 2024 |
| TBA (MBS) | SIFMA class | 48-hour notification |

## Compounding Conventions

| Convention | Formula | Use |
|-----------|---------|-----|
| Simple | $df = \frac{1}{1 + r \cdot t}$ | Money market, short-dated |
| Continuous | $df = e^{-r \cdot t}$ | Curve bootstrap, analytics |
| Annual | $df = \frac{1}{(1+r)^t}$ | Annual bonds |
| Semi-annual | $df = \frac{1}{(1+r/2)^{2t}}$ | US Treasury, corporates |

## IMM Dates

Standard quarterly dates for futures and CDS rolls:

| Month | Date |
|-------|------|
| March | 3rd Wednesday |
| June | 3rd Wednesday |
| September | 3rd Wednesday |
| December | 3rd Wednesday |
