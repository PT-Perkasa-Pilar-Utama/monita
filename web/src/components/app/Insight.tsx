import type { ReactElement } from "react";
import { useSearchParams } from "react-router-dom";
import { TableCard } from "@/components/app/TableCard";
import { Card, CardAction, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { barWidth, grade, pct } from "@/lib/availability";
import { duration, zone } from "@/lib/time";
import type { Chart, Insight as InsightData, Range, Stat } from "@/lib/types";
import { cn } from "@/lib/utils";

const RANGES: [Range, string][] = [
  ["hour", "Hour"],
  ["day", "Day"],
  ["week", "Week"],
  ["month", "Month"],
];

/** Four numbers under the title, each with a tone bar on its left. */
export function StatCards({ stats }: { stats: Stat[] }): ReactElement {
  return (
    <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
      {stats.map((s) => (
        <Card
          key={s.label}
          className={cn(
            "relative gap-1 overflow-hidden px-5 py-4 before:absolute before:inset-y-0 before:left-0 before:w-[3px]",
            s.tone === "up" && "before:bg-up",
            s.tone === "down" && "before:bg-down",
            s.tone === "plain" && "before:bg-neutral-strong",
          )}
        >
          <span className="text-[13px] text-muted-foreground">{s.label}</span>
          <span className="text-2xl font-semibold tracking-tight">{s.value}</span>
          <span className="truncate font-mono text-xs text-muted-foreground">{s.sub}</span>
        </Card>
      ))}
    </div>
  );
}

const f = (n: number): string => n.toFixed(1);

/** The server's chart geometry, drawn as SVG. Plot bounds: x 56 to 952, y 10 to 212. */
function ChartSvg({ chart, title, range }: { chart: Chart; title: string; range: Range }): ReactElement {
  const rect = (cls: string) => (m: Chart["up"][number], i: number) => (
    <rect key={`${cls}${i}`} className={cls} x={f(m.x)} y={f(m.y)} width={f(m.w)} height={f(m.h)}>
      <title>{m.title}</title>
    </rect>
  );
  return (
    <svg viewBox="0 0 960 240" role="img" aria-label={`${title}, last ${range}`} className="block h-auto w-full px-2">
      <defs>
        <linearGradient id="chart-fill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" className="[stop-color:var(--primary)] [stop-opacity:0.38]" />
          <stop offset="1" className="[stop-color:var(--primary)] [stop-opacity:0]" />
        </linearGradient>
      </defs>
      {chart.up.map(rect("fill-up/10"))}
      {chart.down.map(rect("fill-down/20"))}
      {chart.y_ticks.map((t) => (
        <g key={`y${t.label}`}>
          <line x1={56} x2={952} y1={f(t.at)} y2={f(t.at)} className="stroke-border [stroke-dasharray:2_4]" />
          <text x={48} y={f(t.at)} textAnchor="end" dominantBaseline="middle" className="fill-muted-foreground font-mono text-[9px]">
            {t.label}
          </text>
        </g>
      ))}
      <line x1={56} x2={952} y1={212} y2={212} className="stroke-input" />
      {chart.area && (
        <>
          <path d={chart.area} fill="url(#chart-fill)" />
          <path d={chart.line} className="fill-none stroke-primary [stroke-linecap:round] [stroke-linejoin:round]" strokeWidth={1.6} vectorEffect="non-scaling-stroke" />
        </>
      )}
      {chart.dots.map(([x, y]) => (
        <circle key={`${x},${y}`} cx={f(x)} cy={f(y)} r={2.5} className="fill-card stroke-primary" strokeWidth={1.5} />
      ))}
      {chart.oks.map(rect("fill-up/85"))}
      {chart.fails.map(rect("fill-down"))}
      {chart.hovers.map(rect("fill-transparent hover:fill-muted"))}
      {chart.x_ticks.map((t) => (
        <text key={`x${t.label}${t.at}`} x={f(t.at)} y={232} textAnchor="middle" className="fill-muted-foreground font-mono text-[9px]">
          {t.label}
        </text>
      ))}
      {chart.empty && (
        <text x={504} y={112} textAnchor="middle" className="fill-muted-foreground text-xs">
          Checks and pings in this range will show here
        </text>
      )}
    </svg>
  );
}

function Key({ className, label }: { className: string; label: string }): ReactElement {
  return (
    <span className="inline-flex items-center gap-1.5">
      <i className={cn("inline-block size-2.5 rounded-xs", className)} />
      {label}
    </span>
  );
}

/** The chart card: totals, a range switch kept in `?range=`, the chart and its legend. */
export function ChartCard({ insight }: { insight: InsightData }): ReactElement {
  const [params, setParams] = useSearchParams();
  const response = insight.title === "Response time";
  const failed = insight.total - insight.ok;
  const mean = insight.mean_ms === null ? "–" : `${insight.mean_ms} ms`;
  return (
    <Card className="gap-2 pb-3">
      <CardHeader>
        <CardTitle className="text-[15px]">{insight.title}</CardTitle>
        <CardDescription>
          {insight.total.toLocaleString()} {response ? (insight.total === 1 ? "check" : "checks") : insight.total === 1 ? "ping" : "pings"}
          {response && `, mean ${mean}`}
          {failed > 0 && `, ${failed.toLocaleString()} failed`}
        </CardDescription>
        <CardAction>
          <ToggleGroup
            type="single"
            variant="outline"
            size="sm"
            aria-label="Range"
            value={insight.range}
            onValueChange={(r) => {
              if (!r) return;
              const next = new URLSearchParams(params);
              next.set("range", r);
              next.delete("before");
              setParams(next, { replace: true });
            }}
          >
            {RANGES.map(([r, label]) => (
              <ToggleGroupItem key={r} value={r} className="px-3">
                {label}
              </ToggleGroupItem>
            ))}
          </ToggleGroup>
        </CardAction>
      </CardHeader>
      <ChartSvg chart={insight.chart} title={insight.title} range={insight.range} />
      <div className="flex flex-wrap items-center gap-x-4 gap-y-2 px-4 text-xs text-muted-foreground">
        {response ? <Key className="h-0.5 bg-primary" label="Mean response time" /> : <Key className="w-[3px] bg-up" label="Ping received" />}
        <Key className="bg-up/15 ring-1 ring-up/40" label="Up" />
        <Key className="bg-down/25" label="Down" />
        <Key className="bg-down" label="Failed" />
        <span className="ml-auto">{zone()}</span>
      </div>
    </Card>
  );
}

/** Availability over fixed periods, with downtime and incidents. */
export function AvailabilityCard({ insight }: { insight: InsightData }): ReactElement {
  const dur = (s: number | null): string => (s && s > 0 ? duration(s) : "none");
  return (
    <TableCard title="Availability">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Period</TableHead>
            <TableHead>Availability</TableHead>
            <TableHead className="text-right">Downtime</TableHead>
            <TableHead className="text-right">Incidents</TableHead>
            <TableHead className="text-right max-sm:hidden">Longest</TableHead>
            <TableHead className="text-right max-sm:hidden">Average</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {insight.periods.map((p) => {
            const g = grade(p.availability);
            return (
              <TableRow key={p.label}>
                <TableCell className="font-medium">{p.label}</TableCell>
                <TableCell>
                  <div className="flex min-w-32 flex-col gap-1.5">
                    <span className={cn("font-mono", g === "poor" && "text-down")}>{pct(p.availability)}</span>
                    <svg viewBox="0 0 100 4" preserveAspectRatio="none" aria-hidden="true" className="h-1 w-full max-w-40 overflow-hidden rounded-full">
                      <rect width={100} height={4} className="fill-muted" />
                      <rect
                        width={barWidth(p.availability)}
                        height={4}
                        className={cn(g === "good" && "fill-up", g === "fair" && "fill-issue", g === "poor" && "fill-down", g === "none" && "fill-unknown")}
                      />
                    </svg>
                  </div>
                </TableCell>
                <TableCell className="text-right font-mono">{dur(p.downtime_s)}</TableCell>
                <TableCell className="text-right font-mono">{p.incidents}</TableCell>
                <TableCell className="text-right font-mono max-sm:hidden">{dur(p.longest_s)}</TableCell>
                <TableCell className="text-right font-mono max-sm:hidden">{dur(p.average_s)}</TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </TableCard>
  );
}
