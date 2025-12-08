import type { CalibrationConfig, FsDate, MarketContext } from 'finstack-wasm';

/** Common result type for all calibrations */
export interface CalibrationResult {
  curveId: string;
  curveType: string;
  success: boolean;
  iterations: number;
  maxResidual: number;
  sampleValues: CurveDataPoint[];
}

/** Data point for curve charting */
export interface CurveDataPoint {
  time: number;
  value: number;
  label?: string;
}

/** Surface data point for vol surface charting */
export interface SurfaceDataPoint {
  expiry: number;
  strike: number;
  vol: number;
}

/** Common props for calibration components */
export interface BaseCalibrationProps {
  baseDate: FsDate;
  curveId: string;
  currency: string;
  config?: CalibrationConfig;
  market?: MarketContext | null;
  onCalibrated?: (result: CalibrationResult) => void;
  showChart?: boolean;
  className?: string;
}

/** Chart configuration */
export interface ChartConfig {
  title: string;
  xLabel: string;
  yLabel: string;
  color?: string;
  yFormatter?: (value: number) => string;
  xFormatter?: (value: number) => string;
}

/** Status badge variant */
export type CalibrationStatus = 'running' | 'success' | 'failed' | 'idle';
