import type { ReactElement } from "react";
import { useOutletContext, useSearchParams } from "react-router-dom";
import { IncidentMonthList } from "@/components/app/IncidentMonths";
import { IncidentCard } from "@/components/app/PublicStatus";
import { WindowNav } from "@/components/app/WindowNav";
import { PageHeader } from "@/components/layout/Page";
import { Skeleton } from "@/components/ui/skeleton";
import type { PublicMonths, PublicSite, PublicStatus } from "@/lib/types";
import { useApi } from "@/lib/useApi";

/** The public Incidents tab: what is wrong now, then three months of incidents at a time. */
export function Incidents(): ReactElement {
  const project = useOutletContext<PublicSite>();
  const [params, setParams] = useSearchParams();
  const to = params.get("to");
  const now = useApi<PublicStatus>(`/public/${project.slug}/status`);
  const res = useApi<PublicMonths>(`/public/${project.slug}/incidents${to ? `?to=${encodeURIComponent(to)}` : ""}`);
  const open = now.data?.status.incidents ?? [];
  const d = res.data;
  return (
    <div className="flex flex-col">
      <PageHeader title={`${project.name} incidents`} />
      {open.length > 0 && (
        <section aria-labelledby="active" className="mb-10 flex flex-col gap-3">
          <h2 id="active" className="text-lg font-semibold">
            Happening now
          </h2>
          {open.map((i, n) => (
            <IncidentCard key={n} incident={i} />
          ))}
        </section>
      )}
      {!d ? (
        <Skeleton className="h-96 rounded-xl" />
      ) : (
        <div className="flex flex-col gap-10 rounded-xl border bg-card p-5 shadow-xs sm:p-8">
          <WindowNav months={d.months.map((m) => m.month)} prev={d.prev_to} next={d.next_to} onMove={(t) => setParams({ to: String(t) })} />
          {d.months.map((m) => (
            <IncidentMonthList key={m.month} month={m} slug={project.slug} />
          ))}
        </div>
      )}
    </div>
  );
}
