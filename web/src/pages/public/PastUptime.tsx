import type { ReactElement } from "react";
import { Link, useOutletContext, useSearchParams } from "react-router-dom";
import { MonthGrid } from "@/components/app/UptimeCalendar";
import { WindowNav } from "@/components/app/WindowNav";
import { ArrowRight } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import type { PublicCalendar, PublicSite } from "@/lib/types";
import { useApi } from "@/lib/useApi";

/** Past uptime: pick a public system, then three months of days at a time. */
export function PastUptime(): ReactElement {
  const project = useOutletContext<PublicSite>();
  const [params, setParams] = useSearchParams();
  const res = useApi<PublicCalendar>(`/public/${project.slug}/uptime${params.size > 0 ? `?${params.toString()}` : ""}`);
  const d = res.data;
  const key = d?.system ? `${d.system.kind}:${d.system.id}` : "";
  const pick = (v: string): void => {
    const [kind = "", id = ""] = v.split(":");
    const next = new URLSearchParams({ kind, id });
    const to = params.get("to");
    if (to) next.set("to", to);
    setParams(next);
  };
  const move = (to: number): void => {
    const next = new URLSearchParams(params);
    next.set("to", String(to));
    setParams(next);
  };
  return (
    <>
      <PageHeader
        title={`${project.name} past uptime`}
        meta={
          <Link to={`/status/${project.slug}`} className="flex items-center gap-1.5 text-sm text-link underline-offset-4 hover:underline">
            Current status
            <ArrowRight className="size-3.5" />
          </Link>
        }
      />
      {res.error?.status === 404 ? (
        <p className="text-muted-foreground">This system is not on the status page.</p>
      ) : !d ? (
        <Skeleton className="h-96 rounded-xl" />
      ) : !d.system ? (
        <p className="text-muted-foreground">This project has no public systems yet.</p>
      ) : (
        <div className="flex flex-col gap-8 rounded-xl border bg-card p-5 shadow-xs sm:p-8">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <Select value={key} onValueChange={pick}>
              <SelectTrigger aria-label="System" className="w-full sm:w-80">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  {d.systems.map((s) => (
                    <SelectItem key={`${s.kind}:${s.id}`} value={`${s.kind}:${s.id}`}>
                      {s.name}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
            <WindowNav months={d.months.map((m) => m.month)} prev={d.prev_to} next={d.next_to} onMove={move} />
          </div>
          <div className="grid gap-8 sm:grid-cols-2 lg:grid-cols-3 lg:gap-12">
            {d.months.map((m) => (
              <MonthGrid key={m.month} month={m} />
            ))}
          </div>
        </div>
      )}
    </>
  );
}
