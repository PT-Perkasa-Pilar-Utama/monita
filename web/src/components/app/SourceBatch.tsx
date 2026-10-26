import { useState } from "react";
import type { ReactElement } from "react";
import { Link, useNavigate } from "react-router-dom";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { Edit, MoreHorizontal, Pause, Play, Trash2, Wrench } from "@/components/icons";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { api, errorText } from "@/lib/api";
import { useCanEdit } from "@/lib/session";
import { confirmPhrase, maintenancePath, noun, shownNames, type Picked, type SourceKind } from "@/lib/sources";
import { toast } from "sonner";

/** Pause, resume or delete these sources in one all-or-nothing call. */
export async function runBatch(slug: string, kind: SourceKind, action: "pause" | "resume" | "delete", ids: number[]): Promise<void> {
  await api(`/projects/${slug}/${kind}s/batch`, "POST", { action, ids });
}

const PAUSE_TEXT: Record<SourceKind, string> = {
  monitor: "Monita stops checking them and sends no alerts until you resume them, closes any open incident without an alert, and hides them from the status page in the meantime.",
  heartbeat: "Monita keeps their pings but stops watching their deadlines and sends no alerts until you resume them, closes any open incident without an alert, and hides them from the status page in the meantime.",
};

/** The ⋯ menu at the end of a row. Delete opens the page's dialog, so the row never holds it. */
export function SourceRowMenu({
  slug,
  kind,
  base,
  item,
  onDone,
  onDelete,
}: {
  slug: string;
  kind: SourceKind;
  base: string;
  item: Picked;
  onDone: () => void;
  onDelete: (items: Picked[]) => void;
}): ReactElement | null {
  const navigate = useNavigate();
  if (!useCanEdit()) return null;
  const toggle = async (): Promise<void> => {
    const action = item.paused ? "resume" : "pause";
    try {
      await runBatch(slug, kind, action, [item.id]);
      toast.success(item.paused ? `${item.name} runs again.` : `${item.name} is paused.`, {
        action: { label: "Undo", onClick: () => void runBatch(slug, kind, item.paused ? "pause" : "resume", [item.id]).then(onDone) },
      });
      onDone();
    } catch (err) {
      toast.error(errorText(err));
    }
  };
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" className="size-8" aria-label={`Actions for ${item.name}`}>
          <MoreHorizontal />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onSelect={() => navigate(`${base}/${item.id}/edit`)}>
          <Edit />
          Edit
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={() => void toggle()}>
          {item.paused ? <Play /> : <Pause />}
          {item.paused ? "Resume" : "Pause"}
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={() => navigate(maintenancePath(slug, kind, [item.id]))}>
          <Wrench />
          Plan maintenance
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem variant="destructive" onSelect={() => onDelete([item])}>
          <Trash2 />
          Delete
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** The selection's actions, in the list's title band while anything is picked. */
export function BatchBar({
  slug,
  kind,
  picked,
  onClear,
  onDone,
  onDelete,
}: {
  slug: string;
  kind: SourceKind;
  picked: Picked[];
  onClear: () => void;
  onDone: () => void;
  onDelete: (items: Picked[]) => void;
}): ReactElement {
  const [busy, setBusy] = useState(false);
  const ids = picked.map((p) => p.id);
  const n = picked.length;
  const resume = async (): Promise<void> => {
    setBusy(true);
    try {
      await runBatch(slug, kind, "resume", ids);
      toast.success(`${n} ${noun(kind, n)} run again.`);
      onClear();
      onDone();
    } catch (err) {
      toast.error(errorText(err));
    } finally {
      setBusy(false);
    }
  };
  return (
    <span className="flex flex-wrap items-center gap-2">
      <span aria-live="polite">{n} selected</span>
      <ConfirmDialog
        trigger={
          <Button size="sm" variant="secondary" disabled={busy}>
            <Pause data-icon="inline-start" />
            Pause
          </Button>
        }
        title={`Pause ${n} ${noun(kind, n)}?`}
        description={PAUSE_TEXT[kind]}
        action="Pause"
        onConfirm={async () => {
          await runBatch(slug, kind, "pause", ids);
          toast.success(`${n} ${noun(kind, n)} paused.`);
          onClear();
          onDone();
        }}
      />
      {picked.some((p) => p.paused) && (
        <Button size="sm" variant="secondary" disabled={busy} onClick={() => void resume()}>
          {busy ? <Spinner data-icon="inline-start" /> : <Play data-icon="inline-start" />}
          Resume
        </Button>
      )}
      <Button size="sm" variant="secondary" asChild>
        <Link to={maintenancePath(slug, kind, ids)}>
          <Wrench data-icon="inline-start" />
          Plan maintenance
        </Link>
      </Button>
      <Button size="sm" variant="danger" disabled={busy} onClick={() => onDelete(picked)}>
        <Trash2 data-icon="inline-start" />
        Delete
      </Button>
      <Button size="sm" variant="ghost" disabled={busy} onClick={onClear}>
        Clear selection
      </Button>
    </span>
  );
}

function lost(kind: SourceKind, one: boolean): string {
  const [its, it] = one ? ["Its", "it leaves"] : ["Their", "they leave"];
  return kind === "monitor"
    ? `${its} check history, response times and charts go for good, and ${it} the status page and every maintenance window.`
    : `${its} ping history, ping ${one ? "URL" : "URLs"} and charts go for good, so a job that still calls ${one ? "it" : "one"} gets 404, and ${it} the status page and every maintenance window.`;
}

/** Why deleting closes open incidents, in words that fit how many are down. */
function openIncidents(down: number, n: number): string {
  const tail = "and everyone who got the alert gets a resolved alert.";
  if (n === 1) return `It is down right now, so its open incident closes ${tail}`;
  if (down === 1) return `1 of them is down right now, so its open incident closes ${tail}`;
  return `${down} of them are down right now, so their open incidents close ${tail}`;
}

/** Asks before deleting monitors or heartbeats, and says what goes, what stays and how to pause
 * instead. More than one also asks the user to type `delete N`. */
export function DeleteSourcesDialog({
  slug,
  kind,
  items,
  onClose,
  onDone,
}: {
  slug: string;
  kind: SourceKind;
  items: Picked[] | null;
  onClose: () => void;
  onDone: () => void;
}): ReactElement {
  const [typed, setTyped] = useState("");
  const [busy, setBusy] = useState(false);
  const list = items ?? [];
  const n = list.length;
  const phrase = confirmPhrase(n);
  const ready = n === 1 || typed.trim().toLowerCase() === phrase;
  const down = list.filter((i) => i.state === "down" || i.state === "issue").length;
  const { shown, more } = shownNames(list.map((i) => i.name));
  const close = (): void => {
    setTyped("");
    onClose();
  };
  const go = async (action: "delete" | "pause"): Promise<void> => {
    setBusy(true);
    try {
      await runBatch(slug, kind, action, list.map((i) => i.id));
      toast.success(action === "delete" ? `${n} ${noun(kind, n)} deleted.` : `${n} ${noun(kind, n)} paused instead.`);
      close();
      onDone();
    } catch (err) {
      toast.error(errorText(err));
    } finally {
      setBusy(false);
    }
  };
  const title = n === 1 ? `Delete the ${kind} ${list[0]?.name}?` : `Delete ${n} ${noun(kind, n)}?`;
  return (
    <AlertDialog open={items !== null} onOpenChange={(o) => !o && close()}>
      <AlertDialogContent onOpenAutoFocus={(e) => e.preventDefault()}>
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription>
            You cannot undo this. If you only need {n === 1 ? "it" : "them"} quiet for a while, pause {n === 1 ? "it" : "them"} instead, which keeps everything.
          </AlertDialogDescription>
        </AlertDialogHeader>
        {n > 1 && (
          <ul className="max-h-40 list-disc overflow-y-auto rounded-lg border bg-paper py-2 pr-3 pl-8 text-sm">
            {shown.map((name, i) => (
              <li key={`${name}-${i}`}>{name}</li>
            ))}
            {more > 0 && <li className="text-muted-foreground">and {more} more</li>}
          </ul>
        )}
        <dl className="grid gap-3 text-sm">
          <div>
            <dt className="font-medium">What goes</dt>
            <dd className="text-muted-foreground">{lost(kind, n === 1)}</dd>
          </div>
          <div>
            <dt className="font-medium">What stays</dt>
            <dd className="text-muted-foreground">Past incidents stay in the incident list, named as a deleted {kind}.</dd>
          </div>
          {down > 0 && (
            <div>
              <dt className="font-medium text-down">Open incidents</dt>
              <dd className="text-muted-foreground">{openIncidents(down, n)}</dd>
            </div>
          )}
        </dl>
        {n > 1 && (
          <Field>
            <FieldLabel htmlFor="confirm-delete">
              Type <span className="font-mono">{phrase}</span> to confirm
            </FieldLabel>
            <Input id="confirm-delete" autoComplete="off" value={typed} onChange={(e) => setTyped(e.target.value)} />
          </Field>
        )}
        <AlertDialogFooter>
          <AlertDialogCancel autoFocus disabled={busy}>
            Cancel
          </AlertDialogCancel>
          <Button variant="secondary" disabled={busy} onClick={() => void go("pause")}>
            <Pause data-icon="inline-start" />
            Pause instead
          </Button>
          <Button variant="destructive" disabled={busy || !ready} onClick={() => void go("delete")}>
            {busy && <Spinner data-icon="inline-start" />}
            Delete {n === 1 ? kind : `${n} ${noun(kind, n)}`}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
