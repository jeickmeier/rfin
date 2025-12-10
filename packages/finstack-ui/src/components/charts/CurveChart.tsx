import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

export interface CurvePoint {
  tenor: string;
  rate: string;
}

export interface CurveSeries {
  label: string;
  points: CurvePoint[];
  color?: string;
}

export interface CurveChartProps {
  title?: string;
  series: CurveSeries[];
  height?: number;
  yLabel?: string;
}

export function CurveChart({
  title = "Curve",
  series,
  height = 320,
  yLabel = "Rate",
}: CurveChartProps) {
  const merged = series.map((s, idx) => ({
    id: `${s.label}-${idx}`,
    key: s.label,
    color: s.color ?? "#2563eb",
    points: s.points.map((p) => ({
      tenor: p.tenor,
      [s.label]: Number(p.rate),
    })),
  }));

  const data = merged.reduce<Record<string, unknown>[]>((rows, serie) => {
    serie.points.forEach((p, i) => {
      if (!rows[i]) rows[i] = {};
      rows[i] = { ...rows[i], tenor: p.tenor, ...p };
    });
    return rows;
  }, []);

  return (
    <div className="space-y-2" data-testid="curve-chart">
      <header className="flex items-center justify-between">
        <h4 className="text-sm font-semibold">{title}</h4>
      </header>
      <div style={{ width: "100%", height }} className="rounded-md border p-2">
        <ResponsiveContainer>
          <LineChart data={data}>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis dataKey="tenor" />
            <YAxis
              label={{ value: yLabel, angle: -90, position: "insideLeft" }}
            />
            <Tooltip />
            <Legend />
            {merged.map((serie) => (
              <Line
                key={serie.id}
                type="monotone"
                dataKey={serie.key}
                stroke={serie.color}
                dot={false}
              />
            ))}
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
