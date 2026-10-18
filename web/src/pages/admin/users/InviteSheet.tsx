import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { FormSheet, FormSheetBody } from "@/components/app/FormSheet";
import { Plus } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldGroup, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Spinner } from "@/components/ui/spinner";
import { api, ApiError, errorText } from "@/lib/api";
import type { Invited } from "@/lib/types";

type Role = "tl" | "super_admin";

const ROLES: [Role, string, string][] = [
  ["tl", "Tech lead", "manages the projects they own"],
  ["super_admin", "Super admin", "manages every project, the users and the system settings"],
];

/** Invites a new user as a tech lead or a super admin, in the wide panel. Monita emails the one-time link. */
export function InviteSheet({ onInvited }: { onInvited: (i: Invited) => void }): ReactElement {
  const [open, setOpen] = useState(false);
  const [email, setEmail] = useState("");
  const [role, setRole] = useState<Role>("tl");
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const close = (): void => {
    setOpen(false);
    setError(null);
  };
  const submit = async (_e: FormEvent): Promise<void> => {
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
    <>
      <Button onClick={() => setOpen(true)}>
        <Plus data-icon="inline-start" />
        Invite new user
      </Button>
      <FormSheet open={open} onClose={close}>
        <FormSheetBody
          title="Invite new user"
          description="Monita emails a sign-up link that works once and expires in 72 hours."
          onSubmit={(e) => void submit(e)}
          footer={
            <div className="ml-auto flex gap-2">
              <Button type="button" variant="ghost" onClick={close}>
                Cancel
              </Button>
              <Button type="submit" disabled={busy}>
                {busy && <Spinner data-icon="inline-start" />}
                Send invite
              </Button>
            </div>
          }
        >
          <FieldGroup className="gap-5">
            <Field data-invalid={emailError ? true : undefined}>
              <FieldLabel htmlFor="invite-email" required>
                Email
              </FieldLabel>
              <Input id="invite-email" type="email" placeholder="lead@company.co.id" aria-required aria-invalid={emailError ? true : undefined} value={email} onChange={(e) => setEmail(e.target.value)} />
              {emailError && <FieldError>{emailError}</FieldError>}
            </Field>
            <FieldSet>
              <FieldLegend variant="label">Role</FieldLegend>
              <RadioGroup value={role} onValueChange={(v) => setRole(v as Role)} className="grid gap-3 sm:grid-cols-2">
                {ROLES.map(([v, title, what]) => (
                  <FieldLabel key={v} htmlFor={`role-${v}`}>
                    <Field orientation="horizontal">
                      <RadioGroupItem value={v} id={`role-${v}`} />
                      <span>
                        <strong className="font-semibold">{title}</strong> <span className="text-muted-foreground">{what}</span>
                      </span>
                    </Field>
                  </FieldLabel>
                ))}
              </RadioGroup>
              {error?.fields.role && <FieldError>{error.fields.role}</FieldError>}
            </FieldSet>
          </FieldGroup>
        </FormSheetBody>
      </FormSheet>
    </>
  );
}
