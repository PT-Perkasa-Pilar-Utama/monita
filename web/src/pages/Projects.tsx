import { useEffect, useState } from "react";
import type { ReactElement } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { NewProjectDialog } from "@/components/app/NewProjectDialog";
import { ProjectBadge } from "@/components/app/ProjectBadge";
import { Plus } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { PageSearch } from "@/components/list/PageSearch";
import { Pager } from "@/components/list/Pager";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { listQuery, pageCount, pageParam } from "@/lib/page";
import type { Project, ProjectList } from "@/lib/types";
import { useApi } from "@/lib/useApi";

function ProjectCard({ p }: { p: Project }): ReactElement {
  return (
    <Link to={`/projects/${p.slug}`} className="group rounded-xl outline-none focus-visible:ring-[3px] focus-visible:ring-ring">
      <Card className="h-full gap-0 py-0 transition-colors group-hover:border-input">
        <div className="flex items-center gap-3 p-5">
          <ProjectBadge name={p.name} iconUrl={p.icon_url} className="size-9 rounded-lg text-sm" />
          <div className="flex min-w-0 flex-col">
            <strong className="truncate text-[15px] font-semibold">{p.name}</strong>
            <span className="truncate font-mono text-xs text-muted-foreground">{p.slug}</span>
          </div>
        </div>
        {p.description && <p className="line-clamp-2 px-5 pb-4 text-[13px] text-muted-foreground">{p.description}</p>}
        <div className="mt-auto grid grid-cols-2 gap-3 rounded-b-xl border-t bg-paper px-5 py-3 text-[13px]">
          <span className="flex min-w-0 flex-col gap-0.5">
            <span className="text-xs text-muted-foreground">Owner</span>
            <span className="truncate">{p.owner_email}</span>
          </span>
          <span className="flex min-w-0 flex-col gap-0.5">
            <span className="text-xs text-muted-foreground">Status page</span>
            <span className="truncate font-mono text-xs">/status/{p.slug}</span>
          </span>
        </div>
      </Card>
    </Link>
  );
}

/** The projects a user may open: their own for a tech lead, all of them for a super admin. */
export function Projects(): ReactElement {
  const [params, setParams] = useSearchParams();
  const q = params.get("q") ?? "";
  const page = pageParam(params.get("page"));
  const list = useApi<ProjectList>(`/projects${listQuery(q, page)}`);
  const [draft, setDraft] = useState(q);
  const [creating, setCreating] = useState(false);
  const d = list.data;
  const projects = d?.projects ?? [];
  const go = (nextQ: string, nextPage: number, replace: boolean): void =>
    setParams(new URLSearchParams(listQuery(nextQ, nextPage)), { replace, preventScrollReset: replace });

  // The search reads the server, so it waits for a pause in typing.
  useEffect(() => {
    if (draft.trim() === q.trim()) return;
    const t = setTimeout(() => go(draft, 1, true), 250);
    return () => clearTimeout(t);
  }, [draft, q]);

  // Back or a link changed the search: show it in the box.
  useEffect(() => {
    if (draft.trim() !== q.trim()) setDraft(q);
  }, [q]);

  // A page past the end, from an old link or a deleted project, shows the last page.
  useEffect(() => {
    if (d && d.total > 0 && projects.length === 0 && page > 1) go(q, pageCount(d.total, d.page_size), true);
  }, [d]);

  const turn = (p: number): void => {
    go(q, p, false);
    window.scrollTo({ top: 0 });
  };
  const clear = (): void => {
    setDraft("");
    go("", 1, true);
  };

  return (
    <>
      <PageHeader
        title="Projects"
        actions={
          <>
            {(projects.length > 0 || q) && <PageSearch value={draft} onChange={setDraft} placeholder="Search projects" />}
            <Button onClick={() => setCreating(true)}>
              <Plus data-icon="inline-start" />
              New project
            </Button>
          </>
        }
      />
      {list.error && (
        <Alert variant="destructive">
          <AlertDescription>{list.error.message}</AlertDescription>
        </Alert>
      )}
      {list.loading && !list.data ? (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(min(100%,320px),1fr))] gap-4">
          {[0, 1, 2].map((i) => (
            <Skeleton key={i} className="h-36 rounded-xl" />
          ))}
        </div>
      ) : projects.length === 0 && !q ? (
        <Card className="py-0">
          <TableEmpty
            title="Create your first project"
            description="A project groups the monitors, heartbeats and tickets of one system, and gets its own status page."
          />
        </Card>
      ) : projects.length === 0 ? (
        <Card className="py-0">
          <TableEmpty title="" filtered noun="projects" onClear={clear} />
        </Card>
      ) : (
        <>
          <div className="grid grid-cols-[repeat(auto-fill,minmax(min(100%,320px),1fr))] gap-4">
            {projects.map((p) => (
              <ProjectCard key={p.slug} p={p} />
            ))}
          </div>
          {d && <Pager page={d.page} pageSize={d.page_size} total={d.total} noun="project" onPage={turn} />}
        </>
      )}
      <NewProjectDialog open={creating} onOpenChange={setCreating} owners={list.data?.owners ?? []} />
    </>
  );
}
