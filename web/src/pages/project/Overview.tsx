import type { ReactElement } from "react";
import { Writable } from "@/lib/session";
import { Link } from "react-router-dom";
import { LinkRow, ROW_LINK } from "@/components/app/LinkRow";
import { RefreshButton } from "@/components/app/RefreshButton";
import { OverviewChart } from "@/components/app/OverviewChart";
import { ProjectAbout, useProjectEdit } from "@/components/app/ProjectAbout";
import { ProjectBadge } from "@/components/app/ProjectBadge";
import { LabelBadge, SeverityBadge } from "@/components/app/Tone";
import { TableCard } from "@/components/app/TableCard";
import { Pencil, Save, X } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useProject } from "@/lib/project";
import { ago, duration, nowSecs } from "@/lib/time";
import type { AlertCard, Overview as OverviewData } from "@/lib/types";
import { LIVE_MS, useApi } from "@/lib/useApi";

/** Where the project stands: a tile per area, the open incidents, and an About column. */
export function Overview(): ReactElement {
  const { project: p, counts, reload } = useProject();
  const edit = useProjectEdit({ project: p, counts }, reload);
  const o = useApi<OverviewData>(`/projects/${p.slug}/overview`, LIVE_MS);
  const alerts = useApi<AlertCard[]>(`/projects/${p.slug}/alert-cards`);
  const base = `/projects/${p.slug}`;
  const now = nowSecs();
  const d = o.data;

  return (
    <>
      <PageHeader
        lead={<ProjectBadge name={p.name} iconUrl={p.icon_url} className="size-10 rounded-lg text-[15px]" />}
        title={p.name}
        actions={
          edit.editing ? (
            <>
              <Button variant="outline" disabled={edit.saving} onClick={edit.discard}>
                <X data-icon="inline-start" />
                Discard changes
              </Button>
              <Button disabled={edit.saving} onClick={() => void edit.save()}>
                {edit.saving ? <Spinner data-icon="inline-start" /> : <Save data-icon="inline-start" />}
                Save
              </Button>
            </>
          ) : (
            <Writable>
              <Button variant="outline" onClick={edit.start}>
                <Pencil data-icon="inline-start" />
                Edit project
              </Button>
            </Writable>
          )
        }
      />
      <div className="grid items-start gap-8 lg:grid-cols-[minmax(0,1fr)_18rem]">
        <div className="flex min-w-0 flex-col gap-6">
          <Card className="gap-5 p-5">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="flex flex-col gap-0.5">
                <h2 className="font-semibold">Last 24 hours</h2>
                <p className="text-[13px] text-muted-foreground">{d ? `${d.monitors.text} ${d.heartbeats.text}` : "Response time and pings, updated every 30 seconds."}</p>
              </div>
              <RefreshButton onClick={o.refresh} busy={o.refreshing} />
            </div>
            {!d ? <Skeleton className="h-72 rounded-lg" /> : <OverviewChart base={base} chart={d.chart} monitorsTotal={d.monitors.total} />}
          </Card>
          <TableCard
            title="Open incidents"
            tools={
              <Button asChild variant="ghost" size="sm">
                <Link to={`${base}/incidents`}>All incidents</Link>
              </Button>
            }
          >
            {!d ? (
              <Skeleton className="m-5 h-24" />
            ) : d.incidents.length === 0 ? (
              <TableEmpty title="Every system is up" description="Monita opens an incident when a monitor fails the checks in a row you set or a heartbeat misses." />
            ) : (
              <>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Incident</TableHead>
                      <TableHead>Alert level</TableHead>
                      <TableHead className="max-sm:hidden">Latest update</TableHead>
                      <TableHead className="text-right">Duration</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {d.incidents.map((i) => (
                      <LinkRow key={i.id} to={`${base}/incidents/${i.id}`}>
                        <TableCell className="whitespace-normal">
                          <Link to={`${base}/incidents/${i.id}`} className={ROW_LINK}>
                            {i.title}
                          </Link>
                          <div className="font-mono text-xs text-muted-foreground">
                            INC-{i.id}, started {ago(i.started_at, now)}
                          </div>
                        </TableCell>
                        <TableCell>
                          <SeverityBadge severity={i.severity} />
                        </TableCell>
                        <TableCell className="max-sm:hidden">{i.last_label && <LabelBadge label={i.last_label} />}</TableCell>
                        <TableCell className="text-right font-mono">{duration((i.resolved_at ?? now) - i.started_at)}</TableCell>
                      </LinkRow>
                    ))}
                  </TableBody>
                </Table>
                {counts.incidents > d.incidents.length && (
                  <p className="border-t px-5 py-3 text-[13px] text-muted-foreground">
                    Newest {d.incidents.length} of {counts.incidents} open incidents.
                  </p>
                )}
              </>
            )}
          </TableCard>
        </div>
        <ProjectAbout head={{ project: p, counts }} activeKeys={d?.active_keys ?? 0} triage={d?.triage ?? 0} alerts={alerts.data} edit={edit} />
      </div>
    </>
  );
}
