import { MaintenanceGuide } from "@/components/app/Guides";
import { PausedNow } from "@/components/app/PausedNow";
import type { ReactElement } from "react";
import { Link, useNavigate } from "react-router-dom";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { FormSheet } from "@/components/app/FormSheet";
import { ListFilters, type Option } from "@/components/app/ListFilters";
import { TableCard } from "@/components/app/TableCard";
import { Plus, Trash2 } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { Pager } from "@/components/list/Pager";
import { SortHead } from "@/components/list/SortHead";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { api } from "@/lib/api";
import { dateHead, nextDate } from "@/lib/dateSort";
import { useProject } from "@/lib/project";
import { Writable } from "@/lib/session";
import { between } from "@/lib/status";
import type { MaintenancePage, MaintenanceWindow, WindowState } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { useServerList } from "@/lib/useServerList";
import { WindowForm } from "@/pages/project/MaintenanceForm";
import { toast } from "sonner";

// The server works out each window's state from its times.
const STATES: Option[] = [
  { value: "", label: "Any state" },
  { value: "scheduled", label: "Scheduled" },
  { value: "active", label: "Active" },
  { value: "ended", label: "Ended" },
];

const STATE: Record<WindowState, [string, "outline" | "neutral" | "unknown"]> = {
  scheduled: ["Scheduled", "outline"],
  active: ["Active", "neutral"],
  ended: ["Ended", "unknown"],
};

/** A covered item's page, from its `kind:id` value. */
const itemPath = (slug: string, value: string): string => {
  const [kind, id] = value.split(":");
  return `/projects/${slug}/${kind}s/${id}`;
};

/** Delete while scheduled, End now while active; an ended window is a record. */
function WindowAction({ w, base, onDone }: { w: MaintenanceWindow; base: string; onDone: () => void }): ReactElement | null {
  if (w.state === "ended") return null;
  const scheduled = w.state === "scheduled";
  return (
    <ConfirmDialog
      trigger={
        <Button type="button" variant={scheduled ? "danger" : "outline"} size="sm">
          {scheduled && <Trash2 data-icon="inline-start" />}
          {scheduled ? "Delete" : "End now"}
        </Button>
      }
      title={scheduled ? `Delete "${w.title}"?` : `End "${w.title}" now?`}
      description={scheduled ? "The window will not start, and you cannot undo this." : "Failures open incidents and send alerts again from now on."}
      action={scheduled ? "Delete window" : "End now"}
      onConfirm={async () => {
        await (scheduled ? api(`${base}/${w.id}`, "DELETE") : api(`${base}/${w.id}/end`, "POST"));
        toast.success(scheduled ? "Window deleted." : "Window ended.");
        onDone();
      }}
    />
  );
}

/** Project, Maintenance: planned work, then everything paused. While a window is active its items
 * still get checked, but a failure opens no incident and sends no alert. */
export function Maintenance({ creating = false }: { creating?: boolean }): ReactElement {
  const navigate = useNavigate();
  const { project: p } = useProject();
  const base = `/projects/${p.slug}/maintenance`;
  const q = useServerList(`monita:maintenance:${p.slug}`, !creating);
  const list = useApi<MaintenancePage>(`${base}${q.query}`);
  const d = list.data;

  return (
    <>
      <PageHeader
        title="Maintenance"
        filtered={q.filtered}
        actions={
          <>
            <MaintenanceGuide />
            <Writable>
              <Button asChild>
                <Link to={`${base}/new`}>
                  <Plus data-icon="inline-start" />
                  Plan maintenance
                </Link>
              </Button>
            </Writable>
          </>
        }
      />
      <div className="flex flex-col gap-6 pb-4">
        <TableCard
          title={d ? `${d.total} ${d.total === 1 ? "window" : "windows"}` : "Windows"}
          onClear={q.filtered ? q.clear : undefined}
          tools={
            <ListFilters
              noun="windows"
              filter={{ q: q.draft, state: q.params.state, severity: "" }}
              onChange={(c) => (c.q !== undefined ? q.setDraft(c.q) : c.state !== undefined && q.set({ state: c.state }))}
              stateOptions={STATES}
              severity={false}
            />
          }
        >
          {!d ? (
            <Skeleton className="m-5 h-24" />
          ) : d.windows.length === 0 && q.filtered ? (
            <TableEmpty title="" filtered noun="windows" onClear={q.clear} />
          ) : d.windows.length === 0 ? (
            <TableEmpty
              title="Plan maintenance before planned work"
              description="During a window Monita still checks the items, but a failure opens no incident and sends no alert, and the status page shows them as Maintenance."
            />
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Window</TableHead>
                  <SortHead by="starts" label="When" sort={dateHead("starts", q.params.sort, true)} onSort={() => q.set({ sort: nextDate(q.params.sort, true) })} className="max-md:hidden" />
                  <TableHead>State</TableHead>
                  <TableHead className="text-right">
                    <span className="sr-only">Action</span>
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {d.windows.map((w) => (
                  <TableRow key={w.id} data-window={w.id}>
                    <TableCell className="max-w-56 whitespace-normal sm:max-w-96">
                      <div className="font-medium">{w.title}</div>
                      <div className="text-xs text-muted-foreground">
                        {w.items.length === 0
                          ? "Its items were deleted."
                          : w.items.map((i, n) => (
                              <span key={i.value}>
                                {n > 0 && ", "}
                                <Link to={itemPath(p.slug, i.value)} className="underline-offset-2 hover:text-foreground hover:underline">
                                  {i.name}
                                </Link>
                              </span>
                            ))}
                      </div>
                      {w.message && <p className="mt-1 line-clamp-2 text-xs text-muted-foreground">{w.message}</p>}
                    </TableCell>
                    <TableCell className="font-mono text-[13px] text-muted-foreground max-md:hidden">{between(w.starts_at, w.ends_at)}</TableCell>
                    <TableCell>
                      <Badge variant={STATE[w.state][1]}>{STATE[w.state][0]}</Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      <WindowAction w={w} base={base} onDone={list.reload} />
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </TableCard>
        {d && <Pager page={q.params.page} pageSize={d.page_size} total={d.total} noun="window" onPage={(page) => q.set({ page })} disabled={list.loading} />}
        <PausedNow slug={p.slug} />
      </div>
      <FormSheet open={creating} onClose={() => navigate(base)}>
        <WindowForm
          base={base}
          onClose={() => navigate(base)}
          onSaved={() => {
            list.reload();
            navigate(base);
          }}
        />
      </FormSheet>
    </>
  );
}
