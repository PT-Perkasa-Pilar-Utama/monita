import { useState } from "react";
import type { ReactElement } from "react";
import { CopyField } from "@/components/app/CopyField";
import { IdentificationCard } from "@/components/icons";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Field, FieldLabel } from "@/components/ui/field";
import { Spinner } from "@/components/ui/spinner";
import { api, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import { useMe } from "@/lib/session";
import { toast } from "sonner";

/** Issues a ref ID from the board, for someone who reports a bug outside the app. Shown once. */
export function IssueRefId(): ReactElement {
  const { project: p } = useProject();
  const { base_url: origin } = useMe();
  const [open, setOpen] = useState(false);
  const [singleUse, setSingleUse] = useState(false);
  const [busy, setBusy] = useState(false);
  const [issued, setIssued] = useState<string | null>(null);
  const issue = async (): Promise<void> => {
    setBusy(true);
    try {
      setIssued((await api<{ ref_id: string }>(`/projects/${p.slug}/ref-ids`, "POST", { single_use: singleUse })).ref_id);
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const where = p.pages.report
    ? `They enter it on the Ticket page at ${origin}/status/${p.slug}/ticket to file bugs and check their tickets.`
    : "The Ticket tab is turned off on the status page, so they can use it only through your app.";
  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        setOpen(o);
        if (!o) {
          setIssued(null);
          setSingleUse(false);
        }
      }}
    >
      <DialogTrigger asChild>
        <Button variant="secondary">
          <IdentificationCard data-icon="inline-start" />
          Issue a ref ID
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Issue a ref ID</DialogTitle>
          <DialogDescription>Give it to someone who reports a bug outside your app, such as by phone or chat.</DialogDescription>
        </DialogHeader>
        {issued ? (
          <Alert role="status">
            <AlertTitle>Ref ID issued. Copy it now: Monita shows it only once.</AlertTitle>
            <AlertDescription className="flex flex-col gap-3">
              <CopyField id="issued-ref-id" value={issued} />
              <span>{where}</span>
            </AlertDescription>
          </Alert>
        ) : (
          <Field orientation="horizontal">
            <Checkbox id="ref-single" checked={singleUse} onCheckedChange={(v) => setSingleUse(v === true)} />
            <FieldLabel htmlFor="ref-single" className="font-normal">
              It works for one report only
            </FieldLabel>
          </Field>
        )}
        <DialogFooter>
          <DialogClose asChild>
            <Button variant="ghost">{issued ? "Done" : "Cancel"}</Button>
          </DialogClose>
          {!issued && (
            <Button onClick={() => void issue()} disabled={busy}>
              {busy && <Spinner data-icon="inline-start" />}
              Issue ref ID
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
