import { useState } from "react";
import type { ReactElement } from "react";
import { Link } from "react-router-dom";
import { IncidentList } from "@/components/app/PublicStatus";
import { ArrowRight } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { monthName, resolvedOnly } from "@/lib/status";
import type { IncidentMonth } from "@/lib/types";

/** Rows a month shows before "Show all". */
const FIRST = 3;

/** A month heading and its resolved incidents, newest first; open ones sit in Happening now.
 * Three show; the rest open on request, and a month past one page links to the history. */
export function IncidentMonthList({ month, slug }: { month: IncidentMonth; slug: string }): ReactElement {
  const [all, setAll] = useState(false);
  const past = resolvedOnly(month.incidents);
  const shown = all ? past : past.slice(0, FIRST);
  const hidden = past.length - shown.length;
  const more =
    hidden > 0 ? (
      <Button variant="ghost" className="h-11 w-full rounded-none" onClick={() => setAll(true)}>
        Show all {past.length} incidents
      </Button>
    ) : all && month.more ? (
      <Link to={`/status/${slug}/history?before=${encodeURIComponent(month.more)}`} className="flex h-11 items-center justify-center gap-1.5 text-sm text-link underline-offset-4 hover:underline">
        Older incidents of {monthName(month.month)}
        <ArrowRight className="size-3.5" />
      </Link>
    ) : undefined;
  return (
    <section aria-labelledby={`m-${month.month}`} className="flex flex-col gap-3">
      <h3 id={`m-${month.month}`} className="text-[15px] font-semibold">
        {monthName(month.month)}
      </h3>
      {past.length === 0 ? (
        <p className="text-muted-foreground">{month.incidents.length === 0 ? "No incidents reported this month." : "No resolved incidents this month."}</p>
      ) : (
        <IncidentList incidents={shown} footer={more} />
      )}
    </section>
  );
}
