/**
 * CDS Tranche instrument component with interactive form.
 */
import React, { useEffect, useState, useCallback } from 'react';
import { CdsTranche, FsDate, MarketContext, Money, createStandardRegistry } from 'finstack-wasm';
import type { CdsTrancheData } from '../../data/credit';
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
import { Calculator, X, TrendingDown, TrendingUp, Layers } from 'lucide-react';

export interface CDSTrancheInstrumentProps {
  cdsTranches: CdsTrancheData[];
  market: MarketContext;
  asOf: FsDate;
}

interface CDSTrancheFormState {
  notional: number;
  attachmentPoint: number; // As percentage (3 = 3%)
  detachmentPoint: number; // As percentage (7 = 7%)
  spreadBps: number;
  series: number;
  direction: 'buy_protection' | 'sell_protection';
  currency: string;
  indexFamily: string;
}

export const CDSTrancheInstrument: React.FC<CDSTrancheInstrumentProps> = ({
  cdsTranches,
  market,
  asOf,
}) => {
  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);

  // Initialize form state from first tranche
  const initialTranche = cdsTranches[0];
  const [formState, setFormState] = useState<CDSTrancheFormState>({
    notional: initialTranche?.notional.amount ?? 10_000_000,
    attachmentPoint: initialTranche?.attachmentPoint ?? 3,
    detachmentPoint: initialTranche?.detachmentPoint ?? 7,
    spreadBps: initialTranche?.spreadBps ?? 500,
    series: initialTranche?.series ?? 42,
    direction: initialTranche?.direction ?? 'buy_protection',
    currency: initialTranche?.notional.currency ?? 'USD',
    indexFamily: initialTranche?.indexFamily ?? 'CDX.NA.IG',
  });

  const calculateTranche = useCallback(() => {
    try {
      const registry = createStandardRegistry();
      const notional = Money.fromCode(formState.notional, formState.currency);
      const maturityDate = new FsDate(asOf.year + 5, asOf.month, asOf.day);

      const tranche = new CdsTranche(
        'interactive_tranche',
        formState.indexFamily,
        formState.series,
        formState.attachmentPoint,
        formState.detachmentPoint,
        notional,
        maturityDate,
        formState.spreadBps,
        initialTranche?.discountCurveId ?? 'USD-OIS',
        formState.indexFamily, // creditIndexId - must match the key in market.insertCreditIndex()
        formState.direction,
        initialTranche?.frequency ?? 4,
        null
      );

      const trancheResult = registry.priceInstrument(tranche, 'discounting', market, asOf, null);

      const result: InstrumentRow = {
        name: `${formState.indexFamily} Tranche (${formState.attachmentPoint}-${formState.detachmentPoint}%)`,
        type: 'CdsTranche',
        presentValue: trancheResult.presentValue.amount,
      };

      setRows([result]);
      setError(null);
    } catch (err) {
      setError(`CDS Tranche pricing error: ${err}`);
    }
  }, [formState, market, asOf, initialTranche]);

  // Calculate when form is visible and form state changes
  useEffect(() => {
    if (showForm) {
      const timer = setTimeout(() => {
        calculateTranche();
      }, 0);
      return () => clearTimeout(timer);
    }
  }, [showForm, formState, calculateTranche]);

  useEffect(() => {
    if (showForm) return;

    let cancelled = false;

    (async () => {
      try {
        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        for (const trancheData of cdsTranches) {
          const notional = Money.fromCode(
            trancheData.notional.amount,
            trancheData.notional.currency
          );
          const maturityDate = new FsDate(
            trancheData.maturityDate.year,
            trancheData.maturityDate.month,
            trancheData.maturityDate.day
          );

          try {
            const tranche = new CdsTranche(
              trancheData.id,
              trancheData.indexFamily,
              trancheData.series,
              trancheData.attachmentPoint,
              trancheData.detachmentPoint,
              notional,
              maturityDate,
              trancheData.spreadBps,
              trancheData.discountCurveId,
              trancheData.creditIndexId,
              trancheData.direction,
              trancheData.frequency,
              null
            );
            const trancheResult = registry.priceInstrument(
              tranche,
              'discounting',
              market,
              asOf,
              null
            );

            results.push({
              name: `${trancheData.indexFamily} Tranche (${trancheData.attachmentPoint}-${trancheData.detachmentPoint}%)`,
              type: 'CdsTranche',
              presentValue: trancheResult.presentValue.amount,
            });
          } catch (err) {
            console.warn('CDS tranche pricing failed, skipping', err);
          }
        }

        if (!cancelled) {
          if (results.length === 0) {
            setError('No CDS tranches priced');
          } else {
            setRows(results);
            setError(null);
          }
        }
      } catch (err) {
        if (!cancelled) {
          setError(`CDS Tranche pricing error: ${err}`);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [cdsTranches, market, asOf, showForm]);

  const handleInputChange = (field: keyof CDSTrancheFormState, value: string | number) => {
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
    return <p className="text-muted-foreground animate-pulse">Loading CDS tranches...</p>;
  }

  return (
    <Card className="border-border/50">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <CardTitle className="text-lg flex items-center gap-2">
              <Layers className="h-5 w-5 text-primary" />
              CDS Tranches
              <Badge variant="outline" className="font-mono text-xs">
                CDO
              </Badge>
            </CardTitle>
            <CardDescription>
              Synthetic CDO tranches with attachment and detachment points for correlated default
              modeling
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
                <Label htmlFor="tranche-notional">Notional</Label>
                <Input
                  id="tranche-notional"
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
                <Label htmlFor="tranche-attach">Attachment (%)</Label>
                <Input
                  id="tranche-attach"
                  type="number"
                  value={formState.attachmentPoint}
                  onChange={(e) =>
                    handleInputChange('attachmentPoint', Number.parseFloat(e.target.value) || 0)
                  }
                  step={1}
                  min={0}
                  max={100}
                  className="font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="tranche-detach">Detachment (%)</Label>
                <Input
                  id="tranche-detach"
                  type="number"
                  value={formState.detachmentPoint}
                  onChange={(e) =>
                    handleInputChange('detachmentPoint', Number.parseFloat(e.target.value) || 0)
                  }
                  step={1}
                  min={0}
                  max={100}
                  className="font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="tranche-spread">Spread (bps)</Label>
                <Input
                  id="tranche-spread"
                  type="number"
                  value={formState.spreadBps}
                  onChange={(e) =>
                    handleInputChange('spreadBps', Number.parseFloat(e.target.value) || 0)
                  }
                  step={10}
                  className="font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="tranche-series">Series</Label>
                <Input
                  id="tranche-series"
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
                <Label htmlFor="tranche-direction">Direction</Label>
                <Select
                  value={formState.direction}
                  onValueChange={(value) => handleInputChange('direction', value)}
                >
                  <SelectTrigger id="tranche-direction">
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

            {/* Visual representation of tranche structure */}
            <div className="mt-4 space-y-2">
              <Label className="text-xs text-muted-foreground">Tranche Structure</Label>
              <div className="relative h-8 rounded-md overflow-hidden bg-muted">
                <div
                  className="absolute h-full bg-gradient-to-r from-destructive/60 to-destructive/40"
                  style={{
                    left: 0,
                    width: `${formState.attachmentPoint}%`,
                  }}
                />
                <div
                  className="absolute h-full bg-gradient-to-r from-primary to-primary/80"
                  style={{
                    left: `${formState.attachmentPoint}%`,
                    width: `${formState.detachmentPoint - formState.attachmentPoint}%`,
                  }}
                />
                <div
                  className="absolute h-full bg-gradient-to-r from-success/40 to-success/20"
                  style={{
                    left: `${formState.detachmentPoint}%`,
                    right: 0,
                  }}
                />
                {/* Labels */}
                <div className="absolute inset-0 flex items-center justify-center text-xs font-mono text-white/90">
                  {formState.attachmentPoint}% - {formState.detachmentPoint}%
                </div>
              </div>
              <div className="flex justify-between text-xs text-muted-foreground">
                <span>Equity (First Loss)</span>
                <span>Mezzanine</span>
                <span>Senior</span>
              </div>
            </div>
          </div>
        )}

        {error && showForm && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        {rows.length > 0 && (
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
                    {keyMetric ? `${keyMetric.name}: ${keyMetric.value.toFixed(2)}` : '—'}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </CardContent>
    </Card>
  );
};
