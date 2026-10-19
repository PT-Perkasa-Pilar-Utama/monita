import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { FormSheet, FormSheetBody } from "@/components/app/FormSheet";
import { SettingsPage } from "@/components/app/SettingsSection";
import { TableCard } from "@/components/app/TableCard";
import { Plus } from "@/components/icons";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { api, ApiError, dropField, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import { ago } from "@/lib/time";
import type { Secret } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

const usedBy = (n: number): string => (n === 0 ? "Not used" : n === 1 ? "1 monitor" : `${n} monitors`);

/** Settings, Secrets: sealed values a monitor sends, listed by name. A value is never shown again. */
export function Secrets(): ReactElement {
  const { project: p } = useProject();
  const base = `/projects/${p.slug}/secrets`;
  const list = useApi<Secret[]>(base);
  // null: closed; "new": a new secret; a secret: replace its value.
  const [editing, setEditing] = useState<"new" | Secret | null>(null);
  const [name, setName] = useState("");
  const [value, setValue] = useState("");
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const open = (e: "new" | Secret): void => {
    setEditing(e);
    setName("");
    setValue("");
    setError(null);
  };
  const save = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    if (!editing) return;
    setBusy(true);
    setError(null);
    try {
      if (editing === "new") await api(base, "POST", { name, value });
      else await api(`${base}/${editing.id}`, "PUT", { value });
      toast.success(editing === "new" ? `Secret ${name.trim()} saved.` : `New value saved for ${editing.name}.`);
      setEditing(null);
      list.reload();
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };
  const rows = list.data ?? [];
  return (
    <>
      <SettingsPage title="Secrets">
        <TableCard
          title={list.data ? `${rows.length} ${rows.length === 1 ? "secret" : "secrets"}` : "Secrets"}
          tools={
            <Button size="sm" onClick={() => open("new")}>
              <Plus data-icon="inline-start" />
              New secret
            </Button>
          }
        >
          {!list.data ? (
            <Skeleton className="m-5 h-20" />
          ) : rows.length === 0 ? (
            <TableEmpty title="Keep tokens and passwords here" description="A monitor picks a secret under Authentication. Monita sends the value with each check and never shows it again." />
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Used by</TableHead>
                  <TableHead className="max-sm:hidden">Updated</TableHead>
                  <TableHead className="text-right">
                    <span className="sr-only">Actions</span>
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {rows.map((s) => (
                  <TableRow key={s.id}>
                    <TableCell className="font-mono font-medium">{s.name}</TableCell>
                    <TableCell className={s.used_by === 0 ? "text-muted-foreground" : undefined}>{usedBy(s.used_by)}</TableCell>
                    <TableCell className="text-muted-foreground max-sm:hidden">{ago(s.updated_at)}</TableCell>
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-2">
                        <Button variant="outline" size="sm" onClick={() => open(s)}>
                          Replace value
                        </Button>
                        <ConfirmDialog
                          trigger={
                            <Button variant="danger" size="sm" disabled={s.used_by > 0} title={s.used_by > 0 ? `${usedBy(s.used_by)} use it. Change them first.` : undefined}>
                              Delete
                            </Button>
                          }
                          title={`Delete the secret ${s.name}?`}
                          description="Its value is gone for good. No monitor uses it now."
                          action="Delete"
                          onConfirm={async () => {
                            await api(`${base}/${s.id}`, "DELETE");
                            list.reload();
                          }}
                        />
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </TableCard>
      </SettingsPage>
      <FormSheet open={editing !== null} onClose={() => setEditing(null)}>
        <FormSheetBody
          title={editing === "new" || editing === null ? "New secret" : `Replace the value of ${editing.name}`}
          description="Monita never shows the value again. You can replace it or delete it."
          onSubmit={(e) => void save(e)}
          footer={
            <div className="ml-auto flex gap-2">
              <Button type="button" variant="ghost" onClick={() => setEditing(null)}>
                Cancel
              </Button>
              <Button type="submit" disabled={busy}>
                {busy && <Spinner data-icon="inline-start" />}
                {editing === "new" ? "Save secret" : "Save new value"}
              </Button>
            </div>
          }
        >
          <FieldGroup className="gap-5">
            {editing === "new" && (
              <Field data-invalid={error?.fields.name ? true : undefined}>
                <FieldLabel htmlFor="secret-name" required>
                  Name
                </FieldLabel>
                <Input
                  id="secret-name"
                  placeholder="REPORT_API_TOKEN"
                  autoFocus
                  autoComplete="off"
                  className="font-mono"
                  aria-required
                  aria-invalid={error?.fields.name ? true : undefined}
                  value={name}
                  onChange={(e) => {
                    setName(e.target.value);
                    setError((x) => x && dropField(x, "name"));
                  }}
                />
                {error?.fields.name && <FieldError>{error.fields.name}</FieldError>}
              </Field>
            )}
            <Field data-invalid={error?.fields.value ? true : undefined}>
              <FieldLabel htmlFor="secret-value" required>
                Value
              </FieldLabel>
              <Input
                id="secret-value"
                type="password"
                autoComplete="new-password"
                autoFocus={editing !== "new"}
                className="font-mono"
                aria-required
                aria-invalid={error?.fields.value ? true : undefined}
                value={value}
                onChange={(e) => {
                  setValue(e.target.value);
                  setError((x) => x && dropField(x, "value"));
                }}
              />
              {error?.fields.value && <FieldError>{error.fields.value}</FieldError>}
            </Field>
            {error && Object.keys(error.fields).length === 0 && <p className="text-sm text-destructive">{error.message}</p>}
          </FieldGroup>
        </FormSheetBody>
      </FormSheet>
    </>
  );
}
