import type { ReactElement, ReactNode } from "react";
import { Link } from "react-router-dom";
import { ProjectAbout, useProjectEdit } from "@/components/app/ProjectAbout";
import { ProjectBadge } from "@/components/app/ProjectBadge";
import { LabelBadge, SeverityBadge } from "@/components/app/Tone";
import { TableCard } from "@/components/app/TableCard";
import { Activity, ChevronRight, Heartbeat, Pencil, Save, Ticket, X } from "@/components/icons";
import type { AppIcon } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useProject } from "@/lib/project";
import { ago, duration, nowSecs } from "@/lib/time";
import type { Overview as OverviewData, StateKind } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { cn } from "@/lib/utils";

const plural = (n: number, noun: string): string => `${n.toLocaleString()} ${noun}${n === 1 ? "" : "s"}`;

/** A ring for a state: filled for up and down, half for an issue, empty while waiting. */
function StateRing({ kind }: { kind: StateKind | "triage" }): ReactElement {
  return (
    <span
      aria-hidden="true"
      className={cn(
        "inline-block size-3 shrink-0 rounded-full border-[1.5px]",
        kind === "up" && "border-up bg-up",
        kind === "down" && "border-down bg-down",
        kind === "issue" && "border-issue bg-[conic-gradient(var(--issue)_0_50%,transparent_50%_100%)]",
        kind === "unknown" && "border-unknown",
        kind === "triage" && "border-dashed border-muted-foreground",
      )}
    />
  );
}

function Tile({ to, icon: Icon, title, children }: { to: string; icon: AppIcon; title: string; children: ReactNode }): ReactElement {
  return (
    <Link to={to} className="group rounded-xl outline-none focus-visible:ring-[3px] focus-visible:ring-ring">
      <Card className="h-full flex-row items-center gap-3 p-4 transition-colors group-hover:border-input">
        <span className="grid size-9 shrink-0 place-items-center rounded-lg bg-muted text-muted-foreground">
          <Icon className="size-4" />
        </span>
        <span className="flex min-w-0 flex-1 flex-col gap-1">
          <strong className="font-semibold">{title}</strong>
          <span className="flex items-center gap-2 text-[13px] text-muted-foreground">{children}</span>
        </span>
        <ChevronRight className="size-4 text-muted-foreground" />
      </Card>
    </Link>
  );
}

/** Where the project stands: a tile per area, the open incidents, and an About column. */
export function Overview(): ReactElement {
  const { project: p, counts, reload } = useProject();
  const edit = useProjectEdit({ project: p, counts }, reload);
  const o = useApi<OverviewData>(`/projects/${p.slug}/overview`);
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
            <Button variant="outline" onClick={edit.start}>
              <Pencil data-icon="inline-start" />
              Edit project
            </Button>
          )
        }
      />
      <div className="grid items-start gap-8 lg:grid-cols-[minmax(0,1fr)_18rem]">
        <div className="flex min-w-0 flex-col gap-6">
          {!d ? (
            <Skeleton className="h-24 rounded-xl" />
          ) : (
            <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,240px),1fr))] gap-4">
              <Tile to={`${base}/monitors`} icon={Activity} title="Monitors">
                {d.monitors.total === 0 ? "Add your first monitor." : (
                  <>
                    <StateRing kind={d.monitors.kind} />
                    {d.monitors.text}
                  </>
                )}
              </Tile>
              <Tile to={`${base}/heartbeats`} icon={Heartbeat} title="Heartbeats">
                {d.heartbeats.total === 0 ? "Watch your first cron job." : (
                  <>
                    <StateRing kind={d.heartbeats.kind} />
                    {d.heartbeats.text}
                  </>
                )}
              </Tile>
              <Tile to={`${base}/tickets`} icon={Ticket} title="Tickets">
                <StateRing kind="triage" />
                {d.triage === 0 ? "No ticket waits in Triage." : `${plural(d.triage, "ticket")} in Triage.`}
              </Tile>
            </div>
          )}
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
              <TableEmpty title="Every system is up" description="Monita opens an incident when a monitor fails twice in a row or a heartbeat misses." />
            ) : (
              <>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Incident</TableHead>
                      <TableHead>Severity</TableHead>
                      <TableHead className="max-sm:hidden">Latest update</TableHead>
                      <TableHead className="text-right">Duration</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {d.incidents.map((i) => (
                      <TableRow key={i.id}>
                        <TableCell className="whitespace-normal">
                          <Link to={`${base}/incidents/${i.id}`} className="font-medium hover:underline">
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
                      </TableRow>
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
        <ProjectAbout head={{ project: p, counts }} activeKeys={d?.active_keys ?? 0} triage={d?.triage ?? 0} edit={edit} />
      </div>
    </>
  );
}
