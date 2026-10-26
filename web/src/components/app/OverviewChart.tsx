import { useState } from "react";
import type { PointerEvent, ReactElement } from "react";
import { useNavigate } from "react-router-dom";
import { columnAt } from "@/lib/chart";
import type { OverviewChart as ChartData } from "@/lib/types";
import { cn } from "@/lib/utils";
import { Pin } from "@/components/icons";

// Six, so six pinned monitors each get their own colour (Q48).
const SERIES = ["stroke-series-1", "stroke-series-2", "stroke-series-3", "stroke-series-4", "stroke-series-5", "stroke-series-6"];
const SWATCH = ["bg-series-1", "bg-series-2", "bg-series-3", "bg-series-4", "bg-series-5", "bg-series-6"];
const SWATCH_FILL = ["fill-series-1", "fill-series-2", "fill-series-3", "fill-series-4", "fill-series-5", "fill-series-6"];
const WIDTH = 1000;
const f = (n: number): string => n.toFixed(1);

const toneText = { ok: "", fail: "text-down", none: "text-muted-foreground" } as const;

/**
 * The Overview's one chart (v1.1 Q39): response time of the monitors that need the team first as
 * lines on a log scale, the other monitors as faint lines behind them, red where a monitor was down, and a
 * lane of ping marks per heartbeat under it. A line, a lane or a legend entry opens its page.
 */
/** What a chart's series link to and are called: a project's monitors and heartbeats, or projects. */
export type ChartLinks = {
  line: (id: number) => string;
  lane: (id: number) => string;
  others: string;
  /** "monitor" or "project", for "45 other monitors". */
  noun: string;
  /** Shown in the plot when there are no lines. */
  empty: string;
};

const short = (name: string, n: number): string => (name.length > n ? `${name.slice(0, n - 1)}…` : name);

