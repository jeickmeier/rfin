import React, { useEffect, useState } from 'react';
import {
  FsDate,
  DiscountCurve,
  ForwardCurve,
  CmsOption,
  RangeAccrual,
  MarketContext,
  MarketScalar,
  Money,
  VolSurface,
  createStandardRegistry,
} from 'finstack-wasm';
import { ExoticRatesDerivativesProps, DEFAULT_EXOTIC_RATES_PROPS } from './data/exotic-rates';

type RequiredExoticRatesDerivativesProps = Required<ExoticRatesDerivativesProps>;

const currencyFormatter = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 2,
});

type InstrumentRow = {
  name: string;
  type: string;
  notional: number;
  presentValue: number;
  details?: string;
};

export const ExoticRatesDerivativesExample: React.FC<ExoticRatesDerivativesProps> = (props) => {
  const defaults = DEFAULT_EXOTIC_RATES_PROPS as RequiredExoticRatesDerivativesProps;
  const {
    valuationDate = defaults.valuationDate,
    notional = defaults.notional,
    market = defaults.market,
    cmsOptions = defaults.cmsOptions,
    rangeAccruals = defaults.rangeAccruals,
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

        // Build forward curve
        const fwdBaseDate = new FsDate(
          market.forwardCurve.baseDate.year,
          market.forwardCurve.baseDate.month,
          market.forwardCurve.baseDate.day
        );
        const forwardCurve = new ForwardCurve(
          market.forwardCurve.id,
          fwdBaseDate,
          market.forwardCurve.tenor,
          new Float64Array(market.forwardCurve.tenors),
          new Float64Array(market.forwardCurve.rates),
          market.forwardCurve.dayCount,
          market.forwardCurve.compounding,
          market.forwardCurve.interpolation
        );

        // Build vol surface
        const swaptionVol = new VolSurface(
          market.volSurface.id,
          new Float64Array(market.volSurface.expiries),
          new Float64Array(market.volSurface.strikes),
          new Float64Array(market.volSurface.vols)
        );

        // Build market context
        const marketCtx = new MarketContext();
        marketCtx.insertDiscount(discountCurve);
        marketCtx.insertForward(forwardCurve);
        marketCtx.insertSurface(swaptionVol);

        // Add spot prices
        for (const spot of market.spotPrices) {
          marketCtx.insertPrice(
            spot.id,
            MarketScalar.get_price(Money.fromCode(spot.price.amount, spot.price.currency))
          );
        }

        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        // Process CMS options
        for (const opt of cmsOptions) {
          try {
            const cmsJson = JSON.stringify({
              id: opt.id,
              cms_tenor: opt.cms_tenor,
              strike: opt.strike,
              fixing_dates: opt.fixing_dates,
              payment_dates: opt.payment_dates,
              accrual_fractions: opt.accrual_fractions,
              option_type: opt.option_type,
              notional: { amount: opt.notional.amount, currency: opt.notional.currency },
              day_count: opt.day_count,
              swap_fixed_freq: opt.swap_fixed_freq,
              swap_float_freq: opt.swap_float_freq,
              swap_day_count: opt.swap_day_count,
              discount_curve_id: opt.discount_curve_id,
              forward_curve_id: opt.forward_curve_id,
              vol_surface_id: opt.vol_surface_id,
              pricing_overrides: { adaptive_bumps: false },
              attributes: { tags: [], meta: {} },
            });
            const cmsOption = CmsOption.fromJson(cmsJson);
            const result = registry.priceInstrument(
              cmsOption,
              'monte_carlo_hull_white_1f',
              marketCtx,
              asOf,
              null
            );

            results.push({
              name: `CMS ${opt.option_type.charAt(0).toUpperCase() + opt.option_type.slice(1)} (${opt.cms_tenor}Y Swap Rate)`,
              type: 'CmsOption',
              notional: notional.amount,
              presentValue: result.presentValue.amount,
              details: `Strike: ${(opt.strike * 100).toFixed(1)}%, 1Y expiry`,
            });
          } catch (err) {
            console.error(`CMS option ${opt.id} error:`, err);
          }
        }

        // Process range accruals
        for (const opt of rangeAccruals) {
          try {
            const rangeAccrualJson = JSON.stringify({
              id: opt.id,
              underlying_ticker: opt.underlying_ticker,
              observation_dates: opt.observation_dates,
              lower_bound: opt.lower_bound,
              upper_bound: opt.upper_bound,
              coupon_rate: opt.coupon_rate,
              notional: { amount: opt.notional.amount, currency: opt.notional.currency },
              day_count: opt.day_count,
              discount_curve_id: opt.discount_curve_id,
              spot_id: opt.spot_id,
              vol_surface_id: opt.vol_surface_id,
              div_yield_id: null,
              pricing_overrides: { adaptive_bumps: false },
              attributes: { tags: [], meta: {} },
            });
            const rangeAccrual = RangeAccrual.fromJson(rangeAccrualJson);
            const result = registry.priceInstrument(
              rangeAccrual,
              'monte_carlo_gbm',
              marketCtx,
              asOf,
              null
            );

            results.push({
              name: 'Range Accrual Note',
              type: 'RangeAccrual',
              notional: notional.amount,
              presentValue: result.presentValue.amount,
              details: `${(opt.coupon_rate * 100).toFixed(0)}% coupon, range: ${(opt.lower_bound * 100).toFixed(0)}%-${(opt.upper_bound * 100).toFixed(0)}%`,
            });
          } catch (err) {
            console.error(`Range accrual ${opt.id} error:`, err);
          }
        }

        if (!cancelled) {
          setRows(results);
        }

        // Cleanup
        discountCurve.free();
        forwardCurve.free();
        swaptionVol.free();
        marketCtx.free();
      } catch (err) {
        if (!cancelled) {
          console.error('Exotic rates derivatives error:', err);
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [valuationDate, notional, market, cmsOptions, rangeAccruals]);

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (rows.length === 0) {
    return <p>Building exotic rates derivatives…</p>;
  }

  return (
    <section className="example-section">
      <h2>Exotic Rates Derivatives</h2>
      <p>
        Exotic interest rate derivatives including CMS options (options on swap rates) and range
        accrual notes (accrual based on reference rate staying within a range). These instruments
        are priced using Monte Carlo simulation with Hull-White and GBM processes.
      </p>

      <table>
        <thead>
          <tr>
            <th>Instrument</th>
            <th>Type</th>
            <th>Notional</th>
            <th>Present Value</th>
            <th>Details</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ name, type, notional: rowNotional, presentValue, details }) => (
            <tr key={name}>
              <td>{name}</td>
              <td>{type}</td>
              <td>{currencyFormatter.format(rowNotional)}</td>
              <td>{currencyFormatter.format(presentValue)}</td>
              <td>{details ?? '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};
