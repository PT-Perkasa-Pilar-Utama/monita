import { RefreshButton } from "@/components/app/RefreshButton";
import { Writable } from "@/lib/session";
import { useEffect, useState } from "react";
import type { ReactElement } from "react";
import { NewProjectSheet } from "@/components/app/NewProjectSheet";
import { OverviewChart } from "@/components/app/OverviewChart";
import { ProjectCard, ProjectCardSkeletons, ProjectTable } from "@/components/app/ProjectTile";
import { FilterCount, TableCard } from "@/components/app/TableCard";
import { Plus } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { FilterMenu } from "@/components/list/FilterMenu";
import { PageSearch } from "@/components/list/PageSearch";
import { Pager } from "@/components/list/Pager";
import { TableEmpty } from "@/components/list/TableEmpty";
import { ViewToggle } from "@/components/list/ViewToggle";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { listQuery, pageCount, pageParam, skeletonCount } from "@/lib/page";
import type { OverviewChart as ChartData, ProjectList } from "@/lib/types";
import { LIVE_MS, useApi } from "@/lib/useApi";
import { useRememberedQuery } from "@/lib/useRememberedQuery";
import { useViewMode } from "@/lib/useViewMode";

const GRID = "grid grid-cols-[repeat(auto-fill,minmax(min(100%,340px),1fr))] gap-4";

