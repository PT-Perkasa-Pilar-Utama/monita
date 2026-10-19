import { RefreshButton } from "@/components/app/RefreshButton";
import type { ReactElement } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { CopyField } from "@/components/app/CopyField";
import { HeartbeatDetails } from "@/components/app/HeartbeatDetails";
import { HistoryPager } from "@/components/app/History";
import { AvailabilityCard, ChartCard, StatCards } from "@/components/app/Insight";
import { TableCard } from "@/components/app/TableCard";
import { SeverityBadge, StateBadge } from "@/components/app/Tone";
import { Pencil, Terminal } from "@/components/icons";
import { Crumbs, PageHeader } from "@/components/layout/Page";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useProject } from "@/lib/project";
import { stamp } from "@/lib/time";
import type { HeartbeatDetail } from "@/lib/types";
import { useDetail } from "@/lib/useDetail";
import { NotFound } from "@/pages/NotFound";
import { HeartbeatFormSheet } from "@/pages/project/HeartbeatForm";

/** One heartbeat: four numbers led by when the next ping is due, the ping chart, availability
 * and its pings with what each reported. Details holds the ping URL, commands and settings. */
export function Heartbeat({ editing = false }: { editing?: boolean }): ReactElement {
  const navigate = useNavigate();
  const { project: p } = useProject();
  const { id = "" } = useParams();
  const base = `/projects/${p.slug}/heartbeats`;
  const res = useDetail<HeartbeatDetail>(`${base}/${encodeURIComponent(id)}`);
  if (res.error?.status === 404) return <NotFound />;
  const d = res.data;
  const h = d?.heartbeat;
  // Until the job first calls, the ping URL is the one thing to do, so it leads the page.
  const fresh = h?.last === null;

  return (
    <>
      <Crumbs items={[{ label: "Heartbeats", to: base }, { label: h?.name ?? "Heartbeat" }]} />
      {!d || !h ? (
        <Skeleton className="h-96 rounded-xl" />
      ) : (
        <>
          <PageHeader
            title={h.name}
            meta={
              <>
                <StateBadge id="state" kind={h.state} />
                <span>
                  Every {h.every}, can be late by {h.grace}
                </span>
                <SeverityBadge severity={h.severity} />
                {h.is_public && <Badge variant="outline">On the status page</Badge>}
              </>
            }
            actions={
              <>
                <RefreshButton onClick={res.refresh} busy={res.refreshing} />
                <HeartbeatDetails h={h} pingUrl={d.ping_url} />
                <Button asChild variant="secondary">
                  <Link to={`${base}/${h.id}/edit`}>
                    <Pencil data-icon="inline-start" />
                    Edit
                  </Link>
                </Button>
              </>
            }
          />
          <div className="flex flex-col gap-6">
            {fresh && (
              <Card>
                <CardHeader>
                  <CardTitle>The job has not called yet</CardTitle>
                  <CardDescription>Call this URL when the job succeeds. Details has commands to copy and a starter script.</CardDescription>
                </CardHeader>
                <CardContent className="flex flex-col gap-3 sm:flex-row sm:items-center">
                  <div className="min-w-0 flex-1">
                    <CopyField id="ping-url-start" value={d.ping_url} />
                  </div>
                  <HeartbeatDetails
                    h={h}
                    pingUrl={d.ping_url}
                    trigger={
                      <Button variant="outline">
                        <Terminal data-icon="inline-start" />
                        How to call it
                      </Button>
                    }
                  />
                </CardContent>
              </Card>
            )}
            <StatCards stats={d.stats} />
            <ChartCard insight={d.insight} />
            <AvailabilityCard insight={d.insight} />
            <TableCard title="Pings, last 7 days">
              {d.pings.length === 0 ? (
                <TableEmpty title="Waiting for the first ping" description="Call the ping URL when the job next finishes." />
              ) : (
                <>
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Time</TableHead>
                        <TableHead>Result</TableHead>
                        <TableHead className="max-sm:hidden">What the job reported</TableHead>
                        <TableHead className="text-right max-md:hidden">From</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {d.pings.map((r) => (
                        <TableRow key={r.id}>
                          <TableCell className="font-mono text-[13px]">{stamp(r.received_at)}</TableCell>
                          <TableCell>
                            <StateBadge kind={r.kind === "ok" ? "up" : "down"} label={r.kind === "ok" ? "OK" : "Failed"} />
                            {r.kind === "fail" && <div className="mt-1 max-w-48 truncate font-mono text-xs text-muted-foreground sm:hidden">{r.cause ?? "No reason given"}</div>}
                          </TableCell>
                          <TableCell className="max-w-96 truncate font-mono text-[13px] text-muted-foreground max-sm:hidden" title={r.cause ?? undefined}>
                            {r.kind === "ok" ? "Success" : (r.cause ?? "No reason given")}
                          </TableCell>
                          <TableCell className="text-right font-mono text-muted-foreground max-md:hidden">{r.source_ip ?? "–"}</TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                  <HistoryPager noun="pings" older={d.older ? res.older(d.older) : null} newest={res.newest} />
                </>
              )}
            </TableCard>
          </div>
        </>
      )}
      <HeartbeatFormSheet open={editing} id={id} onClose={() => navigate(`${base}/${id}`)} onSaved={res.reload} />
    </>
  );
}
