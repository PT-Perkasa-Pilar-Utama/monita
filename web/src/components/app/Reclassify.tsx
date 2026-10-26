import { useState } from "react";
import type { ReactElement } from "react";
import { tape } from "@/components/app/StateStamp";
import { ChevronDown } from "@/components/icons";
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
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Spinner } from "@/components/ui/spinner";
import { api, errorText } from "@/lib/api";
import { useCanEdit } from "@/lib/session";
import { ago } from "@/lib/time";
import type { IncidentRow } from "@/lib/types";
import { toast } from "sonner";

type Class = "planned" | "false_alarm" | "counted";

export const CLASS_LABEL: Record<"planned" | "false_alarm", string> = { planned: "Planned work", false_alarm: "False alarm" };

/** Each choice's dialog: when to use it and what it changes. */
function explain(c: Class, open: boolean, auto: boolean): { title: string; text: string; action: string } {
  const closes = open ? ` It closes now${auto ? ", and whoever got the first alert gets a resolved one" : ""}.` : "";
  const keeps = " The team still sees it here, and you can count it again later.";
  if (c === "planned")
    return {
      title: "Mark this incident as planned work?",
      text: `Use this when the outage was maintenance, or a pause that came too late, so it leaves uptime, the charts and the status page.${closes}${keeps}`,
      action: "Mark as planned work",
    };
  if (c === "false_alarm")
    return {
      title: "Mark this incident as a false alarm?",
      text: `Use this when the check was wrong and nothing was really down, so it leaves uptime, the charts and the status page.${closes}${keeps}`,
      action: "Mark as false alarm",
    };
  return { title: "Count this incident again?", text: "It counts in uptime, the charts and the status page again, but it stays closed.", action: "Count it again" };
}

/** v1.1 Q46: a menu to reclassify an incident as planned work or a false alarm, or count it again. */
export function Reclassify({ path, incident, onDone }: { path: string; incident: IncidentRow; onDone: () => void }): ReactElement | null {
  const [pick, setPick] = useState<Class | null>(null);
  const [busy, setBusy] = useState(false);
  if (!useCanEdit()) return null;
  const go = async (): Promise<void> => {
    if (!pick) return;
    setBusy(true);
    try {
      await api(`${path}/class`, "PUT", { class: pick });
      toast.success(pick === "counted" ? "The incident counts again." : `Marked as ${CLASS_LABEL[pick].toLowerCase()}.`);
      setPick(null);
      onDone();
    } catch (err) {
      toast.error(errorText(err));
    } finally {
      setBusy(false);
    }
  };
  const d = pick ? explain(pick, incident.resolved_at === null, incident.origin === "auto") : null;
  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="secondary">
            Reclassify
            <ChevronDown data-icon="inline-end" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          {incident.excluded !== "planned" && <DropdownMenuItem onSelect={() => setPick("planned")}>Planned work</DropdownMenuItem>}
          {incident.excluded !== "false_alarm" && <DropdownMenuItem onSelect={() => setPick("false_alarm")}>False alarm</DropdownMenuItem>}
          {incident.excluded !== null && <DropdownMenuItem onSelect={() => setPick("counted")}>Count it again</DropdownMenuItem>}
        </DropdownMenuContent>
      </DropdownMenu>
      <AlertDialog open={pick !== null} onOpenChange={(o) => !o && setPick(null)}>
        <AlertDialogContent onOpenAutoFocus={(e) => e.preventDefault()}>
          <AlertDialogHeader>
            <AlertDialogTitle>{d?.title}</AlertDialogTitle>
            <AlertDialogDescription>{d?.text}</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel autoFocus disabled={busy}>
              Cancel
            </AlertDialogCancel>
            <Button disabled={busy} onClick={() => void go()}>
              {busy && <Spinner data-icon="inline-start" />}
              {d?.action}
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}

/** The note on a reclassified incident: what it is, who marked it, and what that changes. */
export function ClassNotice({ incident }: { incident: IncidentRow }): ReactElement | null {
  if (incident.excluded === null) return null;
  return (
    <div role="status" className="overflow-hidden rounded-xl border bg-card">
      <div aria-hidden="true" className="h-2" style={tape("--neutral-strong")} />
      <p className="px-5 py-3 text-sm text-muted-foreground">
        <span className="mr-2 rounded-md border-2 border-neutral-strong px-2 py-0.5 text-xs font-bold tracking-[0.14em] text-neutral-strong uppercase">{CLASS_LABEL[incident.excluded]}</span>
        Marked by {incident.excluded_by ?? "someone"} {incident.excluded_at ? ago(incident.excluded_at) : ""}, so it does not count in uptime, the charts or the status page.
      </p>
    </div>
  );
}
