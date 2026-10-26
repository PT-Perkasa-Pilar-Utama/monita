import type { ReactElement } from "react";
import { Link } from "react-router-dom";
import { LinkRow, ROW_LINK } from "@/components/app/LinkRow";
import { PinButton } from "@/components/app/PinButton";
import { ProjectBadge } from "@/components/app/ProjectBadge";
import { DayBars } from "@/components/app/PublicStatus";
import { StateBadge } from "@/components/app/Tone";
import { ExternalLink } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { day } from "@/lib/time";
import { meanUptime } from "@/lib/status";
import type { ProjectRow, StateKind } from "@/lib/types";
import { cn } from "@/lib/utils";
import { SortHead } from "@/components/list/SortHead";
import type { Sort } from "@/lib/sort";

/** The public status page, in a new tab. A sibling of the card's link, never inside it. */
function StatusLink({ slug, labelled }: { slug: string; labelled: boolean }): ReactElement {
  const title = "Open the status page in a new tab";
  return (
    <Button asChild variant="ghost" size={labelled ? "sm" : "icon-sm"} className="relative z-10">
      <a href={`/status/${slug}`} target="_blank" rel="noreferrer" title={title} aria-label={labelled ? undefined : title}>
        {labelled && "Status page"}
        <ExternalLink data-icon={labelled ? "inline-end" : undefined} />
      </a>
    </Button>
  );
}

const STATE_WORD: Record<StateKind, string> = { up: "All up", down: "Down", issue: "Issue", maintenance: "Maintenance", paused: "Paused", unknown: "No data yet" };
const toneText = (k: StateKind): string | undefined => (k === "down" ? "text-down" : k === "issue" ? "text-issue" : undefined);

/** A number with its name under it; red or yellow while that part has a problem. */
function Stat({ label, value, kind, title }: { label: string; value: number; kind?: StateKind | undefined; title?: string }): ReactElement {
  return (
    <div title={title} className="flex min-w-0 flex-col gap-0.5 px-3 py-2.5">
      <span className={cn("text-lg leading-tight font-semibold tabular-nums", kind && toneText(kind))}>{value.toLocaleString()}</span>
      <span className="truncate text-xs text-muted-foreground">{label}</span>
    </div>
  );
}

/** A project as a card: its state, a bar per hour of the last day, and the four counts a tech
 * lead scans for. The whole card opens it; its foot links the status page. */
export function ProjectCard({ p, onPin }: { p: ProjectRow; onPin: () => void }): ReactElement {
  const pulse = p.pulse;
  const mean = meanUptime(pulse.hours);
  return (
    <Card className="group relative h-full gap-0 py-0 transition-[border-color,box-shadow,translate] duration-150 hover:-translate-y-0.5 hover:border-foreground/25 hover:shadow-md has-[a:focus-visible]:border-ring motion-reduce:hover:translate-y-0">
      <div className="flex items-center gap-3 p-5 pb-4">
        <ProjectBadge name={p.name} iconUrl={p.icon_url} className="size-10 rounded-lg text-sm" />
        <div className="flex min-w-0 flex-1 flex-col">
          <Link to={`/projects/${p.slug}`} className="truncate text-[15px] font-semibold outline-none after:absolute after:inset-0 after:rounded-xl group-hover:underline">
            {p.name}
          </Link>
          <span className="truncate text-[13px] text-muted-foreground">{p.owner_email}</span>
        </div>
        <StateBadge kind={pulse.kind} label={STATE_WORD[pulse.kind]} />
        {/* Above the card's stretched link, so the pin takes its own click. */}
        <PinButton path={`/projects/${p.slug}`} name={p.name} pinned={p.pinned} onChange={onPin} className="relative z-10 -mr-2" />
      </div>
      <div className="flex flex-col gap-2 px-5 pb-4">
        <DayBars days={pulse.hours} />
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <span>Last 24 hours</span>
          <span className="tabular-nums">{mean ? `${mean} uptime` : "No systems yet"}</span>
        </div>
      </div>
      <div className="mx-5 mb-4 grid grid-cols-4 divide-x rounded-lg border">
        <Stat label="Monitors" value={pulse.monitors.total} kind={pulse.monitors.kind} title={pulse.monitors.text} />
        <Stat label="Heartbeats" value={pulse.heartbeats.total} kind={pulse.heartbeats.kind} title={pulse.heartbeats.text} />
        <Stat label="Incidents" value={pulse.incidents} kind={pulse.incidents > 0 ? "down" : undefined} />
        <Stat label="Tickets" value={pulse.tickets} />
      </div>
      <div className="mt-auto flex items-center justify-between gap-3 rounded-b-xl border-t bg-paper py-2 pr-3 pl-5 text-[13px]">
        <span className="min-w-0 truncate text-muted-foreground">{p.description || `Created ${day(p.created_at)}`}</span>
        <StatusLink slug={p.slug} labelled />
      </div>
    </Card>
  );
}

