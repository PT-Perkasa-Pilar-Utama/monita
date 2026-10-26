import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { CodeCopy } from "@/components/app/CopyField";
import { TableCard } from "@/components/app/TableCard";
import { Plus, Trash2 } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { api, ApiError, errorText } from "@/lib/api";
import { lastSeen, probeCommand } from "@/lib/locations";
import { useMe, Writable } from "@/lib/session";
import { nowSecs } from "@/lib/time";
import type { LocationList } from "@/lib/types";
import { useApi } from "@/lib/useApi";

/** Admin, Locations: other machines that run checks for this Monita (technical-specs 14). */
export function Locations(): ReactElement {
  const res = useApi<LocationList>("/admin/locations");
  const me = useMe();
  const [made, setMade] = useState<{ name: string; token: string } | null>(null);
  const rows = res.data?.locations ?? [];
  const full = res.data ? rows.length >= res.data.max : false;
  const now = nowSecs();
  return (
    <>
      <PageHeader
        title="Locations"
        actions={<Writable><AddLocation disabled={full} onMade={(m) => (setMade(m), res.reload())} /></Writable>}
      />
      <p className="mb-5 max-w-3xl text-sm text-muted-foreground">
        A location is another machine that runs checks for this Monita and reports the results, so a monitor can check from this server and from any locations, and count as failed only when enough of them agree.
      </p>
      {made && (
        <Alert role="status" className="mb-5">
          <AlertTitle>{made.name} is ready, so copy its token now because Monita shows it only once.</AlertTitle>
          <AlertDescription className="mt-3 flex flex-col gap-3">
            <span>On the location's machine, download Monita as on the Install page, then run:</span>
            <CodeCopy id="probe-command" title="Start the probe" code={probeCommand(me.base_url, made.token)} />
          </AlertDescription>
        </Alert>
      )}
      <TableCard title={res.data ? `${rows.length} of ${res.data.max} locations` : "Locations"}>
        {!res.data ? (
          <Skeleton className="m-5 h-24" />
        ) : rows.length === 0 ? (
          <TableEmpty title="No locations yet." description="Every check runs from this server." />
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Token</TableHead>
                <TableHead>Last seen</TableHead>
                <TableHead>Monitors</TableHead>
                <TableHead className="w-12" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map((l) => {
                const seen = lastSeen(l.last_seen_at, now);
                return (
                  <TableRow key={l.id}>
                    <TableCell className="font-medium">{l.name}</TableCell>
                    <TableCell className="font-mono text-muted-foreground">ml_{l.token_prefix}…</TableCell>
                    <TableCell className={seen.stale ? "text-down" : undefined}>{seen.text}</TableCell>
                    <TableCell>{l.monitors}</TableCell>
                    <TableCell>
                      <ConfirmDialog
                        trigger={
                          <Button variant="ghost" size="icon" className="size-8" aria-label={`Delete ${l.name}`}>
                            <Trash2 />
                          </Button>
                        }
                        title={`Delete ${l.name}?`}
                        description="Its probe stops working, its monitors check only from the other places, and its check results are deleted for good."
                        action="Delete location"
                        onConfirm={async () => {
                          await api(`/admin/locations/${l.id}`, "DELETE");
                          res.reload();
                        }}
                      />
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        )}
      </TableCard>
    </>
  );
}

/** The Add location button and its one-field form. */
function AddLocation({ disabled, onMade }: { disabled: boolean; onMade: (m: { name: string; token: string }) => void }): ReactElement {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const made = await api<{ name: string; token: string }>("/admin/locations", "POST", { name });
      setOpen(false);
      setName("");
      onMade(made);
    } catch (err) {
      setError(err instanceof ApiError ? (err.fields.name ?? err.message) : errorText(err));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button disabled={disabled}>
          <Plus data-icon="inline-start" />
          Add location
        </Button>
      </DialogTrigger>
      <DialogContent>
        <form onSubmit={(e) => void submit(e)} className="flex flex-col gap-5">
          <DialogHeader>
            <DialogTitle>Add location</DialogTitle>
            <DialogDescription>Name it after where the machine is, because Monita shows its token only once, after you add it.</DialogDescription>
          </DialogHeader>
          <Field data-invalid={error ? true : undefined}>
            <FieldLabel htmlFor="loc-name" required>
              Name
            </FieldLabel>
            <Input id="loc-name" autoFocus maxLength={40} placeholder="Jakarta" aria-invalid={error ? true : undefined} value={name} onChange={(e) => (setName(e.target.value), setError(null))} />
            {error && <FieldError>{error}</FieldError>}
          </Field>
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button type="submit" disabled={busy}>
              {busy && <Spinner data-icon="inline-start" />}
              Add location
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
