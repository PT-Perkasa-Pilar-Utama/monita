import { ApiKeysGuide } from "@/components/app/ApiKeysGuide";
import { ChoiceCards } from "@/components/app/ChoiceCards";
import { useState } from "react";
import { Writable } from "@/lib/session";
import type { FormEvent, ReactElement } from "react";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { CopyField } from "@/components/app/CopyField";
import { SettingsPage } from "@/components/app/SettingsSection";
import { TableCard } from "@/components/app/TableCard";
import { StateBadge } from "@/components/app/Tone";
import { Plus } from "@/components/icons";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { FormSheet, FormSheetBody } from "@/components/app/FormSheet";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { api, ApiError, dropField, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import { ago } from "@/lib/time";
import type { ApiKey, KeyScope } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { SortHead } from "@/components/list/SortHead";
import { sortRows } from "@/lib/sort";
import { useSort } from "@/lib/useSort";

const SCOPES: Record<KeyScope, { title: string; description: string }> = {
  ref_ids: { title: "Ref IDs only", description: "Issues ref IDs for your users and nothing else." },
  read: { title: "Read", description: "Reads monitors, heartbeats, incidents and maintenance, but changes nothing." },
  write: { title: "Read and write", description: "Also creates, changes and pauses them, posts incident updates and issues ref IDs." },
};

/** Settings, API keys: a project's keys, a new one shown once, and revoke behind a confirm. */
export function Keys(): ReactElement {
  const { project: p } = useProject();
  const keys = useApi<ApiKey[]>(`/projects/${p.slug}/keys`);
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [scope, setScope] = useState<KeyScope>("ref_ids");
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const [newKey, setNewKey] = useState("");

  const clear = (field: string): void => setError((e) => e && dropField(e, field));

  const create = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const made = await api<{ key: string }>(`/projects/${p.slug}/keys`, "POST", { name, scope });
      setNewKey(made.key);
      setName("");
      setScope("ref_ids");
      setOpen(false);
      keys.reload();
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };

  const [sort, onSort] = useSort(["name", "key", "scope", "used", "status"]);
  const list = sortRows(keys.data ?? [], sort, {
    name: (k) => k.name,
    key: (k) => k.prefix,
    scope: (k) => SCOPES[k.scope].title,
    used: (k) => k.last_used_at,
    status: (k) => (k.revoked_at ? "revoked" : "active"),
  });
  return (
    <>
      <SettingsPage title="API keys" actions={<ApiKeysGuide />}>
      {newKey && (
        <Alert>
          <AlertTitle>Key created, so copy it now because Monita shows it only once.</AlertTitle>
          <AlertDescription className="mt-2">
            <CopyField id="new-key" value={newKey} />
          </AlertDescription>
        </Alert>
      )}
      <TableCard
        title={keys.data ? `${list.length} ${list.length === 1 ? "key" : "keys"}` : "Keys"}
        tools={
          <Writable>
            <Button size="sm" onClick={() => setOpen(true)}>
              <Plus data-icon="inline-start" />
              New API key
            </Button>
          </Writable>
        }
      >
        {!keys.data ? (
          <Skeleton className="m-5 h-20" />
        ) : list.length === 0 ? (
          <TableEmpty
            title="Create a key for your backend"
            description="Your backend sends the key with each call to the Monita API, to issue ref IDs or to manage this project's monitors and incidents, and Monita shows each key only once."
          />
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <SortHead by="name" label="Name" sort={sort} onSort={onSort} />
                <SortHead by="key" label="Key" sort={sort} onSort={onSort} />
                <SortHead by="scope" label="Scope" sort={sort} onSort={onSort} className="max-sm:hidden" />
                <SortHead by="used" label="Last used" sort={sort} onSort={onSort} className="max-sm:hidden" />
                <SortHead by="status" label="Status" sort={sort} onSort={onSort} className="text-right" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {list.map((k) => (
                <TableRow key={k.id}>
                  <TableCell className="font-medium">{k.name}</TableCell>
                  <TableCell className="font-mono">{k.prefix}…</TableCell>
                  <TableCell className="max-sm:hidden">{SCOPES[k.scope].title}</TableCell>
                  <TableCell className="max-sm:hidden">{k.last_used_at ? ago(k.last_used_at) : "never"}</TableCell>
                  <TableCell className="text-right">
                    {k.revoked_at ? (
                      <StateBadge kind="unknown" label="Revoked" />
                    ) : (
                      <ConfirmDialog
                        trigger={
                          <Button variant="danger" size="sm">
                            Revoke
                          </Button>
                        }
                        title={`Revoke the key ${k.name}?`}
                        description="Every call that sends this key gets 401 from now on, and you cannot undo this."
                        action="Revoke"
                        onConfirm={async () => {
                          await api(`/projects/${p.slug}/keys/${k.id}/revoke`, "POST");
                          keys.reload();
                        }}
                      />
                    )}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </TableCard>
      </SettingsPage>
      <FormSheet open={open} onClose={() => setOpen(false)}>
        <FormSheetBody
          title="New API key"
          description="Name the key after the program that uses it, because Monita shows the key once, right after you create it."
          onSubmit={(e) => void create(e)}
          footer={
            <div className="ml-auto flex gap-2">
              <Button type="button" variant="ghost" onClick={() => setOpen(false)}>
                Cancel
              </Button>
              <Button type="submit" disabled={busy}>
                {busy && <Spinner data-icon="inline-start" />}
                Create key
              </Button>
            </div>
          }
        >
            <FieldGroup>
            <Field data-invalid={error && !error.fields.scope ? true : undefined}>
              <FieldLabel htmlFor="key-name" required>
                Key name
              </FieldLabel>
              <Input id="key-name" placeholder="billing-backend" aria-required aria-invalid={error ? true : undefined} maxLength={60} value={name} onChange={(e) => {
                setName(e.target.value);
                clear("name");
              }} />
              {error && !error.fields.scope && <FieldError>{error.fields.name ?? error.message}</FieldError>}
            </Field>
            <ChoiceCards
              id="key-scope"
              legend="What the key may do"
              value={scope}
              onChange={(v) => {
                setScope(v);
                clear("scope");
              }}
              choices={(Object.keys(SCOPES) as KeyScope[]).map((value) => ({ value, ...SCOPES[value] }))}
            />
            {error?.fields.scope && <FieldError>{error.fields.scope}</FieldError>}
            </FieldGroup>
        </FormSheetBody>
      </FormSheet>
    </>
  );
}