/** Card placeholders, the same size as the cards they stand in for. */
export function ProjectCardSkeletons({ count }: { count: number }): ReactElement {
  return (
    <>
      {Array.from({ length: count }, (_, i) => (
        <Skeleton key={i} className="h-[276px] rounded-xl" />
      ))}
    </>
  );
}

/** Projects as table rows; a click anywhere on a row opens the project. The server sorts by name or by age. */
export function ProjectTable({
  projects,
  loading,
  skeletons,
  sort,
  onSort,
  onPin,
}: {
  projects: ProjectRow[];
  loading: boolean;
  skeletons: number;
  sort: Sort;
  onSort: (by: string) => void;
  onPin: () => void;
}): ReactElement {
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <SortHead by="name" label="Project" sort={sort} onSort={onSort} />
          <TableHead>State</TableHead>
          <TableHead className="max-md:hidden">Last 24 hours</TableHead>
          <TableHead className="text-right max-sm:hidden">Incidents</TableHead>
          <TableHead className="text-right max-sm:hidden">Tickets</TableHead>
          <TableHead className="max-lg:hidden">Owner</TableHead>
          <TableHead className="w-12 text-right">
            <span className="sr-only">Status page</span>
          </TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {loading
          ? Array.from({ length: skeletons }, (_, i) => (
              <TableRow key={i}>
                <TableCell colSpan={7}>
                  <Skeleton className="h-9 w-full" />
                </TableCell>
              </TableRow>
            ))
          : projects.map((p) => (
              <LinkRow key={p.slug} to={`/projects/${p.slug}`}>
                <TableCell className="max-w-56 sm:max-w-80">
                  <div className="flex items-center gap-2">
                    <PinButton path={`/projects/${p.slug}`} name={p.name} pinned={p.pinned} onChange={onPin} className="-ml-1.5" />
                    <ProjectBadge name={p.name} iconUrl={p.icon_url} className="size-8 rounded-lg text-xs" />
                    <div className="flex min-w-0 flex-col">
                      <Link to={`/projects/${p.slug}`} className={cn("truncate", ROW_LINK)}>
                        {p.name}
                      </Link>
                      <span className="truncate text-xs text-muted-foreground">{p.description || "No description."}</span>
                    </div>
                  </div>
                </TableCell>
                <TableCell>
                  <div className="flex flex-col items-start gap-1">
                    <StateBadge kind={p.pulse.kind} label={STATE_WORD[p.pulse.kind]} />
                    <span className="text-xs text-muted-foreground max-sm:hidden">
                      {p.pulse.monitors.total} monitors, {p.pulse.heartbeats.total} heartbeats
                    </span>
                  </div>
                </TableCell>
                <TableCell className="w-56 max-md:hidden">
                  <div className="flex flex-col gap-1">
                    <DayBars days={p.pulse.hours} />
                    <span className="text-xs text-muted-foreground tabular-nums">{meanUptime(p.pulse.hours) ?? "No systems yet"}</span>
                  </div>
                </TableCell>
                <TableCell className={cn("text-right tabular-nums max-sm:hidden", p.pulse.incidents > 0 && "text-down")}>{p.pulse.incidents}</TableCell>
                <TableCell className="text-right tabular-nums max-sm:hidden">{p.pulse.tickets}</TableCell>
                <TableCell className="text-muted-foreground max-lg:hidden">{p.owner_email}</TableCell>
                <TableCell className="py-0 text-right">
                  <StatusLink slug={p.slug} labelled={false} />
                </TableCell>
              </LinkRow>
            ))}
      </TableBody>
    </Table>
  );
}
