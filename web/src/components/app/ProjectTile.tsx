import type { ReactElement } from "react";
import { Link, useNavigate } from "react-router-dom";
import { ProjectBadge } from "@/components/app/ProjectBadge";
import { ExternalLink } from "@/components/icons";
import { Card } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { day } from "@/lib/time";
import type { Project } from "@/lib/types";

/** The public status page, in a new tab. A sibling of the card's link, never inside it. */
function StatusLink({ slug, label }: { slug: string; label: string }): ReactElement {
  return (
    <a
      href={`/status/${slug}`}
      target="_blank"
      rel="noreferrer"
      className="relative z-10 inline-flex items-center gap-1.5 rounded-md text-[13px] font-medium text-link hover:underline focus-visible:ring-[3px] focus-visible:ring-ring focus-visible:outline-none"
    >
      {label}
      <ExternalLink className="size-3.5" />
    </a>
  );
}

/** A project as a card: the whole card opens it, and its foot links the status page. */
export function ProjectCard({ p }: { p: Project }): ReactElement {
  return (
    <Card className="group relative h-full gap-0 py-0 transition-[border-color,box-shadow,translate] duration-150 hover:-translate-y-0.5 hover:border-foreground/25 hover:shadow-md has-[a:focus-visible]:border-ring motion-reduce:hover:translate-y-0">
      <div className="flex items-center gap-3 p-5">
        <ProjectBadge name={p.name} iconUrl={p.icon_url} className="size-10 rounded-lg text-sm" />
        <div className="flex min-w-0 flex-col">
          <Link to={`/projects/${p.slug}`} className="truncate text-[15px] font-semibold outline-none after:absolute after:inset-0 after:rounded-xl group-hover:underline">
            {p.name}
          </Link>
          <span className="truncate text-[13px] text-muted-foreground">{p.owner_email}</span>
        </div>
      </div>
      <p className="line-clamp-2 min-h-10 px-5 pb-4 text-[13px] text-muted-foreground">{p.description || "No description."}</p>
      <div className="mt-auto flex items-center justify-between gap-3 rounded-b-xl border-t bg-paper px-5 py-3 text-[13px]">
        <span className="text-muted-foreground">Created {day(p.created_at)}</span>
        <StatusLink slug={p.slug} label="Status page" />
      </div>
    </Card>
  );
}

/** Card placeholders, the same size as the cards they stand in for. */
export function ProjectCardSkeletons({ count }: { count: number }): ReactElement {
  return (
    <>
      {Array.from({ length: count }, (_, i) => (
        <Skeleton key={i} className="h-[154px] rounded-xl" />
      ))}
    </>
  );
}

/** Projects as table rows; a click anywhere on a row opens the project. */
export function ProjectTable({ projects, loading, skeletons }: { projects: Project[]; loading: boolean; skeletons: number }): ReactElement {
  const navigate = useNavigate();
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Project</TableHead>
          <TableHead className="max-md:hidden">Owner</TableHead>
          <TableHead className="max-sm:hidden">Created</TableHead>
          <TableHead className="text-right">Status page</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {loading
          ? Array.from({ length: skeletons }, (_, i) => (
              <TableRow key={i}>
                <TableCell colSpan={4}>
                  <Skeleton className="h-9 w-full" />
                </TableCell>
              </TableRow>
            ))
          : projects.map((p) => (
              <TableRow key={p.slug} className="group cursor-pointer hover:bg-accent" onClick={() => navigate(`/projects/${p.slug}`)}>
                <TableCell className="max-w-56 sm:max-w-96">
                  <div className="flex items-center gap-3">
                    <ProjectBadge name={p.name} iconUrl={p.icon_url} className="size-8 rounded-lg text-xs" />
                    <div className="flex min-w-0 flex-col">
                      <Link to={`/projects/${p.slug}`} className="truncate font-medium group-hover:underline" onClick={(e) => e.stopPropagation()}>
                        {p.name}
                      </Link>
                      <span className="truncate text-xs text-muted-foreground">{p.description || "No description."}</span>
                    </div>
                  </div>
                </TableCell>
                <TableCell className="text-muted-foreground max-md:hidden">{p.owner_email}</TableCell>
                <TableCell className="text-muted-foreground tabular-nums max-sm:hidden">{day(p.created_at)}</TableCell>
                <TableCell className="text-right" onClick={(e) => e.stopPropagation()}>
                  <StatusLink slug={p.slug} label="Open" />
                </TableCell>
              </TableRow>
            ))}
      </TableBody>
    </Table>
  );
}
