# Golden Test Files for FX Settlement

This directory contains golden reference data used to validate FX spot date calculations against market-standard conventions.

## Purpose

Golden files serve as:
1. **Regression Tests**: Detect unintended changes in settlement logic
2. **Convention Documentation**: Record expected behavior with explanations
3. **Vendor Validation**: Store dates verified against Bloomberg, ISDA, ECB calendars
4. **Change Tracking**: Maintain history of updates with rationale

## File Structure

### `fx_spot_dates.json`

Primary golden file containing:
- **Metadata**: Version, convention references, validation sources
- **Legacy Behavior Changes**: Documents differences from pre-Phase 2 implementation
- **Test Cases**: Array of spot date scenarios with detailed breakdowns
- **Calendar Definitions**: Reference information for each market calendar
- **Change Log**: Version history with rationale for updates

## Test Case Format

Each test case includes:
```json
{
  "id": "unique_test_identifier",
  "description": "Human-readable description",
  "currency_pair": "CCY1/CCY2",
  "trade_date": "YYYY-MM-DD",
  "spot_lag": 2,
  "base_calendar": "calendar_id",
  "quote_calendar": "calendar_id",
  "expected_spot_date": "YYYY-MM-DD",
  "business_day_breakdown": [
    "Step-by-step explanation of each day"
  ],
  "rationale": "Why this date is correct",
  "verified_against": "Bloomberg FXFA, ISDA calendar, etc."
}
```

## Maintenance Guidelines

### When to Update Golden Files

Update golden files when:
1. **Calendar Data Changes**: New holidays published by exchanges/central banks
2. **Convention Changes**: ISDA or market-wide convention updates
3. **Bug Fixes**: Correcting previously incorrect expected dates
4. **New Test Cases**: Adding coverage for new currency pairs or scenarios

### How to Update Golden Files

1. **Verify the Change**:
   - Check official calendar sources (ECB, NYSE, Bank of England, etc.)
   - Cross-reference with Bloomberg FXFA or equivalent vendor tools
   - Document the source in the test case

2. **Update the JSON**:
   - Modify `expected_spot_date` and `business_day_breakdown`
   - Update `rationale` with explanation
   - Add/update `verified_against` sources

3. **Update Change Log**:
   ```json
   {
     "date": "YYYY-MM-DD",
     "version": "X.Y.Z",
     "changes": "Description of what changed",
     "author": "Your Name",
     "rationale": "Why the change was made"
   }
   ```

4. **Run Tests**:
   ```bash
   cd finstack/valuations
   cargo test --test integration_tests fx_settlement
   ```

5. **Document in Commit**:
   - Reference the test case ID
   - Link to official calendar source
   - Explain impact on existing settlements

### DO NOT Update Without Verification

❌ **Never** change expected dates based solely on:
- Test failures without investigation
- Assumptions about holiday dates
- Untested calendar changes

✅ **Always** verify against:
- Official exchange/central bank calendars
- ISDA published calendars
- Bloomberg FXFA or equivalent vendor tools
- Historical settlement data where available

## Calendar Sources

### Official Calendar URLs

- **NYSE**: https://www.nyse.com/markets/hours-calendars
- **ECB TARGET2**: https://www.ecb.europa.eu/paym/target/target2/profuse/calendar/html/index.en.html
- **Bank of England**: https://www.bankofengland.co.uk/
- **JPX (Tokyo)**: https://www.jpx.co.jp/english/corporate/about-jpx/calendar/
- **ISDA**: https://www.isda.org/category/legal/confirmation-closeout-settlement/

### Vendor Validation

- **Bloomberg**: FXFA function (FX Forward Analysis)
- **Refinitiv**: FXFA equivalent
- **CME**: Holiday calendar tools

## Common Pitfalls

### Weekend Adjustment Rules

Different markets have different rules for when holidays fall on weekends:
- **US (NYSE)**: Saturday holiday → observe Friday; Sunday → observe Monday
- **Japan (JPX)**: Sunday holiday → observe next non-holiday Monday (substitute holiday)
- **UK (GBLO)**: Weekend holiday → substitute Monday
- **TARGET2**: No substitute days; fixed dates only

### Golden Week (Japan)

Japan's Golden Week (late April - early May) includes:
- Showa Day (April 29)
- Constitution Day (May 3)
- Greenery Day (May 4)
- Children's Day (May 5)

When these fall on weekends, substitute holidays apply. This creates extended closures that impact settlement.

### Year-End Closures

December 25-26 and January 1 are widely observed but with variations:
- **Christmas**: Universal closure (Dec 25)
- **Boxing Day**: UK, Eurozone (Dec 26), not US
- **New Year's Day**: Universal closure (Jan 1)

## Integration with Tests

The integration tests in `tests/integration/fx_settlement.rs` use these golden files for:
1. **Regression Detection**: Alert if implementation changes unexpectedly
2. **Convention Validation**: Verify joint business day counting logic
3. **Edge Case Coverage**: Test holiday clusters, substitute days, multi-currency scenarios

Each test case in the golden file corresponds to at least one integration test.

## Versioning

Golden files use semantic versioning:
- **Major**: Breaking changes to file format or convention interpretation
- **Minor**: New test cases or calendar updates
- **Patch**: Corrections to existing test cases

Current version: `1.0.0`

## Support

For questions about:
- **Calendar accuracy**: Consult official exchange calendars
- **ISDA conventions**: Reference ISDA documentation and industry contacts
- **Implementation**: See `fx_dates.rs` and integration test documentation
- **Vendor validation**: Use Bloomberg FXFA, Refinitiv tools

## References

- [ISDA FX Settlement](https://www.isda.org/category/legal/confirmation-closeout-settlement/)
- [ECB TARGET2 Calendar](https://www.ecb.europa.eu/paym/target/target2/profuse/calendar/html/index.en.html)
- [NYSE Holidays](https://www.nyse.com/markets/hours-calendars)
- [Bank of England Calendar](https://www.bankofengland.co.uk/)
- [JPX Calendar](https://www.jpx.co.jp/english/corporate/about-jpx/calendar/)
