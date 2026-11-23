import type { ReactElement } from "react";
import { Link } from "react-router-dom";
import { ProjectBadge } from "@/components/app/ProjectBadge";
import { StateIcon } from "@/components/app/PublicStatus";
import { Skeleton } from "@/components/ui/skeleton";
import { tileLabel } from "@/lib/projectGrid";
import type { ProjectTileRow, StateKind } from "@/lib/types";
import { cn } from "@/lib/utils";

// The ring carries the state's colour. Maintenance takes the neutral tone, as its stamp does,
// so it never reads as an issue.
const RING: Record<StateKind, string> = {
  up: "ring-up",
  down: "ring-down",
  issue: "ring-issue",
  maintenance: "ring-neutral-strong",
  paused: "ring-muted-foreground",
  unknown: "ring-muted-foreground",
};

/** The compact view (v1.2 Q53): each project as its icon in a ring of its state's colour. Every
 * state but up adds its glyph, so colour is never the only sign. */
export function ProjectGrid({ tiles, loading, skeletons }: { tiles: ProjectTileRow[]; loading: boolean; skeletons: number }): ReactElement {
  return (
    <ul className="grid grid-cols-[repeat(auto-fill,4rem)] gap-4" aria-busy={loading} aria-label="Projects">
      {loading
        ? Array.from({ length: skeletons }, (_, i) => (
            <li key={i}>
              <Skeleton className="size-16 rounded-xl" />
            </li>
          ))
        : tiles.map((t) => {
            const label = tileLabel(t.name, t.kind);
            return (
              <li key={t.slug}>
                <Link
                  to={`/projects/${t.slug}`}
                  aria-label={label}
                  title={label}
                  className={cn("relative grid size-16 place-items-center rounded-xl ring-2 outline-none focus-visible:ring-[3px] focus-visible:ring-ring", RING[t.kind])}
                >
                  <ProjectBadge name={t.name} iconUrl={t.icon_url} className="size-12 rounded-lg text-lg" />
                  {t.kind !== "up" && (
                    <span aria-hidden className="absolute -right-1.5 -bottom-1.5 grid place-items-center rounded-full bg-background">
                      <StateIcon kind={t.kind} label="" className="size-5" />
                    </span>
                  )}
                </Link>
              </li>
            );
          })}
    </ul>
  );
}
