import React from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  ReferenceLine,
  Area,
  AreaChart,
} from 'recharts';
import type { CalibrationResult, CurveDataPoint, ChartConfig } from './types';

interface CurveChartProps {
  data: CurveDataPoint[];
  config: ChartConfig;
  height?: number;
  showArea?: boolean;
  referenceLines?: { y: number; label: string; stroke?: string }[];
}

const CHART_COLORS = {
  primary: 'hsl(var(--chart-1))',
  secondary: 'hsl(var(--chart-2))',
  tertiary: 'hsl(var(--chart-3))',
  quaternary: 'hsl(var(--chart-4))',
  quinary: 'hsl(var(--chart-5))',
};

export const CurveChart: React.FC<CurveChartProps> = ({
  data,
  config,
  height = 200,
  showArea = false,
  referenceLines = [],
}) => {
  const { title, xLabel, yLabel, color = CHART_COLORS.primary, yFormatter, xFormatter } = config;

  const formatY = yFormatter || ((v: number) => v.toFixed(4));
  const formatX = xFormatter || ((v: number) => `${v}Y`);

  if (data.length === 0) {
    return (
      <div className="flex items-center justify-center h-[200px] text-muted-foreground text-sm border border-dashed rounded-lg">
        No data available
      </div>
    );
  }

  const ChartComponent = showArea ? AreaChart : LineChart;

  return (
    <div className="w-full">
      <h4 className="text-sm font-medium mb-2 text-muted-foreground">{title}</h4>
      <ResponsiveContainer width="100%" height={height}>
        <ChartComponent data={data} margin={{ top: 5, right: 20, left: 10, bottom: 25 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" opacity={0.5} />
          <XAxis
            dataKey="time"
            tickFormatter={formatX}
            stroke="hsl(var(--muted-foreground))"
            fontSize={11}
            tickLine={false}
            axisLine={{ stroke: 'hsl(var(--border))' }}
            label={{
              value: xLabel,
              position: 'bottom',
              offset: 10,
              style: { fill: 'hsl(var(--muted-foreground))', fontSize: 11 },
            }}
          />
          <YAxis
            tickFormatter={formatY}
            stroke="hsl(var(--muted-foreground))"
            fontSize={11}
            tickLine={false}
            axisLine={{ stroke: 'hsl(var(--border))' }}
            width={60}
            label={{
              value: yLabel,
              angle: -90,
              position: 'insideLeft',
              offset: 5,
              style: { fill: 'hsl(var(--muted-foreground))', fontSize: 11, textAnchor: 'middle' },
            }}
          />
          <Tooltip
            contentStyle={{
              backgroundColor: 'hsl(var(--popover))',
              border: '1px solid hsl(var(--border))',
              borderRadius: '6px',
              fontSize: '12px',
            }}
            labelStyle={{ color: 'hsl(var(--foreground))' }}
            formatter={(value: number) => [formatY(value), yLabel]}
            labelFormatter={(label) => `${formatX(label as number)}`}
          />
          {referenceLines.map((ref) => (
            <ReferenceLine
              key={`${ref.y}-${ref.label}`}
              y={ref.y}
              stroke={ref.stroke || 'hsl(var(--muted-foreground))'}
              strokeDasharray="5 5"
              label={{
                value: ref.label,
                fill: 'hsl(var(--muted-foreground))',
                fontSize: 10,
              }}
            />
          ))}
          {showArea ? (
            <Area
              type="monotone"
              dataKey="value"
              stroke={color}
              fill={color}
              fillOpacity={0.1}
              strokeWidth={2}
              dot={{ fill: color, strokeWidth: 0, r: 3 }}
              activeDot={{ r: 5, stroke: color, strokeWidth: 2, fill: 'hsl(var(--background))' }}
            />
          ) : (
            <Line
              type="monotone"
              dataKey="value"
              stroke={color}
              strokeWidth={2}
              dot={{ fill: color, strokeWidth: 0, r: 3 }}
              activeDot={{ r: 5, stroke: color, strokeWidth: 2, fill: 'hsl(var(--background))' }}
            />
          )}
        </ChartComponent>
      </ResponsiveContainer>
    </div>
  );
};

/** Status badge component */
interface StatusBadgeProps {
  status: 'running' | 'success' | 'failed' | 'idle';
}

export const StatusBadge: React.FC<StatusBadgeProps> = ({ status }) => {
  const styles: Record<string, string> = {
    running: 'bg-warning/20 text-warning border-warning/30',
    success: 'bg-success/20 text-success border-success/30',
    failed: 'bg-destructive/20 text-destructive border-destructive/30',
    idle: 'bg-muted text-muted-foreground border-border',
  };

  const labels: Record<string, string> = {
    running: 'Calibrating...',
    success: 'Converged',
    failed: 'Failed',
    idle: 'Ready',
  };

  return (
    <span
      className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium border ${styles[status]}`}
    >
      {status === 'running' && <span className="mr-1 animate-spin">⟳</span>}
      {labels[status]}
    </span>
  );
};

/** Calibration metrics display */
interface CalibrationMetricsProps {
  iterations: number;
  maxResidual: number;
  success: boolean;
}

export const CalibrationMetrics: React.FC<CalibrationMetricsProps> = ({
  iterations,
  maxResidual,
  success,
}) => {
  return (
    <div className="grid grid-cols-3 gap-4 p-3 bg-muted/50 rounded-lg text-sm">
      <div>
        <span className="text-muted-foreground block text-xs uppercase tracking-wide">Status</span>
        <span className={success ? 'text-success font-medium' : 'text-destructive font-medium'}>
          {success ? '✓ Converged' : '✗ Failed'}
        </span>
      </div>
      <div>
        <span className="text-muted-foreground block text-xs uppercase tracking-wide">
          Iterations
        </span>
        <span className="font-mono">{iterations}</span>
      </div>
      <div>
        <span className="text-muted-foreground block text-xs uppercase tracking-wide">
          Max Residual
        </span>
        <span className="font-mono">{maxResidual.toExponential(3)}</span>
      </div>
    </div>
  );
};

interface CalibrationResultPanelProps {
  result: CalibrationResult | null;
  chartConfig: ChartConfig;
  showChart?: boolean;
  showArea?: boolean;
  referenceLines?: { y: number; label: string; stroke?: string }[];
}

export const CalibrationResultPanel: React.FC<CalibrationResultPanelProps> = ({
  result,
  chartConfig,
  showChart = true,
  showArea = false,
  referenceLines = [],
}) => {
  if (!result) {
    return null;
  }

  return (
    <>
      <CalibrationMetrics
        iterations={result.iterations}
        maxResidual={result.maxResidual}
        success={result.success}
      />
      {showChart && result.sampleValues.length > 0 && (
        <CurveChart
          data={result.sampleValues}
          config={chartConfig}
          showArea={showArea}
          referenceLines={referenceLines}
        />
      )}
    </>
  );
};
