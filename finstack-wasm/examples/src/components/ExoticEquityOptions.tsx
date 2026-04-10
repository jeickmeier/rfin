import React, { useEffect, useState } from 'react';
import {
  FsDate,
  DiscountCurve,
  BarrierOptionBuilder,
  AsianOptionBuilder,
  LookbackOptionBuilder,
  CliquetOptionBuilder,
  MarketContext,
  MarketScalar,
  Money,
  PricingRequest,
  VolSurface,
  standardRegistry,
} from 'finstack-wasm';
import { ExoticEquityOptionsProps, DEFAULT_EXOTIC_EQUITY_PROPS } from './data/exotic-equity';

type RequiredExoticEquityOptionsProps = Required<ExoticEquityOptionsProps>;

const currencyFormatter = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 2,
});

type InstrumentRow = {
  name: string;
  type: string;
  presentValue: number;
  keyMetric?: { name: string; value: number };
  details?: string;
};

export const ExoticEquityOptionsExample: React.FC<ExoticEquityOptionsProps> = (props) => {
  const defaults = DEFAULT_EXOTIC_EQUITY_PROPS as RequiredExoticEquityOptionsProps;
  const {
    valuationDate = defaults.valuationDate,
    market = defaults.market,
    barrierOptions = defaults.barrierOptions,
    asianOptions = defaults.asianOptions,
    lookbackOptions = defaults.lookbackOptions,
    cliquetOptions = defaults.cliquetOptions,
  } = props;

  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(valuationDate.year, valuationDate.month, valuationDate.day);

        // Build discount curve
        const discountCurve = new DiscountCurve(
          market.discountCurve.id,
          asOf,
          new Float64Array(market.discountCurve.tenors),
          new Float64Array(market.discountCurve.discountFactors),
          market.discountCurve.dayCount,
          market.discountCurve.interpolation,
          market.discountCurve.extrapolation,
          market.discountCurve.continuous
        );

        // Build vol surface
        const equityVol = new VolSurface(
          market.volSurface.id,
          new Float64Array(market.volSurface.expiries),
          new Float64Array(market.volSurface.strikes),
          new Float64Array(market.volSurface.vols)
        );

        // Build market context
        const marketCtx = new MarketContext();
        marketCtx.insertDiscount(discountCurve);
        marketCtx.insertSurface(equityVol);

        // Add spot prices
        for (const spot of market.spotPrices) {
          marketCtx.insertPrice(
            spot.id,
            MarketScalar.price(Money.fromCode(spot.price.amount, spot.price.currency))
          );
        }

        // Add dividend yields
        for (const divYield of market.divYields) {
          marketCtx.insertPrice(divYield.id, MarketScalar.unitless(divYield.value));
        }

        const registry = standardRegistry();
        const results: InstrumentRow[] = [];

        // Process barrier options
        for (const opt of barrierOptions) {
          try {
            const expiry = new FsDate(opt.expiry.year, opt.expiry.month, opt.expiry.day);
            const barrierOption = new BarrierOptionBuilder(opt.id)
              .ticker(opt.underlyingTicker)
              .strike(opt.strike)
              .barrier(opt.barrier)
              .optionType(opt.optionType)
              .barrierType(opt.barrierType)
              .expiry(expiry)
              .money(Money.fromCode(opt.notional.amount, opt.notional.currency))
              .discountCurve(opt.discountCurveId)
              .spotId(opt.spotId)
              .volSurface(opt.volId)
              .divYieldId(opt.divYieldId)
              .useGobetMiri(opt.useGobetMiri ?? false)
              .build();
            const pricingOpts = new PricingRequest().withMetrics(['delta', 'gamma']);
            const result = registry.priceInstrument(
              barrierOption,
              'monte_carlo_gbm',
              marketCtx,
              asOf,
              pricingOpts
            );

            const barrierTypeName = opt.barrierType.replace(/_/g, '-').replace(/and/g, '&');
            results.push({
              name: `Barrier ${barrierTypeName} ${opt.optionType.charAt(0).toUpperCase() + opt.optionType.slice(1)}`,
              type: 'BarrierOption',
              presentValue: result.presentValue.amount,
              keyMetric: { name: 'Delta', value: result.metric('delta') ?? 0 },
              details: `Strike: $${opt.strike}, Barrier: $${opt.barrier}`,
            });
          } catch (err) {
            console.error(`Barrier option ${opt.id} error:`, err);
          }
        }

        // Process Asian options
        for (const opt of asianOptions) {
          try {
            const expiry = new FsDate(opt.expiry.year, opt.expiry.month, opt.expiry.day);
            const fixingDates = opt.fixingDates.map(
              (d) =>
                `${d.year}-${String(d.month).padStart(2, '0')}-${String(d.day).padStart(2, '0')}`
            );

            const asianOption = new AsianOptionBuilder(opt.id)
              .ticker(opt.underlyingTicker)
              .strike(opt.strike)
              .expiry(expiry)
              .fixingDates(fixingDates)
              .money(Money.fromCode(opt.notional.amount, opt.notional.currency))
              .discountCurve(opt.discountCurveId)
              .spotId(opt.spotId)
              .volSurface(opt.volId)
              .averagingMethod(opt.averagingMethod)
              .optionType(opt.optionType)
              .divYieldId(opt.divYieldId)
              .build();
            const result = registry.priceInstrument(
              asianOption,
              'monte_carlo_gbm',
              marketCtx,
              asOf,
              null
            );

            const avgMethod =
              opt.averagingMethod.charAt(0).toUpperCase() + opt.averagingMethod.slice(1);
            results.push({
              name: `Asian ${avgMethod} ${opt.optionType.charAt(0).toUpperCase() + opt.optionType.slice(1)}`,
              type: 'AsianOption',
              presentValue: result.presentValue.amount,
              details: `Strike: $${opt.strike}, ${fixingDates.length} monthly fixings`,
            });
          } catch (err) {
            console.error(`Asian option ${opt.id} error:`, err);
          }
        }

        // Process Lookback options
        for (const opt of lookbackOptions) {
          try {
            const lookbackJson = JSON.stringify({
              id: opt.id,
              underlying_ticker: opt.underlying_ticker,
              strike: { amount: opt.strike.amount, currency: opt.strike.currency },
              expiry: opt.expiry,
              lookback_type: opt.lookback_type,
              option_type: opt.option_type,
              notional: { amount: opt.notional.amount, currency: opt.notional.currency },
              day_count: opt.day_count,
              discount_curve_id: opt.discount_curve_id,
              spot_id: opt.spot_id,
              vol_surface_id: opt.vol_surface_id,
              div_yield_id: opt.div_yield_id,
              pricing_overrides: {},
              attributes: { tags: [], meta: {} },
            });
            const lookbackOption = new LookbackOptionBuilder().jsonString(lookbackJson).build();
            const result = registry.priceInstrument(
              lookbackOption,
              'monte_carlo_gbm',
              marketCtx,
              asOf,
              null
            );

            results.push({
              name: `Lookback ${opt.lookback_type.replace(/([A-Z])/g, ' $1').trim()} ${opt.option_type.charAt(0).toUpperCase() + opt.option_type.slice(1)}`,
              type: 'LookbackOption',
              presentValue: result.presentValue.amount,
              details: `Strike: $${opt.strike.amount}`,
            });
          } catch (err) {
            console.error(`Lookback option ${opt.id} error:`, err);
          }
        }

        // Process Cliquet options
        for (const opt of cliquetOptions) {
          try {
            const cliquetJson = JSON.stringify({
              id: opt.id,
              underlying_ticker: opt.underlying_ticker,
              reset_dates: opt.reset_dates,
              local_cap: opt.local_cap,
              local_floor: opt.local_floor,
              global_cap: opt.global_cap,
              global_floor: opt.global_floor,
              notional: { amount: opt.notional.amount, currency: opt.notional.currency },
              day_count: opt.day_count,
              discount_curve_id: opt.discount_curve_id,
              spot_id: opt.spot_id,
              vol_surface_id: opt.vol_surface_id,
              div_yield_id: opt.div_yield_id,
              pricing_overrides: {},
              attributes: { tags: [], meta: {} },
            });
            const cliquetOption = new CliquetOptionBuilder().jsonString(cliquetJson).build();
            const result = registry.priceInstrument(
              cliquetOption,
              'monte_carlo_gbm',
              marketCtx,
              asOf,
              null
            );

            results.push({
              name: 'Cliquet Option',
              type: 'CliquetOption',
              presentValue: result.presentValue.amount,
              details: `Local cap: ${(opt.local_cap * 100).toFixed(0)}%, Global cap: ${(opt.global_cap * 100).toFixed(0)}%`,
            });
          } catch (err) {
            console.error(`Cliquet option ${opt.id} error:`, err);
          }
        }

        if (!cancelled) {
          setRows(results);
        }

        // Cleanup
        discountCurve.free();
        equityVol.free();
        marketCtx.free();
      } catch (err) {
        if (!cancelled) {
          console.error('Exotic equity options error:', err);
          const errorMessage = err instanceof Error ? err.message : String(err);
          setError(errorMessage);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [valuationDate, market, barrierOptions, asianOptions, lookbackOptions, cliquetOptions]);

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (rows.length === 0) {
    return <p>Building exotic equity options…</p>;
  }

  return (
    <section className="example-section">
      <h2>Exotic Equity Options</h2>
      <p>
        Exotic equity options including barrier options (up-and-out, down-and-in), Asian options
        (arithmetic and geometric averaging), lookback options, and cliquet options. These
        instruments are priced using Monte Carlo simulation and other advanced models.
      </p>

      <table>
        <thead>
          <tr>
            <th>Instrument</th>
            <th>Type</th>
            <th>Present Value</th>
            <th>Key Metric</th>
            <th>Details</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ name, type, presentValue, keyMetric, details }) => (
            <tr key={name}>
              <td>{name}</td>
              <td>{type}</td>
              <td>{currencyFormatter.format(presentValue)}</td>
              <td>{keyMetric ? `${keyMetric.name}: ${keyMetric.value.toFixed(4)}` : '—'}</td>
              <td>{details ?? '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};
