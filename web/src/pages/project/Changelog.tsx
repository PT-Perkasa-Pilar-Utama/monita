import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { useSearchParams } from "react-router-dom";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { FormSheet, FormSheetBody } from "@/components/app/FormSheet";
import { TableCard } from "@/components/app/TableCard";
import { Plus, Trash2 } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { Pager } from "@/components/list/Pager";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldGroup, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, dropField, errorText } from "@/lib/api";
import { CHANGE_BOXES, changeCount, releaseForm } from "@/lib/changelog";
import type { ReleaseForm } from "@/lib/changelog";
import { pageParam } from "@/lib/page";
import { useProject } from "@/lib/project";
import { day, nowSecs } from "@/lib/time";
import type { Release, ReleaseList } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

/** Project, Changelog: the releases the public Changelog tab shows, newest first. A release
 * opens in the side panel to edit or delete. */
export function Changelog(): ReactElement {
  const { project: p, reload: reloadHead } = useProject();
  const base = `/projects/${p.slug}/changelog`;
  const [params, setParams] = useSearchParams();
  const page = pageParam(params.get("page"));
  const list = useApi<ReleaseList>(`${base}?page=${page}`);
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
  const set = (k: keyof ReleaseForm, v: string): void => {
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
      if (editing === "new") await api(base, "POST", form);
      else await api(`${base}/${editing.id}`, "PUT", form);
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
        actions={
          <Button onClick={() => open("new")}>
            <Plus data-icon="inline-start" />
            New release
          </Button>
        }
      />
      <div className="flex flex-col gap-6 pb-4">
        <TableCard title={d ? `${d.total} ${d.total === 1 ? "release" : "releases"}` : "Releases"}>
          {!d ? (
            <Skeleton className="m-4 h-40" />
          ) : d.releases.length === 0 ? (
            <TableEmpty title="Write your first release" description="Each release lists what was added, changed, fixed or removed. Turn on the Changelog tab in Status page settings to show it." />
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Version</TableHead>
                  <TableHead>Title</TableHead>
                  <TableHead className="max-sm:hidden">Released</TableHead>
                  <TableHead className="text-right max-md:hidden">Changes</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {d.releases.map((r) => (
                  <TableRow key={r.id} className="cursor-pointer" onClick={() => open(r)}>
                    <TableCell className="font-mono font-medium">
                      <button type="button" className="hover:underline" onClick={() => open(r)}>
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
        {d && <Pager page={page} pageSize={d.page_size} total={d.total} noun="release" onPage={(n) => setParams({ page: String(n) })} disabled={list.loading} />}
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
                  description="It leaves the public Changelog at once. You cannot undo this."
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
            <FieldSet className="gap-5">
              <FieldLegend>Changes</FieldLegend>
              {CHANGE_BOXES.map(([tag, key]) => (
                <Field key={key} data-invalid={err(key) ? true : undefined}>
                  <FieldLabel htmlFor={`r-${key}`}>{tag}</FieldLabel>
                  <Textarea id={`r-${key}`} rows={3} aria-invalid={err(key) ? true : undefined} placeholder="One change per line" value={form[key]} onChange={(e) => set(key, e.target.value)} />
                  {err(key) && <FieldError>{err(key)}</FieldError>}
                </Field>
              ))}
            </FieldSet>
          </FieldGroup>
        </FormSheetBody>
      </FormSheet>
    </>
  );
}
