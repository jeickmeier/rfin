/**
 * Revolving Credit instrument component with interactive form.
 */
import React, { useEffect, useState, useCallback } from 'react';
import { FsDate, MarketContext, RevolvingCredit, standardRegistry } from 'finstack-wasm';
import type { RevolvingCreditData } from '../../data/credit';
import { currencyFormatter, type InstrumentRow } from './useCreditMarket';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
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
import { Calculator, X, Building2, Percent } from 'lucide-react';

export interface RevolvingCreditInstrumentProps {
  revolvingCredits: RevolvingCreditData[];
  market: MarketContext;
  asOf: FsDate;
}

interface RevolvingCreditFormState {
  commitmentAmount: number;
  drawnAmount: number;
  fixedRate: number;
  commitmentFeeBp: number;
  usageFeeBp: number;
  facilityFeeBp: number;
  tenorYears: number;
  currency: string;
}

export const RevolvingCreditInstrument: React.FC<RevolvingCreditInstrumentProps> = ({
  revolvingCredits,
  market,
  asOf,
}) => {
  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);

  // Initialize form state from first revolving credit
  const initialRc = revolvingCredits[0];
  const [formState, setFormState] = useState<RevolvingCreditFormState>({
    commitmentAmount: initialRc?.commitmentAmount.amount ?? 10_000_000,
    drawnAmount: initialRc?.drawnAmount.amount ?? 5_000_000,
    fixedRate: (initialRc?.baseRateSpec as { Fixed?: { rate: number } })?.Fixed?.rate ?? 0.05,
    commitmentFeeBp: initialRc?.fees.commitmentFeeBp ?? 25,
    usageFeeBp: initialRc?.fees.usageFeeBp ?? 10,
    facilityFeeBp: initialRc?.fees.facilityFeeBp ?? 5,
    tenorYears: 2,
    currency: initialRc?.commitmentAmount.currency ?? 'USD',
  });

  const calculateRevolvingCredit = useCallback(() => {
    try {
      const registry = standardRegistry();

      const commitmentDate = `${asOf.year}-${String(asOf.month).padStart(2, '0')}-${String(asOf.day).padStart(2, '0')}`;
      const maturityDate = `${asOf.year + formState.tenorYears}-${String(asOf.month).padStart(2, '0')}-${String(asOf.day).padStart(2, '0')}`;

      const revolvingCreditJson = JSON.stringify({
        id: 'interactive_rc',
        commitment_amount: { amount: formState.commitmentAmount, currency: formState.currency },
        drawn_amount: { amount: formState.drawnAmount, currency: formState.currency },
        commitment_date: commitmentDate,
        maturity_date: maturityDate,
        base_rate_spec: { Fixed: { rate: formState.fixedRate } },
        day_count: 'act360',
        payment_frequency: { count: 3, unit: 'months' },
        fees: {
          upfront_fee: null,
          commitment_fee_bp: formState.commitmentFeeBp,
          usage_fee_bp: formState.usageFeeBp,
          facility_fee_bp: formState.facilityFeeBp,
        },
        draw_repay_spec: { Deterministic: [] },
        discount_curve_id: initialRc?.discountCurveId ?? 'USD-OIS',
        attributes: { tags: [], meta: {} },
      });

      const revolvingCredit = RevolvingCredit.fromJson(revolvingCreditJson);
      const rcResult = registry.priceInstrument(revolvingCredit, 'discounting', market, asOf, null);
      const utilization = (formState.drawnAmount / formState.commitmentAmount) * 100;

      const result: InstrumentRow = {
        name: 'Revolving Credit (Interactive)',
        type: 'RevolvingCredit',
        presentValue: rcResult.presentValue.amount,
        keyMetric: {
          name: 'Utilization',
          value: utilization,
        },
      };

      setRows([result]);
      setError(null);
    } catch (err) {
      setError(`Revolving Credit pricing error: ${err}`);
    }
  }, [formState, market, asOf, initialRc]);

  // Calculate when form is visible and form state changes
  useEffect(() => {
    if (showForm) {
      const timer = setTimeout(() => {
        calculateRevolvingCredit();
      }, 0);
      return () => clearTimeout(timer);
    }
  }, [showForm, formState, calculateRevolvingCredit]);

  useEffect(() => {
    if (showForm) return;

    let cancelled = false;

    (async () => {
      try {
        const registry = standardRegistry();
        const results: InstrumentRow[] = [];

        for (const rcData of revolvingCredits) {
          try {
            const revolvingCreditJson = JSON.stringify({
              id: rcData.id,
              commitment_amount: {
                amount: rcData.commitmentAmount.amount,
                currency: rcData.commitmentAmount.currency,
              },
              drawn_amount: {
                amount: rcData.drawnAmount.amount,
                currency: rcData.drawnAmount.currency,
              },
              commitment_date: rcData.commitmentDate,
              maturity_date: rcData.maturityDate,
              base_rate_spec: rcData.baseRateSpec,
              day_count: rcData.dayCount,
              payment_frequency: rcData.paymentFrequency,
              fees: {
                upfront_fee: rcData.fees.upfrontFee,
                commitment_fee_bp: rcData.fees.commitmentFeeBp,
                usage_fee_bp: rcData.fees.usageFeeBp,
                facility_fee_bp: rcData.fees.facilityFeeBp,
              },
              draw_repay_spec: rcData.drawRepaySpec,
              discount_curve_id: rcData.discountCurveId,
              attributes: { tags: [], meta: {} },
            });
            const revolvingCredit = RevolvingCredit.fromJson(revolvingCreditJson);
            const rcResult = registry.priceInstrument(
              revolvingCredit,
              'discounting',
              market,
              asOf,
              null
            );
            const utilization = (rcData.drawnAmount.amount / rcData.commitmentAmount.amount) * 100;

            const mode = 'Deterministic' in rcData.drawRepaySpec ? 'Deterministic' : 'Stochastic';

            results.push({
              name: `Revolving Credit (${mode})`,
              type: 'RevolvingCredit',
              presentValue: rcResult.presentValue.amount,
              keyMetric: {
                name: 'Utilization',
                value: utilization,
              },
            });
          } catch (err) {
            console.warn('Revolving credit failed, skipping', err);
          }
        }

        if (!cancelled) {
          if (results.length === 0) {
            setError('No revolving credits priced');
          } else {
            setRows(results);
            setError(null);
          }
        }
      } catch (err) {
        if (!cancelled) {
          setError(`Revolving Credit pricing error: ${err}`);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [revolvingCredits, market, asOf, showForm]);

  const handleInputChange = (field: keyof RevolvingCreditFormState, value: number) => {
    setFormState((prev) => ({ ...prev, [field]: value }));
  };

  // Calculate utilization percentage
  const utilization = (formState.drawnAmount / formState.commitmentAmount) * 100;

  if (error && !showForm) {
    return (
      <Alert variant="destructive">
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    );
  }

  if (rows.length === 0 && !showForm) {
    return <p className="text-muted-foreground animate-pulse">Loading revolving credits...</p>;
  }

  return (
    <Card className="border-border/50">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <CardTitle className="text-lg flex items-center gap-2">
              <Building2 className="h-5 w-5 text-primary" />
              Revolving Credit Facilities
              <Badge variant="outline" className="font-mono text-xs">
                RCF
              </Badge>
            </CardTitle>
            <CardDescription>
              Bank credit facilities with flexible draw/repay schedules and fee structures
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
            {/* Utilization bar */}
            <div className="space-y-2">
              <div className="flex items-center justify-between text-sm">
                <Label className="text-muted-foreground">Utilization</Label>
                <span className="font-mono font-semibold flex items-center gap-1">
                  <Percent className="h-3 w-3" />
                  {utilization.toFixed(1)}%
                </span>
              </div>
              <div className="h-3 rounded-full bg-muted overflow-hidden">
                <div
                  className={`h-full transition-all duration-300 ${
                    utilization > 80
                      ? 'bg-destructive'
                      : utilization > 50
                        ? 'bg-warning'
                        : 'bg-success'
                  }`}
                  style={{ width: `${Math.min(utilization, 100)}%` }}
                />
              </div>
            </div>

            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <div className="space-y-2">
                <Label htmlFor="rc-commitment">Commitment</Label>
                <Input
                  id="rc-commitment"
                  type="number"
                  value={formState.commitmentAmount}
                  onChange={(e) =>
                    handleInputChange('commitmentAmount', Number.parseFloat(e.target.value) || 0)
                  }
                  step={1000000}
                  className="font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="rc-drawn">Drawn Amount</Label>
                <Input
                  id="rc-drawn"
                  type="number"
                  value={formState.drawnAmount}
                  onChange={(e) =>
                    handleInputChange('drawnAmount', Number.parseFloat(e.target.value) || 0)
                  }
                  step={500000}
                  max={formState.commitmentAmount}
                  className="font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="rc-tenor">Tenor (years)</Label>
                <Input
                  id="rc-tenor"
                  type="number"
                  value={formState.tenorYears}
                  onChange={(e) =>
                    handleInputChange('tenorYears', Number.parseInt(e.target.value, 10) || 1)
                  }
                  min={1}
                  max={10}
                  className="font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="rc-rate">Fixed Rate (%)</Label>
                <Input
                  id="rc-rate"
                  type="number"
                  value={(formState.fixedRate * 100).toFixed(2)}
                  onChange={(e) =>
                    handleInputChange('fixedRate', (Number.parseFloat(e.target.value) || 0) / 100)
                  }
                  step={0.25}
                  min={0}
                  className="font-mono"
                />
              </div>
            </div>

            {/* Fee structure */}
            <div className="space-y-2">
              <Label className="text-xs text-muted-foreground">Fee Structure (bps)</Label>
              <div className="grid grid-cols-3 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="rc-commit-fee" className="text-xs">
                    Commitment
                  </Label>
                  <Input
                    id="rc-commit-fee"
                    type="number"
                    value={formState.commitmentFeeBp}
                    onChange={(e) =>
                      handleInputChange('commitmentFeeBp', Number.parseFloat(e.target.value) || 0)
                    }
                    step={5}
                    min={0}
                    className="font-mono"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="rc-usage-fee" className="text-xs">
                    Usage
                  </Label>
                  <Input
                    id="rc-usage-fee"
                    type="number"
                    value={formState.usageFeeBp}
                    onChange={(e) =>
                      handleInputChange('usageFeeBp', Number.parseFloat(e.target.value) || 0)
                    }
                    step={5}
                    min={0}
                    className="font-mono"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="rc-facility-fee" className="text-xs">
                    Facility
                  </Label>
                  <Input
                    id="rc-facility-fee"
                    type="number"
                    value={formState.facilityFeeBp}
                    onChange={(e) =>
                      handleInputChange('facilityFeeBp', Number.parseFloat(e.target.value) || 0)
                    }
                    step={5}
                    min={0}
                    className="font-mono"
                  />
                </div>
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
                      <span
                        className={`font-semibold ${
                          keyMetric.value > 80
                            ? 'text-destructive'
                            : keyMetric.value > 50
                              ? 'text-warning'
                              : 'text-success'
                        }`}
                      >
                        {keyMetric.value.toFixed(1)}%
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
