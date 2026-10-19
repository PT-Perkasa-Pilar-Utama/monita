import type { ReactElement } from "react";
import { Writable } from "@/lib/session";
import { Link, useNavigate } from "react-router-dom";
import { LinkRow, ROW_LINK } from "@/components/app/LinkRow";
import { ListFilters, type Option } from "@/components/app/ListFilters";
import { TableCard } from "@/components/app/TableCard";
import { LabelBadge, SeverityBadge } from "@/components/app/Tone";
import { Plus } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHeader, TableRow } from "@/components/ui/table";
import { countLabel, keeps } from "@/lib/filter";
import { useProject } from "@/lib/project";
import { duration, nowSecs, stamp } from "@/lib/time";
import type { IncidentRow } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { SortHead } from "@/components/list/SortHead";
import { sortRows } from "@/lib/sort";
import { useSort } from "@/lib/useSort";
import { useListFilter } from "@/lib/useListFilter";
import { IncidentFormSheet } from "@/pages/project/IncidentForm";
import { IncidentGuide } from "@/components/app/Guides";

// The State select picks which list the server sends: open, resolved or all incidents.
const STATES: Option[] = [
  { value: "", label: "Open" },
  { value: "resolved", label: "Resolved" },
  { value: "all", label: "All" },
];
const NOUN: Record<string, string> = { "": "open incident", resolved: "resolved incident", all: "incident" };

/** A project's incidents, open by default, searchable by title or INC id. */
export function Incidents({ creating = false }: { creating?: boolean }): ReactElement {
  const navigate = useNavigate();
  const { project: p } = useProject();
  const base = `/projects/${p.slug}/incidents`;
  const [filter, setFilter, clear] = useListFilter();
  const resolved = filter.state === "resolved";
  const res = useApi<{ incidents: IncidentRow[] }>(`${base}${filter.state ? `?state=${filter.state}` : ""}`);
  const all = res.data?.incidents ?? [];
  const [sort, onSort] = useSort(["title", "severity", "update", "started", "duration"]);
  const now = nowSecs();
  const rows = sortRows(
    all.filter((i) => keeps(filter, [i.title, `INC-${i.id}`], filter.state, i.severity)),
    sort,
    { title: (i) => i.title, severity: (i) => i.severity, update: (i) => i.last_label, started: (i) => i.started_at, duration: (i) => (i.resolved_at ?? now) - i.started_at },
  );
  const searching = filter.q.trim() !== "" || filter.severity !== "";
  // Open is the default list; Resolved or All counts as a filter, so Clear filters goes back to Open.
  const filtered = searching || filter.state !== "";

  return (
    <>
      <PageHeader
        title="Incidents"
        filtered={filtered}
        actions={
          <>
            <IncidentGuide />
            <Writable>
              <Button asChild>
                <Link to={`${base}/new`}>
                  <Plus data-icon="inline-start" />
                  Report incident
                </Link>
              </Button>
            </Writable>
          </>
        }
      />
      <TableCard
        title={res.data ? countLabel(rows.length, all.length, NOUN[filter.state] ?? "incident") : "Incidents"}
        onClear={filtered ? clear : undefined}
        tools={<ListFilters noun="incidents" filter={filter} onChange={setFilter} stateOptions={STATES} />}
      >
        {!res.data ? (
          <Skeleton className="m-5 h-24" />
        ) : rows.length === 0 && searching && all.length > 0 ? (
          <TableEmpty title="" filtered noun="incidents" onClear={clear} />
        ) : rows.length === 0 ? (
          resolved ? (
            <TableEmpty title="Resolved incidents appear here" description="Each one keeps its timeline of updates." />
          ) : (
            <TableEmpty
              title="Every system is up"
              description="Monita opens an incident when a monitor fails the checks in a row you set or a heartbeat misses. You can also report one by hand."
            />
          )
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <SortHead by="title" label="Incident" sort={sort} onSort={onSort} />
                <SortHead by="severity" label="Severity" sort={sort} onSort={onSort} />
                <SortHead by="update" label="Latest update" sort={sort} onSort={onSort} className="max-md:hidden" />
                <SortHead by="started" label="Started" sort={sort} onSort={onSort} className="max-lg:hidden" />
                <SortHead by="duration" label="Duration" sort={sort} onSort={onSort} className="text-right" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map((i) => (
                <LinkRow key={i.id} to={`${base}/${i.id}`}>
                  <TableCell className="max-w-44 whitespace-normal sm:max-w-96">
                    <Link to={`${base}/${i.id}`} className={ROW_LINK}>
                      {i.title}
                    </Link>
                    <div className="font-mono text-xs text-muted-foreground">
                      INC-{i.id} {i.origin === "auto" ? "detected by Monita" : "reported by hand"}
                    </div>
                  </TableCell>
                  <TableCell>
                    <SeverityBadge severity={i.severity} />
                  </TableCell>
                  <TableCell className="max-md:hidden">{i.last_label && <LabelBadge label={i.last_label} />}</TableCell>
                  <TableCell className="font-mono text-[13px] max-lg:hidden">{stamp(i.started_at)}</TableCell>
                  <TableCell className="text-right font-mono">{duration((i.resolved_at ?? now) - i.started_at)}</TableCell>
                </LinkRow>
              ))}
            </TableBody>
          </Table>
        )}
      </TableCard>
      <IncidentFormSheet open={creating} onClose={() => navigate(base)} />
    </>
  );
}
