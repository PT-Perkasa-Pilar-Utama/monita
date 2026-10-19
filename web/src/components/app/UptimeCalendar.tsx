import type { ReactElement } from "react";
import { barTitle, barTone, blanksBefore, monthName } from "@/lib/status";
import type { UptimeMonth } from "@/lib/types";
import { cn } from "@/lib/utils";

/** One month as a Sunday-first grid of day squares, with the month's uptime. */
export function MonthGrid({ month }: { month: UptimeMonth }): ReactElement {
  const first = month.days[0]?.day ?? `${month.month}-01`;
  return (
    <section aria-label={monthName(month.month)} className="flex min-w-0 flex-col gap-3">
      <div className="flex items-baseline justify-between gap-3">
        <h2 className="text-lg font-semibold">{monthName(month.month)}</h2>
        <span className="text-sm text-muted-foreground tabular-nums">{month.uptime === null ? "No data" : `${month.uptime.toFixed(2)}%`}</span>
      </div>
      <div className="grid grid-cols-7 gap-1.5">
        {Array.from({ length: blanksBefore(first) }, (_, n) => (
          <span key={`b${n}`} aria-hidden />
        ))}
        {month.days.map((d) => {
          const tone = barTone(d);
          return (
            <span
              key={d.day}
              title={barTitle(d)}
              className={cn(
                "aspect-square rounded-[3px]",
                tone === "up" && "bg-up/70",
                tone === "partial" && "bg-issue",
                tone === "down" && "bg-down",
                tone === "none" && "bg-muted",
              )}
            />
          );
        })}
      </div>
    </section>
  );
}
