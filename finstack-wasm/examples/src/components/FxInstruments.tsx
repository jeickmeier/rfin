import React, { useEffect, useState } from 'react';
import {
  Currency,
  FsDate,
  DiscountCurve,
  FxMatrix,
  FxOption,
  FxSpot,
  FxSwap,
  MarketContext,
  Money,
  PricingRequest,
  VolSurface,
  createStandardRegistry,
} from 'finstack-wasm';
import { FxInstrumentsProps, DEFAULT_FX_PROPS } from './data/fx';

const currencyFormatter = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 2,
});

type InstrumentRow = {
  name: string;
  type: string;
  pair: string;
  presentValue: number;
  keyMetric?: { name: string; value: number };
};

export const FxInstrumentsExample: React.FC<FxInstrumentsProps> = (props) => {
  // Merge with defaults
  const {
    valuationDate = DEFAULT_FX_PROPS.valuationDate!,
    discountCurves = DEFAULT_FX_PROPS.discountCurves!,
    volSurface = DEFAULT_FX_PROPS.volSurface!,
    fxQuotes = DEFAULT_FX_PROPS.fxQuotes!,
    spots = DEFAULT_FX_PROPS.spots!,
    options = DEFAULT_FX_PROPS.options!,
    swaps = DEFAULT_FX_PROPS.swaps!,
  } = props;

  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(valuationDate.year, valuationDate.month, valuationDate.day);

        // Build market context
        const market = new MarketContext();

        // Build discount curves from props
        for (const curveData of discountCurves) {
          const curveBaseDate = new FsDate(
            curveData.baseDate.year,
            curveData.baseDate.month,
            curveData.baseDate.day
          );
          const curve = new DiscountCurve(
            curveData.id,
            curveBaseDate,
            new Float64Array(curveData.tenors),
            new Float64Array(curveData.discountFactors),
            curveData.dayCount,
            curveData.interpolation,
            curveData.extrapolation,
            curveData.continuous
          );
          market.insertDiscount(curve);
        }

        // Build FX matrix from props
        const fx = new FxMatrix();
        for (const quote of fxQuotes) {
          const base = new Currency(quote.base);
          const quoteCcy = new Currency(quote.quote);
          fx.setQuote(base, quoteCcy, quote.rate);
        }
        market.insertFx(fx);

        // Build volatility surface from props
        const fxVol = new VolSurface(
          volSurface.id,
          new Float64Array(volSurface.expiries),
          new Float64Array(volSurface.strikes),
          new Float64Array(volSurface.vols)
        );
        market.insertSurface(fxVol);

        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        // Process FX spots
        for (const spot of spots) {
          const baseCcy = new Currency(spot.baseCurrency);
          const quoteCcy = new Currency(spot.quoteCurrency);
          const settlementDate = new FsDate(
            spot.settlementDate.year,
            spot.settlementDate.month,
            spot.settlementDate.day
          );
          const notional = Money.fromCode(spot.notional.amount, spot.notional.currency);

          const fxSpot = new FxSpot(
            spot.id,
            baseCcy,
            quoteCcy,
            settlementDate,
            spot.rate,
            notional
          );
          const spotResult = registry.priceFxSpot(fxSpot, 'discounting', market, asOf);
          results.push({
            name: `${spot.baseCurrency}/${spot.quoteCurrency} Spot`,
            type: 'FxSpot',
            pair: `${spot.baseCurrency}${spot.quoteCurrency}`,
            presentValue: spotResult.presentValue.amount,
          });
        }

        // Process FX options
        for (const opt of options) {
          const baseCcy = new Currency(opt.baseCurrency);
          const quoteCcy = new Currency(opt.quoteCurrency);
          const expiryDate = new FsDate(opt.expiryDate.year, opt.expiryDate.month, opt.expiryDate.day);
          const notional = Money.fromCode(opt.notional.amount, opt.notional.currency);

          const option =
            opt.optionType === 'call'
              ? FxOption.europeanCall(
                  opt.id,
                  baseCcy,
                  quoteCcy,
                  opt.strike,
                  expiryDate,
                  notional,
                  opt.domesticCurveId,
                  opt.foreignCurveId,
                  opt.volSurfaceId
                )
              : FxOption.europeanPut(
                  opt.id,
                  baseCcy,
                  quoteCcy,
                  opt.strike,
                  expiryDate,
                  notional,
                  opt.domesticCurveId,
                  opt.foreignCurveId,
                  opt.volSurfaceId
                );

          const isCall = opt.optionType === 'call';
          const optReq = isCall ? new PricingRequest().withMetrics(['delta']) : null;
          const optResult = registry.priceFxOption(option, 'discounting', market, asOf, optReq);

          const tenorMonths =
            (opt.expiryDate.year - valuationDate.year) * 12 +
            (opt.expiryDate.month - valuationDate.month);
          const tenorDesc = tenorMonths >= 12 ? `${tenorMonths / 12}Y` : `${tenorMonths}M`;

          results.push({
            name: `${tenorDesc} ${opt.optionType === 'call' ? 'Call' : 'Put'} @ ${opt.strike.toFixed(2)}`,
            type: 'FxOption',
            pair: `${opt.baseCurrency}${opt.quoteCurrency}`,
            presentValue: optResult.presentValue.amount,
            keyMetric: isCall
              ? {
                  name: 'Delta',
                  // Normalize delta to per-unit if it comes back as notional-adjusted
                  value:
                    Math.abs(optResult.metric('delta') ?? 0) > 100
                      ? (optResult.metric('delta') ?? 0) / opt.notional.amount
                      : (optResult.metric('delta') ?? 0),
                }
              : undefined,
          });
        }

        // Process FX swaps
        for (const swap of swaps) {
          const baseCcy = new Currency(swap.baseCurrency);
          const quoteCcy = new Currency(swap.quoteCurrency);
          const notional = Money.fromCode(swap.notional.amount, swap.notional.currency);
          const nearDate = new FsDate(swap.nearDate.year, swap.nearDate.month, swap.nearDate.day);
          const farDate = new FsDate(swap.farDate.year, swap.farDate.month, swap.farDate.day);

          const fxSwap = new FxSwap(
            swap.id,
            baseCcy,
            quoteCcy,
            notional,
            nearDate,
            farDate,
            swap.domesticCurveId,
            swap.foreignCurveId,
            swap.nearRate,
            swap.farRate
          );
          const swapResult = registry.priceFxSwap(fxSwap, 'discounting', market, asOf);

          const tenorMonths =
            (swap.farDate.year - swap.nearDate.year) * 12 +
            (swap.farDate.month - swap.nearDate.month);

          results.push({
            name: `${tenorMonths}M FX Swap`,
            type: 'FxSwap',
            pair: `${swap.baseCurrency}${swap.quoteCurrency}`,
            presentValue: swapResult.presentValue.amount,
          });
        }

        if (!cancelled) {
          setRows(results);
        }
      } catch (err) {
        if (!cancelled) {
          console.error('FX instruments error:', err);
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [valuationDate, discountCurves, volSurface, fxQuotes, spots, options, swaps]);

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (rows.length === 0) {
    return <p>Building FX instruments…</p>;
  }

  return (
    <section className="example-section">
      <h2>FX Instruments</h2>
      <p>
        Foreign exchange instruments including spot transactions, European options (calls/puts), and
        FX swaps with near and far legs.
      </p>

      <table>
        <thead>
          <tr>
            <th>Instrument</th>
            <th>Type</th>
            <th>Pair</th>
            <th>Present Value</th>
            <th>Key Metric</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ name, type, pair, presentValue, keyMetric }) => (
            <tr key={name}>
              <td>{name}</td>
              <td>{type}</td>
              <td>{pair}</td>
              <td>{currencyFormatter.format(presentValue)}</td>
              <td>{keyMetric ? `${keyMetric.name}: ${keyMetric.value.toFixed(4)}` : '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};
