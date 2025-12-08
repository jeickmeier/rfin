/**
 * Credit Instruments - Composite component for all credit derivatives.
 *
 * Renders CDS, CDS Index, CDS Tranche, CDS Option, and Revolving Credit
 * instruments using a shared market context.
 */
import React from 'react';
import { CreditInstrumentsProps, DEFAULT_CREDIT_PROPS } from './data/credit';
import {
  CDSInstrument,
  CDSIndexInstrument,
  CDSTrancheInstrument,
  CDSOptionInstrument,
  RevolvingCreditInstrument,
  useCreditMarket,
} from './instruments/credit';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';

export const CreditInstrumentsExample: React.FC<CreditInstrumentsProps> = (props) => {
  const defaults = DEFAULT_CREDIT_PROPS as Required<CreditInstrumentsProps>;
  const {
    valuationDate = defaults.valuationDate,
    discountCurve = defaults.discountCurve,
    hazardCurves = defaults.hazardCurves,
    baseCorrelation = defaults.baseCorrelation,
    cdsVolSurface = defaults.cdsVolSurface,
    creditIndexData = defaults.creditIndexData,
    cdsSwaps = defaults.cdsSwaps,
    cdsIndices = defaults.cdsIndices,
    cdsTranches = defaults.cdsTranches,
    cdsOptions = defaults.cdsOptions,
    revolvingCredits = defaults.revolvingCredits,
  } = props;

  const marketResult = useCreditMarket({
    valuationDate,
    discountCurve,
    hazardCurves,
    baseCorrelation,
    cdsVolSurface,
    creditIndexData,
  });

  if (!marketResult) {
    return (
      <Alert variant="destructive">
        <AlertDescription>Failed to build credit market context</AlertDescription>
      </Alert>
    );
  }

  const { market, asOf } = marketResult;

  return (
    <Card>
      <CardHeader>
        <CardTitle>Credit Derivatives</CardTitle>
        <CardDescription>
          Credit instruments including single-name CDS, CDS indices, tranches, options on CDS, and
          revolving credit facilities. Uses hazard curves for survival probabilities, base
          correlation for tranche pricing, and supports both deterministic and stochastic
          utilization for revolving credit.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-8">
        {cdsSwaps.length > 0 && <CDSInstrument cdsSwaps={cdsSwaps} market={market} asOf={asOf} />}

        {cdsIndices.length > 0 && (
          <CDSIndexInstrument cdsIndices={cdsIndices} market={market} asOf={asOf} />
        )}

        {cdsTranches.length > 0 && (
          <CDSTrancheInstrument cdsTranches={cdsTranches} market={market} asOf={asOf} />
        )}

        {cdsOptions.length > 0 && (
          <CDSOptionInstrument cdsOptions={cdsOptions} market={market} asOf={asOf} />
        )}

        {revolvingCredits.length > 0 && (
          <RevolvingCreditInstrument
            revolvingCredits={revolvingCredits}
            market={market}
            asOf={asOf}
          />
        )}
      </CardContent>
    </Card>
  );
};

// Re-export individual components for standalone use
export {
  CDSInstrument,
  CDSIndexInstrument,
  CDSTrancheInstrument,
  CDSOptionInstrument,
  RevolvingCreditInstrument,
} from './instruments/credit';
