import type { ReactElement } from "react";
import { Link } from "react-router-dom";
import { ListFilters } from "@/components/app/ListFilters";
import { Overall } from "@/components/app/Overall";
import { Sparkline } from "@/components/app/Sparkline";
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
import type { MonitorList } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { useListFilter } from "@/lib/useListFilter";
import { cn } from "@/lib/utils";

/** A project's monitors: one sentence on their state, then a row per monitor with its last day. */
export function Monitors(): ReactElement {
  const { project: p } = useProject();
  const base = `/projects/${p.slug}/monitors`;
  const res = useApi<MonitorList>(`/projects/${p.slug}/monitors`);
  const [filter, setFilter, clear] = useListFilter();
  const all = res.data?.monitors ?? [];
  const rows = all.filter((m) => keeps(filter, [m.name, m.url], m.state, m.severity));
  const newButton = (
    <Button asChild>
      <Link to={`${base}/new`}>
        <Plus data-icon="inline-start" />
        New monitor
      </Link>
    </Button>
  );

  return (
    <>
      <PageHeader title="Monitors" actions={newButton} />
      {!res.data ? (
        <Skeleton className="h-64 rounded-xl" />
      ) : all.length === 0 ? (
        <TableCard title="0 monitors">
          <TableEmpty
            title="Add your first monitor"
            description="A monitor checks a URL on an interval and tells you when it stops answering. Add the health endpoint of your service first."
          />
        </TableCard>
      ) : (
        <>
          <Overall summary={res.data.summary} />
          <TableCard title={countLabel(rows.length, all.length, "monitor")} tools={<ListFilters noun="monitors" filter={filter} onChange={setFilter} />}>
            {rows.length === 0 ? (
              <TableEmpty title="" filtered noun="monitors" onClear={clear} />
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Monitor</TableHead>
                    <TableHead>State</TableHead>
                    <TableHead className="max-lg:hidden">Last 24 hours</TableHead>
                    <TableHead className="max-md:hidden">Every</TableHead>
                    <TableHead className="max-md:hidden">Severity</TableHead>
                    <TableHead className="text-right max-sm:hidden">Last check</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {rows.map((m) => (
                    <TableRow key={m.id}>
                      <TableCell className="max-w-44 whitespace-normal sm:max-w-80">
                        <Link to={`${base}/${m.id}`} className="font-medium hover:underline">
                          {m.name}
                        </Link>
                        <div className="truncate font-mono text-xs text-muted-foreground">{m.url}</div>
                      </TableCell>
                      <TableCell>
                        <StateBadge kind={m.state} />
                      </TableCell>
                      <TableCell className="max-lg:hidden">
                        {m.day ? (
                          <div className="flex items-center gap-3">
                            <Sparkline day={m.day} />
                            <span className="font-mono text-xs text-muted-foreground">{m.day.uptime ?? "–"}</span>
                          </div>
                        ) : (
                          <span className="text-muted-foreground">No checks yet</span>
                        )}
                      </TableCell>
                      <TableCell className="font-mono max-md:hidden">{m.every}</TableCell>
                      <TableCell className="max-md:hidden">
                        <SeverityBadge severity={m.severity} />
                      </TableCell>
                      <TableCell className="max-w-64 text-right max-sm:hidden">
                        {m.last ? (
                          <>
                            <div className={cn("truncate font-mono text-[13px]", !m.last.ok && "text-down")} title={m.last.summary}>
                              {m.last.summary}
                            </div>
                            <div className="text-xs text-muted-foreground">{ago(m.last.checked_at)}</div>
                          </>
                        ) : (
                          <span className="text-muted-foreground">Waiting</span>
                        )}
                      </TableCell>
                    </TableRow>
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
