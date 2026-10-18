import type { ReactElement } from "react";
import { Link, useNavigate } from "react-router-dom";
import { LinkRow, ROW_LINK } from "@/components/app/LinkRow";
import { DueBar } from "@/components/app/DueBar";
import { ListFilters } from "@/components/app/ListFilters";
import { Overall } from "@/components/app/Overall";
import { TableCard } from "@/components/app/TableCard";
import { SeverityBadge, StateBadge } from "@/components/app/Tone";
import { Plus } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHeader, TableRow } from "@/components/ui/table";
import { countLabel, keeps } from "@/lib/filter";
import { useProject } from "@/lib/project";
import { ago } from "@/lib/time";
import type { HeartbeatList } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { SortHead } from "@/components/list/SortHead";
import { sortRows } from "@/lib/sort";
import { useSort } from "@/lib/useSort";
import { useListFilter } from "@/lib/useListFilter";
import { HeartbeatGuide } from "@/components/app/Guides";
import { HeartbeatFormSheet } from "@/pages/project/HeartbeatForm";

/** A project's heartbeats: one sentence on their state, then a row per job with its deadline. */
export function Heartbeats({ creating = false }: { creating?: boolean }): ReactElement {
  const navigate = useNavigate();
  const { project: p } = useProject();
  const base = `/projects/${p.slug}/heartbeats`;
  const res = useApi<HeartbeatList>(`/projects/${p.slug}/heartbeats`);
  const [filter, setFilter, clear] = useListFilter();
  const filtered = filter.q.trim() !== "" || filter.state !== "" || filter.severity !== "";
  const all = res.data?.heartbeats ?? [];
  const [sort, onSort] = useSort(["name", "state", "deadline", "every", "severity", "last"]);
  const rows = sortRows(
    all.filter((h) => keeps(filter, [h.name], h.state, h.severity)),
    sort,
    { name: (h) => h.name, state: (h) => h.state, deadline: (h) => h.deadline_at, every: (h) => h.period_s, severity: (h) => h.severity, last: (h) => h.last_ping_at },
  );
  const newButton = (
    <Button asChild>
      <Link to={`${base}/new`}>
        <Plus data-icon="inline-start" />
        New heartbeat
      </Link>
    </Button>
  );

  return (
    <>
      <PageHeader
        title="Heartbeats"
        filtered={filtered}
        actions={
          <>
            <HeartbeatGuide />
            {newButton}
          </>
        }
      />
      {!res.data ? (
        <Skeleton className="h-64 rounded-xl" />
      ) : all.length === 0 ? (
        <TableCard title="0 heartbeats">
          <TableEmpty
            title="Watch your first cron job"
            description="A heartbeat tells you when a cron job stops running. Create one and add its URL to the end of the job."
          />
        </TableCard>
      ) : (
        <>
          <Overall summary={res.data.summary} />
          <TableCard title={countLabel(rows.length, all.length, "heartbeat")} onClear={filtered ? clear : undefined} tools={<ListFilters noun="heartbeats" filter={filter} onChange={setFilter} />}>
            {rows.length === 0 ? (
              <TableEmpty title="" filtered noun="heartbeats" onClear={clear} />
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <SortHead by="name" label="Heartbeat" sort={sort} onSort={onSort} />
                    <SortHead by="state" label="State" sort={sort} onSort={onSort} />
                    <SortHead by="deadline" label="Next deadline" sort={sort} onSort={onSort} className="max-lg:hidden" />
                    <SortHead by="every" label="Every" sort={sort} onSort={onSort} className="max-md:hidden" />
                    <SortHead by="severity" label="Severity" sort={sort} onSort={onSort} className="max-md:hidden" />
                    <SortHead by="last" label="Last ping" sort={sort} onSort={onSort} className="text-right max-sm:hidden" />
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {rows.map((h) => (
                    <LinkRow key={h.id} to={`${base}/${h.id}`}>
                      <TableCell className="max-w-44 whitespace-normal sm:max-w-80">
                        <Link to={`${base}/${h.id}`} className={ROW_LINK}>
                          {h.name}
                        </Link>
                        <div className="text-xs text-muted-foreground">{h.grace} grace time</div>
                      </TableCell>
                      <TableCell>
                        <StateBadge kind={h.state} />
                      </TableCell>
                      <TableCell className="max-lg:hidden">
                        <DueBar due={h.due} />
                      </TableCell>
                      <TableCell className="max-md:hidden">{h.every}</TableCell>
                      <TableCell className="max-md:hidden">
                        <SeverityBadge severity={h.severity} />
                      </TableCell>
                      <TableCell className="text-right text-muted-foreground max-sm:hidden">{h.last_ping_at ? ago(h.last_ping_at) : "Never"}</TableCell>
                    </LinkRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </TableCard>
        </>
      )}
      <HeartbeatFormSheet open={creating} onClose={() => navigate(base)} />
    </>
  );
}
