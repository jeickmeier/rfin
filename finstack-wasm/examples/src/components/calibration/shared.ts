import { useEffect, useState, type Dispatch, type SetStateAction } from 'react';
import { CalibrationConfig, FsDate, Frequency, SolverKind } from 'finstack-wasm';

import type { FrequencyType } from './CurrencyConventions';
import type { CalibrationConfigJson, DateJson } from './state-types';

export const mapFrequency = (freq: FrequencyType): ReturnType<typeof Frequency.annual> => {
  switch (freq) {
    case 'annual':
      return Frequency.annual();
    case 'semi_annual':
      return Frequency.semiAnnual();
    case 'quarterly':
      return Frequency.quarterly();
    case 'monthly':
      return Frequency.monthly();
    default:
      return Frequency.quarterly();
  }
};

export const buildWasmConfig = (
  config: CalibrationConfigJson,
  tolerance = config.tolerance
): CalibrationConfig => {
  let wasmConfig = new CalibrationConfig();
  switch (config.solverKind) {
    case 'Brent':
      wasmConfig = wasmConfig.withSolverKind(SolverKind.Brent());
      break;
    case 'Newton':
      wasmConfig = wasmConfig.withSolverKind(SolverKind.Newton());
      break;
  }

  return wasmConfig
    .withMaxIterations(config.maxIterations)
    .withTolerance(tolerance)
    .withVerbose(config.verbose);
};

export const toFsDate = (date: DateJson): FsDate => new FsDate(date.year, date.month, date.day);

export const isoDate = (date: FsDate): string => {
  const y = String(date.year).padStart(4, '0');
  const m = String(date.month).padStart(2, '0');
  const d = String(date.day).padStart(2, '0');
  return `${y}-${m}-${d}`;
};

export const useEffectiveQuotes = <T>(
  externalQuotes: T[],
  defaultQuotes: T[]
): [T[], Dispatch<SetStateAction<T[]>>] => {
  const [localQuotes, setLocalQuotes] = useState<T[]>(() =>
    externalQuotes.length > 0 ? externalQuotes : defaultQuotes
  );

  useEffect(() => {
    if (externalQuotes.length > 0) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setLocalQuotes(externalQuotes);
    }
  }, [externalQuotes]);

  return [externalQuotes.length > 0 ? externalQuotes : localQuotes, setLocalQuotes];
};
