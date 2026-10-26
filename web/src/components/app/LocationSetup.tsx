import { useState } from "react";
import type { ReactElement } from "react";
import { Link } from "react-router-dom";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { CodeCopy } from "@/components/app/CopyField";
import { CheckCircle, KeyRound, MoreHorizontal, Trash2 } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { Spinner } from "@/components/ui/spinner";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { api, errorText } from "@/lib/api";
import { setupCommands } from "@/lib/locations";
import { useCanEdit } from "@/lib/session";
import type { LocationList } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

/** A token to set up, from Add location or New token. */
export type Setup = { id: number; name: string; token: string };

/** Polls the list this often while the panel waits for the probe's first call. */
const WAIT_MS = 3000;

/**
 * Sets up a location's machine: the commands for Docker, a Linux service or one run, with this
 * Monita's address and the new token in them, then a line that turns to Connected when the probe
 * first calls. The token shows only here.
 */
export function LocationSetup({ setup, server, onClose }: { setup: Setup | null; server: string; onClose: () => void }): ReactElement {
  const list = useApi<LocationList>(setup ? "/admin/locations" : null, WAIT_MS);
  const row = list.data?.locations.find((l) => l.id === setup?.id);
  const connected = row?.last_seen_at != null;
  const c = setup ? setupCommands(server, setup.token) : null;
  return (
    <Sheet open={setup !== null} onOpenChange={(o) => !o && onClose()}>
      <SheetContent size="half">
        <SheetHeader>
          <SheetTitle>Set up {setup?.name}</SheetTitle>
          <SheetDescription>Run one of these on the machine in {setup?.name}, which only needs to reach this Monita over HTTPS. Copy the token now, because Monita shows it only here.</SheetDescription>
        </SheetHeader>
        {c && (
          <div className="flex flex-col gap-6 overflow-y-auto px-6 py-6 text-sm">
            <Tabs defaultValue="docker" className="gap-4">
              <TabsList>
                <TabsTrigger value="docker">Docker</TabsTrigger>
                <TabsTrigger value="linux">Linux service</TabsTrigger>
                <TabsTrigger value="once">Run once</TabsTrigger>
              </TabsList>
              <TabsContent value="docker" className="flex flex-col gap-3">
                <p className="text-muted-foreground">Any machine with Docker, and the container starts again by itself after a restart.</p>
                <CodeCopy id="setup-docker" title="Start the probe in Docker" code={c.docker} />
              </TabsContent>
              <TabsContent value="linux" className="flex flex-col gap-3">
                <p className="text-muted-foreground">A Linux machine with systemd, where these lines download Monita and install the probe as a service that starts with the machine.</p>
                <CodeCopy id="setup-linux" title="Install the probe as a service" code={c.linux} />
              </TabsContent>
              <TabsContent value="once" className="flex flex-col gap-3">
                <p className="text-muted-foreground">To try it from a terminal with Monita already downloaded, and it stops when you close the terminal.</p>
                <CodeCopy id="setup-once" title="Run the probe once" code={c.once} />
              </TabsContent>
            </Tabs>
            <div role="status" aria-live="polite" className="flex items-center gap-3 rounded-lg border bg-card px-4 py-3">
              {connected ? (
                <>
                  <CheckCircle weight="fill" className="size-5 text-up" />
                  <span>
                    <strong className="font-medium">{setup?.name} is connected.</strong> Tick it under Check from on any monitor to use it.
                  </span>
                </>
              ) : (
                <>
                  <Spinner className="size-4 text-muted-foreground" />
                  <span className="text-muted-foreground">Waiting for the probe in {setup?.name} to call, which it does within a few seconds of starting.</span>
                </>
              )}
            </div>
          </div>
        )}
        {setup && (
          <div className="mt-auto flex flex-wrap items-center gap-2 border-t bg-paper px-6 py-3">
            {!connected && (
              <ConfirmDialog
                trigger={
                  <Button variant="danger">
                    <Trash2 data-icon="inline-start" />
                    Remove {setup.name}
                  </Button>
                }
                title={`Remove ${setup.name}?`}
                description="Its token stops working, its monitors check only from the other places, and its check results are deleted for good."
                action={`Remove ${setup.name}`}
                onConfirm={async () => {
                  await api(`/admin/locations/${setup.id}`, "DELETE");
                  toast.success(`${setup.name} is removed.`);
                  onClose();
                }}
              />
            )}
            <Button className="ml-auto" variant={connected ? "default" : "secondary"} onClick={onClose}>
              {connected ? "Done" : "Set up later"}
            </Button>
          </div>
        )}
      </SheetContent>
    </Sheet>
  );
}

