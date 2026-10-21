import type { ReactElement } from "react";
import { Link, useLocation } from "react-router-dom";
import { LinkRow, ROW_LINK } from "@/components/app/LinkRow";
import { ListFilters, type Option } from "@/components/app/ListFilters";
import { ProjectBadge } from "@/components/app/ProjectBadge";
import { RefreshButton } from "@/components/app/RefreshButton";
import { TableCard } from "@/components/app/TableCard";
import { LabelBadge, SeverityBadge } from "@/components/app/Tone";
import { PageHeader } from "@/components/layout/Page";
import { Pager } from "@/components/list/Pager";
import { SortHead } from "@/components/list/SortHead";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { dateHead, nextDate } from "@/lib/dateSort";
import { duration, nowSecs, stamp } from "@/lib/time";
import type { HomeIncident, HomeList } from "@/lib/types";
import { LIVE_MS, useApi } from "@/lib/useApi";
import { useServerList } from "@/lib/useServerList";

const STATES: Option[] = [
  { value: "", label: "Open" },
  { value: "resolved", label: "Resolved" },
  { value: "all", label: "Open and resolved" },
];

/** The incidents of every project you can open, open ones by default, critical first. */
export function AllIncidents(): ReactElement {
  const list = useServerList("monita:incidents:query");
  const { params } = list;
  const res = useApi<HomeList<"incidents", HomeIncident>>(`/incidents${list.query}`, LIVE_MS);
  const { pathname, search } = useLocation();
  const d = res.data;
  const now = nowSecs();
  const noun = params.state === "resolved" ? "resolved incident" : params.state === "all" ? "incident" : "open incident";
  const sort = params.sort === "" ? { by: "severity", dir: "desc" as const } : dateHead("started", params.sort, false);
  const onSort = (by: string): void => list.set({ sort: by === "severity" ? "" : nextDate(params.sort, false) });
  return (
    <>
      <PageHeader title="Incidents" filtered={list.filtered} actions={<RefreshButton onClick={res.refresh} busy={res.refreshing} />} />
      <div className="flex flex-col gap-6 pb-4">
        <TableCard
          title={d ? `${d.total} ${d.total === 1 ? noun : `${noun}s`}` : "Incidents"}
          onClear={list.filtered ? list.clear : undefined}
          tools={
            <ListFilters
              noun="incidents"
              filter={{ q: list.draft, state: params.state, severity: params.severity }}
              onChange={(p) => (p.q !== undefined ? list.setDraft(p.q) : list.set(p))}
              stateOptions={STATES}
            />
          }
        >
          {!d ? (
            <Skeleton className="m-5 h-24" />
          ) : d.incidents.length === 0 && list.filtered ? (
            <TableEmpty title="" filtered noun="incidents" onClear={list.clear} />
          ) : d.incidents.length === 0 ? (
            <TableEmpty title="Every system is up" description="When a monitor or heartbeat fails in any of your projects, its incident shows here." />
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Incident</TableHead>
                  <TableHead className="max-md:hidden">Project</TableHead>
                  <SortHead by="severity" label="Alert level" sort={sort} onSort={onSort} />
                  <TableHead className="max-lg:hidden">Latest update</TableHead>
                  <SortHead by="started" label="Started" sort={sort} onSort={onSort} className="max-lg:hidden" />
                  <TableHead className="text-right">Duration</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {d.incidents.map((i) => {
                  const to = `/projects/${i.project.slug}/incidents/${i.id}`;
                  // The incident page links back here, to the same search and page.
                  const back = { from: `${pathname}${search}` };
                  return (
                    <LinkRow key={i.id} to={to} state={back}>
                      <TableCell className="max-w-44 whitespace-normal sm:max-w-96">
                        <Link to={to} state={back} className={ROW_LINK}>
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
                      <TableCell className="max-lg:hidden">{i.last_label && <LabelBadge label={i.last_label} />}</TableCell>
                      <TableCell className="font-mono text-[13px] max-lg:hidden">{stamp(i.started_at)}</TableCell>
                      <TableCell className="text-right font-mono">{duration((i.resolved_at ?? now) - i.started_at)}</TableCell>
                    </LinkRow>
                  );
                })}
              </TableBody>
            </Table>
          )}
        </TableCard>
        {d && <Pager page={params.page} pageSize={d.page_size} total={d.total} noun="incident" onPage={(page) => list.set({ page })} disabled={res.loading} />}
      </div>
    </>
  );
}
