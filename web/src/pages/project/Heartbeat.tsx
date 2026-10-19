import type { ReactElement } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { CopyField } from "@/components/app/CopyField";
import { Facts } from "@/components/app/Facts";
import { HistoryPager } from "@/components/app/History";
import { AvailabilityCard, ChartCard, StatCards } from "@/components/app/Insight";
import { TableCard } from "@/components/app/TableCard";
import { SeverityBadge, StateBadge } from "@/components/app/Tone";
import { Pencil } from "@/components/icons";
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

/** One heartbeat: its ping URL, four numbers, the ping chart, availability and its pings. */
export function Heartbeat({ editing = false }: { editing?: boolean }): ReactElement {
  const navigate = useNavigate();
  const { project: p } = useProject();
  const { id = "" } = useParams();
  const base = `/projects/${p.slug}/heartbeats`;
  const res = useDetail<HeartbeatDetail>(`${base}/${encodeURIComponent(id)}`);
  if (res.error?.status === 404) return <NotFound />;
  const d = res.data;
  const h = d?.heartbeat;

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
                  Every {h.every}, {h.grace} grace time
                </span>
                <SeverityBadge severity={h.severity} />
                {h.is_public && <Badge variant="outline">On the status page</Badge>}
              </>
            }
            actions={
              <Button asChild variant="secondary">
                <Link to={`${base}/${h.id}/edit`}>
                  <Pencil data-icon="inline-start" />
                  Edit
                </Link>
              </Button>
            }
          />
          <div className="flex flex-col gap-6">
            <Card>
              <CardHeader>
                <CardTitle className="text-[15px]">Ping URL</CardTitle>
                <CardDescription>
                  Call it with GET, HEAD or POST when the job succeeds. Add /fail to report a failure; the first 1 KB of the body becomes the cause.{" "}
                  {h.due.late || !h.deadline_at ? "The deadline was missed; the next ping resets it." : `Next deadline ${stamp(h.deadline_at)}.`}
                </CardDescription>
              </CardHeader>
              <CardContent>
                <CopyField id="ping-url" value={d.ping_url} />
              </CardContent>
            </Card>
            <StatCards stats={d.stats} />
            <ChartCard insight={d.insight} />
            <div className="grid items-start gap-6 xl:grid-cols-[minmax(0,2fr)_minmax(280px,1fr)]">
              <AvailabilityCard insight={d.insight} />
              <Card>
                <CardHeader>
                  <CardTitle className="text-[15px]">Settings</CardTitle>
                </CardHeader>
                <CardContent>
                  <Facts
                    rows={[
                      ["Runs every", h.every],
                      ["Grace time", h.grace],
                      ["Deadline", h.deadline_at ? stamp(h.deadline_at) : "none"],
                      ["Created", stamp(h.created_at)],
                    ]}
                  />
                </CardContent>
              </Card>
            </div>
            <TableCard title="Pings, last 7 days">
              {d.pings.length === 0 ? (
                <TableEmpty title="Waiting for the first ping" description="The first ping arrives when the job next runs." />
              ) : (
                <>
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Time</TableHead>
                        <TableHead>Kind</TableHead>
                        <TableHead className="text-right">From</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {d.pings.map((r) => (
                        <TableRow key={r.id}>
                          <TableCell className="font-mono text-[13px]">{stamp(r.received_at)}</TableCell>
                          <TableCell>
                            <StateBadge kind={r.kind === "ok" ? "up" : "down"} label={r.kind === "ok" ? "OK" : "Fail"} />
                          </TableCell>
                          <TableCell className="text-right font-mono text-muted-foreground">{r.source_ip ?? "–"}</TableCell>
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
