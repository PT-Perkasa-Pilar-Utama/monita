import { useState } from "react";
import { Writable } from "@/lib/session";
import type { FormEvent, ReactElement } from "react";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { FormSheet, FormSheetBody } from "@/components/app/FormSheet";
import { ListFilters } from "@/components/app/ListFilters";
import { ReleaseChanges } from "@/components/app/ReleaseChanges";
import { TableCard } from "@/components/app/TableCard";
import { Plus, Trash2 } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { Pager } from "@/components/list/Pager";
import { SortHead } from "@/components/list/SortHead";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { api, ApiError, dropField, errorText } from "@/lib/api";
import { changeCount, releaseBody, releaseForm } from "@/lib/changelog";
import type { ReleaseForm } from "@/lib/changelog";
import { dateHead, nextDate } from "@/lib/dateSort";
import { useProject } from "@/lib/project";
import { day, nowSecs } from "@/lib/time";
import type { Release, ReleaseList } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { useServerList } from "@/lib/useServerList";
import { toast } from "sonner";

/** Project, Changelog: the releases the public Changelog tab shows, newest first. A release
 * opens in the side panel to edit or delete. */
export function Changelog(): ReactElement {
  const { project: p, reload: reloadHead } = useProject();
  const base = `/projects/${p.slug}/changelog`;
  const q = useServerList(`monita:changelog:${p.slug}`);
  const list = useApi<ReleaseList>(`${base}${q.query}`);
  // null: closed; "new": a new release; a release: edit it.
  const [editing, setEditing] = useState<"new" | Release | null>(null);
  const [form, setForm] = useState<ReleaseForm>(releaseForm(null, ""));
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const d = list.data;

  const open = (r: "new" | Release): void => {
    setEditing(r);
    setForm(releaseForm(r === "new" ? null : r, day(nowSecs())));
    setError(null);
  };
  const set = <K extends keyof ReleaseForm>(k: K, v: ReleaseForm[K]): void => {
    setForm((f) => ({ ...f, [k]: v }));
    setError((e) => e && dropField(e, k));
  };
  const err = (k: string): string | undefined => error?.fields[k];
  const done = (): void => {
    setEditing(null);
    list.reload();
    reloadHead();
  };
  const save = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    if (!editing) return;
    setBusy(true);
    setError(null);
    try {
      if (editing === "new") await api(base, "POST", releaseBody(form));
      else await api(`${base}/${editing.id}`, "PUT", releaseBody(form));
      toast.success(`Release ${form.version.trim()} saved.`);
      done();
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <PageHeader
        title="Changelog"
        filtered={q.filtered}
        actions={
          <Writable>
            <Button onClick={() => open("new")}>
              <Plus data-icon="inline-start" />
              New release
            </Button>
          </Writable>
        }
      />
      <div className="flex flex-col gap-6 pb-4">
        <TableCard
          title={d ? `${d.total} ${d.total === 1 ? "release" : "releases"}` : "Releases"}
          onClear={q.filtered ? q.clear : undefined}
          tools={<ListFilters noun="releases" filter={{ q: q.draft, state: "", severity: "" }} onChange={(c) => c.q !== undefined && q.setDraft(c.q)} selects={false} />}
        >
          {!d ? (
            <Skeleton className="m-5 h-24" />
          ) : d.releases.length === 0 && q.filtered ? (
            <TableEmpty title="" filtered noun="releases" onClear={q.clear} />
          ) : d.releases.length === 0 ? (
            <TableEmpty title="Write your first release" description="Each release lists what was added, changed, fixed or removed, and you show it by turning on the Changelog tab in Status page settings." />
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Version</TableHead>
                  <TableHead>Title</TableHead>
                  <SortHead by="released" label="Released" sort={dateHead("released", q.params.sort, true)} onSort={() => q.set({ sort: nextDate(q.params.sort, true) })} className="max-sm:hidden" />
                  <TableHead className="text-right max-md:hidden">Changes</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {d.releases.map((r) => (
                  <TableRow key={r.id} className="cursor-pointer hover:bg-accent" onClick={() => open(r)}>
                    <TableCell className="font-mono font-medium">
                      <button type="button" className="outline-none focus-visible:underline" onClick={() => open(r)}>
                        {r.version}
                      </button>
                    </TableCell>
                    <TableCell className="max-w-56 truncate sm:max-w-96">{r.title}</TableCell>
                    <TableCell className="font-mono text-[13px] text-muted-foreground max-sm:hidden">{r.released_on}</TableCell>
                    <TableCell className="text-right text-muted-foreground max-md:hidden">{changeCount(r)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </TableCard>
        {d && <Pager page={q.params.page} pageSize={d.page_size} total={d.total} noun="release" onPage={(page) => q.set({ page })} disabled={list.loading} />}
      </div>
      <FormSheet open={editing !== null} onClose={() => setEditing(null)}>
        <FormSheetBody
          title={editing === "new" || editing === null ? "New release" : `Release ${editing.version}`}
          onSubmit={(e) => void save(e)}
          footer={
            <>
              {editing !== null && editing !== "new" && (
                <ConfirmDialog
                  trigger={
                    <Button type="button" variant="danger">
                      <Trash2 data-icon="inline-start" />
                      Delete release
                    </Button>
                  }
                  title={`Delete release ${editing.version}?`}
                  description="It leaves the public Changelog at once, and you cannot undo this."
                  action="Delete release"
                  onConfirm={async () => {
                    await api(`${base}/${editing.id}`, "DELETE");
                    toast.success(`Release ${editing.version} deleted.`);
                    done();
                  }}
                />
              )}
              <div className="ml-auto flex gap-2">
                <Button type="button" variant="ghost" onClick={() => setEditing(null)}>
                  Cancel
                </Button>
                <Button type="submit" disabled={busy}>
                  {busy && <Spinner data-icon="inline-start" />}
                  Save release
                </Button>
              </div>
            </>
          }
        >
          <FieldGroup className="gap-5">
            <div className="grid gap-5 sm:grid-cols-2">
              <Field data-invalid={err("version") ? true : undefined}>
                <FieldLabel htmlFor="r-version" required>
                  Version
                </FieldLabel>
                <Input id="r-version" autoFocus aria-required aria-invalid={err("version") ? true : undefined} className="font-mono" placeholder="1.4.0" value={form.version} onChange={(e) => set("version", e.target.value)} />
                {err("version") && <FieldError>{err("version")}</FieldError>}
              </Field>
              <Field data-invalid={err("released_on") ? true : undefined}>
                <FieldLabel htmlFor="r-day" required>
                  Released on
                </FieldLabel>
                <Input id="r-day" type="date" aria-required aria-invalid={err("released_on") ? true : undefined} value={form.released_on} onChange={(e) => set("released_on", e.target.value)} />
                {err("released_on") && <FieldError>{err("released_on")}</FieldError>}
              </Field>
            </div>
            <Field data-invalid={err("title") ? true : undefined}>
              <FieldLabel htmlFor="r-title" required>
                Title
              </FieldLabel>
              <Input id="r-title" maxLength={120} aria-required aria-invalid={err("title") ? true : undefined} placeholder="Faster exports and a new CSV format" value={form.title} onChange={(e) => set("title", e.target.value)} />
              {err("title") && <FieldError>{err("title")}</FieldError>}
            </Field>
            <ReleaseChanges form={form} onChange={set} err={err} />
          </FieldGroup>
        </FormSheetBody>
      </FormSheet>
    </>
  );
}
