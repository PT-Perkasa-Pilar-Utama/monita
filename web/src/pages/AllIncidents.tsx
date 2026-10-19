import type { ReactElement } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { LinkRow, ROW_LINK } from "@/components/app/LinkRow";
import { ProjectBadge } from "@/components/app/ProjectBadge";
import { RefreshButton } from "@/components/app/RefreshButton";
import { TableCard } from "@/components/app/TableCard";
import { LabelBadge, SeverityBadge } from "@/components/app/Tone";
import { PageHeader } from "@/components/layout/Page";
import { Pager } from "@/components/list/Pager";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { pageParam } from "@/lib/page";
import { duration, nowSecs } from "@/lib/time";
import type { HomeIncident, HomeList } from "@/lib/types";
import { LIVE_MS, useApi } from "@/lib/useApi";

/** Every open incident in the projects you can open, critical first, then newest. */
export function AllIncidents(): ReactElement {
  const [params, setParams] = useSearchParams();
  const page = pageParam(params.get("page"));
  const res = useApi<HomeList<"incidents", HomeIncident>>(`/incidents?page=${page}`, LIVE_MS);
  const d = res.data;
  const now = nowSecs();
  return (
    <>
      <PageHeader title="Incidents" actions={<RefreshButton onClick={res.refresh} busy={res.refreshing} />} />
      <div className="flex flex-col gap-6 pb-4">
        <TableCard title={d ? `${d.total} open ${d.total === 1 ? "incident" : "incidents"}` : "Open incidents"}>
          {!d ? (
            <Skeleton className="m-4 h-40" />
          ) : d.incidents.length === 0 ? (
            <TableEmpty title="No incident is open" description="When a monitor or heartbeat fails, its incident shows here with its project." />
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Incident</TableHead>
                  <TableHead className="max-md:hidden">Project</TableHead>
                  <TableHead>Severity</TableHead>
                  <TableHead className="max-md:hidden">Latest update</TableHead>
                  <TableHead className="text-right">Duration</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {d.incidents.map((i) => {
                  const to = `/projects/${i.project.slug}/incidents/${i.id}`;
                  return (
                    <LinkRow key={i.id} to={to}>
                      <TableCell className="max-w-44 whitespace-normal sm:max-w-96">
                        <Link to={to} className={ROW_LINK}>
                          {i.title}
                        </Link>
                        <div className="font-mono text-xs text-muted-foreground">INC-{i.id}</div>
                      </TableCell>
                      <TableCell className="max-md:hidden">
                        <span className="flex items-center gap-2">
                          <ProjectBadge name={i.project.name} iconUrl={i.project.icon_url} className="size-6 rounded-md text-[11px]" />
                          <span className="truncate">{i.project.name}</span>
                        </span>
                      </TableCell>
                      <TableCell>
                        <SeverityBadge severity={i.severity} />
                      </TableCell>
                      <TableCell className="max-md:hidden">{i.last_label && <LabelBadge label={i.last_label} />}</TableCell>
                      <TableCell className="text-right font-mono">{duration(now - i.started_at)}</TableCell>
                    </LinkRow>
                  );
                })}
              </TableBody>
            </Table>
          )}
        </TableCard>
        {d && <Pager page={page} pageSize={d.page_size} total={d.total} noun="incident" onPage={(p) => setParams({ page: String(p) })} disabled={res.loading} />}
      </div>
    </>
  );
}
