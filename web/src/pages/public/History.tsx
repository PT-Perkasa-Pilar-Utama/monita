import type { ReactElement } from "react";
import { Link, useOutletContext, useSearchParams } from "react-router-dom";
import { HistoryPager } from "@/components/app/History";
import { IncidentCard } from "@/components/app/PublicStatus";
import { PageHeader } from "@/components/layout/Page";
import { Skeleton } from "@/components/ui/skeleton";
import { month } from "@/lib/status";
import type { PublicHistory, PublicProject } from "@/lib/types";
import { useApi } from "@/lib/useApi";

/** Every past public incident, newest first, by month, 25 per page. */
export function History(): ReactElement {
  const project = useOutletContext<PublicProject>();
  const [params] = useSearchParams();
  const before = params.get("before");
  const res = useApi<PublicHistory>(`/public/${project.slug}/history${before ? `?before=${encodeURIComponent(before)}` : ""}`);
  const d = res.data;
  return (
    <div className="mx-auto flex max-w-3xl flex-col">
      <PageHeader
        title={`${project.name} incident history`}
        meta={
          <Link to={`/status/${project.slug}`} className="text-sm text-link underline-offset-4 hover:underline">
            Current status
          </Link>
        }
      />
      {!d ? (
        <Skeleton className="h-64 rounded-xl" />
      ) : (
        <section aria-label="Incidents" className="flex flex-col gap-3">
          {d.incidents.length === 0 && <p className="text-muted-foreground">This project has had no incidents.</p>}
          {d.incidents.map((i, n) => {
            const m = month(i.started_at);
            const first = n === 0 || month(d.incidents[n - 1]!.started_at) !== m;
            return (
              <div key={n} className="flex flex-col gap-3">
                {first && <h2 className="mt-4 font-semibold first:mt-0">{m}</h2>}
                <IncidentCard incident={i} />
              </div>
            );
          })}
          <div className="rounded-xl border bg-card empty:hidden">
            <HistoryPager noun="incidents" older={d.older ? `?before=${encodeURIComponent(d.older)}` : null} newest={before ? "?" : null} />
          </div>
        </section>
      )}
    </div>
  );
}
