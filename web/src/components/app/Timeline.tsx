import type { ReactElement } from "react";
import { Link } from "react-router-dom";
import { DayBars } from "@/components/app/PublicStatus";
import { StateBadge } from "@/components/app/Tone";
import { Button } from "@/components/ui/button";
import type { Lane } from "@/lib/types";

/** One kind's lanes: a link to all of them, then a row each with its state, 24 hourly bars and its
 * uptime. A row opens the monitor or heartbeat; each bar names its hour on hover. */
export function TimelineGroup({ title, base, lanes, total, empty, addLabel }: { title: string; base: string; lanes: Lane[]; total: number; empty: string; addLabel: string }): ReactElement {
  return (
    <section className="flex flex-col gap-2" aria-label={title}>
      <div className="flex items-center justify-between gap-3">
        <h3 className="text-sm font-semibold">
          {title}
          {total > lanes.length && <span className="font-normal text-muted-foreground"> {lanes.length} of {total}, those that need you first</span>}
        </h3>
        <Button asChild variant="ghost" size="sm">
          <Link to={base}>{total === 0 ? addLabel : `All ${title.toLowerCase()}`}</Link>
        </Button>
      </div>
      {lanes.length === 0 ? (
        <p className="rounded-lg border border-dashed px-4 py-3 text-sm text-muted-foreground">{empty}</p>
      ) : (
        <ul className="flex flex-col">
          {lanes.map((l) => (
            <li key={l.id}>
              <Link
                to={`${base}/${l.id}`}
                aria-label={`${l.name}, open its page`}
                className="grid grid-cols-[minmax(0,10rem)_minmax(0,1fr)_4.5rem] items-center gap-3 rounded-md px-2 py-1.5 outline-none hover:bg-accent focus-visible:ring-[3px] focus-visible:ring-ring sm:grid-cols-[minmax(0,14rem)_minmax(0,1fr)_5rem]"
              >
                <span className="flex min-w-0 flex-col gap-0.5">
                  <span className="truncate text-sm font-medium">{l.name}</span>
                  <span className="w-fit scale-90 origin-left">
                    <StateBadge kind={l.state} />
                  </span>
                </span>
                <DayBars days={l.hours} className="h-6" />
                <span className="text-right font-mono text-[13px] text-muted-foreground">{l.uptime === null ? "–" : `${l.uptime.toFixed(2)}%`}</span>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