/** The monitors that check from a location, loaded when the count is opened. */
export function LocationMonitors({ id, count }: { id: number; count: number }): ReactElement {
  const [open, setOpen] = useState(false);
  const res = useApi<{ monitors: { id: number; name: string; project: string; slug: string }[]; shown_max: number }>(open ? `/admin/locations/${id}/monitors` : null);
  if (count === 0) return <span className="text-muted-foreground">None</span>;
  return (
    <DropdownMenu open={open} onOpenChange={setOpen}>
      <DropdownMenuTrigger asChild>
        <Button variant="link" className="h-auto p-0">
          {count} {count === 1 ? "monitor" : "monitors"}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="max-h-80 overflow-y-auto">
        {!res.data ? (
          <DropdownMenuLabel className="flex items-center gap-2 font-normal text-muted-foreground">
            <Spinner className="size-3.5" />
            Loading
          </DropdownMenuLabel>
        ) : (
          <>
            {res.data.monitors.map((m) => (
              <DropdownMenuItem key={m.id} asChild>
                <Link to={`/projects/${m.slug}/monitors/${m.id}`} className="flex flex-col items-start gap-0">
                  <span>{m.name}</span>
                  <span className="text-xs text-muted-foreground">{m.project}</span>
                </Link>
              </DropdownMenuItem>
            ))}
            {count > res.data.monitors.length && <DropdownMenuLabel className="font-normal text-muted-foreground">and {count - res.data.monitors.length} more</DropdownMenuLabel>}
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** A location's ⋯ menu: a new token for its machine, or delete it. */
export function LocationMenu({
  id,
  name,
  running,
  everSeen,
  onToken,
  onDeleted,
}: {
  id: number;
  name: string;
  running: boolean;
  /** A probe called with the current token at least once, so a new token would cut it off. */
  everSeen: boolean;
  onToken: (s: Setup) => void;
  onDeleted: () => void;
}): ReactElement | null {
  const [confirm, setConfirm] = useState<"token" | "delete" | null>(null);
  const [busy, setBusy] = useState(false);
  if (!useCanEdit()) return null;
  const newToken = async (): Promise<void> => {
    const r = await api<{ token: string }>(`/admin/locations/${id}/token`, "POST");
    onToken({ id, name, token: r.token });
  };
  // Never seen: no probe holds the token, so a new one breaks nothing and needs no question.
  const setUp = async (): Promise<void> => {
    if (everSeen) return setConfirm("token");
    setBusy(true);
    try {
      await newToken();
    } catch (err) {
      toast.error(errorText(err));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="flex items-center justify-end gap-2">
      {!running && (
        <Button size="sm" variant="secondary" disabled={busy} onClick={() => void setUp()}>
          {busy && <Spinner data-icon="inline-start" />}
          Set up
        </Button>
      )}
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="ghost" size="icon" className="size-8" aria-label={`Actions for ${name}`}>
            <MoreHorizontal />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem onSelect={() => setConfirm("token")}>
            <KeyRound />
            New token
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem variant="destructive" onSelect={() => setConfirm("delete")}>
            <Trash2 />
            Delete
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      <ConfirmDialog
        open={confirm === "token"}
        onOpenChange={(o) => !o && setConfirm(null)}
        title={`Give ${name} a new token?`}
        description="The old token stops working at once, so the probe that uses it stops until you set it up again with the new one, which is how you move the probe or recover a lost token."
        action="New token"
        onConfirm={newToken}
      />
      <ConfirmDialog
        open={confirm === "delete"}
        onOpenChange={(o) => !o && setConfirm(null)}
        title={`Delete ${name}?`}
        description="Its probe stops working, its monitors check only from the other places, and its check results are deleted for good."
        action="Delete location"
        onConfirm={async () => {
          await api(`/admin/locations/${id}`, "DELETE");
          onDeleted();
        }}
      />
    </div>
  );
}
