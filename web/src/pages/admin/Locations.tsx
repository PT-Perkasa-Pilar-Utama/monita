import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { LocationsGuide } from "@/components/app/Guides";
import { LocationMenu, LocationMonitors, LocationSetup, type Setup } from "@/components/app/LocationSetup";
import { TableCard } from "@/components/app/TableCard";
import { Plus } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { TableEmpty } from "@/components/list/TableEmpty";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { api, ApiError, errorText } from "@/lib/api";
import { probeState } from "@/lib/locations";
import { useMe, Writable } from "@/lib/session";
import { nowSecs } from "@/lib/time";
import type { LocationList } from "@/lib/types";
import { useApi } from "@/lib/useApi";

/** Admin, Locations: other machines that run checks for this Monita (technical-specs 14). */
export function Locations(): ReactElement {
  const res = useApi<LocationList>("/admin/locations");
  const me = useMe();
  const [setup, setSetup] = useState<Setup | null>(null);
  const rows = res.data?.locations ?? [];
  const full = res.data ? rows.length >= res.data.max : false;
  const now = nowSecs();
  return (
    <>
      <PageHeader
        title="Locations"
        actions={
          <>
            <LocationsGuide />
            <Writable>
              <AddLocation disabled={full} onMade={(m) => (setSetup(m), res.reload())} />
            </Writable>
          </>
        }
      />
      <p className="mb-5 max-w-3xl text-sm text-muted-foreground">
        A location is another machine, somewhere else, that runs your monitors' checks too, so a fault in this server's own network is not mistaken for an outage when the other places still get through.
      </p>
      <LocationSetup
        setup={setup}
        server={me.base_url}
        onClose={() => {
          setSetup(null);
          res.reload();
        }}
      />
      <TableCard title={res.data ? `${rows.length} of ${res.data.max} locations` : "Locations"}>
        {!res.data ? (
          <Skeleton className="m-5 h-24" />
        ) : rows.length === 0 ? (
          <TableEmpty title="No locations yet" description="Every check runs from this server for now, so add a location on a machine in another network to check from there too." />
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>State</TableHead>
                <TableHead>Checks</TableHead>
                <TableHead className="max-md:hidden">Token</TableHead>
                <TableHead className="w-12" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map((l) => {
                const state = probeState(l.last_seen_at, now);
                return (
                  <TableRow key={l.id}>
                    <TableCell className="font-medium">{l.name}</TableCell>
                    <TableCell>
                      <Badge variant={state.running ? "up" : "down"}>{state.label}</Badge>
                    </TableCell>
                    <TableCell>
                      <LocationMonitors id={l.id} count={l.monitors} />
                    </TableCell>
                    <TableCell className="font-mono text-muted-foreground max-md:hidden">ml_{l.token_prefix}…</TableCell>
                    <TableCell className="text-right">
                      <LocationMenu id={l.id} name={l.name} running={state.running} everSeen={l.last_seen_at !== null} onToken={setSetup} onDeleted={res.reload} />
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
function AddLocation({ disabled, onMade }: { disabled: boolean; onMade: (m: Setup) => void }): ReactElement {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const made = await api<Setup>("/admin/locations", "POST", { name });
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
            <DialogDescription>Name it after where the machine is, and the next step gives you the commands to run there.</DialogDescription>
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
