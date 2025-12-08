/**
 * Credit Default Swap (CDS) instrument component with interactive form.
 */
import React, { useEffect, useState, useCallback } from 'react';
import {
  CreditDefaultSwap,
  FsDate,
  MarketContext,
  Money,
  PricingRequest,
  createStandardRegistry,
} from 'finstack-wasm';
import type { CdsData } from '../../data/credit';
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
import { Calculator, X, TrendingDown, TrendingUp } from 'lucide-react';

export interface CDSInstrumentProps {
  cdsSwaps: CdsData[];
  market: MarketContext;
  asOf: FsDate;
}

interface CDSFormState {
  notional: number;
  spreadBps: number;
  tenorYears: number;
  direction: 'buy_protection' | 'sell_protection';
  currency: string;
}

export const CDSInstrument: React.FC<CDSInstrumentProps> = ({ cdsSwaps, market, asOf }) => {
  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);

  // Initialize form state from first CDS swap
  const initialCds = cdsSwaps[0];
  const [formState, setFormState] = useState<CDSFormState>({
    notional: initialCds?.notional.amount ?? 10_000_000,
    spreadBps: initialCds?.spreadBps ?? 100,
    tenorYears: initialCds ? initialCds.maturityDate.year - initialCds.effectiveDate.year : 5,
    direction: initialCds?.direction ?? 'buy_protection',
    currency: initialCds?.notional.currency ?? 'USD',
  });

  const calculateCDS = useCallback(() => {
    try {
      const registry = createStandardRegistry();
      const notional = Money.fromCode(formState.notional, formState.currency);

      // Use asOf date as effective date
      const effectiveDate = asOf;
      const maturityDate = new FsDate(asOf.year + formState.tenorYears, asOf.month, asOf.day);

      const cds =
        formState.direction === 'buy_protection'
          ? CreditDefaultSwap.buyProtection(
              'interactive_cds',
              notional,
              formState.spreadBps,
              effectiveDate,
              maturityDate,
              initialCds?.discountCurveId ?? 'USD-OIS',
              initialCds?.hazardCurveId ?? 'ACME-HZD',
              null
            )
          : CreditDefaultSwap.sellProtection(
              'interactive_cds',
              notional,
              formState.spreadBps,
              effectiveDate,
              maturityDate,
              initialCds?.discountCurveId ?? 'USD-OIS',
              initialCds?.hazardCurveId ?? 'ACME-HZD',
              null
            );

      const cdsOpts = new PricingRequest().withMetrics(['par_spread', 'pv01']);
      const cdsResult = registry.priceCreditDefaultSwap(cds, 'discounting', market, asOf, cdsOpts);

      const result: InstrumentRow = {
        name: `${formState.tenorYears}Y CDS`,
        type: 'CreditDefaultSwap',
        presentValue: cdsResult.presentValue.amount,
        keyMetric: {
          name: 'Par Spread',
          value: (() => {
            const raw = cdsResult.metric('par_spread') ?? 0;
            return Math.abs(raw) > 10 ? raw : raw * 10000;
          })(),
        },
      };

      setRows([result]);
      setError(null);
    } catch (err) {
      setError(`CDS pricing error: ${err}`);
    }
  }, [formState, market, asOf, initialCds]);

  // Calculate when form is visible and form state changes
  useEffect(() => {
    if (showForm) {
      const timer = setTimeout(() => {
        calculateCDS();
      }, 0);
      return () => clearTimeout(timer);
    }
  }, [showForm, formState, calculateCDS]);

  // Initial calculation from props
  useEffect(() => {
    if (showForm) return; // Skip if using form

    let cancelled = false;

    (async () => {
      try {
        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        for (const cdsData of cdsSwaps) {
          const notional = Money.fromCode(cdsData.notional.amount, cdsData.notional.currency);
          const effectiveDate = new FsDate(
            cdsData.effectiveDate.year,
            cdsData.effectiveDate.month,
            cdsData.effectiveDate.day
          );
          const maturityDate = new FsDate(
            cdsData.maturityDate.year,
            cdsData.maturityDate.month,
            cdsData.maturityDate.day
          );

          const cds =
            cdsData.direction === 'buy_protection'
              ? CreditDefaultSwap.buyProtection(
                  cdsData.id,
                  notional,
                  cdsData.spreadBps,
                  effectiveDate,
                  maturityDate,
                  cdsData.discountCurveId,
                  cdsData.hazardCurveId,
                  null
                )
              : CreditDefaultSwap.sellProtection(
                  cdsData.id,
                  notional,
                  cdsData.spreadBps,
                  effectiveDate,
                  maturityDate,
                  cdsData.discountCurveId,
                  cdsData.hazardCurveId,
                  null
                );

          const cdsOpts = new PricingRequest().withMetrics(['par_spread', 'pv01']);
          try {
            const cdsResult = registry.priceCreditDefaultSwap(
              cds,
              'discounting',
              market,
              asOf,
              cdsOpts
            );
            const tenorYears = cdsData.maturityDate.year - cdsData.effectiveDate.year;
            results.push({
              name: `${tenorYears}Y CDS`,
              type: 'CreditDefaultSwap',
              presentValue: cdsResult.presentValue.amount,
              keyMetric: {
                name: 'Par Spread',
                value: (() => {
                  const raw = cdsResult.metric('par_spread') ?? 0;
                  return Math.abs(raw) > 10 ? raw : raw * 10000;
                })(),
              },
            });
          } catch (err) {
            console.warn('CDS pricing failed, skipping', err);
          }
        }

        if (!cancelled) {
          if (results.length === 0) {
            setError('No CDS instruments priced');
          } else {
            setRows(results);
            setError(null);
          }
        }
      } catch (err) {
        if (!cancelled) {
          setError(`CDS pricing error: ${err}`);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [cdsSwaps, market, asOf, showForm]);

  const handleInputChange = (field: keyof CDSFormState, value: string | number) => {
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
    return <p className="text-muted-foreground animate-pulse">Loading CDS instruments...</p>;
  }

  return (
    <Card className="border-border/50">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <CardTitle className="text-lg flex items-center gap-2">
              Credit Default Swaps
              <Badge variant="outline" className="font-mono text-xs">
                CDS
              </Badge>
            </CardTitle>
            <CardDescription>
              Single-name CDS contracts providing credit protection on reference entities
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
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <div className="space-y-2">
                <Label htmlFor="cds-notional">Notional</Label>
                <Input
                  id="cds-notional"
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
                <Label htmlFor="cds-spread">Spread (bps)</Label>
                <Input
                  id="cds-spread"
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
                <Label htmlFor="cds-tenor">Tenor (years)</Label>
                <Input
                  id="cds-tenor"
                  type="number"
                  value={formState.tenorYears}
                  onChange={(e) =>
                    handleInputChange('tenorYears', Number.parseInt(e.target.value, 10) || 1)
                  }
                  min={1}
                  max={30}
                  className="font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="cds-direction">Direction</Label>
                <Select
                  value={formState.direction}
                  onValueChange={(value) => handleInputChange('direction', value)}
                >
                  <SelectTrigger id="cds-direction">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="buy_protection">
                      <span className="flex items-center gap-2">
                        <TrendingDown className="h-3 w-3 text-destructive" />
                        Buy Protection
                      </span>
                    </SelectItem>
                    <SelectItem value="sell_protection">
                      <span className="flex items-center gap-2">
                        <TrendingUp className="h-3 w-3 text-success" />
                        Sell Protection
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
