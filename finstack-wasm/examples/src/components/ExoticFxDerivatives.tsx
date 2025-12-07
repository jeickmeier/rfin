import React, { useEffect, useState } from 'react';
import {
  Currency,
  FsDate,
  DiscountCurve,
  FxBarrierOption,
  QuantoOption,
  FxMatrix,
  MarketContext,
  MarketScalar,
  Money,
  VolSurface,
  createStandardRegistry,
} from 'finstack-wasm';
import { ExoticFxDerivativesProps, DEFAULT_EXOTIC_FX_PROPS } from './data/exotic-fx';

type RequiredExoticFxDerivativesProps = Required<ExoticFxDerivativesProps>;

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
  details?: string;
};

export const ExoticFxDerivativesExample: React.FC<ExoticFxDerivativesProps> = (props) => {
  const defaults = DEFAULT_EXOTIC_FX_PROPS as RequiredExoticFxDerivativesProps;
  const {
    valuationDate = defaults.valuationDate,
    market = defaults.market,
    fxBarrierOptions = defaults.fxBarrierOptions,
    quantoOptions = defaults.quantoOptions,
  } = props;

  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(valuationDate.year, valuationDate.month, valuationDate.day);

        // Build discount curves
        const discountCurves: DiscountCurve[] = [];
        for (const curveData of market.discountCurves) {
          const baseDate = new FsDate(
            curveData.baseDate.year,
            curveData.baseDate.month,
            curveData.baseDate.day
          );
          const curve = new DiscountCurve(
            curveData.id,
            baseDate,
            new Float64Array(curveData.tenors),
            new Float64Array(curveData.discountFactors),
            curveData.dayCount,
            curveData.interpolation,
            curveData.extrapolation,
            curveData.continuous
          );
          discountCurves.push(curve);
        }

        // Build FX matrix
        const fx = new FxMatrix();
        for (const quote of market.fxQuotes) {
          const baseCurrency = new Currency(quote.baseCurrency);
          const quoteCurrency = new Currency(quote.quoteCurrency);
          fx.setQuote(baseCurrency, quoteCurrency, quote.rate);
        }

        // Build vol surfaces
        const fxVol = new VolSurface(
          market.fxVolSurface.id,
          new Float64Array(market.fxVolSurface.expiries),
          new Float64Array(market.fxVolSurface.strikes),
          new Float64Array(market.fxVolSurface.vols)
        );

        const equityVol = new VolSurface(
          market.equityVolSurface.id,
          new Float64Array(market.equityVolSurface.expiries),
          new Float64Array(market.equityVolSurface.strikes),
          new Float64Array(market.equityVolSurface.vols)
        );

        // Build market context
        const marketCtx = new MarketContext();
        for (const curve of discountCurves) {
          marketCtx.insertDiscount(curve);
        }
        marketCtx.insertFx(fx);
        marketCtx.insertSurface(fxVol);
        marketCtx.insertSurface(equityVol);

        // Add spot prices
        for (const spot of market.spotPrices) {
          marketCtx.insertPrice(
            spot.id,
            MarketScalar.price(Money.fromCode(spot.price.amount, spot.price.currency))
          );
        }

        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        // Process FX barrier options
        for (const opt of fxBarrierOptions) {
          try {
            const fxBarrierJson = JSON.stringify({
              id: opt.id,
              domestic_currency: opt.domestic_currency,
              foreign_currency: opt.foreign_currency,
              strike: { amount: opt.strike.amount, currency: opt.strike.currency },
              barrier: { amount: opt.barrier.amount, currency: opt.barrier.currency },
              option_type: opt.option_type,
              barrier_type: opt.barrier_type,
              expiry: opt.expiry,
              notional: { amount: opt.notional.amount, currency: opt.notional.currency },
              day_count: opt.day_count,
              use_gobet_miri: opt.use_gobet_miri,
              domestic_discount_curve_id: opt.domestic_discount_curve_id,
              foreign_discount_curve_id: opt.foreign_discount_curve_id,
              fx_spot_id: opt.fx_spot_id,
              fx_vol_id: opt.fx_vol_id,
              pricing_overrides: { adaptive_bumps: false },
              attributes: { tags: [], meta: {} },
            });
            const fxBarrierOption = FxBarrierOption.fromJson(fxBarrierJson);
            const result = registry.priceFxBarrierOption(
              fxBarrierOption,
              'monte_carlo_gbm',
              marketCtx,
              asOf,
              null
            );

            const barrierTypeName = opt.barrier_type
              .replace(/([A-Z])/g, '-$1')
              .slice(1)
              .toLowerCase();
            results.push({
              name: `FX Barrier ${barrierTypeName.replace(/-/g, ' ').replace('and', '&')} ${opt.option_type.charAt(0).toUpperCase() + opt.option_type.slice(1)}`,
              type: 'FxBarrierOption',
              pair: `${opt.foreign_currency}/${opt.domestic_currency}`,
              presentValue: result.presentValue.amount,
              details: `Strike: ${opt.strike.amount.toFixed(2)}, Barrier: ${opt.barrier.amount.toFixed(2)}`,
            });
          } catch (err) {
            console.error(`FX barrier option ${opt.id} error:`, err);
          }
        }

        // Process quanto options
        for (const opt of quantoOptions) {
          try {
            const quantoJson = JSON.stringify({
              id: opt.id,
              underlying_ticker: opt.underlying_ticker,
              equity_strike: {
                amount: opt.equity_strike.amount,
                currency: opt.equity_strike.currency,
              },
              expiry: opt.expiry,
              option_type: opt.option_type,
              notional: { amount: opt.notional.amount, currency: opt.notional.currency },
              domestic_currency: opt.domestic_currency,
              foreign_currency: opt.foreign_currency,
              correlation: opt.correlation,
              day_count: opt.day_count,
              discount_curve_id: opt.discount_curve_id,
              foreign_discount_curve_id: opt.foreign_discount_curve_id,
              spot_id: opt.spot_id,
              vol_surface_id: opt.vol_surface_id,
              div_yield_id: null,
              fx_rate_id: null,
              fx_vol_id: opt.fx_vol_id,
              pricing_overrides: { adaptive_bumps: false },
              attributes: { tags: [], meta: {} },
            });
            const quantoOption = QuantoOption.fromJson(quantoJson);
            const result = registry.priceQuantoOption(
              quantoOption,
              'monte_carlo_gbm',
              marketCtx,
              asOf,
              null
            );

            results.push({
              name: `Quanto ${opt.option_type.charAt(0).toUpperCase() + opt.option_type.slice(1)} (${opt.foreign_currency} equity, ${opt.domestic_currency} payment)`,
              type: 'QuantoOption',
              pair: `${opt.foreign_currency}/${opt.domestic_currency}`,
              presentValue: result.presentValue.amount,
              details: `Strike: ${opt.equity_strike.amount.toFixed(1)}, Cross-currency`,
            });
          } catch (err) {
            console.error(`Quanto option ${opt.id} error:`, err);
          }
        }

        if (!cancelled) {
          setRows(results);
        }

        // Cleanup
        for (const curve of discountCurves) {
          curve.free();
        }
        fx.free();
        fxVol.free();
        equityVol.free();
        marketCtx.free();
      } catch (err) {
        if (!cancelled) {
          console.error('Exotic FX derivatives error:', err);
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [valuationDate, market, fxBarrierOptions, quantoOptions]);

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (rows.length === 0) {
    return <p>Building exotic FX derivatives…</p>;
  }

  return (
    <section className="example-section">
      <h2>Exotic FX Derivatives</h2>
      <p>
        Exotic FX derivatives including FX barrier options (up-and-out, down-and-in) and quanto
        options (cross-currency equity options). These instruments are priced using Monte Carlo
        simulation with GBM processes.
      </p>

      <table>
        <thead>
          <tr>
            <th>Instrument</th>
            <th>Type</th>
            <th>Pair</th>
            <th>Present Value</th>
            <th>Details</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ name, type, pair, presentValue, details }) => (
            <tr key={name}>
              <td>{name}</td>
              <td>{type}</td>
              <td>{pair}</td>
              <td>{currencyFormatter.format(presentValue)}</td>
              <td>{details ?? '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};
