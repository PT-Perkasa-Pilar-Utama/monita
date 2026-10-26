import { RefreshButton } from "@/components/app/RefreshButton";
import { BatchBar, DeleteSourcesDialog, SourceRowMenu } from "@/components/app/SourceBatch";
import { rowTape } from "@/components/app/StateStamp";
import { Writable, useCanEdit } from "@/lib/session";
import { useState } from "react";
import type { ReactElement } from "react";
import { Link, useNavigate } from "react-router-dom";
import { LinkRow, ROW_LINK } from "@/components/app/LinkRow";
import { ListFilters } from "@/components/app/ListFilters";
import { Overall } from "@/components/app/Overall";
import { Sparkline } from "@/components/app/Sparkline";
import { TableCard } from "@/components/app/TableCard";
import { SeverityBadge, StateBadge } from "@/components/app/Tone";
import { Plus } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { countLabel, keeps } from "@/lib/filter";
import { useProject } from "@/lib/project";
import { ago } from "@/lib/time";
import type { MonitorList } from "@/lib/types";
import { LIVE_MS, useApi } from "@/lib/useApi";
import { SortHead } from "@/components/list/SortHead";
import { sortRows } from "@/lib/sort";
import { useSort } from "@/lib/useSort";
import { useListFilter } from "@/lib/useListFilter";
import { cn } from "@/lib/utils";
import { MonitorGuide } from "@/components/app/Guides";
import { MonitorFormSheet } from "@/pages/project/MonitorForm";
import type { Picked } from "@/lib/sources";
import { useSelection } from "@/lib/useSelection";
import { PinButton } from "@/components/app/PinButton";
import { pinnedFirst } from "@/lib/pins";

/** A project's monitors: one sentence on their state, then a row per monitor with its last day. */
export function Monitors({ creating = false }: { creating?: boolean }): ReactElement {
  const navigate = useNavigate();
  const { project: p } = useProject();
  const base = `/projects/${p.slug}/monitors`;
  const res = useApi<MonitorList>(`/projects/${p.slug}/monitors`, LIVE_MS);
  const [filter, setFilter, clear] = useListFilter();
  const filtered = filter.q.trim() !== "" || filter.state !== "" || filter.severity !== "";
  const all = res.data?.monitors ?? [];
  const [sort, onSort] = useSort(["name", "state", "every", "severity", "last"]);
  const rows = pinnedFirst(sortRows(
    all.filter((m) => keeps(filter, [m.name, m.url], m.state, m.severity)),
    sort,
    { name: (m) => m.name, state: (m) => m.state, every: (m) => m.interval_s, severity: (m) => m.severity, last: (m) => m.last?.checked_at },
  ));
  const canEdit = useCanEdit();
  const { selected, toggle, toggleShown, headerBox, clear: clearPicked } = useSelection(rows);
  const [deleting, setDeleting] = useState<Picked[] | null>(null);
  const pick = (m: (typeof all)[number]): Picked => ({ id: m.id, name: m.name, state: m.state, paused: m.paused_at !== null });
  const chosen = all.filter((m) => selected.has(m.id)).map(pick);
  const newButton = (
    <Writable>
      <Button asChild>
        <Link to={`${base}/new`}>
          <Plus data-icon="inline-start" />
          New monitor
        </Link>
      </Button>
    </Writable>
  );

  return (
    <>
      <PageHeader
        title="Monitors"
        filtered={filtered}
        actions={
          <>
            <RefreshButton onClick={res.refresh} busy={res.refreshing} />
            <MonitorGuide />
            {newButton}
          </>
        }
      />
      {!res.data ? (
        <Skeleton className="h-64 rounded-xl" />
      ) : all.length === 0 ? (
        <TableCard title="0 monitors">
          <TableEmpty
            title="Add your first monitor"
            description="A monitor checks a URL on an interval and tells you when it stops answering, so add the health endpoint of your service first."
          />
        </TableCard>
      ) : (
        <>
          <Overall summary={res.data.summary} />
          <TableCard
            title={chosen.length > 0 ? <BatchBar slug={p.slug} kind="monitor" picked={chosen} onClear={clearPicked} onDone={res.refresh} onDelete={setDeleting} /> : countLabel(rows.length, all.length, "monitor")}
            onClear={filtered && chosen.length === 0 ? clear : undefined}
            tools={<ListFilters noun="monitors" filter={filter} onChange={setFilter} />}
          >
            {rows.length === 0 ? (
              <TableEmpty title="" filtered noun="monitors" onClear={clear} />
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    {canEdit && (
                      <TableHead className="w-10">
                        <Checkbox aria-label="Select every monitor shown" checked={headerBox} onCheckedChange={(on) => toggleShown(on === true)} />
                      </TableHead>
                    )}
                    <SortHead by="name" label="Monitor" sort={sort} onSort={onSort} />
                    <SortHead by="state" label="State" sort={sort} onSort={onSort} />
                    <TableHead className="max-lg:hidden">Last 24 hours</TableHead>
                    <SortHead by="every" label="Every" sort={sort} onSort={onSort} className="max-md:hidden" />
                    <SortHead by="severity" label="Alert level" sort={sort} onSort={onSort} className="max-md:hidden" />
                    <SortHead by="last" label="Last check" sort={sort} onSort={onSort} className="text-right max-sm:hidden" />
                    {canEdit && <TableHead className="w-10" aria-label="Actions" />}
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {rows.map((m) => (
                    <LinkRow key={m.id} to={`${base}/${m.id}`} className={selected.has(m.id) ? "bg-accent/60" : ""} style={rowTape(m.state)}>
                      {canEdit && (
                        <TableCell>
                          <Checkbox aria-label={`Select ${m.name}`} checked={selected.has(m.id)} onCheckedChange={() => toggle(m.id)} />
                        </TableCell>
                      )}
                      <TableCell className="max-w-44 whitespace-normal sm:max-w-80">
                        <div className="flex items-start gap-1">
                          <PinButton path={`${base}/${m.id}`} name={m.name} pinned={m.pinned} onChange={res.reload} className="-my-1 -ml-1.5" />
                          <div className="min-w-0">
                            <Link to={`${base}/${m.id}`} className={ROW_LINK}>
                              {m.name}
                            </Link>
                            <div className="truncate font-mono text-xs text-muted-foreground">{m.url}</div>
                          </div>
                        </div>
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
                      <TableCell className="max-md:hidden">{m.every}</TableCell>
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
                          <span className="text-muted-foreground">Not checked yet</span>
                        )}
                      </TableCell>
                      {canEdit && (
                        <TableCell className="text-right">
                          <SourceRowMenu slug={p.slug} kind="monitor" base={base} item={pick(m)} onDone={res.refresh} onDelete={setDeleting} />
                        </TableCell>
                      )}
                    </LinkRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </TableCard>
        </>
      )}
      <MonitorFormSheet open={creating} onClose={() => navigate(base)} />
      <DeleteSourcesDialog
        slug={p.slug}
        kind="monitor"
        items={deleting}
        onClose={() => setDeleting(null)}
        onDone={() => {
          clearPicked();
          res.refresh();
        }}
      />
    </>
  );
}
