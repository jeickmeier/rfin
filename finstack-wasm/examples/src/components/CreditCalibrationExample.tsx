/**
 * Credit Calibration Example - Demonstrates end-to-end credit workflow.
 *
 * This component shows how to:
 * 1. Calibrate market data using the CreditCalibrationSuite
 * 2. Use the calibrated market to price credit instruments
 * 3. Interactively update calibration and see pricing changes
 */
import React, { useState, useCallback } from 'react';
import {
  CreditDefaultSwapBuilder,
  FsDate,
  MarketContext,
  Money,
  PricingRequest,
  standardRegistry,
} from 'finstack-wasm';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ArrowRight, CheckCircle2, TrendingUp, Shield, Percent } from 'lucide-react';
import { CreditCalibrationSuite, type CreditMarketInfo } from './calibration';

interface PricingResult {
  name: string;
  notional: number;
  presentValue: number;
  parSpread: number;
  pv01: number;
}

export const CreditCalibrationExample: React.FC = () => {
  const [market, setMarket] = useState<MarketContext | null>(null);
  const [asOf, setAsOf] = useState<FsDate | null>(null);
  const [pricingResults, setPricingResults] = useState<PricingResult[]>([]);
  const [activeTab, setActiveTab] = useState<'calibration' | 'pricing'>('calibration');

  // Handle market ready callback (fires after hazard curve calibration)
  const handleMarketReady = useCallback((info: CreditMarketInfo) => {
    const { market: newMarket, asOf: newAsOf, discountCurveId, hazardCurveId } = info;

    setMarket(newMarket);
    setAsOf(newAsOf);

    if (!hazardCurveId) {
      console.warn('Hazard curve not available for CDS pricing');
      return;
    }

    // Price sample CDS instruments using the calibrated market
    try {
      const registry = standardRegistry();
      const results: PricingResult[] = [];

      // Price 3Y, 5Y, 7Y CDS
      const tenors = [3, 5, 7];
      for (const tenor of tenors) {
        const notional = Money.fromCode(10_000_000, 'USD');
        const maturity = new FsDate(newAsOf.year + tenor, newAsOf.month, newAsOf.day);

        const cds = new CreditDefaultSwapBuilder(`CDS_${tenor}Y`)
          .money(notional)
          .spreadBp(100)
          .startDate(newAsOf)
          .maturity(maturity)
          .discountCurve(discountCurveId)
          .creditCurve(hazardCurveId)
          .side('buy_protection')
          .recoveryRate(0.4)
          .build();

        try {
          const request = new PricingRequest().withMetrics(['par_spread', 'pv01']);
          const result = registry.priceInstrument(cds, 'discounting', newMarket, newAsOf, request);

          results.push({
            name: `${tenor}Y CDS`,
            notional: 10_000_000,
            presentValue: result.presentValue.amount,
            parSpread:
              Math.abs(result.metric('par_spread') ?? 0) > 10
                ? (result.metric('par_spread') ?? 0)
                : (result.metric('par_spread') ?? 0) * 10000,
            pv01: result.metric('pv01') ?? 0,
          });
        } catch (err) {
          console.warn(`Failed to price ${tenor}Y CDS:`, err);
        }
      }

      setPricingResults(results);
    } catch (err) {
      console.warn('Failed to price CDS instruments:', err);
    }
  }, []);

  // Format currency
  const formatCurrency = (value: number) =>
    new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      maximumFractionDigits: 0,
    }).format(value);

  const formatBps = (value: number) => `${value.toFixed(2)} bps`;

  return (
    <section className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-primary mb-2">Credit Derivatives Workflow</h2>
        <p className="text-muted-foreground">
          Complete end-to-end workflow for credit derivatives: calibrate market data, then price
          instruments. Changes to calibration automatically update pricing.
        </p>
      </div>

      {/* Workflow Status */}
      <div className="flex items-center gap-4 p-4 bg-muted/30 rounded-lg">
        <div className="flex items-center gap-2">
          <div
            className={`w-8 h-8 rounded-full flex items-center justify-center ${
              market ? 'bg-green-500/20 text-green-500' : 'bg-muted text-muted-foreground'
            }`}
          >
            <CheckCircle2 className="h-4 w-4" />
          </div>
          <span className="text-sm font-medium">Market Calibrated</span>
        </div>
        <ArrowRight className="h-4 w-4 text-muted-foreground" />
        <div className="flex items-center gap-2">
          <div
            className={`w-8 h-8 rounded-full flex items-center justify-center ${
              pricingResults.length > 0
                ? 'bg-green-500/20 text-green-500'
                : 'bg-muted text-muted-foreground'
            }`}
          >
            <TrendingUp className="h-4 w-4" />
          </div>
          <span className="text-sm font-medium">Instruments Priced</span>
        </div>
        {market && (
          <Badge variant="outline" className="ml-auto">
            As of: {asOf?.toString() ?? 'N/A'}
          </Badge>
        )}
      </div>

      <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as typeof activeTab)}>
        <TabsList className="grid w-full grid-cols-2">
          <TabsTrigger value="calibration" className="gap-2">
            <Shield className="h-4 w-4" />
            Market Calibration
          </TabsTrigger>
          <TabsTrigger value="pricing" className="gap-2" disabled={!market}>
            <TrendingUp className="h-4 w-4" />
            Instrument Pricing
            {pricingResults.length > 0 && (
              <Badge variant="secondary" className="ml-1 text-xs">
                {pricingResults.length}
              </Badge>
            )}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="calibration" className="mt-4">
          <CreditCalibrationSuite onMarketReady={handleMarketReady} />
        </TabsContent>

        <TabsContent value="pricing" className="mt-4 space-y-4">
          {!market ? (
            <Card>
              <CardContent className="py-12 text-center text-muted-foreground">
                <Shield className="h-12 w-12 mx-auto mb-4 opacity-50" />
                <p>Calibrate market data first to enable pricing.</p>
              </CardContent>
            </Card>
          ) : (
            <>
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <TrendingUp className="h-5 w-5" />
                    Credit Default Swaps
                  </CardTitle>
                  <CardDescription>
                    Single-name CDS priced using the calibrated discount and hazard curves.
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  {pricingResults.length === 0 ? (
                    <div className="text-center text-muted-foreground py-8">
                      No pricing results available. Complete hazard curve calibration to price CDS.
                    </div>
                  ) : (
                    <div className="border rounded-lg overflow-hidden">
                      <table className="w-full text-sm">
                        <thead className="bg-muted/50">
                          <tr>
                            <th className="p-3 text-left font-medium">Instrument</th>
                            <th className="p-3 text-right font-medium">Notional</th>
                            <th className="p-3 text-right font-medium">Present Value</th>
                            <th className="p-3 text-right font-medium">Par Spread</th>
                            <th className="p-3 text-right font-medium">PV01</th>
                          </tr>
                        </thead>
                        <tbody>
                          {pricingResults.map((result) => (
                            <tr key={result.name} className="border-t border-border/50">
                              <td className="p-3 font-medium">{result.name}</td>
                              <td className="p-3 text-right font-mono text-muted-foreground">
                                {formatCurrency(result.notional)}
                              </td>
                              <td
                                className={`p-3 text-right font-mono ${
                                  result.presentValue >= 0 ? 'text-green-600' : 'text-red-600'
                                }`}
                              >
                                {formatCurrency(result.presentValue)}
                              </td>
                              <td className="p-3 text-right font-mono">
                                {formatBps(result.parSpread)}
                              </td>
                              <td className="p-3 text-right font-mono text-muted-foreground">
                                {formatCurrency(result.pv01)}
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  )}
                </CardContent>
              </Card>

              {/* Summary Stats */}
              {pricingResults.length > 0 && (
                <div className="grid grid-cols-3 gap-4">
                  <Card>
                    <CardContent className="pt-6">
                      <div className="text-center">
                        <div className="text-3xl font-bold text-primary">
                          {formatCurrency(
                            pricingResults.reduce((sum, r) => sum + r.presentValue, 0)
                          )}
                        </div>
                        <div className="text-sm text-muted-foreground mt-1">Total Portfolio PV</div>
                      </div>
                    </CardContent>
                  </Card>
                  <Card>
                    <CardContent className="pt-6">
                      <div className="text-center">
                        <div className="text-3xl font-bold">
                          {formatBps(
                            pricingResults.reduce((sum, r) => sum + r.parSpread, 0) /
                              pricingResults.length
                          )}
                        </div>
                        <div className="text-sm text-muted-foreground mt-1">Avg Par Spread</div>
                      </div>
                    </CardContent>
                  </Card>
                  <Card>
                    <CardContent className="pt-6">
                      <div className="text-center">
                        <div className="text-3xl font-bold">
                          {formatCurrency(pricingResults.reduce((sum, r) => sum + r.pv01, 0))}
                        </div>
                        <div className="text-sm text-muted-foreground mt-1">Total PV01</div>
                      </div>
                    </CardContent>
                  </Card>
                </div>
              )}

              {/* Information */}
              <Card className="bg-primary/5 border-primary/20">
                <CardHeader className="pb-3">
                  <CardTitle className="text-lg flex items-center gap-2">
                    <Percent className="h-5 w-5" />
                    Pricing Notes
                  </CardTitle>
                </CardHeader>
                <CardContent className="text-sm text-muted-foreground space-y-2">
                  <p>
                    <strong>Present Value:</strong> MTM of the CDS protection leg minus premium leg.
                    Positive = protection worth more than premiums paid.
                  </p>
                  <p>
                    <strong>Par Spread:</strong> The fair CDS spread that makes PV = 0 at trade
                    inception.
                  </p>
                  <p>
                    <strong>PV01:</strong> Change in PV for a 1bp parallel shift in the CDS spread
                    curve.
                  </p>
                </CardContent>
              </Card>
            </>
          )}
        </TabsContent>
      </Tabs>
    </section>
  );
};

export default CreditCalibrationExample;
