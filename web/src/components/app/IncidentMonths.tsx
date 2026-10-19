import { useState } from "react";
import type { ReactElement } from "react";
import { Link } from "react-router-dom";
import { Updates } from "@/components/app/PublicStatus";
import { AlertCircle, ArrowRight } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { monthName, span } from "@/lib/status";
import type { IncidentMonth, PublicIncident } from "@/lib/types";
import { cn } from "@/lib/utils";

/** Rows a month shows before "Show all". */
const FIRST = 3;

/** One incident on the timeline: the title in its severity's colour, the newest update, and when.
 * It opens to every update. */
function TimelineRow({ incident: i }: { incident: PublicIncident }): ReactElement {
  const latest = i.updates[0];
  return (
    <li className="group/row relative flex gap-4 pb-6 last:pb-0">
      <span aria-hidden className="absolute top-9 bottom-0 left-[17px] w-px bg-border group-last/row:hidden" />
      <span className="relative flex size-9 shrink-0 items-center justify-center rounded-full border bg-muted text-muted-foreground">
        <AlertCircle className="size-4" />
      </span>
      <details className="group min-w-0 flex-1 border-b pb-6 group-last/row:border-0 group-last/row:pb-0">
        <summary className="flex cursor-pointer list-none flex-col gap-1.5 [&::-webkit-details-marker]:hidden">
          <strong className={cn("text-[17px] leading-snug font-semibold group-hover:underline", i.severity === "critical" ? "text-down" : "text-issue")}>{i.title}</strong>
          {latest && <span className="line-clamp-3 whitespace-pre-wrap break-words">{latest.message}</span>}
          <span className="text-sm text-muted-foreground">{span(i)}</span>
        </summary>
        <div className="mt-4 flex flex-col gap-4">
          <Updates incident={i} />
        </div>
      </details>
    </li>
  );
}

/** A month heading and its incidents, newest first. Three show; the rest open on request, and a
 * month past one page links to the history. */
export function IncidentMonthList({ month, slug }: { month: IncidentMonth; slug: string }): ReactElement {
  const [all, setAll] = useState(false);
  const shown = all ? month.incidents : month.incidents.slice(0, FIRST);
  const hidden = month.incidents.length - shown.length;
  return (
    <section aria-labelledby={`m-${month.month}`} className="flex flex-col gap-5">
      <h2 id={`m-${month.month}`} className="border-b pb-3 text-2xl font-semibold tracking-tight">
        {monthName(month.month)}
      </h2>
      {month.incidents.length === 0 ? (
        <p className="text-muted-foreground">No incidents reported this month.</p>
      ) : (
        <ol className="flex flex-col">
          {shown.map((i, n) => (
            <TimelineRow key={n} incident={i} />
          ))}
        </ol>
      )}
      {hidden > 0 && (
        <Button variant="outline" className="w-full" onClick={() => setAll(true)}>
          Show all {month.incidents.length} incidents
        </Button>
      )}
      {all && month.more && (
        <Link to={`/status/${slug}/history?before=${encodeURIComponent(month.more)}`} className="flex items-center gap-1.5 self-start text-sm text-link underline-offset-4 hover:underline">
          Older incidents of {monthName(month.month)}
          <ArrowRight className="size-3.5" />
        </Link>
      )}
    </section>
  );
}
