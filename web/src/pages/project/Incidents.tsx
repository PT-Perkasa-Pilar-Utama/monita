import type { ReactElement } from "react";
import { Link } from "react-router-dom";
import { LinkRow, ROW_LINK } from "@/components/app/LinkRow";
import { ListFilters, type Option } from "@/components/app/ListFilters";
import { TableCard } from "@/components/app/TableCard";
import { LabelBadge, SeverityBadge } from "@/components/app/Tone";
import { Plus } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { countLabel, keeps } from "@/lib/filter";
import { useProject } from "@/lib/project";
import { duration, nowSecs, utc } from "@/lib/time";
import type { IncidentRow } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { useListFilter } from "@/lib/useListFilter";

// The State select picks which list the server sends: open incidents, or resolved ones.
const STATES: Option[] = [
  { value: "", label: "Open" },
  { value: "resolved", label: "Resolved" },
];

/** A project's open or resolved incidents, searchable by title or INC id. */
export function Incidents(): ReactElement {
  const { project: p } = useProject();
  const base = `/projects/${p.slug}/incidents`;
  const [filter, setFilter, clear] = useListFilter();
  const resolved = filter.state === "resolved";
  const res = useApi<{ incidents: IncidentRow[] }>(`${base}${resolved ? "?state=resolved" : ""}`);
  const all = res.data?.incidents ?? [];
  const rows = all.filter((i) => keeps(filter, [i.title, `INC-${i.id}`], filter.state, i.severity));
  const now = nowSecs();
  const searching = filter.q.trim() !== "" || filter.severity !== "";

  return (
    <>
      <PageHeader
        title="Incidents"
        actions={
          <Button asChild>
            <Link to={`${base}/new`}>
              <Plus data-icon="inline-start" />
              Report incident
            </Link>
          </Button>
        }
      />
      <TableCard
        title={res.data ? countLabel(rows.length, all.length, resolved ? "resolved incident" : "open incident") : "Incidents"}
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
              description="Monita opens an incident when a monitor fails twice in a row or a heartbeat misses. You can also report one by hand."
            />
          )
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Incident</TableHead>
                <TableHead>Severity</TableHead>
                <TableHead className="max-md:hidden">Latest update</TableHead>
                <TableHead className="max-lg:hidden">Started</TableHead>
                <TableHead className="text-right">Duration</TableHead>
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
                  <TableCell className="font-mono text-[13px] max-lg:hidden">{utc(i.started_at)}</TableCell>
                  <TableCell className="text-right font-mono">{duration((i.resolved_at ?? now) - i.started_at)}</TableCell>
                </LinkRow>
              ))}
            </TableBody>
          </Table>
        )}
      </TableCard>
    </>
  );
}
