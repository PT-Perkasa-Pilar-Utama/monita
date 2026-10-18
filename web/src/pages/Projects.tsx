import { useEffect, useState } from "react";
import type { ReactElement } from "react";
import { NewProjectDialog } from "@/components/app/NewProjectDialog";
import { ProjectCard, ProjectCardSkeletons, ProjectTable } from "@/components/app/ProjectTile";
import { TableCard } from "@/components/app/TableCard";
import { Plus } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { PageSearch } from "@/components/list/PageSearch";
import { Pager } from "@/components/list/Pager";
import { TableEmpty } from "@/components/list/TableEmpty";
import { ViewToggle } from "@/components/list/ViewToggle";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { listQuery, pageCount, pageParam, skeletonCount } from "@/lib/page";
import type { ProjectList } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { useRememberedQuery } from "@/lib/useRememberedQuery";
import { useViewMode } from "@/lib/useViewMode";

const GRID = "grid grid-cols-[repeat(auto-fill,minmax(min(100%,320px),1fr))] gap-4";

/** The projects a user may open: their own for a tech lead, all of them for a super admin. */
export function Projects(): ReactElement {
  const [search, setSearch] = useRememberedQuery("monita:projects:query");
  const params = new URLSearchParams(search);
  const q = params.get("q") ?? "";
  const page = pageParam(params.get("page"));
  const list = useApi<ProjectList>(`/projects${listQuery(q, page)}`);
  const [view, setView] = useViewMode("monita:projects:view");
  const [draft, setDraft] = useState(q);
  const [creating, setCreating] = useState(false);
  const d = list.data;
  const projects = d?.projects ?? [];
  const go = (nextQ: string, nextPage: number, replace: boolean): void => setSearch(listQuery(nextQ, nextPage), replace);

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

  const loading = list.loading;
  const skeletons = skeletonCount(page, d?.page_size ?? 20, d?.total);
  const none = !loading && d !== undefined && d.total === 0;

  return (
    <>
      <PageHeader
        title="Projects"
        actions={
          <>
            <ViewToggle value={view} onChange={setView} />
            <PageSearch value={draft} onChange={setDraft} placeholder="Search projects" />
            <Button onClick={() => setCreating(true)}>
              <Plus data-icon="inline-start" />
              New project
            </Button>
          </>
        }
      />
      {list.error && (
        <Alert variant="destructive" className="mb-6">
          <AlertDescription>{list.error.message}</AlertDescription>
        </Alert>
      )}
      <div className="flex flex-col gap-6 pb-4">
        {none ? (
          <Card className="py-0">
            {q ? (
              <TableEmpty title="" filtered noun="projects" onClear={clear} />
            ) : (
              <TableEmpty
                title="Create your first project"
                description="A project groups the monitors, heartbeats and tickets of one system, and gets its own status page."
              />
            )}
          </Card>
        ) : view === "card" ? (
          <div className={GRID} aria-busy={loading}>
            {loading ? <ProjectCardSkeletons count={skeletons} /> : projects.map((p) => <ProjectCard key={p.slug} p={p} />)}
          </div>
        ) : (
          <TableCard title={d ? `${d.total} ${d.total === 1 ? "project" : "projects"}` : "Projects"}>
            <ProjectTable projects={projects} loading={loading} skeletons={skeletons} />
          </TableCard>
        )}
        {d && <Pager page={page} pageSize={d.page_size} total={d.total} noun="project" onPage={turn} disabled={loading} />}
      </div>
      <NewProjectDialog open={creating} onOpenChange={setCreating} owners={d?.owners ?? []} />
    </>
  );
}
