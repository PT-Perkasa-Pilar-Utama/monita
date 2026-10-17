import type { ReactElement } from "react";
import type { Day } from "@/lib/types";

/** A monitor's last 24 hours: hourly mean response time, failed hours shaded red. */
export function Sparkline({ day }: { day: Day }): ReactElement {
  return (
    <svg
      viewBox="0 0 160 32"
      preserveAspectRatio="none"
      role="img"
      aria-label={`Response time over the last 24 hours, mean ${day.mean_ms === null ? "unknown" : `${day.mean_ms} ms`}`}
      className="h-8 w-40"
    >
      {day.fails.map((x) => (
        <rect key={x} x={x} y={0} width={3} height={32} className="fill-down/25" />
      ))}
      <path d={day.path} className="fill-none stroke-primary" strokeWidth={1.5} vectorEffect="non-scaling-stroke" />
    </svg>
  );
}
