import type { ReactElement } from "react";
import { Link, useNavigate } from "react-router-dom";
import { ArrowLeft, ArrowRight } from "@/components/icons";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import type { PublicRelease, ReleaseHead } from "@/lib/types";
import { cn } from "@/lib/utils";

export const releasePath = (slug: string, version: string): string => `/status/${slug}/changelog/${encodeURIComponent(version)}`;

/** One release: its version, day and title, then each kind of change and its lines. */
export function ReleaseCard({ release }: { release: PublicRelease["release"] }): ReactElement {
  return (
    <article className="rounded-xl border bg-card p-6 shadow-xs sm:p-8">
      <header className="flex flex-col gap-2 border-b pb-5">
        <div className="flex flex-wrap items-baseline gap-3">
          <h2 className="font-mono text-2xl font-semibold tracking-tight">{release.version}</h2>
          <time className="ml-auto text-sm text-muted-foreground" dateTime={release.released_on}>
            {release.released_on}
          </time>
        </div>
        <p className="text-lg font-medium">{release.title}</p>
      </header>
      <div className="flex flex-col gap-6 pt-6">
        {release.sections.map((s) => (
          <section key={s.tag} className="flex flex-col gap-3">
            <Badge variant="neutral" className="self-start">
              {s.tag}
            </Badge>
            <ul className="flex list-disc flex-col gap-1.5 pl-5 text-sm leading-relaxed marker:text-muted-foreground">
              {s.items.map((item, n) => (
                <li key={n} className="break-words">
                  {item}
                </li>
              ))}
            </ul>
          </section>
        ))}
      </div>
    </article>
  );
}

/** A link to the release just newer or just older. */
export function Neighbour({ slug, release, side }: { slug: string; release: ReleaseHead | null; side: "newer" | "older" }): ReactElement {
  if (!release) return <span />;
  return (
    <Link to={releasePath(slug, release.version)} className={cn("flex flex-col gap-0.5 rounded-xl border bg-card p-4 hover:border-input", side === "older" && "items-end text-right")}>
      <span className="flex items-center gap-1 text-xs text-muted-foreground">
        {side === "newer" && <ArrowLeft className="size-3" />}
        {side === "newer" ? "Newer" : "Older"}
        {side === "older" && <ArrowRight className="size-3" />}
      </span>
      <span className="font-mono font-semibold">{release.version}</span>
      <span className="line-clamp-1 text-sm text-muted-foreground">{release.title}</span>
    </Link>
  );
}

/** The releases beside the open one. The search asks the server, so it finds any release, not
 * only the loaded ones. A phone gets a select. */
export function VersionIndex({ slug, releases, open, older, onOlder, query, onQuery }: { slug: string; releases: ReleaseHead[]; open: string; older: boolean; onOlder: () => void; query: string; onQuery: (q: string) => void }): ReactElement {
  const navigate = useNavigate();
  const shown = releases;
  return (
    <>
      <nav aria-label="Releases" className="sticky top-[calc(var(--bar-h)+24px)] hidden flex-col gap-3 lg:flex">
        <Input type="search" value={query} onChange={(e) => onQuery(e.target.value)} placeholder="Find a version or title" aria-label="Find a release" />
        <ol className="flex max-h-[60vh] flex-col gap-0.5 overflow-y-auto border-l pl-3 text-sm">
          {shown.map((r) => (
            <li key={r.version}>
              <Link
                to={releasePath(slug, r.version)}
                aria-current={r.version === open ? "page" : undefined}
                className={cn("flex flex-col rounded-md px-2 py-1", r.version === open ? "bg-card font-semibold text-foreground" : "text-muted-foreground hover:text-foreground")}
              >
                <span className="font-mono">{r.version}</span>
                <span className="text-xs font-normal text-muted-foreground">{r.released_on}</span>
              </Link>
            </li>
          ))}
          {shown.length === 0 && <li className="px-2 py-1 text-muted-foreground">No version matches.</li>}
          {older && (
            <li>
              <button type="button" onClick={onOlder} className="px-2 py-1 text-link underline-offset-4 hover:underline">
                Older releases
              </button>
            </li>
          )}
        </ol>
      </nav>
      <select
        aria-label="Release"
        value={open}
        onChange={(e) => navigate(releasePath(slug, e.target.value))}
        className="h-10 rounded-md border bg-card px-3 font-mono text-sm lg:hidden"
      >
        {releases.map((r) => (
          <option key={r.version} value={r.version}>
            {r.version}, {r.released_on}
          </option>
        ))}
      </select>
    </>
  );
}
