import type { ReactElement } from "react";
import { Link, useOutletContext, useSearchParams } from "react-router-dom";
import { HistoryPager } from "@/components/app/History";
import { IncidentList } from "@/components/app/PublicStatus";
import { PageHeader } from "@/components/layout/Page";
import { Skeleton } from "@/components/ui/skeleton";
import { byMonth } from "@/lib/status";
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
    <div className="flex flex-col">
      <PageHeader
        title={`${project.name} incident history`}
        meta={
          <Link to={`/status/${project.slug}/incidents`} className="text-sm text-link underline-offset-4 hover:underline">
            All incidents
          </Link>
        }
      />
      {!d ? (
        <Skeleton className="h-64 rounded-xl" />
      ) : (
        <div className="flex flex-col gap-6">
          {d.incidents.length === 0 && <p className="text-muted-foreground">This project has had no incidents.</p>}
          {byMonth(d.incidents).map((g) => (
            <section key={g.month} aria-label={g.month} className="flex flex-col gap-3">
              <h2 className="text-[15px] font-semibold">{g.month}</h2>
              <IncidentList incidents={g.incidents} />
            </section>
          ))}
          <div className="rounded-xl border bg-card empty:hidden">
            <HistoryPager noun="incidents" older={d.older ? `?before=${encodeURIComponent(d.older)}` : null} newest={before ? "?" : null} />
          </div>
        </div>
      )}
    </div>
  );
}
