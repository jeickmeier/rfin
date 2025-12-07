import React, { useEffect, useState } from 'react';
import {
  Currency,
  FsDate,
  DiscountCurve,
  Equity,
  EquityOption,
  MarketContext,
  MarketScalar,
  Money,
  PricingRequest,
  VolSurface,
  createStandardRegistry,
} from 'finstack-wasm';
import { EquityInstrumentsProps, DEFAULT_EQUITY_PROPS } from './data/equity';

type RequiredEquityInstrumentsProps = Required<EquityInstrumentsProps>;

const currencyFormatter = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 2,
});

type InstrumentRow = {
  name: string;
  type: string;
  ticker: string;
  presentValue: number;
  keyMetric?: { name: string; value: number };
};

export const EquityInstrumentsExample: React.FC<EquityInstrumentsProps> = (props) => {
  // Merge with defaults - DEFAULT_EQUITY_PROPS always has these values defined
  const defaults = DEFAULT_EQUITY_PROPS as RequiredEquityInstrumentsProps;
  const {
    valuationDate = defaults.valuationDate,
    discountCurve = defaults.discountCurve,
    volSurface = defaults.volSurface,
    marketData = defaults.marketData,
    positions = defaults.positions,
    options = defaults.options,
  } = props;

  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(valuationDate.year, valuationDate.month, valuationDate.day);
        const usd = new Currency('USD');

        // Build discount curve from props
        const curveBaseDate = new FsDate(
          discountCurve.baseDate.year,
          discountCurve.baseDate.month,
          discountCurve.baseDate.day
        );
        const curve = new DiscountCurve(
          discountCurve.id,
          curveBaseDate,
          new Float64Array(discountCurve.tenors),
          new Float64Array(discountCurve.discountFactors),
          discountCurve.dayCount,
          discountCurve.interpolation,
          discountCurve.extrapolation,
          discountCurve.continuous
        );

        // Build volatility surface from props
        const equityVol = new VolSurface(
          volSurface.id,
          new Float64Array(volSurface.expiries),
          new Float64Array(volSurface.strikes),
          new Float64Array(volSurface.vols)
        );

        const market = new MarketContext();
        market.insertDiscount(curve);
        market.insertSurface(equityVol);

        // Insert market data for each ticker
        for (const data of marketData) {
          const spotPrice = Money.fromCode(data.spotPrice.amount, data.spotPrice.currency);
          market.insertPrice(data.ticker, MarketScalar.price(spotPrice));
          market.insertPrice(`${data.ticker}-SPOT`, MarketScalar.price(spotPrice));
          market.insertPrice('EQUITY-SPOT', MarketScalar.price(spotPrice));
          market.insertPrice(`${data.ticker}-DIVYIELD`, MarketScalar.unitless(data.dividendYield));
          market.insertPrice('EQUITY-DIVYIELD', MarketScalar.unitless(data.dividendYield));
        }

        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        // Process equity positions
        for (const pos of positions) {
          const equity = new Equity(pos.id, pos.ticker, usd, pos.quantity, pos.costBasis);
          const equityResult = registry.priceEquity(equity, 'discounting', market, asOf);
          results.push({
            name: `${pos.ticker} Stock (${pos.quantity} shares)`,
            type: 'Equity',
            ticker: pos.ticker,
            presentValue: equityResult.presentValue.amount,
          });
        }

        // Process options
        for (const opt of options) {
          const expiryDate = new FsDate(
            opt.expiryDate.year,
            opt.expiryDate.month,
            opt.expiryDate.day
          );
          const spotPrice = Money.fromCode(opt.spotPrice.amount, opt.spotPrice.currency);

          const option =
            opt.optionType === 'call'
              ? EquityOption.europeanCall(
                  opt.id,
                  opt.ticker,
                  opt.strike,
                  expiryDate,
                  spotPrice,
                  opt.quantity
                )
              : EquityOption.europeanPut(
                  opt.id,
                  opt.ticker,
                  opt.strike,
                  expiryDate,
                  spotPrice,
                  opt.quantity
                );

          const isCall = opt.optionType === 'call';
          const opts = isCall ? new PricingRequest().withMetrics(['delta', 'gamma']) : null;
          const optResult = registry.priceEquityOption(option, 'discounting', market, asOf, opts);

          const tenorDesc =
            opt.expiryDate.month === 12 ? '1Y' : `${opt.expiryDate.month - valuationDate.month}M`;

          results.push({
            name: `${opt.ticker} ${opt.optionType === 'call' ? 'Call' : 'Put'} @ $${opt.strike} (${tenorDesc})`,
            type: 'EquityOption',
            ticker: opt.ticker,
            presentValue: optResult.presentValue.amount,
            keyMetric: isCall
              ? { name: 'Delta', value: optResult.metric('delta') ?? 0 }
              : undefined,
          });
        }

        if (!cancelled) {
          setRows(results);
        }
      } catch (err) {
        if (!cancelled) {
          console.error('Equity instruments error:', err);
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [valuationDate, discountCurve, volSurface, marketData, positions, options]);

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (rows.length === 0) {
    return <p>Building equity instruments…</p>;
  }

  return (
    <section className="example-section">
      <h2>Equity Instruments</h2>
      <p>
        Equity spot positions and European-style equity options (calls and puts). Options are priced
        using market data for spot prices and dividend yields.
      </p>

      <table>
        <thead>
          <tr>
            <th>Instrument</th>
            <th>Type</th>
            <th>Ticker</th>
            <th>Present Value</th>
            <th>Key Metric</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ name, type, ticker, presentValue, keyMetric }) => (
            <tr key={name}>
              <td>{name}</td>
              <td>{type}</td>
              <td>{ticker}</td>
              <td>{currencyFormatter.format(presentValue)}</td>
              <td>{keyMetric ? `${keyMetric.name}: ${keyMetric.value.toFixed(4)}` : '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};
