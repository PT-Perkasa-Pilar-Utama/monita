import type { ReactElement } from "react";
import { Link } from "react-router-dom";
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
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { countLabel, keeps } from "@/lib/filter";
import { useProject } from "@/lib/project";
import { ago } from "@/lib/time";
import type { HeartbeatList } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { useListFilter } from "@/lib/useListFilter";

/** A project's heartbeats: one sentence on their state, then a row per job with its deadline. */
export function Heartbeats(): ReactElement {
  const { project: p } = useProject();
  const base = `/projects/${p.slug}/heartbeats`;
  const res = useApi<HeartbeatList>(`/projects/${p.slug}/heartbeats`);
  const [filter, setFilter, clear] = useListFilter();
  const all = res.data?.heartbeats ?? [];
  const rows = all.filter((h) => keeps(filter, [h.name], h.state, h.severity));
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
      <PageHeader title="Heartbeats" actions={newButton} />
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
          <TableCard title={countLabel(rows.length, all.length, "heartbeat")} tools={<ListFilters noun="heartbeats" filter={filter} onChange={setFilter} />}>
            {rows.length === 0 ? (
              <TableEmpty title="" filtered noun="heartbeats" onClear={clear} />
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Heartbeat</TableHead>
                    <TableHead>State</TableHead>
                    <TableHead className="max-lg:hidden">Next deadline</TableHead>
                    <TableHead className="max-md:hidden">Every</TableHead>
                    <TableHead className="max-md:hidden">Severity</TableHead>
                    <TableHead className="text-right max-sm:hidden">Last ping</TableHead>
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
    </>
  );
}
