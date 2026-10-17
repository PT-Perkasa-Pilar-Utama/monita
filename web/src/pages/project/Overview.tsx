import type { ReactElement, ReactNode } from "react";
import { Link } from "react-router-dom";
import { ProjectBadge } from "@/components/app/ProjectBadge";
import { LabelBadge, SeverityBadge } from "@/components/app/Tone";
import { TableCard } from "@/components/app/TableCard";
import {
  Activity,
  AlertTriangle,
  ChevronRight,
  Folder,
  Globe,
  Heartbeat,
  Key,
  Plus,
  Shield,
  Ticket,
  Users,
} from "@/components/icons";
import type { AppIcon } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useProject } from "@/lib/project";
import { ago, duration, nowSecs, utc } from "@/lib/time";
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

function Tile({ to, icon: Icon, title, children, server }: { to: string; icon: AppIcon; title: string; children: ReactNode; server?: boolean }): ReactElement {
  const body = (
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
  );
  const cls = "group rounded-xl outline-none focus-visible:ring-[3px] focus-visible:ring-ring";
  // ponytail: tickets are a server page until R4.
  return server ? (
    <a href={to} className={cls}>
      {body}
    </a>
  ) : (
    <Link to={to} className={cls}>
      {body}
    </Link>
  );
}

function Fact({ icon: Icon, children }: { icon: AppIcon; children: ReactNode }): ReactElement {
  return (
    <li className="flex items-start gap-3 text-foreground/90">
      <Icon className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
      <span className="min-w-0 break-words">{children}</span>
    </li>
  );
}

/** Where the project stands: a tile per area, the open incidents, and an About column. */
export function Overview(): ReactElement {
  const { project: p, counts } = useProject();
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
          <>
            <Button asChild>
              <Link to={`${base}/monitors/new`}>
                <Plus data-icon="inline-start" />
                New monitor
              </Link>
            </Button>
            <Button asChild variant="secondary">
              <Link to={`${base}/incidents/new`}>Report incident</Link>
            </Button>
          </>
        }
      />
      <div className="grid items-start gap-6 xl:grid-cols-[minmax(0,2fr)_minmax(280px,1fr)]">
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
              <Tile to={`${base}/tickets`} icon={Ticket} title="Tickets" server>
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
        <aside aria-labelledby="about-title" className="flex flex-col gap-4 xl:sticky xl:top-[calc(var(--bar-h)+24px)]">
          <h2 id="about-title" className="text-xl font-semibold">
            About
          </h2>
          {p.description && <p className="text-foreground/90">{p.description}</p>}
          <ul className="flex flex-col gap-3">
            <Fact icon={Globe}>
              Status page{" "}
              <a href={`/status/${p.slug}`} className="font-mono text-link hover:underline">
                /status/{p.slug}
              </a>
            </Fact>
            <Fact icon={Users}>Owned by {p.owner_email}</Fact>
            <Fact icon={Folder}>Created {utc(p.created_at)}</Fact>
            <Fact icon={Key}>
              <Link to={`${base}/settings/keys`} className="text-link hover:underline">
                {plural(d?.active_keys ?? 0, "active API key")}
              </Link>
            </Fact>
            <Fact icon={Shield}>{p.ip_allowlist ? "Only allowed IP addresses may use the keys" : "Any IP address may use the keys"}</Fact>
          </ul>
          <Separator />
          <ul className="flex flex-col gap-3 font-semibold">
            <Fact icon={Activity}>{plural(counts.monitors, "monitor")}</Fact>
            <Fact icon={Heartbeat}>{plural(counts.heartbeats, "heartbeat")}</Fact>
            <Fact icon={AlertTriangle}>{plural(counts.incidents, "open incident")}</Fact>
            <Fact icon={Ticket}>
              {plural(counts.tickets, "open ticket")}
              <span className="block text-xs font-normal text-muted-foreground">{d?.triage ?? 0} in Triage</span>
            </Fact>
          </ul>
        </aside>
      </div>
    </>
  );
}
