import type { ReactElement } from "react";
import { useOutletContext, useSearchParams } from "react-router-dom";
import { IncidentMonthList } from "@/components/app/IncidentMonths";
import { IncidentList } from "@/components/app/PublicStatus";
import { WindowNav } from "@/components/app/WindowNav";
import { PageHeader } from "@/components/layout/Page";
import { Skeleton } from "@/components/ui/skeleton";
import type { PublicMonths, PublicSite, PublicStatus } from "@/lib/types";
import { useApi } from "@/lib/useApi";

/** The public Incidents tab: what is wrong now, then three months of past incidents at a time.
 * Both use the same incident row. */
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
          <IncidentList incidents={open} />
        </section>
      )}
      <section aria-labelledby="past" className="flex flex-col gap-6">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h2 id="past" className="text-lg font-semibold">
            Past incidents
          </h2>
          {d && <WindowNav months={d.months.map((m) => m.month)} prev={d.prev_to} next={d.next_to} onMove={(t) => setParams({ to: String(t) })} />}
        </div>
        {!d ? <Skeleton className="h-96 rounded-xl" /> : d.months.map((m) => <IncidentMonthList key={m.month} month={m} slug={project.slug} />)}
      </section>
    </div>
  );
}
