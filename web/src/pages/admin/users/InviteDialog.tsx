import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { Plus } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Field, FieldError, FieldGroup, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { api, ApiError, errorText } from "@/lib/api";
import type { Invited } from "@/lib/types";

/** Invites a new user as a tech lead or a super admin. Monita emails the one-time link. */
export function InviteDialog({ onInvited }: { onInvited: (i: Invited) => void }): ReactElement {
  const [open, setOpen] = useState(false);
  const [email, setEmail] = useState("");
  const [role, setRole] = useState<"tl" | "super_admin">("tl");
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      onInvited(await api<Invited>("/admin/users", "POST", { email, role }));
      setEmail("");
      setRole("tl");
      setOpen(false);
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };
  const emailError = error?.fields.email ?? (error && !error.fields.role ? error.message : undefined);
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>
          <Plus data-icon="inline-start" />
          Invite new user
        </Button>
      </DialogTrigger>
      <DialogContent>
        <form onSubmit={(e) => void submit(e)} noValidate className="flex flex-col gap-6">
          <DialogHeader>
            <DialogTitle>Invite new user</DialogTitle>
            <DialogDescription>Monita emails a sign-up link that works once and expires in 72 hours.</DialogDescription>
          </DialogHeader>
          <FieldGroup className="gap-5">
            <Field data-invalid={emailError ? true : undefined}>
              <FieldLabel htmlFor="invite-email" required>
                Email
              </FieldLabel>
              <Input id="invite-email" type="email" autoFocus placeholder="lead@company.co.id" aria-required aria-invalid={emailError ? true : undefined} value={email} onChange={(e) => setEmail(e.target.value)} />
              {emailError && <FieldError>{emailError}</FieldError>}
            </Field>
            <FieldSet>
              <FieldLegend variant="label">Role</FieldLegend>
              <ToggleGroup type="single" variant="outline" value={role} onValueChange={(v) => v && setRole(v as "tl" | "super_admin")}>
                <ToggleGroupItem value="tl" className="px-3">
                  Tech lead
                </ToggleGroupItem>
                <ToggleGroupItem value="super_admin" className="px-3">
                  Super admin
                </ToggleGroupItem>
              </ToggleGroup>
              <p className="text-[13px] text-muted-foreground">
                {role === "tl" ? "A tech lead manages the projects they own." : "A super admin manages every project, the users and the system settings."}
              </p>
            </FieldSet>
          </FieldGroup>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="ghost">
                Cancel
              </Button>
            </DialogClose>
            <Button type="submit" disabled={busy}>
              {busy && <Spinner data-icon="inline-start" />}
              Send invite
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