/** The projects a user may open: their own for a tech lead, all of them for a super admin. */
export function Projects(): ReactElement {
  const [search, setSearch] = useRememberedQuery("monita:projects:query");
  const params = new URLSearchParams(search);
  const q = params.get("q") ?? "";
  const owner = params.get("owner") ?? "";
  // Any other value reads newest first, as the server does.
  const sort = ["name", "oldest"].includes(params.get("sort") ?? "") ? params.get("sort")! : "";
  const state = ["down", "issue", "up"].includes(params.get("state") ?? "") ? params.get("state")! : "";
  const page = pageParam(params.get("page"));
  const query = listQuery({ q, owner, sort, state, page });
  // The cards and rows show live state, so the page reads again every 30 seconds.
  const list = useApi<ProjectList>(`/projects${query}`, LIVE_MS);
  // The chart is the first view (v1.1 Q39); the view picked last is remembered.
  const [view, setView] = useViewMode("monita:projects:view", "chart");
  const chart = useApi<{ chart: ChartData; slugs: Record<string, string> }>(view === "chart" ? `/projects-chart${query}` : null, LIVE_MS);
  const [draft, setDraft] = useState(q);
  const [creating, setCreating] = useState(false);
  const d = list.data;
  const projects = d?.projects ?? [];
  const go = (next: { q?: string; owner?: string; sort?: string; state?: string }, nextPage: number, replace: boolean): void =>
    setSearch(listQuery({ q, owner, sort, state, ...next, page: nextPage }), replace);

  // The search reads the server, so it waits for a pause in typing.
  useEffect(() => {
    if (draft.trim() === q.trim()) return;
    const t = setTimeout(() => go({ q: draft }, 1, true), 250);
    return () => clearTimeout(t);
  }, [draft, q]);

  // Back or a link changed the search: show it in the box.
  useEffect(() => {
    if (draft.trim() !== q.trim()) setDraft(q);
  }, [q]);

  // A page past the end, from an old link or a deleted project, shows the last page.
  useEffect(() => {
    if (d && d.total > 0 && projects.length === 0 && page > 1) go({}, pageCount(d.total, d.page_size), true);
  }, [d]);

  const turn = (p: number): void => {
    go({}, p, false);
    window.scrollTo({ top: 0 });
  };
  const clear = (): void => {
    setDraft("");
    go({ q: "", owner: "", state: "" }, 1, true);
  };

  const loading = list.loading;
  const skeletons = skeletonCount(page, d?.page_size ?? 20, d?.total);
  const none = !loading && d !== undefined && d.total === 0;
  const filtered = q !== "" || owner !== "" || state !== "";
  const countText = d ? `${d.total} ${d.total === 1 ? "project" : "projects"}` : "Projects";
  const onClear = filtered ? clear : undefined;
  const empty = filtered ? (
    <TableEmpty title="" filtered noun="projects" onClear={clear} />
  ) : (
    <TableEmpty
      title="Create your first project"
      description="A project groups the monitors, heartbeats and tickets of one system, and gets its own status page."
    />
  );
  const filters = (
    <>
      <FilterMenu
        label="State"
        heading="Filter by state"
        value={state}
        options={[
          { value: "", label: "All states" },
          { value: "down", label: "Down" },
          { value: "issue", label: "Issue" },
          { value: "up", label: "Up" },
        ]}
        onChange={(v) => go({ state: v }, 1, true)}
      />
      {(d?.owner_options.length ?? 0) > 0 && (
        <FilterMenu
          label="Owner"
          heading="Filter by owner"
          value={owner}
          options={[{ value: "", label: "All owners" }, ...(d?.owner_options ?? []).map((e) => ({ value: e, label: e }))]}
          onChange={(v) => go({ owner: v }, 1, true)}
        />
      )}
      <FilterMenu
        label="Sort"
        heading="Sort by"
        value={sort}
        options={[
          { value: "", label: "Newest first" },
          { value: "oldest", label: "Oldest first" },
          { value: "name", label: "Name" },
        ]}
        onChange={(v) => go({ sort: v }, 1, true)}
      />
    </>
  );

  return (
    <>
      <PageHeader
        title="Projects"
        filtered={filtered}
        actions={
          <>
            <div className="flex w-full items-center gap-2 sm:w-auto">
              <ViewToggle value={view} onChange={setView} chart />
              <PageSearch value={draft} onChange={setDraft} placeholder="Search projects" />
            </div>
            <RefreshButton
              onClick={() => {
                list.refresh();
                chart.refresh();
              }}
              busy={list.refreshing || chart.refreshing}
            />
            <Writable>
              <Button onClick={() => setCreating(true)}>
                <Plus data-icon="inline-start" />
                New project
              </Button>
            </Writable>
          </>
        }
      />
      {list.error && (
        <Alert variant="destructive" className="mb-6">
          <AlertDescription>{list.error.message}</AlertDescription>
        </Alert>
      )}
      <div className="flex flex-col gap-6 pb-4">
        {view !== "list" && (
          <div className="-mb-2 flex min-h-9 flex-wrap items-center justify-between gap-2">
            <span className="text-[13px] font-semibold">
              <FilterCount onClear={onClear}>{countText}</FilterCount>
            </span>
            <div className="flex flex-wrap items-center gap-1">{filters}</div>
          </div>
        )}
        {view === "chart" && !none && (
          <Card className="gap-4 p-5">
            <div className="flex flex-col gap-0.5">
              <h2 className="font-semibold">Last 24 hours</h2>
              <p className="text-[13px] text-muted-foreground">How each project did in the last 24 hours. Lines show how fast its monitors answered. Dots show its heartbeats calling in. Red marks trouble.</p>
            </div>
            {!chart.data ? (
              <Skeleton className="h-72 rounded-lg" />
            ) : (
              <OverviewChart
                chart={chart.data.chart}
                links={{
                  line: (id) => `/projects/${chart.data!.slugs[String(id)] ?? ""}`,
                  lane: (id) => `/projects/${chart.data!.slugs[String(id)] ?? ""}`,
                  others: "/",
                  noun: "project",
                  empty: "No monitor checked in the last 24 hours",
                }}
              />
            )}
          </Card>
        )}
        {none && view !== "list" ? (
          <Card className="py-0">{empty}</Card>
        ) : view === "card" ? (
          <div className={GRID} aria-busy={loading}>
            {loading ? <ProjectCardSkeletons count={skeletons} /> : projects.map((p) => <ProjectCard key={p.slug} p={p} />)}
          </div>
        ) : (
          // Under the chart the count and filters already sit above, so the table repeats neither.
          <TableCard title={view === "chart" ? "On this page" : countText} onClear={view === "chart" ? undefined : onClear} tools={view === "chart" ? undefined : filters}>
            {none ? empty : <ProjectTable
                projects={projects}
                loading={loading}
                skeletons={skeletons}
                sort={sort === "name" ? { by: "name", dir: "asc" } : sort === "oldest" ? { by: "created", dir: "asc" } : { by: "created", dir: "desc" }}
                onSort={(by) => go({ sort: by === "name" ? "name" : sort === "oldest" ? "" : "oldest" }, 1, true)}
              />}
          </TableCard>
        )}
        {d && <Pager page={page} pageSize={d.page_size} total={d.total} noun="project" onPage={turn} disabled={loading} />}
      </div>
      <NewProjectSheet open={creating} onOpenChange={setCreating} owners={d?.owners ?? []} />
    </>
  );
}
