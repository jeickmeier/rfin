import React, { useEffect, useState } from 'react';
import {
  FsDate,
  DayCount,
  DiscountCurve,
  ForwardCurve,
  ForwardRateAgreement,
  InterestRateFuture,
  InterestRateOption,
  InterestRateSwap,
  MarketContext,
  Money,
  PricingRequest,
  Swaption,
  VolSurface,
  createStandardRegistry,
} from 'finstack-wasm';
import { RatesInstrumentsProps, DEFAULT_RATES_PROPS } from './data/rates';

type RequiredRatesInstrumentsProps = Required<RatesInstrumentsProps>;

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
  keyMetric?: { name: string; value: number };
};

export const RatesInstrumentsExample: React.FC<RatesInstrumentsProps> = (props) => {
  // Merge with defaults - DEFAULT_RATES_PROPS always has these values defined
  const defaults = DEFAULT_RATES_PROPS as RequiredRatesInstrumentsProps;
  const {
    valuationDate = defaults.valuationDate,
    discountCurve = defaults.discountCurve,
    forwardCurve = defaults.forwardCurve,
    swaptionVolSurface = defaults.swaptionVolSurface,
    capVolSurface = defaults.capVolSurface,
    swaps = defaults.swaps,
    fras = defaults.fras,
    swaptions = defaults.swaptions,
    capsFloors = defaults.capsFloors,
    futures = defaults.futures,
  } = props;

  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(valuationDate.year, valuationDate.month, valuationDate.day);

        // Build discount curve from props
        const curveBaseDate = new FsDate(
          discountCurve.baseDate.year,
          discountCurve.baseDate.month,
          discountCurve.baseDate.day
        );
        const discCurve = new DiscountCurve(
          discountCurve.id,
          curveBaseDate,
          new Float64Array(discountCurve.tenors),
          new Float64Array(discountCurve.discountFactors),
          discountCurve.dayCount,
          discountCurve.interpolation,
          discountCurve.extrapolation,
          discountCurve.continuous
        );

        // Build forward curve from props
        const fwdCurveBaseDate = new FsDate(
          forwardCurve.baseDate.year,
          forwardCurve.baseDate.month,
          forwardCurve.baseDate.day
        );
        const fwdCurve = new ForwardCurve(
          forwardCurve.id,
          fwdCurveBaseDate,
          forwardCurve.tenor,
          new Float64Array(forwardCurve.tenors),
          new Float64Array(forwardCurve.rates),
          forwardCurve.dayCount,
          forwardCurve.compounding,
          forwardCurve.interpolation
        );

        // Build volatility surfaces from props
        const swaptionVol = new VolSurface(
          swaptionVolSurface.id,
          new Float64Array(swaptionVolSurface.expiries),
          new Float64Array(swaptionVolSurface.strikes),
          new Float64Array(swaptionVolSurface.vols)
        );

        const capVol = new VolSurface(
          capVolSurface.id,
          new Float64Array(capVolSurface.expiries),
          new Float64Array(capVolSurface.strikes),
          new Float64Array(capVolSurface.vols)
        );

        const market = new MarketContext();
        market.insertDiscount(discCurve);
        market.insertForward(fwdCurve);
        market.insertSurface(swaptionVol);
        market.insertSurface(capVol);

        console.debug('FsDate checks (rates)', asOf instanceof FsDate);

        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        // Process Interest Rate Swaps
        for (const swapData of swaps) {
          const notional = Money.fromCode(swapData.notional.amount, swapData.notional.currency);
          const startDate = new FsDate(
            swapData.startDate.year,
            swapData.startDate.month,
            swapData.startDate.day
          );
          const endDate = new FsDate(
            swapData.endDate.year,
            swapData.endDate.month,
            swapData.endDate.day
          );

          const swap = new InterestRateSwap(
            swapData.id,
            notional,
            swapData.fixedRate,
            startDate,
            endDate,
            swapData.discountCurveId,
            swapData.forwardCurveId,
            swapData.direction,
            null,
            swapData.fixedDayCount === 'thirty360' ? DayCount.thirty360() : DayCount.act360(),
            null,
            swapData.floatDayCount === 'act360' ? DayCount.act360() : DayCount.thirty360(),
            null,
            null,
            null,
            swapData.fixedFrequency
          );
          const swapOpts = new PricingRequest().withMetrics(['dv01', 'annuity', 'par_rate']);
          try {
            const swapResult = registry.priceInstrument(
              swap,
              'discounting',
              market,
              asOf,
              swapOpts
            );
            const tenorYears = swapData.endDate.year - swapData.startDate.year;
            results.push({
              name: `${tenorYears}Y IRS (${swapData.direction === 'receive_fixed' ? 'Receive' : 'Pay'} Fixed)`,
              type: 'InterestRateSwap',
              notional: notional.amount,
              presentValue: swapResult.presentValue.amount,
              keyMetric: { name: 'DV01', value: swapResult.metric('dv01') ?? 0 },
            });
          } catch (err) {
            console.warn('Swap pricing failed, skipping', err);
          }
        }

        // Process FRAs
        for (const fraData of fras) {
          const notional = Money.fromCode(fraData.notional.amount, fraData.notional.currency);
          const fixingDate = new FsDate(
            fraData.fixingDate.year,
            fraData.fixingDate.month,
            fraData.fixingDate.day
          );
          const settlementDate = new FsDate(
            fraData.settlementDate.year,
            fraData.settlementDate.month,
            fraData.settlementDate.day
          );
          const maturityDate = new FsDate(
            fraData.maturityDate.year,
            fraData.maturityDate.month,
            fraData.maturityDate.day
          );

          const fra = new ForwardRateAgreement(
            fraData.id,
            notional,
            fraData.fixedRate,
            fixingDate,
            settlementDate,
            maturityDate,
            fraData.discountCurveId,
            fraData.forwardCurveId,
            fraData.dayCount === 'act360' ? DayCount.act360() : DayCount.thirty360(),
            fraData.compounding,
            fraData.payAtMaturity
          );
          const fraOpts = new PricingRequest().withMetrics(['par_rate']);
          try {
            const fraResult = registry.priceInstrument(fra, 'discounting', market, asOf, fraOpts);
            results.push({
              name: '3x6 FRA',
              type: 'ForwardRateAgreement',
              notional: notional.amount,
              presentValue: fraResult.presentValue.amount,
              keyMetric: {
                name: 'Par Rate (bps)',
                value: Math.abs((fraResult.metric('par_rate') ?? 0) * 10000),
              },
            });
          } catch (err) {
            console.warn('FRA pricing failed, skipping', err);
          }
        }

        // Process Swaptions
        for (const swaptionData of swaptions) {
          const notional = Money.fromCode(
            swaptionData.notional.amount,
            swaptionData.notional.currency
          );
          const optionExpiry = new FsDate(
            swaptionData.optionExpiry.year,
            swaptionData.optionExpiry.month,
            swaptionData.optionExpiry.day
          );
          const swapStart = new FsDate(
            swaptionData.swapStart.year,
            swaptionData.swapStart.month,
            swaptionData.swapStart.day
          );
          const swapEnd = new FsDate(
            swaptionData.swapEnd.year,
            swaptionData.swapEnd.month,
            swaptionData.swapEnd.day
          );

          const swaption =
            swaptionData.optionType === 'payer'
              ? new Swaption(
                  swaptionData.id,
                  notional,
                  swaptionData.strike,
                  'payer',
                  optionExpiry,
                  swapStart,
                  swapEnd,
                  swaptionData.discountCurveId,
                  swaptionData.forwardCurveId,
                  swaptionData.volSurfaceId,
                  null,
                  null,
                  null,
                  null,
                  null
                )
              : new Swaption(
                  swaptionData.id,
                  notional,
                  swaptionData.strike,
                  'receiver',
                  optionExpiry,
                  swapStart,
                  swapEnd,
                  swaptionData.discountCurveId,
                  swaptionData.forwardCurveId,
                  swaptionData.volSurfaceId,
                  null,
                  null,
                  null,
                  null,
                  null
                );
          try {
            const swaptionResult = registry.priceInstrument(
              swaption,
              'discounting',
              market,
              asOf,
              null
            );
            const expiryYears = swaptionData.optionExpiry.year - valuationDate.year;
            const swapYears = swaptionData.swapEnd.year - swaptionData.swapStart.year;
            results.push({
              name: `${expiryYears}Yx${swapYears}Y ${swaptionData.optionType === 'payer' ? 'Payer' : 'Receiver'} Swaption`,
              type: 'Swaption',
              notional: notional.amount,
              presentValue: swaptionResult.presentValue.amount,
            });
          } catch (err) {
            console.warn('Swaption pricing failed, skipping', err);
          }
        }

        // Process Caps/Floors
        for (const capFloorData of capsFloors) {
          const notional = Money.fromCode(
            capFloorData.notional.amount,
            capFloorData.notional.currency
          );
          const startDate = new FsDate(
            capFloorData.startDate.year,
            capFloorData.startDate.month,
            capFloorData.startDate.day
          );
          const endDate = new FsDate(
            capFloorData.endDate.year,
            capFloorData.endDate.month,
            capFloorData.endDate.day
          );

          const capFloor =
            capFloorData.capOrFloor === 'cap'
              ? InterestRateOption.cap(
                  capFloorData.id,
                  notional,
                  capFloorData.strike,
                  startDate,
                  endDate,
                  capFloorData.discountCurveId,
                  capFloorData.forwardCurveId,
                  capFloorData.volSurfaceId,
                  capFloorData.frequency,
                  capFloorData.dayCount === 'act360' ? DayCount.act360() : DayCount.thirty360()
                )
              : InterestRateOption.floor(
                  capFloorData.id,
                  notional,
                  capFloorData.strike,
                  startDate,
                  endDate,
                  capFloorData.discountCurveId,
                  capFloorData.forwardCurveId,
                  capFloorData.volSurfaceId,
                  capFloorData.frequency,
                  capFloorData.dayCount === 'act360' ? DayCount.act360() : DayCount.thirty360()
                );
          try {
            const capFloorResult = registry.priceInstrument(
              capFloor,
              'discounting',
              market,
              asOf,
              null
            );
            const tenorYears = capFloorData.endDate.year - capFloorData.startDate.year;
            results.push({
              name: `${tenorYears}Y ${capFloorData.capOrFloor === 'cap' ? 'Cap' : 'Floor'} @ ${(capFloorData.strike * 100).toFixed(0)}%`,
              type: 'InterestRateOption',
              notional: notional.amount,
              presentValue: capFloorResult.presentValue.amount,
            });
          } catch (err) {
            console.warn('Cap/Floor pricing failed, skipping', err);
          }
        }

        // Process Futures
        for (const futureData of futures) {
          const notional = Money.fromCode(futureData.notional.amount, futureData.notional.currency);
          const lastTradeDate = new FsDate(
            futureData.lastTradeDate.year,
            futureData.lastTradeDate.month,
            futureData.lastTradeDate.day
          );
          const settlementDate = new FsDate(
            futureData.settlementDate.year,
            futureData.settlementDate.month,
            futureData.settlementDate.day
          );
          const accrualStart = new FsDate(
            futureData.accrualStart.year,
            futureData.accrualStart.month,
            futureData.accrualStart.day
          );
          const accrualEnd = new FsDate(
            futureData.accrualEnd.year,
            futureData.accrualEnd.month,
            futureData.accrualEnd.day
          );

          const future = new InterestRateFuture(
            futureData.id,
            notional,
            futureData.price,
            lastTradeDate,
            settlementDate,
            accrualStart,
            accrualEnd,
            futureData.discountCurveId,
            futureData.forwardCurveId,
            futureData.direction,
            futureData.dayCount === 'act360' ? DayCount.act360() : DayCount.thirty360()
          );
          try {
            const futureResult = registry.priceInstrument(
              future,
              'discounting',
              market,
              asOf,
              null
            );
            results.push({
              name: 'SOFR Future (Mar 24)',
              type: 'InterestRateFuture',
              notional: notional.amount,
              presentValue: futureResult.presentValue.amount,
            });
          } catch (err) {
            console.warn('Future pricing failed, skipping', err);
          }
        }

        if (!cancelled) {
          setRows(results);
        }
      } catch (err) {
        if (!cancelled) {
          console.error('Rates instruments error:', err);
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [
    valuationDate,
    discountCurve,
    forwardCurve,
    swaptionVolSurface,
    capVolSurface,
    swaps,
    fras,
    swaptions,
    capsFloors,
    futures,
  ]);

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (rows.length === 0) {
    return <p>Building rates instruments…</p>;
  }

  return (
    <section className="example-section">
      <h2>Interest Rate Instruments</h2>
      <p>
        Comprehensive suite of interest rate derivatives including swaps, FRAs, swaptions, basis
        swaps, caps/floors, and futures. All instruments are priced using the standard registry with
        market curves.
      </p>

      <table>
        <thead>
          <tr>
            <th>Instrument</th>
            <th>Type</th>
            <th>Notional</th>
            <th>Present Value</th>
            <th>Key Metric</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ name, type, notional, presentValue, keyMetric }) => (
            <tr key={name}>
              <td>{name}</td>
              <td>{type}</td>
              <td>{currencyFormatter.format(notional)}</td>
              <td>{currencyFormatter.format(presentValue)}</td>
              <td>{keyMetric ? `${keyMetric.name}: ${keyMetric.value.toFixed(2)}` : '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};
