/**
 * CDS Option instrument component with interactive form.
 */
import React, { useEffect, useState, useCallback } from 'react';
import { CdsOption, FsDate, MarketContext, Money, createStandardRegistry } from 'finstack-wasm';
import type { CdsOptionData } from '../../data/credit';
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
import { Switch } from '@/components/ui/switch';
import { Calculator, X, ArrowUpRight, ArrowDownRight, Zap } from 'lucide-react';

export interface CDSOptionInstrumentProps {
  cdsOptions: CdsOptionData[];
  market: MarketContext;
  asOf: FsDate;
}

interface CDSOptionFormState {
  notional: number;
  strikeBps: number;
  optionType: 'call' | 'put';
  recoveryRate: number;
  expiryMonths: number;
  underlyingTenorYears: number;
  knockedOut: boolean;
  currency: string;
}

export const CDSOptionInstrument: React.FC<CDSOptionInstrumentProps> = ({
  cdsOptions,
  market,
  asOf,
}) => {
  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);

  // Initialize form state from first option
  const initialOption = cdsOptions[0];
  const [formState, setFormState] = useState<CDSOptionFormState>({
    notional: initialOption?.notional.amount ?? 5_000_000,
    strikeBps: initialOption?.strikeBps ?? 150,
    optionType: initialOption?.optionType ?? 'call',
    recoveryRate: initialOption?.recoveryRate ?? 0.4,
    expiryMonths: initialOption
      ? (initialOption.expiryDate.year - 2024) * 12 + initialOption.expiryDate.month
      : 12,
    underlyingTenorYears: initialOption
      ? initialOption.underlyingMaturity.year - initialOption.expiryDate.year
      : 5,
    knockedOut: initialOption?.knockedOut ?? false,
    currency: initialOption?.notional.currency ?? 'USD',
  });

  const calculateOption = useCallback(() => {
    try {
      const registry = createStandardRegistry();
      const notional = Money.fromCode(formState.notional, formState.currency);

      // Calculate expiry date from months
      const expiryYear = asOf.year + Math.floor(formState.expiryMonths / 12);
      const expiryMonth = ((asOf.month - 1 + formState.expiryMonths) % 12) + 1;
      const expiryDate = new FsDate(expiryYear, expiryMonth, asOf.day);

      const underlyingMaturity = new FsDate(
        expiryYear + formState.underlyingTenorYears,
        expiryMonth,
        asOf.day
      );

      const option = new CdsOption(
        'interactive_option',
        notional,
        formState.strikeBps,
        expiryDate,
        underlyingMaturity,
        initialOption?.discountCurveId ?? 'USD-OIS',
        initialOption?.hazardCurveId ?? 'ACME-HZD',
        initialOption?.volSurfaceId ?? 'CDS-VOL',
        formState.optionType,
        formState.recoveryRate,
        formState.knockedOut,
        null
      );

      const optionResult = registry.priceInstrument(option, 'discounting', market, asOf, null);

      const result: InstrumentRow = {
        name: `CDS ${formState.optionType.charAt(0).toUpperCase() + formState.optionType.slice(1)} @ ${formState.strikeBps}bp`,
        type: 'CdsOption',
        presentValue: optionResult.presentValue.amount,
      };

      setRows([result]);
      setError(null);
    } catch (err) {
      setError(`CDS Option pricing error: ${err}`);
    }
  }, [formState, market, asOf, initialOption]);

  // Calculate when form is visible and form state changes
  useEffect(() => {
    if (showForm) {
      const timer = setTimeout(() => {
        calculateOption();
      }, 0);
      return () => clearTimeout(timer);
    }
  }, [showForm, formState, calculateOption]);

  useEffect(() => {
    if (showForm) return;

    let cancelled = false;

    (async () => {
      try {
        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        for (const optionData of cdsOptions) {
          const notional = Money.fromCode(optionData.notional.amount, optionData.notional.currency);
          const expiryDate = new FsDate(
            optionData.expiryDate.year,
            optionData.expiryDate.month,
            optionData.expiryDate.day
          );
          const underlyingMaturity = new FsDate(
            optionData.underlyingMaturity.year,
            optionData.underlyingMaturity.month,
            optionData.underlyingMaturity.day
          );

          try {
            const option = new CdsOption(
              optionData.id,
              notional,
              optionData.strikeBps,
              expiryDate,
              underlyingMaturity,
              optionData.discountCurveId,
              optionData.hazardCurveId,
              optionData.volSurfaceId,
              optionData.optionType,
              optionData.recoveryRate,
              optionData.knockedOut,
              null
            );
            const optionResult = registry.priceInstrument(
              option,
              'discounting',
              market,
              asOf,
              null
            );
            results.push({
              name: `CDS ${optionData.optionType.charAt(0).toUpperCase() + optionData.optionType.slice(1)} @ ${optionData.strikeBps}bp`,
              type: 'CdsOption',
              presentValue: optionResult.presentValue.amount,
            });
          } catch (err) {
            console.warn('CDS option pricing failed, skipping', err);
          }
        }

        if (!cancelled) {
          if (results.length === 0) {
            setError('No CDS options priced');
          } else {
            setRows(results);
            setError(null);
          }
        }
      } catch (err) {
        if (!cancelled) {
          setError(`CDS Option pricing error: ${err}`);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [cdsOptions, market, asOf, showForm]);

  const handleInputChange = (field: keyof CDSOptionFormState, value: string | number | boolean) => {
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
    return <p className="text-muted-foreground animate-pulse">Loading CDS options...</p>;
  }

  return (
    <Card className="border-border/50">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <CardTitle className="text-lg flex items-center gap-2">
              <Zap className="h-5 w-5 text-primary" />
              CDS Options
              <Badge variant="outline" className="font-mono text-xs">
                Swaptions
              </Badge>
            </CardTitle>
            <CardDescription>
              Options on credit default swaps with Black-style pricing and knockout feature support
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
            <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
              <div className="space-y-2">
                <Label htmlFor="opt-notional">Notional</Label>
                <Input
                  id="opt-notional"
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
                <Label htmlFor="opt-strike">Strike (bps)</Label>
                <Input
                  id="opt-strike"
                  type="number"
                  value={formState.strikeBps}
                  onChange={(e) =>
                    handleInputChange('strikeBps', Number.parseFloat(e.target.value) || 0)
                  }
                  step={10}
                  className="font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="opt-expiry">Expiry (months)</Label>
                <Input
                  id="opt-expiry"
                  type="number"
                  value={formState.expiryMonths}
                  onChange={(e) =>
                    handleInputChange('expiryMonths', Number.parseInt(e.target.value, 10) || 1)
                  }
                  min={1}
                  max={60}
                  className="font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="opt-tenor">Underlying Tenor (yrs)</Label>
                <Input
                  id="opt-tenor"
                  type="number"
                  value={formState.underlyingTenorYears}
                  onChange={(e) =>
                    handleInputChange(
                      'underlyingTenorYears',
                      Number.parseInt(e.target.value, 10) || 1
                    )
                  }
                  min={1}
                  max={10}
                  className="font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="opt-type">Option Type</Label>
                <Select
                  value={formState.optionType}
                  onValueChange={(value) => handleInputChange('optionType', value)}
                >
                  <SelectTrigger id="opt-type">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="call">
                      <span className="flex items-center gap-2">
                        <ArrowUpRight className="h-3 w-3 text-success" />
                        Call (Payer)
                      </span>
                    </SelectItem>
                    <SelectItem value="put">
                      <span className="flex items-center gap-2">
                        <ArrowDownRight className="h-3 w-3 text-destructive" />
                        Put (Receiver)
                      </span>
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="opt-recovery">Recovery Rate</Label>
                <Input
                  id="opt-recovery"
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
              <div className="space-y-2 col-span-2 md:col-span-1">
                <Label htmlFor="opt-knockout">Knockout Feature</Label>
                <div className="flex items-center gap-3 h-10">
                  <Switch
                    id="opt-knockout"
                    checked={formState.knockedOut}
                    onCheckedChange={(checked) => handleInputChange('knockedOut', checked)}
                  />
                  <span className="text-sm text-muted-foreground">
                    {formState.knockedOut ? 'Knocked Out' : 'Active'}
                  </span>
                </div>
              </div>
            </div>

            {formState.knockedOut && (
              <Alert>
                <AlertDescription className="text-sm">
                  Option is knocked out — will be worthless if a credit event occurs before expiry.
                </AlertDescription>
              </Alert>
            )}
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
                  {keyMetric ? `${keyMetric.name}: ${keyMetric.value.toFixed(2)}` : '—'}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
};
