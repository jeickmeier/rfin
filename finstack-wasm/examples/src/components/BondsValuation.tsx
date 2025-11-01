import React, { useEffect, useState } from 'react';
import {
  AmortizationSpec,
  Bond,
  BusinessDayConvention,
  FsDate,
  DayCount,
  DiscountCurve,
  ForwardCurve,
  Frequency,
  MarketContext,
  Money,
  PricingRequest,
  StubKind,
  createStandardRegistry,
} from 'finstack-wasm';

const currencyFormatter = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 2,
});

type BondRow = {
  id: string;
  name: string;
  kind: string;
  couponLabel: string;
  marginLabel?: string;
  notes: string[];
  presentValue: number;
  cleanPrice: number;
  accrued: number;
  duration: number;
  dv01: number;
  ytm: number | null;
  zSpread: number | null;
  quotedPrice: number | null;
  bond: Bond;
};

type CashflowRow = {
  date: string;
  amount: number;
  kind: string;
  outstanding: number;
};

const metricKeys = ['accrued', 'clean_price', 'duration_mod','ytm','z_spread'];

export const BondsValuationExample: React.FC = () => {
  const [rows, setRows] = useState<BondRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [expandedBondId, setExpandedBondId] = useState<string | null>(null);
  const [cashflows, setCashflows] = useState<Map<string, CashflowRow[]>>(new Map());
  const [marketContext, setMarketContext] = useState<MarketContext | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const notional = Money.fromCode(1_000_000, 'USD');
        const issue = new FsDate(2024, 1, 15);
        const valuationDate = new FsDate(2024, 3, 15);

        const discountCurve = new DiscountCurve(
          'USD-OIS',
          valuationDate,
          new Float64Array([0.0, 0.25, 0.5, 1.0, 2.0, 3.0, 5.0]),
          new Float64Array([1.0, 0.9975, 0.994, 0.985, 0.965, 0.945, 0.915]),
          'act_365f',
          'monotone_convex',
          'flat_forward',
          true
        );

        const forwardCurve = new ForwardCurve(
          'USD-SOFR-3M',
          valuationDate,
          0.25,  // Tenor in years (3 months = 0.25)
          new Float64Array([0.25, 0.5, 1.0, 2.0, 3.0]),
          new Float64Array([0.053, 0.054, 0.055, 0.056, 0.057]),
          'act_360',
          2,
          'linear'
        );

        const market = new MarketContext();
        market.insertDiscount(discountCurve);
        market.insertForward(forwardCurve);

        const registry = createStandardRegistry();

        // Store market context for cashflow generation
        if (!cancelled) {
          setMarketContext(market);
        }

        const evaluateBond = (bond: Bond, bondName: string): BondRow => {
          const opts = new PricingRequest().withMetrics(metricKeys);
          const result = registry.priceBond(bond, 'discounting', market, opts);
          
          // Extract primitives immediately to avoid GC issues
          const presentValue = result.presentValue.amount;
          const quotedPrice = bond.quotedCleanPrice ?? null;
          
          const cleanPrice = result.metric('clean_price') ?? 0;
          const accrued = result.metric('accrued') ?? 0;
          const duration = result.metric('duration_mod') ?? 0;
          const dv01 = result.metric('dv01') ?? 0;
          const ytm = result.metric('ytm') ?? null;
          const zSpread = result.metric('z_spread') ?? null;

          return {
            id: bond.instrumentId,
            name: bondName,
            kind: '',
            couponLabel: '',
            marginLabel: undefined,
            notes: [],
            presentValue,
            cleanPrice,
            accrued,
            duration,
            dv01,
            ytm,
            zSpread,
            quotedPrice,
            bond,
          };
        };

        // Create bonds with quoted clean prices
        // IMPORTANT: quoted_clean_price expects a percentage of par (e.g., 99.5 for 99.5% of par),
        // NOT an absolute dollar amount.
        const quotedPricePct = 99.5; // 99.5% of par
        const fixedBond = Bond.fixedSemiannual('corp_fixed_2029', notional, 0.045, issue, new FsDate(2029, 1, 15), 'USD-OIS', quotedPricePct);
        
        const fixedRow = evaluateBond(fixedBond, '5Y Corporate Fixed');
        fixedRow.kind = 'Fixed Coupon';
        fixedRow.couponLabel = '4.50% semi-annual';
        fixedRow.notes = ['Constructed with Bond.fixedSemiannual helper', `Quoted at ${quotedPricePct}% of par`];

        const zeroQuotedPricePct = 95.0; // 95.0% of par
        const zeroBond = Bond.zeroCoupon('corp_zero_2027', notional, issue, new FsDate(2027, 1, 15), 'USD-OIS', zeroQuotedPricePct);
        const zeroRow = evaluateBond(zeroBond, '3Y Discount Note');
        zeroRow.kind = 'Zero Coupon';
        zeroRow.couponLabel = '0.00% (discount)';
        zeroRow.notes = ['Created via Bond.zeroCoupon', `Quoted at ${zeroQuotedPricePct}% of par`];

        // Floating rate bond
        const floatQuotedPricePct = 100.25;
        let floatingBond: Bond;
        try {
          floatingBond = Bond.floating(
            'corp_frn_2027',
            notional,
            issue,
            new FsDate(2027, 1, 15),
            'USD-OIS',
            'USD-SOFR-3M',
            150.0, // 150 bps margin
            floatQuotedPricePct
          );
        } catch (err) {
          console.error('Floating bond construction error:', err);
          throw err; // Re-throw to be caught by outer try-catch
        }
        const floatingRow = evaluateBond(floatingBond, '3Y Floating Rate Note');
        floatingRow.kind = 'Floating Rate';
        floatingRow.couponLabel = 'SOFR 3M';
        floatingRow.marginLabel = '+150 bps';
        floatingRow.notes = [
          'Created via Bond.floating',
          'Quarterly resets tied to USD-SOFR-3M',
          `Quoted at ${floatQuotedPricePct}% of par`,
        ];

        // Amortizing bond with linear amortization
        // EXAMPLE: Using the full Bond constructor (builder pattern) for maximum control
        const amortQuotedPricePct = 98.5;
        const finalNotional = Money.fromCode(200_000, 'USD'); // Amortize down to 200k
        const amortSpec = AmortizationSpec.linearTo(finalNotional);
        
        const amortBond = new Bond(
          'corp_amort_2029',           // instrumentId
          notional,                     // notional
          issue,                        // issue date
          new FsDate(2029, 1, 15),     // maturity date
          'USD-OIS',                    // discount curve
          0.055,                        // coupon rate
          Frequency.semiAnnual(),       // frequency
          DayCount.thirty360(),         // day count
          BusinessDayConvention.ModifiedFollowing,  // business day convention
          undefined,                    // calendar (optional)
          StubKind.none(),             // stub kind
          amortSpec,                    // amortization spec
          undefined,                    // call schedule (optional)
          undefined,                    // put schedule (optional)
          amortQuotedPricePct          // quoted clean price (optional)
        );
        const amortRow = evaluateBond(amortBond, '5Y Amortizing Bond');
        amortRow.kind = 'Amortizing';
        amortRow.couponLabel = '5.50% semi-annual';
        amortRow.notes = [
          'Built using full Bond constructor (all parameters explicit)',
          'Linear amortization from $1M to $200K',
          'Principal payments reduce outstanding balance',
          `Quoted at ${amortQuotedPricePct}% of par`,
        ];

        // Callable bond with call schedule
        const callQuotedPricePct = 102.0;
        const callSchedule = [
          ['2026-01-15', 103.0],  // Callable after 2 years at 103%
          ['2027-01-15', 102.0],  // After 3 years at 102%
          ['2028-01-15', 101.0],  // After 4 years at 101%
        ];
        
        const callableBond = new Bond(
          'corp_call_2029',
          notional,
          issue,
          new FsDate(2029, 1, 15),
          'USD-OIS',
          0.06, // 6.0% coupon
          Frequency.semiAnnual(),
          DayCount.thirty360(),
          BusinessDayConvention.ModifiedFollowing,
          undefined, // calendar
          StubKind.none(),
          undefined, // amortization
          callSchedule,
          undefined, // put schedule
          callQuotedPricePct
        );
        const callableRow = evaluateBond(callableBond, '5Y Callable Bond');
        callableRow.kind = 'Callable';
        callableRow.couponLabel = '6.00% semi-annual';
        callableRow.notes = [
          'Callable starting 2026-01-15 at 103%',
          'Call prices step down: 103% → 102% → 101%',
          `Quoted at ${callQuotedPricePct}% of par`,
        ];

        // Fixed-to-Floating bond
        // EXAMPLE: Using Bond.fixedToFloating() helper for windowed cashflows
        const fixToFloatQuotedPricePct = 99.75;
        const switchDate = new FsDate(2026, 1, 15); // Switch after 2 years
        const fixToFloatBond = Bond.fixedToFloating(
          'corp_fix2flt_2029',
          notional,
          0.05,              // 5% fixed rate for first 2 years
          switchDate,        // Switch date
          'USD-SOFR-3M',     // Forward curve after switch
          100.0,             // 100 bps margin (DM) over SOFR
          issue,
          new FsDate(2029, 1, 15),
          Frequency.quarterly(),
          DayCount.act360(),
          'USD-OIS',
          fixToFloatQuotedPricePct,
          market             // Market context for forward rate lookup
        );
        const fixToFloatRow = evaluateBond(fixToFloatBond, '5Y Fixed-to-Floating');
        fixToFloatRow.kind = 'Fixed-to-Floating';
        fixToFloatRow.couponLabel = '5.00% → SOFR 3M + 100bps';
        fixToFloatRow.notes = [
          'Created via Bond.fixedToFloating helper',
          'Fixed at 5% until 2026-01-15',
          'Then floats at SOFR 3M + 100 bps (discount margin)',
          'Cashflows show Fixed column, then Float column',
          `Quoted at ${fixToFloatQuotedPricePct}% of par`,
        ];

        // PIK Toggle bond (50/50 Cash/PIK)
        // EXAMPLE: Using Bond.pikToggle() helper method
        const pikQuotedPricePct = 97.0;
        const pikBond = Bond.pikToggle(
          'corp_pik_2029',
          notional,
          0.08, // 8% coupon
          0.5,  // 50% cash
          0.5,  // 50% PIK
          issue,
          new FsDate(2029, 1, 15),
          'USD-OIS',
          pikQuotedPricePct,
          market             // Market context (not used for fixed PIK, but required)
        );
        const pikRow = evaluateBond(pikBond, '5Y PIK-Toggle Bond');
        pikRow.kind = 'PIK Toggle';
        pikRow.couponLabel = '8.00% (50/50 Cash/PIK)';
        pikRow.notes = [
          'Created via Bond.pikToggle helper',
          '50% of each coupon paid in cash (Fixed column)',
          '50% of each coupon capitalized into principal (PIK column)',
          'Outstanding balance increases as PIK interest compounds',
          `Quoted at ${pikQuotedPricePct}% of par`,
        ];

        if (!cancelled) {
          const allRows = [fixedRow, zeroRow, amortRow, callableRow, fixToFloatRow, pikRow];
          if (floatingRow) {
            allRows.splice(2, 0, floatingRow); // Insert floating row at position 2
          }
          setRows(allRows);
        }
      } catch (err) {
        if (!cancelled) {
          console.error('Error:', err);
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const loadCashflows = (bondId: string, bond: Bond) => {
    if (!marketContext) {
      console.error('Market context not available');
      return;
    }

    try {
      // Toggle expanded state
      if (expandedBondId === bondId) {
        setExpandedBondId(null);
        return;
      }

      // Load cashflows if not already loaded
      if (!cashflows.has(bondId)) {
        const rawCashflows = bond.getCashflows(marketContext);
        const flowData: CashflowRow[] = [];
        
        for (let i = 0; i < rawCashflows.length; i++) {
          const entry = rawCashflows[i] as any;
          const date = entry[0];
          const money = entry[1];
          const kind = entry[2] as string;
          const outstanding = entry[3] as number;
          
          flowData.push({
            date: `${date.year}-${String(date.month).padStart(2, '0')}-${String(date.day).padStart(2, '0')}`,
            amount: money.amount,
            kind: kind,
            outstanding: outstanding,
          });
        }

        setCashflows(new Map(cashflows).set(bondId, flowData));
      }

      setExpandedBondId(bondId);
    } catch (err) {
      console.error('Failed to load cashflows:', err);
      setError(`Failed to load cashflows: ${(err as Error).message}`);
    }
  };

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (rows.length === 0) {
    return <p>Building bond valuations…</p>;
  }

  return (
    <section className="example-section">
      <h2>Bond Instruments &amp; Valuation Metrics</h2>
      <p>
        Pricing and risk metrics are sourced directly from the finstack Rust pricing registry. This
        example demonstrates a variety of bond structures including fixed and floating rate bonds,
        zero-coupon bonds, amortizing bonds, callable bonds, PIK-toggle structures, and fixed-to-floating
        notes. Each bond is valued using market curves and priced with standard metrics like YTM,
        Z-spread, modified duration, and DV01.
      </p>

      <table>
        <thead>
          <tr>
            <th>Instrument</th>
            <th>Type</th>
            <th>Coupon</th>
            <th>Quoted Price</th>
            <th>PV</th>
            <th>Clean Price</th>
            <th>Accrued</th>
            <th>Mod. Duration</th>
            <th>YTM</th>
            <th>Z-Spread</th>
            <th>Cashflows</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ id, name, kind, couponLabel, marginLabel, quotedPrice, presentValue, cleanPrice, accrued, ytm, zSpread, duration, bond }) => (
            <React.Fragment key={id}>
              <tr>
                <td>{name}</td>
                <td>{kind}</td>
                <td>
                  {couponLabel}
                  {marginLabel ? (
                    <>
                      <br />
                      {marginLabel}
                    </>
                  ) : null}
                </td>
                <td>{quotedPrice !== null ? `${quotedPrice.toFixed(2)}%` : '—'}</td>
                <td>{currencyFormatter.format(presentValue)}</td>
                <td>{currencyFormatter.format(cleanPrice)}</td>
                <td>{currencyFormatter.format(accrued)}</td>
                <td>{duration.toFixed(4)}</td>
                <td>{ytm ? (ytm * 100).toFixed(2) : '—'}</td>
                <td>{zSpread ? (zSpread * 10000).toFixed(0) : '—'}</td>
                <td>
                  <button
                    onClick={() => loadCashflows(id, bond)}
                    style={{
                      padding: '0.5rem 1rem',
                      cursor: 'pointer',
                      backgroundColor: expandedBondId === id ? '#646cff' : 'rgba(100, 108, 255, 0.1)',
                      color: expandedBondId === id ? 'white' : 'inherit',
                      border: '1px solid #646cff',
                      borderRadius: '4px',
                    }}
                  >
                    {expandedBondId === id ? 'Hide' : 'Show'}
                  </button>
                </td>
              </tr>
              {expandedBondId === id && cashflows.has(id) && (() => {
                const flows = cashflows.get(id)!;
                
                // Determine which columns are needed
                const hasFixed = flows.some(f => f.kind === 'Fixed');
                const hasFloat = flows.some(f => f.kind === 'Float');
                const hasNotional = flows.some(f => f.kind === 'Notional');
                const hasAmortization = flows.some(f => f.kind === 'Amortization');
                const hasPIK = flows.some(f => f.kind === 'PIK');
                const hasFee = flows.some(f => f.kind === 'Fee');
                
                return (
                  <tr>
                    <td colSpan={11} style={{ padding: '1rem', backgroundColor: 'rgba(100, 108, 255, 0.05)' }}>
                      <h4 style={{ marginBottom: '0.5rem' }}>Cashflow Schedule for {name}</h4>
                      <table style={{ width: '100%', marginTop: '0.5rem' }}>
                        <thead>
                          <tr>
                            <th style={{ textAlign: 'left' }}>Date</th>
                            {hasFixed && <th style={{ textAlign: 'right' }}>Fixed</th>}
                            {hasFloat && <th style={{ textAlign: 'right' }}>Float</th>}
                            {hasPIK && <th style={{ textAlign: 'right' }}>PIK</th>}
                            {hasAmortization && <th style={{ textAlign: 'right' }}>Amortization</th>}
                            {hasNotional && <th style={{ textAlign: 'right' }}>Notional</th>}
                            {hasFee && <th style={{ textAlign: 'right' }}>Fee</th>}
                            <th style={{ textAlign: 'right' }}>Outstanding</th>
                          </tr>
                        </thead>
                        <tbody>
                          {flows.map((flow, idx) => (
                            <tr key={idx}>
                              <td>{flow.date}</td>
                              {hasFixed && <td style={{ textAlign: 'right' }}>{flow.kind === 'Fixed' ? currencyFormatter.format(flow.amount) : '—'}</td>}
                              {hasFloat && <td style={{ textAlign: 'right' }}>{flow.kind === 'Float' ? currencyFormatter.format(flow.amount) : '—'}</td>}
                              {hasPIK && <td style={{ textAlign: 'right' }}>{flow.kind === 'PIK' ? currencyFormatter.format(flow.amount) : '—'}</td>}
                              {hasAmortization && <td style={{ textAlign: 'right' }}>{flow.kind === 'Amortization' ? currencyFormatter.format(flow.amount) : '—'}</td>}
                              {hasNotional && <td style={{ textAlign: 'right' }}>{flow.kind === 'Notional' ? currencyFormatter.format(flow.amount) : '—'}</td>}
                              {hasFee && <td style={{ textAlign: 'right' }}>{flow.kind === 'Fee' ? currencyFormatter.format(flow.amount) : '—'}</td>}
                              <td style={{ textAlign: 'right', fontWeight: 'bold' }}>{currencyFormatter.format(flow.outstanding)}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </td>
                  </tr>
                );
              })()}
            </React.Fragment>
          ))}
        </tbody>
      </table>

      {rows.map(({ id, name, notes }) => (
        <details key={`${id}-details`}>
          <summary>{name} details</summary>
          <ul>
            {notes.map((note) => (
              <li key={note}>{note}</li>
            ))}
          </ul>
        </details>
      ))}
    </section>
  );
};
