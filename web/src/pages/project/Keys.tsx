import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { CopyField } from "@/components/app/CopyField";
import { SettingsPage, SettingsSection } from "@/components/app/SettingsSection";
import { StateBadge } from "@/components/app/Tone";
import { Plus } from "@/components/icons";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { api, ApiError, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import { ago } from "@/lib/time";
import type { ApiKey } from "@/lib/types";
import { useApi } from "@/lib/useApi";

/** Settings, API keys: a project's keys, a new one shown once, and revoke behind a confirm. */
export function Keys(): ReactElement {
  const { project: p } = useProject();
  const keys = useApi<ApiKey[]>(`/projects/${p.slug}/keys`);
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const [newKey, setNewKey] = useState("");

  const create = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const made = await api<{ key: string }>(`/projects/${p.slug}/keys`, "POST", { name });
      setNewKey(made.key);
      setName("");
      setOpen(false);
      keys.reload();
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };

  const list = keys.data ?? [];
  return (
    <>
      <SettingsPage title="API keys">
      {newKey && (
        <Alert>
          <AlertTitle>Key created. Copy it now: Monita shows it only once.</AlertTitle>
          <AlertDescription className="mt-2">
            <CopyField id="new-key" value={newKey} />
          </AlertDescription>
        </Alert>
      )}
      <SettingsSection
        title="Keys"
        description="Your backend sends a key with each call to the Monita API, for example to issue ref IDs."
        action={
          <Button variant="outline" size="sm" onClick={() => setOpen(true)}>
            <Plus data-icon="inline-start" />
            New API key
          </Button>
        }
        flush
      >
        {!keys.data ? (
          <Skeleton className="m-5 h-20" />
        ) : list.length === 0 ? (
          <TableEmpty
            title="Create a key for your backend"
            description="Your backend sends the key with each call to the Monita API, for example to issue ref IDs. Monita shows each key once."
          />
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Key</TableHead>
                <TableHead className="max-sm:hidden">Last used</TableHead>
                <TableHead className="text-right">Status</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {list.map((k) => (
                <TableRow key={k.id}>
                  <TableCell className="font-medium">{k.name}</TableCell>
                  <TableCell className="font-mono">{k.prefix}…</TableCell>
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
                        description="Every call that sends this key gets 401 from now on. You cannot undo this."
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
      </SettingsSection>
      </SettingsPage>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <form onSubmit={(e) => void create(e)} className="flex flex-col gap-6">
            <DialogHeader>
              <DialogTitle>New API key</DialogTitle>
              <DialogDescription>Name the key after the program that uses it. Monita shows the key once, right after you create it.</DialogDescription>
            </DialogHeader>
            <Field data-invalid={error ? true : undefined}>
              <FieldLabel htmlFor="key-name" required>
                Key name
              </FieldLabel>
              <Input id="key-name" autoFocus aria-required aria-invalid={error ? true : undefined} maxLength={60} value={name} onChange={(e) => setName(e.target.value)} />
              {error && <FieldError>{error.fields.name ?? error.message}</FieldError>}
            </Field>
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" variant="ghost">
                  Cancel
                </Button>
              </DialogClose>
              <Button type="submit" disabled={busy}>
                {busy && <Spinner data-icon="inline-start" />}
                Create key
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  );
}
