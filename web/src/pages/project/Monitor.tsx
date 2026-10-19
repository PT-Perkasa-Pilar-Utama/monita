import type { ReactElement } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
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
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useProject } from "@/lib/project";
import { headerCount, whenItFails } from "@/lib/checks";
import { stamp } from "@/lib/time";
import type { MonitorDetail } from "@/lib/types";
import { useDetail } from "@/lib/useDetail";
import { NotFound } from "@/pages/NotFound";
import { MonitorFormSheet } from "@/pages/project/MonitorForm";

/** One monitor: four numbers, the response-time chart, availability, settings and its checks. */
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
                <a href={m.url} target="_blank" rel="noreferrer" className="min-w-0 truncate font-mono text-sm text-link hover:underline">
                  {m.url}
                </a>
                <SeverityBadge severity={m.severity} />
                {m.is_public && <Badge variant="outline">On the status page</Badge>}
              </>
            }
            actions={
              <Button asChild variant="secondary">
                <Link to={`${base}/${m.id}/edit`}>
                  <Pencil data-icon="inline-start" />
                  Edit
                </Link>
              </Button>
            }
          />
          <div className="flex flex-col gap-6">
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
                      ["Every", m.every],
                      ["Request", headerCount(m.headers) ? `${m.method}, ${headerCount(m.headers)} ${headerCount(m.headers) === 1 ? "header" : "headers"}` : m.method],
                      ["Timeout", `${m.timeout_s} seconds`],
                      ["Expects", m.keyword ? `${m.expected_status} with "${m.keyword}"` : m.expected_status],
                      ["TLS", m.skip_tls_verify ? "Not verified" : "Verified"],
                      ["When it fails", whenItFails(m.alert, m.auto_incident)],
                      ["Next check", stamp(m.next_check_at)],
                    ]}
                  />
                </CardContent>
              </Card>
            </div>
            <TableCard title="Checks, last 7 days">
              {d.checks.length === 0 ? (
                <TableEmpty title="Waiting for the first check" description="The first check runs within one interval of saving." />
              ) : (
                <>
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Time</TableHead>
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
