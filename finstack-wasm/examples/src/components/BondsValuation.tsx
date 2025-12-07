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
import { BondsValuationProps, DEFAULT_BONDS_PROPS, BondData } from './data/bonds';

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

const metricKeys = ['accrued', 'clean_price', 'duration_mod', 'ytm', 'z_spread'];

export const BondsValuationExample: React.FC<BondsValuationProps> = (props) => {
  // Merge with defaults
  const {
    valuationDate = DEFAULT_BONDS_PROPS.valuationDate!,
    discountCurve = DEFAULT_BONDS_PROPS.discountCurve!,
    forwardCurve = DEFAULT_BONDS_PROPS.forwardCurve!,
    bonds = DEFAULT_BONDS_PROPS.bonds!,
  } = props;

  const [rows, setRows] = useState<BondRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [expandedBondId, setExpandedBondId] = useState<string | null>(null);
  const [cashflows, setCashflows] = useState<Map<string, CashflowRow[]>>(new Map());
  const [marketContext, setMarketContext] = useState<MarketContext | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const valDate = new FsDate(valuationDate.year, valuationDate.month, valuationDate.day);

        // Build discount curve from props
        const discCurve = new DiscountCurve(
          discountCurve.id,
          valDate,
          new Float64Array(discountCurve.tenors),
          new Float64Array(discountCurve.discountFactors),
          discountCurve.dayCount,
          discountCurve.interpolation,
          discountCurve.extrapolation,
          discountCurve.continuous
        );

        // Build forward curve from props
        const fwdCurve = new ForwardCurve(
          forwardCurve.id,
          valDate,
          forwardCurve.tenor,
          new Float64Array(forwardCurve.tenors),
          new Float64Array(forwardCurve.rates),
          forwardCurve.dayCount,
          forwardCurve.compounding,
          forwardCurve.interpolation
        );

        const market = new MarketContext();
        market.insertDiscount(discCurve);
        market.insertForward(fwdCurve);

        const registry = createStandardRegistry();

        // Store market context for cashflow generation
        if (!cancelled) {
          setMarketContext(market);
        }

        const evaluateBond = (bond: Bond, bondName: string): BondRow => {
          const opts = new PricingRequest().withMetrics(metricKeys);
          const result = registry.priceBond(bond, 'discounting', market, valDate, opts);

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

        const buildBond = (bondData: BondData): { bond: Bond; row: BondRow } | null => {
          const notional = Money.fromCode(bondData.notional.amount, bondData.notional.currency);
          const issue = new FsDate(bondData.issueDate.year, bondData.issueDate.month, bondData.issueDate.day);
          const maturity = new FsDate(bondData.maturityDate.year, bondData.maturityDate.month, bondData.maturityDate.day);

          let bond: Bond;
          let row: BondRow;

          try {
            switch (bondData.bondType.type) {
              case 'fixed': {
                bond = Bond.fixedSemiannual(
                  bondData.id,
                  notional,
                  bondData.bondType.couponRate,
                  issue,
                  maturity,
                  bondData.discountCurveId,
                  bondData.quotedCleanPrice
                );
                row = evaluateBond(bond, bondData.name);
                row.kind = 'Fixed Coupon';
                row.couponLabel = `${(bondData.bondType.couponRate * 100).toFixed(2)}% semi-annual`;
                row.notes = [
                  'Constructed with Bond.fixedSemiannual helper',
                  bondData.quotedCleanPrice ? `Quoted at ${bondData.quotedCleanPrice}% of par` : '',
                ].filter(Boolean);
                break;
              }
              case 'zero': {
                bond = Bond.zeroCoupon(
                  bondData.id,
                  notional,
                  issue,
                  maturity,
                  bondData.discountCurveId,
                  bondData.quotedCleanPrice
                );
                row = evaluateBond(bond, bondData.name);
                row.kind = 'Zero Coupon';
                row.couponLabel = '0.00% (discount)';
                row.notes = [
                  'Created via Bond.zeroCoupon',
                  bondData.quotedCleanPrice ? `Quoted at ${bondData.quotedCleanPrice}% of par` : '',
                ].filter(Boolean);
                break;
              }
              case 'floating': {
                bond = Bond.floating(
                  bondData.id,
                  notional,
                  issue,
                  maturity,
                  bondData.discountCurveId,
                  bondData.bondType.forwardCurveId,
                  bondData.bondType.marginBps,
                  bondData.quotedCleanPrice
                );
                row = evaluateBond(bond, bondData.name);
                row.kind = 'Floating Rate';
                row.couponLabel = 'SOFR 3M';
                row.marginLabel = `+${bondData.bondType.marginBps} bps`;
                row.notes = [
                  'Created via Bond.floating',
                  `Quarterly resets tied to ${bondData.bondType.forwardCurveId}`,
                  bondData.quotedCleanPrice ? `Quoted at ${bondData.quotedCleanPrice}% of par` : '',
                ].filter(Boolean);
                break;
              }
              case 'amortizing': {
                const finalNotional = Money.fromCode(
                  bondData.bondType.finalNotional.amount,
                  bondData.bondType.finalNotional.currency
                );
                const amortSpec = AmortizationSpec.linearTo(finalNotional);
                bond = new Bond(
                  bondData.id,
                  notional,
                  issue,
                  maturity,
                  bondData.discountCurveId,
                  bondData.bondType.couponRate,
                  Frequency.semiAnnual(),
                  DayCount.thirty360(),
                  BusinessDayConvention.ModifiedFollowing,
                  undefined,
                  StubKind.none(),
                  amortSpec,
                  undefined,
                  undefined,
                  bondData.quotedCleanPrice
                );
                row = evaluateBond(bond, bondData.name);
                row.kind = 'Amortizing';
                row.couponLabel = `${(bondData.bondType.couponRate * 100).toFixed(2)}% semi-annual`;
                row.notes = [
                  'Built using full Bond constructor (all parameters explicit)',
                  `Linear amortization from $${(bondData.notional.amount / 1000).toFixed(0)}K to $${(bondData.bondType.finalNotional.amount / 1000).toFixed(0)}K`,
                  'Principal payments reduce outstanding balance',
                  bondData.quotedCleanPrice ? `Quoted at ${bondData.quotedCleanPrice}% of par` : '',
                ].filter(Boolean);
                break;
              }
              case 'callable': {
                bond = new Bond(
                  bondData.id,
                  notional,
                  issue,
                  maturity,
                  bondData.discountCurveId,
                  bondData.bondType.couponRate,
                  Frequency.semiAnnual(),
                  DayCount.thirty360(),
                  BusinessDayConvention.ModifiedFollowing,
                  undefined,
                  StubKind.none(),
                  undefined,
                  bondData.bondType.callSchedule,
                  undefined,
                  bondData.quotedCleanPrice
                );
                row = evaluateBond(bond, bondData.name);
                row.kind = 'Callable';
                row.couponLabel = `${(bondData.bondType.couponRate * 100).toFixed(2)}% semi-annual`;
                const firstCall = bondData.bondType.callSchedule[0];
                row.notes = [
                  `Callable starting ${firstCall[0]} at ${firstCall[1]}%`,
                  `Call prices step down: ${bondData.bondType.callSchedule.map(([, p]) => `${p}%`).join(' → ')}`,
                  bondData.quotedCleanPrice ? `Quoted at ${bondData.quotedCleanPrice}% of par` : '',
                ].filter(Boolean);
                break;
              }
              case 'fixedToFloating': {
                const switchDate = new FsDate(
                  bondData.bondType.switchDate.year,
                  bondData.bondType.switchDate.month,
                  bondData.bondType.switchDate.day
                );
                bond = Bond.fixedToFloating(
                  bondData.id,
                  notional,
                  bondData.bondType.fixedRate,
                  switchDate,
                  bondData.bondType.forwardCurveId,
                  bondData.bondType.marginBps,
                  issue,
                  maturity,
                  Frequency.quarterly(),
                  DayCount.act360(),
                  bondData.discountCurveId,
                  bondData.quotedCleanPrice,
                  market
                );
                row = evaluateBond(bond, bondData.name);
                row.kind = 'Fixed-to-Floating';
                row.couponLabel = `${(bondData.bondType.fixedRate * 100).toFixed(2)}% → SOFR 3M + ${bondData.bondType.marginBps}bps`;
                row.notes = [
                  'Created via Bond.fixedToFloating helper',
                  `Fixed at ${(bondData.bondType.fixedRate * 100).toFixed(0)}% until ${bondData.bondType.switchDate.year}-${String(bondData.bondType.switchDate.month).padStart(2, '0')}-${String(bondData.bondType.switchDate.day).padStart(2, '0')}`,
                  `Then floats at SOFR 3M + ${bondData.bondType.marginBps} bps (discount margin)`,
                  'Cashflows show Fixed column, then Float column',
                  bondData.quotedCleanPrice ? `Quoted at ${bondData.quotedCleanPrice}% of par` : '',
                ].filter(Boolean);
                break;
              }
              case 'pikToggle': {
                bond = Bond.pikToggle(
                  bondData.id,
                  notional,
                  bondData.bondType.couponRate,
                  bondData.bondType.cashPct,
                  bondData.bondType.pikPct,
                  issue,
                  maturity,
                  bondData.discountCurveId,
                  bondData.quotedCleanPrice,
                  market
                );
                row = evaluateBond(bond, bondData.name);
                row.kind = 'PIK Toggle';
                row.couponLabel = `${(bondData.bondType.couponRate * 100).toFixed(2)}% (${bondData.bondType.cashPct * 100}/${bondData.bondType.pikPct * 100} Cash/PIK)`;
                row.notes = [
                  'Created via Bond.pikToggle helper',
                  `${bondData.bondType.cashPct * 100}% of each coupon paid in cash (Fixed column)`,
                  `${bondData.bondType.pikPct * 100}% of each coupon capitalized into principal (PIK column)`,
                  'Outstanding balance increases as PIK interest compounds',
                  bondData.quotedCleanPrice ? `Quoted at ${bondData.quotedCleanPrice}% of par` : '',
                ].filter(Boolean);
                break;
              }
              default:
                return null;
            }
            return { bond, row };
          } catch (err) {
            console.warn(`Bond ${bondData.id} creation failed, skipping:`, err);
            return null;
          }
        };

        // Build all bonds
        const allRows: BondRow[] = [];
        for (const bondData of bonds) {
          const result = buildBond(bondData);
          if (result) {
            allRows.push(result.row);
          }
        }

        if (!cancelled) {
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
  }, [valuationDate, discountCurve, forwardCurve, bonds]);

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
          const entry = rawCashflows[i] as [FsDate, Money, string, number];
          const date = entry[0];
          const money = entry[1];
          const kind = entry[2] as string;
          const outstanding = entry[3] as number;

          flowData.push({
            date: `${date.year}-${String(date.month).padStart(2, '0')}-${String(date.day).padStart(2, '0')}`,
            amount: money.amount,
            kind,
            outstanding,
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
        zero-coupon bonds, amortizing bonds, callable bonds, PIK-toggle structures, and
        fixed-to-floating notes. Each bond is valued using market curves and priced with standard
        metrics like YTM, Z-spread, modified duration, and DV01.
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
          {rows.map(
            ({
              id,
              name,
              kind,
              couponLabel,
              marginLabel,
              quotedPrice,
              presentValue,
              cleanPrice,
              accrued,
              ytm,
              zSpread,
              duration,
              bond,
            }) => (
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
                        backgroundColor:
                          expandedBondId === id ? '#646cff' : 'rgba(100, 108, 255, 0.1)',
                        color: expandedBondId === id ? 'white' : 'inherit',
                        border: '1px solid #646cff',
                        borderRadius: '4px',
                      }}
                    >
                      {expandedBondId === id ? 'Hide' : 'Show'}
                    </button>
                  </td>
                </tr>
                {expandedBondId === id &&
                  cashflows.has(id) &&
                  (() => {
                    const flows = cashflows.get(id);
                    if (!flows) return null;

                    // Determine which columns are needed
                    const hasFixed = flows.some((f) => f.kind === 'Fixed');
                    const hasFloat = flows.some((f) => f.kind === 'Float');
                    const hasNotional = flows.some((f) => f.kind === 'Notional');
                    const hasAmortization = flows.some((f) => f.kind === 'Amortization');
                    const hasPIK = flows.some((f) => f.kind === 'PIK');
                    const hasFee = flows.some((f) => f.kind === 'Fee');

                    return (
                      <tr>
                        <td
                          colSpan={11}
                          style={{ padding: '1rem', backgroundColor: 'rgba(100, 108, 255, 0.05)' }}
                        >
                          <h4 style={{ marginBottom: '0.5rem' }}>Cashflow Schedule for {name}</h4>
                          <table style={{ width: '100%', marginTop: '0.5rem' }}>
                            <thead>
                              <tr>
                                <th style={{ textAlign: 'left' }}>Date</th>
                                {hasFixed && <th style={{ textAlign: 'right' }}>Fixed</th>}
                                {hasFloat && <th style={{ textAlign: 'right' }}>Float</th>}
                                {hasPIK && <th style={{ textAlign: 'right' }}>PIK</th>}
                                {hasAmortization && (
                                  <th style={{ textAlign: 'right' }}>Amortization</th>
                                )}
                                {hasNotional && <th style={{ textAlign: 'right' }}>Notional</th>}
                                {hasFee && <th style={{ textAlign: 'right' }}>Fee</th>}
                                <th style={{ textAlign: 'right' }}>Outstanding</th>
                              </tr>
                            </thead>
                            <tbody>
                              {flows.map((flow, idx) => (
                                <tr key={idx}>
                                  <td>{flow.date}</td>
                                  {hasFixed && (
                                    <td style={{ textAlign: 'right' }}>
                                      {flow.kind === 'Fixed'
                                        ? currencyFormatter.format(flow.amount)
                                        : '—'}
                                    </td>
                                  )}
                                  {hasFloat && (
                                    <td style={{ textAlign: 'right' }}>
                                      {flow.kind === 'Float'
                                        ? currencyFormatter.format(flow.amount)
                                        : '—'}
                                    </td>
                                  )}
                                  {hasPIK && (
                                    <td style={{ textAlign: 'right' }}>
                                      {flow.kind === 'PIK'
                                        ? currencyFormatter.format(flow.amount)
                                        : '—'}
                                    </td>
                                  )}
                                  {hasAmortization && (
                                    <td style={{ textAlign: 'right' }}>
                                      {flow.kind === 'Amortization'
                                        ? currencyFormatter.format(flow.amount)
                                        : '—'}
                                    </td>
                                  )}
                                  {hasNotional && (
                                    <td style={{ textAlign: 'right' }}>
                                      {flow.kind === 'Notional'
                                        ? currencyFormatter.format(flow.amount)
                                        : '—'}
                                    </td>
                                  )}
                                  {hasFee && (
                                    <td style={{ textAlign: 'right' }}>
                                      {flow.kind === 'Fee'
                                        ? currencyFormatter.format(flow.amount)
                                        : '—'}
                                    </td>
                                  )}
                                  <td style={{ textAlign: 'right', fontWeight: 'bold' }}>
                                    {currencyFormatter.format(flow.outstanding)}
                                  </td>
                                </tr>
                              ))}
                            </tbody>
                          </table>
                        </td>
                      </tr>
                    );
                  })()}
              </React.Fragment>
            )
          )}
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
