/**
 * CDS Index instrument component with interactive form.
 */
import React, { useEffect, useState, useCallback } from 'react';
import {
  CDSIndexBuilder,
  FsDate,
  MarketContext,
  Money,
  PricingRequest,
  standardRegistry,
} from 'finstack-wasm';
import type { CdsIndexInstrumentData } from '../../data/credit';
import { currencyFormatter, type InstrumentRow } from './useCreditMarket';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Calculator, X, TrendingDown, TrendingUp, BarChart3 } from 'lucide-react';

export interface CDSIndexInstrumentProps {
  cdsIndices: CdsIndexInstrumentData[];
  market: MarketContext;
  asOf: FsDate;
}

interface CDSIndexFormState {
  notional: number;
  spreadBps: number;
  series: number;
  version: number;
  recoveryRate: number;
  direction: 'pay_protection' | 'receive_protection';
  currency: string;
  indexFamily: string;
}

export const CDSIndexInstrument: React.FC<CDSIndexInstrumentProps> = ({
  cdsIndices,
  market,
  asOf,
}) => {
  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);

  // Initialize form state from first index
  const initialIndex = cdsIndices[0];
  const [formState, setFormState] = useState<CDSIndexFormState>({
    notional: initialIndex?.notional.amount ?? 25_000_000,
    spreadBps: initialIndex?.spreadBps ?? 100,
    series: initialIndex?.series ?? 42,
    version: initialIndex?.version ?? 1,
    recoveryRate: initialIndex?.recoveryRate ?? 0.4,
    direction: initialIndex?.direction ?? 'pay_protection',
    currency: initialIndex?.notional.currency ?? 'USD',
    indexFamily: initialIndex?.indexFamily ?? 'CDX.NA.IG',
  });

  const calculateIndex = useCallback(() => {
    try {
      const registry = standardRegistry();
      const notional = Money.fromCode(formState.notional, formState.currency);

      const effectiveDate = asOf;
      const maturityDate = new FsDate(asOf.year + 5, asOf.month, asOf.day);

      const index = new CDSIndexBuilder('interactive_index')
        .indexName(formState.indexFamily)
        .series(formState.series)
        .version(formState.version)
        .money(notional)
        .fixedCouponBp(formState.spreadBps)
        .startDate(effectiveDate)
        .maturity(maturityDate)
        .discountCurve(initialIndex?.discountCurveId ?? 'USD-OIS')
        .creditCurve(initialIndex?.hazardCurveId ?? 'CDX-IG-HZD')
        .side(formState.direction)
        .recoveryRate(formState.recoveryRate)
        .build();

      const indexOpts = new PricingRequest().withMetrics(['par_spread']);
      const indexResult = registry.priceInstrument(index, 'discounting', market, asOf, indexOpts);

      const result: InstrumentRow = {
        name: `${formState.indexFamily} S${formState.series} V${formState.version}`,
        type: 'CDSIndex',
        presentValue: indexResult.presentValue.amount,
        keyMetric: {
          name: 'Par Spread',
          value: (() => {
            const raw = indexResult.metric('par_spread') ?? 0;
            return Math.abs(raw) > 10 ? raw : raw * 10000;
          })(),
        },
      };

      setRows([result]);
      setError(null);
    } catch (err) {
      setError(`CDS Index pricing error: ${err}`);
    }
  }, [formState, market, asOf, initialIndex]);

  // Calculate when form is visible and form state changes
  useEffect(() => {
    if (showForm) {
      const timer = setTimeout(() => {
        calculateIndex();
      }, 0);
      return () => clearTimeout(timer);
    }
  }, [showForm, formState, calculateIndex]);

  useEffect(() => {
    if (showForm) return;

    let cancelled = false;

    (async () => {
      try {
        const registry = standardRegistry();
        const results: InstrumentRow[] = [];

        for (const indexInstrData of cdsIndices) {
          const notional = Money.fromCode(
            indexInstrData.notional.amount,
            indexInstrData.notional.currency
          );
          const effectiveDate = new FsDate(
            indexInstrData.effectiveDate.year,
            indexInstrData.effectiveDate.month,
            indexInstrData.effectiveDate.day
          );
          const maturityDate = new FsDate(
            indexInstrData.maturityDate.year,
            indexInstrData.maturityDate.month,
            indexInstrData.maturityDate.day
          );

          const index = new CDSIndexBuilder(indexInstrData.id)
            .indexName(indexInstrData.indexFamily)
            .series(indexInstrData.series)
            .version(indexInstrData.version)
            .money(notional)
            .fixedCouponBp(indexInstrData.spreadBps)
            .startDate(effectiveDate)
            .maturity(maturityDate)
            .discountCurve(indexInstrData.discountCurveId)
            .creditCurve(indexInstrData.hazardCurveId)
            .side(indexInstrData.direction)
            .recoveryRate(indexInstrData.recoveryRate)
            .build();

          const indexOpts = new PricingRequest().withMetrics(['par_spread']);
          try {
            const indexResult = registry.priceInstrument(
              index,
              'discounting',
              market,
              asOf,
              indexOpts
            );
            results.push({
              name: `${indexInstrData.indexFamily} S${indexInstrData.series} V${indexInstrData.version}`,
              type: 'CDSIndex',
              presentValue: indexResult.presentValue.amount,
              keyMetric: {
                name: 'Par Spread',
                value: (() => {
                  const raw = indexResult.metric('par_spread') ?? 0;
                  return Math.abs(raw) > 10 ? raw : raw * 10000;
                })(),
              },
            });
          } catch (err) {
            console.warn('CDS index pricing failed, skipping', err);
          }
        }

        if (!cancelled) {
          if (results.length === 0) {
            setError('No CDS indices priced');
          } else {
            setRows(results);
            setError(null);
          }
        }
      } catch (err) {
        if (!cancelled) {
          setError(`CDS Index pricing error: ${err}`);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [cdsIndices, market, asOf, showForm]);

  const handleInputChange = (field: keyof CDSIndexFormState, value: string | number) => {
    setFormState((prev) => ({ ...prev, [field]: value }));
  };

  if (error && !showForm) {
    return (
      <Alert variant="destructive">
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    );
  }

  if (rows.length === 0 && !showForm) {
    return <p className="text-muted-foreground animate-pulse">Loading CDS indices...</p>;
  }

  return (
    <Card className="border-border/50">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <CardTitle className="text-lg flex items-center gap-2">
              <BarChart3 className="h-5 w-5 text-primary" />
              CDS Indices
              <Badge variant="outline" className="font-mono text-xs">
                CDX/iTraxx
              </Badge>
            </CardTitle>
            <CardDescription>
              Standardized credit index contracts representing a basket of credit default swaps
            </CardDescription>
          </div>
          <Button
            variant={showForm ? 'default' : 'outline'}
            size="sm"
            onClick={() => setShowForm(!showForm)}
            className="gap-2"
          >
            {showForm ? (
              <>
                <X className="h-4 w-4" />
                Hide
              </>
            ) : (
              <>
                <Calculator className="h-4 w-4" />
                Calculator
              </>
            )}
          </Button>
        </div>
      </CardHeader>

      <CardContent className="space-y-4">
        {showForm && (
          <div className="rounded-lg border border-border/50 bg-muted/30 p-4 space-y-4">
            <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
              <div className="space-y-2">
                <Label htmlFor="idx-notional">Notional</Label>
                <Input
                  id="idx-notional"
                  type="number"
                  value={formState.notional}
                  onChange={(e) =>
                    handleInputChange('notional', Number.parseFloat(e.target.value) || 0)
                  }
                  step={1000000}
                  className="font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="idx-spread">Spread (bps)</Label>
                <Input
                  id="idx-spread"
                  type="number"
                  value={formState.spreadBps}
                  onChange={(e) =>
                    handleInputChange('spreadBps', Number.parseFloat(e.target.value) || 0)
                  }
                  step={5}
                  className="font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="idx-series">Series</Label>
                <Input
                  id="idx-series"
                  type="number"
                  value={formState.series}
                  onChange={(e) =>
                    handleInputChange('series', Number.parseInt(e.target.value, 10) || 1)
                  }
                  min={1}
                  className="font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="idx-version">Version</Label>
                <Input
                  id="idx-version"
                  type="number"
                  value={formState.version}
                  onChange={(e) =>
                    handleInputChange('version', Number.parseInt(e.target.value, 10) || 1)
                  }
                  min={1}
                  className="font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="idx-recovery">Recovery Rate</Label>
                <Input
                  id="idx-recovery"
                  type="number"
                  value={formState.recoveryRate}
                  onChange={(e) =>
                    handleInputChange('recoveryRate', Number.parseFloat(e.target.value) || 0)
                  }
                  step={0.05}
                  min={0}
                  max={1}
                  className="font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="idx-direction">Direction</Label>
                <Select
                  value={formState.direction}
                  onValueChange={(value) => handleInputChange('direction', value)}
                >
                  <SelectTrigger id="idx-direction">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="pay_protection">
                      <span className="flex items-center gap-2">
                        <TrendingDown className="h-3 w-3 text-destructive" />
                        Pay Protection
                      </span>
                    </SelectItem>
                    <SelectItem value="receive_protection">
                      <span className="flex items-center gap-2">
                        <TrendingUp className="h-3 w-3 text-success" />
                        Receive Protection
                      </span>
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
          </div>
        )}

        {error && showForm && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Instrument</TableHead>
              <TableHead>Type</TableHead>
              <TableHead className="text-right">Present Value</TableHead>
              <TableHead className="text-right">Key Metric</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map(({ name, type, presentValue, keyMetric }) => (
              <TableRow key={name}>
                <TableCell className="font-medium">{name}</TableCell>
                <TableCell>
                  <Badge variant="secondary" className="font-mono text-xs">
                    {type}
                  </Badge>
                </TableCell>
                <TableCell className="text-right font-mono">
                  {currencyFormatter.format(presentValue)}
                </TableCell>
                <TableCell className="text-right font-mono">
                  {keyMetric ? (
                    <span className="text-muted-foreground">
                      {keyMetric.name}:{' '}
                      <span className="text-foreground font-semibold">
                        {keyMetric.value.toFixed(2)} bps
                      </span>
                    </span>
                  ) : (
                    '—'
                  )}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
};
