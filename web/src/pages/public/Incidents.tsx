import type { ReactElement } from "react";
import { Link, useOutletContext } from "react-router-dom";
import { IncidentCard, IncidentRow } from "@/components/app/PublicStatus";
import { ArrowRight, RefreshCw } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import type { PublicSite, PublicStatus } from "@/lib/types";
import { useApi } from "@/lib/useApi";

/** The public Incidents tab: what is wrong now, the last 7 days, and a link to the history. */
export function Incidents(): ReactElement {
  const project = useOutletContext<PublicSite>();
  const res = useApi<PublicStatus>(`/public/${project.slug}/status`);
  const d = res.data;
  return (
    <div className="mx-auto flex max-w-3xl flex-col">
      <PageHeader
        title={`${project.name} incidents`}
        actions={
          <Button variant="secondary" onClick={res.reload} disabled={res.loading}>
            {res.loading ? <Spinner data-icon="inline-start" /> : <RefreshCw data-icon="inline-start" />}
            Refresh
          </Button>
        }
      />
      {!d ? (
        <Skeleton className="h-96 rounded-xl" />
      ) : (
        <div className="flex flex-col gap-10">
          <section aria-labelledby="active" className="flex flex-col gap-3">
            <h2 id="active" className="font-semibold">
              Active incidents
            </h2>
            {d.status.incidents.length === 0 && <p className="text-muted-foreground">Every system is running normally.</p>}
            {d.status.incidents.map((i, n) => (
              <IncidentCard key={n} incident={i} />
            ))}
          </section>
          <section aria-labelledby="past" className="flex flex-col gap-3">
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
      )}
    </div>
  );
}
