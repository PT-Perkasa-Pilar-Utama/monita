import type { ReactElement } from "react";
import { Link, useOutletContext } from "react-router-dom";
import { IncidentCard, IncidentRow, SystemRow, uptimeText } from "@/components/app/PublicStatus";
import { StateBadge } from "@/components/app/Tone";
import { ArrowRight, RefreshCw } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { headline, splitSystems } from "@/lib/status";
import type { PublicProject, PublicStatus } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { cn } from "@/lib/utils";

/** The public status page: one sentence, the systems with 90 days of bars, then incidents. */
export function Status(): ReactElement {
  const project = useOutletContext<PublicProject>();
  const res = useApi<PublicStatus>(`/public/${project.slug}/status`);
  const d = res.data;
  if (!d) return <Skeleton className="h-96 rounded-xl" />;
  const { overall, items, incidents } = d.status;
  const { barred, rest } = splitSystems(items);
  const up = items.filter((i) => i.state === "up").length;

  return (
    <>
      <PageHeader
        title={`${project.name} status`}
        actions={
          <Button variant="secondary" onClick={res.reload} disabled={res.loading}>
            {res.loading ? <Spinner data-icon="inline-start" /> : <RefreshCw data-icon="inline-start" />}
            Refresh
          </Button>
        }
      />
      <p
        className={cn(
          "mb-8 flex flex-wrap items-center justify-between gap-3 rounded-xl border px-6 py-5 text-xl font-semibold",
          overall === "down" && "border-down/30 bg-down/8",
          overall === "issue" && "border-issue/30 bg-issue/8",
          overall === "up" && "border-up/30 bg-up/8",
        )}
      >
        <span>{headline(overall, items)}</span>
        <span className="text-base font-normal text-muted-foreground">
          {up} of {items.length} systems up
        </span>
      </p>
      {/* One column below 1000 px with active incidents first; two columns above, systems left.
          Past incidents run under both, so a busy week never stretches one column. */}
      <div className="grid items-start gap-10 min-[1000px]:grid-cols-[minmax(0,7fr)_minmax(0,5fr)]">
        <section aria-labelledby="active" className="flex flex-col gap-3 min-[1000px]:col-start-2 min-[1000px]:row-start-1">
          <h2 id="active" className="font-semibold">
            Active incidents
          </h2>
          {incidents.length === 0 && <p className="text-muted-foreground">Every system is running normally.</p>}
          {incidents.map((i, n) => (
            <IncidentCard key={n} incident={i} />
          ))}
        </section>
        <section aria-labelledby="systems" className="flex flex-col gap-5 min-[1000px]:col-start-1 min-[1000px]:row-start-1">
          <div className="flex items-baseline justify-between">
            <h2 id="systems" className="font-semibold">
              Systems
            </h2>
            <span className="text-sm text-muted-foreground">Last 90 days</span>
          </div>
          {items.length === 0 && <p className="text-muted-foreground">This project has no public systems yet.</p>}
          {barred.map((i) => (
            <SystemRow key={`${i.kind}${i.name}`} item={i} />
          ))}
          {rest.length > 0 && (
            <details className="rounded-xl border bg-card">
              <summary className="cursor-pointer px-5 py-3 text-sm font-medium">{rest.length} more systems</summary>
              <ul className="flex flex-col border-t">
                {rest.map((i) => (
                  <li key={`${i.kind}${i.name}`} className="flex items-center justify-between gap-3 border-b px-5 py-2.5 text-sm last:border-0">
                    <span className="flex items-center gap-2">
                      {i.state !== "up" && <StateBadge kind={i.state} />}
                      {i.name}
                    </span>
                    <span className="text-muted-foreground">{uptimeText(i)}</span>
                  </li>
                ))}
              </ul>
            </details>
          )}
        </section>
        <section aria-labelledby="past" className="flex flex-col gap-3 min-[1000px]:col-span-2">
          <h2 id="past" className="font-semibold">
            Past 7 days
          </h2>
          {d.recent.length === 0 ? (
            <p className="text-muted-foreground">There were no incidents in the last 7 days.</p>
          ) : (
            <div className="overflow-hidden rounded-xl border bg-card shadow-xs">
              {d.recent.map((i, n) => (
                <IncidentRow key={n} incident={i} />
              ))}
            </div>
          )}
          <Link to={`/status/${project.slug}/history`} className="flex items-center gap-1.5 text-sm text-link underline-offset-4 hover:underline">
            Incident history
            <ArrowRight className="size-3.5" />
          </Link>
        </section>
      </div>
    </>
  );
}