export function OverviewChart({ chart, links }: { chart: ChartData; links: ChartLinks }): ReactElement {
  const navigate = useNavigate();
  const [x, setX] = useState<number | null>(null);
  const [focus, setFocus] = useState<number | null>(null);
  const [faint, setFaint] = useState<number | null>(null);
  const [left, top, right, bottom] = chart.plot;
  const col = x === null ? null : columnAt(chart.columns, x);
  const move = (e: PointerEvent<SVGSVGElement>): void => {
    const box = e.currentTarget.getBoundingClientRect();
    setX(((e.clientX - box.left) / box.width) * WIDTH);
  };
  const pct = col ? ((col.x + col.w / 2) / WIDTH) * 100 : 0;
  const empty = chart.lines.length === 0 && chart.others.length === 0;
  const laneEnd = chart.height - 22;

  return (
    <div className="flex flex-col gap-4">
      <div className="relative overflow-x-auto">
        <svg
          viewBox={`0 0 ${WIDTH} ${f(chart.height)}`}
          role="img"
          aria-label="Response time of the monitors and pings of the heartbeats, last 24 hours"
          className="block h-auto w-full min-w-[640px] touch-pan-y select-none"
          onPointerMove={move}
          onPointerDown={move}
          onPointerLeave={() => setX(null)}
        >
          {chart.down.map((d, i) => (
            <rect key={`d${i}`} x={f(d.x)} y={f(d.y)} width={f(d.w)} height={f(d.h)} className="fill-down/12" />
          ))}
          {chart.y_ticks.map((t) => (
            <g key={`y${t.label}`}>
              <line x1={left} x2={right} y1={f(t.at)} y2={f(t.at)} className="stroke-border [stroke-dasharray:2_4]" />
              <text x={left - 8} y={f(t.at)} textAnchor="end" dominantBaseline="middle" className="fill-muted-foreground font-mono text-[11px]">
                {t.label}
              </text>
            </g>
          ))}
          <line x1={left} x2={right} y1={bottom} y2={bottom} className="stroke-input" />
          {chart.others.map((o) => (
            <g key={`o${o.id}`}>
              <path d={o.path} className={cn("fill-none", faint === o.id ? "stroke-foreground/70" : "stroke-muted-foreground/25")} strokeWidth={faint === o.id ? 1.6 : 1} vectorEffect="non-scaling-stroke" />
              <path
                d={o.path}
                className="cursor-pointer fill-none stroke-transparent"
                strokeWidth={8}
                vectorEffect="non-scaling-stroke"
                onPointerEnter={() => setFaint(o.id)}
                onPointerLeave={() => setFaint(null)}
                onClick={() => navigate(links.line(o.id))}
              >
                <title>{o.name}</title>
              </path>
            </g>
          ))}
          {/* The lines' legend fills the column left of the plot, where the lanes' names sit below. */}
          {chart.lines.map((l, i) => {
            const y = top + 14 + i * 40;
            return (
              <g
                key={`k${l.id}`}
                className="cursor-pointer"
                onPointerEnter={() => setFocus(i)}
                onPointerLeave={() => setFocus(null)}
                onClick={() => navigate(links.line(l.id))}
              >
                <title>{l.name}</title>
                <rect x={8} y={y - 4} width={14} height={3} rx={1.5} className={SWATCH_FILL[i]} />
                {l.pinned && <Pin x={9} y={y + 11} size={11} weight="fill" aria-label="Pinned" className="fill-muted-foreground" />}
                <text x={28} y={y} dominantBaseline="middle" className={cn("fill-foreground text-[13px] font-medium", focus === i && "underline")}>
                  {short(l.name, 17)}
                </text>
                <text x={28} y={y + 17} dominantBaseline="middle" className="fill-muted-foreground font-mono text-[11px]">
                  {l.mean_ms === null ? "no check" : `${l.mean_ms} ms`}
                  {l.failed > 0 && <tspan className="fill-down">{`, ${l.failed} failed`}</tspan>}
                </text>
              </g>
            );
          })}
          {chart.others.length > 0 && (
            <text
              x={8}
              y={f(top + 14 + chart.lines.length * 40)}
              dominantBaseline="middle"
              className="cursor-pointer fill-muted-foreground text-[12px] hover:underline"
              onClick={() => navigate(links.others)}
            >
              {faint !== null ? short(chart.others.find((o) => o.id === faint)?.name ?? "", 20) : `${chart.others.length} other ${links.noun}${chart.others.length === 1 ? "" : "s"}`}
            </text>
          )}
          {chart.lines.map((l, i) => (
            <g key={l.id} className={cn("transition-opacity", focus !== null && focus !== i && "opacity-20")}>
              <path d={l.path} className={cn("fill-none [stroke-linecap:round] [stroke-linejoin:round]", SERIES[i])} strokeWidth={1.8} vectorEffect="non-scaling-stroke" />
              {/* A wide clear stroke, so the line is easy to hover and click. */}
              <path
                d={l.path}
                className="cursor-pointer fill-none stroke-transparent"
                strokeWidth={12}
                vectorEffect="non-scaling-stroke"
                onPointerEnter={() => setFocus(i)}
                onPointerLeave={() => setFocus(null)}
                onClick={() => navigate(links.line(l.id))}
              >
                <title>{l.name}</title>
              </path>
            </g>
          ))}
          {empty && (
            <text x={(left + right) / 2} y={(top + bottom) / 2} textAnchor="middle" className="fill-muted-foreground text-[13px]">
              {links.empty}
            </text>
          )}
          {chart.lanes.map((lane) => (
            <g key={lane.id ?? "more"}>
              <line x1={left} x2={right} y1={f(lane.y + 12)} y2={f(lane.y + 12)} className="stroke-border" />
              <text
                x={lane.pinned ? 22 : 8}
                y={f(lane.y + 12)}
                dominantBaseline="middle"
                className={cn("text-[13px]", lane.id === null ? "fill-muted-foreground" : "cursor-pointer fill-foreground hover:underline")}
                onClick={lane.id === null ? undefined : () => navigate(links.lane(lane.id!))}
              >
                {lane.name}
              </text>
              {lane.pinned && <Pin x={7} y={f(lane.y + 6.5)} size={11} weight="fill" aria-label="Pinned" className="fill-muted-foreground" />}
              {lane.marks.map((m, i) => (
                <rect
                  key={i}
                  x={f(m.x)}
                  y={f(m.y)}
                  width={f(m.w)}
                  height={f(m.h)}
                  rx={m.tone === "down" ? 2 : f(m.w / 2)}
                  className={m.tone === "ok" ? "fill-up/85" : m.tone === "fail" ? "fill-down" : "fill-down/20"}
                />
              ))}
            </g>
          ))}
          {col && (
            <line x1={f(col.x + col.w / 2)} x2={f(col.x + col.w / 2)} y1={top} y2={f(laneEnd)} className="stroke-foreground/40" vectorEffect="non-scaling-stroke" />
          )}
          {chart.x_ticks.map((t) => (
            <text key={`x${t.at}`} x={f(t.at)} y={f(chart.height - 6)} textAnchor="middle" className="fill-muted-foreground font-mono text-[11px]">
              {t.label}
            </text>
          ))}
        </svg>
        {col && col.rows.length > 0 && (
          <div
            className={cn(
              "pointer-events-none absolute top-1 z-10 flex min-w-44 flex-col gap-1 rounded-md border bg-popover px-2.5 py-2 text-xs text-popover-foreground shadow-sm",
              pct < 30 ? "translate-x-2" : pct > 70 ? "-translate-x-[calc(100%+0.5rem)]" : "-translate-x-1/2",
            )}
            style={{ left: `${pct}%` }}
          >
            <span className="font-mono text-muted-foreground">{col.label}</span>
            {col.rows.map(([name, text, tone], i) => (
              <span key={name} className="flex items-center justify-between gap-4 whitespace-nowrap">
                <span className="flex min-w-0 items-center gap-1.5">
                  <i className={cn("inline-block size-2 shrink-0 rounded-full", i < chart.lines.length ? SWATCH[i] : "bg-muted-foreground/40")} />
                  <span className="truncate">{name}</span>
                </span>
                <span className={cn("font-mono", toneText[tone])}>{text}</span>
              </span>
            ))}
          </div>
        )}
      </div>
      {/* Monitors are lines and full-height shading; heartbeats are dots in the lanes. */}
      <ul className="flex flex-wrap gap-x-5 gap-y-2 text-xs text-muted-foreground">
        {(chart.lines.length > 0 || chart.others.length > 0) && <Key mark={<i className="inline-block h-0.5 w-3 bg-foreground/60" />} label="Monitor response time" />}
        {chart.down.length > 0 && <Key mark={<i className="inline-block h-3 w-2.5 rounded-xs bg-down/25" />} label="Monitor down" />}
        {chart.lanes.length > 0 && (
          <>
            <Key mark={<i className="inline-block size-2 rounded-full bg-up" />} label="Heartbeat ping" />
            <Key mark={<i className="inline-block size-2 rounded-full bg-down" />} label="Failed ping" />
            {chart.lanes.some((l) => l.marks.some((m) => m.tone === "down")) && <Key mark={<i className="inline-block h-2 w-3 rounded-xs bg-down/25" />} label="Heartbeat missed or failed" />}
          </>
        )}
        {chart.paused > 0 && <li>{chart.paused} paused, not shown</li>}
      </ul>
    </div>
  );
}

function Key({ mark, label }: { mark: ReactElement; label: string }): ReactElement {
  return (
    <li className="inline-flex items-center gap-1.5">
      {mark}
      {label}
    </li>
  );
}
