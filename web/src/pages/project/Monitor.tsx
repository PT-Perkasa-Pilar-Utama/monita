import { RefreshButton } from "@/components/app/RefreshButton";
import { SourceActions } from "@/components/app/SourceActions";
import { StateStamp } from "@/components/app/StateStamp";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Writable } from "@/lib/session";
import { PlacesCard } from "@/components/app/PlacesCard";
import { kindTag } from "@/lib/checks";
import type { ReactElement } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { HistoryPager } from "@/components/app/History";
import { MonitorDetails } from "@/components/app/MonitorDetails";
import { AvailabilityCard, ChartCard, StatCards } from "@/components/app/Insight";
import { TableCard } from "@/components/app/TableCard";
import { SeverityBadge, StateBadge } from "@/components/app/Tone";
import { Pencil } from "@/components/icons";
import { Crumbs, PageHeader } from "@/components/layout/Page";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useProject } from "@/lib/project";
import { stamp } from "@/lib/time";
import type { MonitorDetail } from "@/lib/types";
import { useDetail } from "@/lib/useDetail";
import { NotFound } from "@/pages/NotFound";
import { MonitorFormSheet } from "@/pages/project/MonitorForm";

/** One monitor: four numbers, the response-time chart, availability and its checks. Details
 * holds the request, the pass rule, the failure rule and a curl command. */
export function Monitor({ editing = false }: { editing?: boolean }): ReactElement {
  const navigate = useNavigate();
  const { project: p } = useProject();
  const { id = "" } = useParams();
  const base = `/projects/${p.slug}/monitors`;
  const res = useDetail<MonitorDetail>(`${base}/${encodeURIComponent(id)}`);
  if (res.error?.status === 404) return <NotFound />;
  const d = res.data;
  const m = d?.monitor;

  return (
    <>
      <Crumbs items={[{ label: "Monitors", to: base }, { label: m?.name ?? "Monitor" }]} />
      {!d || !m ? (
        <Skeleton className="h-96 rounded-xl" />
      ) : (
        <>
          <PageHeader
            title={m.name}
            meta={
              <>
                <StateBadge id="state" kind={m.state} />
                {m.kind === "http" ? (
                  <a href={m.url} target="_blank" rel="noreferrer" className="min-w-0 truncate font-mono text-sm text-link hover:underline">
                    {m.url}
                  </a>
                ) : (
                  <span className="min-w-0 truncate font-mono text-sm text-muted-foreground">
                    {kindTag(m.kind)} {m.url}
                  </span>
                )}
                <SeverityBadge severity={m.severity} />
                {m.is_public && <Badge variant="outline">On the status page</Badge>}
              </>
            }
            actions={
              <>
                <RefreshButton onClick={res.refresh} busy={res.refreshing} />
                <MonitorDetails m={m} />
                <SourceActions kind="monitor" path={`${base}/${m.id}`} paused={m.paused_at !== null} onChange={res.reload} />
                <Writable>
                  <Button asChild variant="secondary">
                    <Link to={`${base}/${m.id}/edit`}>
                      <Pencil data-icon="inline-start" />
                      Edit
                    </Link>
                  </Button>
                </Writable>
              </>
            }
          />
          <div className="flex flex-col gap-6">
            <StateStamp kind="monitor" pausedAt={m.paused_at} maintenance={d.maintenance} slug={p.slug} />
            <StatCards stats={d.stats} />
            <ChartCard insight={d.insight} />
            <AvailabilityCard insight={d.insight} />
            {d.places.length > 0 && <PlacesCard places={d.places} intervalS={m.interval_s} quorum={m.quorum} />}
            <TableCard title="Checks, last 7 days">
              {d.checks.length === 0 ? (
                <TableEmpty title="Waiting for the first check" description="The first check runs within one interval of saving." />
              ) : (
                <>
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Time</TableHead>
                        {d.places.length > 0 && <TableHead>Place</TableHead>}
                        <TableHead>Result</TableHead>
                        <TableHead>Status</TableHead>
                        <TableHead className="text-right">Latency</TableHead>
                        <TableHead className="max-md:hidden">Error</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {d.checks.map((c) => (
                        <TableRow key={c.id}>
                          <TableCell className="font-mono text-[13px]">{stamp(c.checked_at)}</TableCell>
                          {d.places.length > 0 && <TableCell>{c.place ?? "This server"}</TableCell>}
                          <TableCell>
                            <StateBadge kind={c.ok ? "up" : "down"} label={c.ok ? "OK" : "Failed"} />
                          </TableCell>
                          <TableCell className="font-mono">{c.status_code ?? "–"}</TableCell>
                          <TableCell className="text-right font-mono">{c.latency_ms === null ? "–" : `${c.latency_ms} ms`}</TableCell>
                          <TableCell className="max-w-96 truncate text-muted-foreground max-md:hidden" title={c.error ?? undefined}>
                            {c.error ?? ""}
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                  <HistoryPager noun="checks" older={d.older ? res.older(d.older) : null} newest={res.newest} />
                </>
              )}
            </TableCard>
          </div>
        </>
      )}
      <MonitorFormSheet open={editing} id={id} onClose={() => navigate(`${base}/${id}`)} onSaved={res.reload} />
    </>
  );
}
