import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { ChoiceCards } from "@/components/app/ChoiceCards";
import { FormSheet, FormSheetBody } from "@/components/app/FormSheet";
import { Plus } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { api, ApiError, errorText } from "@/lib/api";
import type { Invited } from "@/lib/types";

type Role = "tl" | "super_admin";


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
            <ChoiceCards
              id="role"
              legend="Role"
              value={role}
              onChange={setRole}
              choices={[
                { value: "tl", title: "Tech lead", description: "Manages the projects they own." },
                { value: "super_admin", title: "Super admin", description: "Manages every project, the users and the system settings." },
              ]}
            />
            {error?.fields.role && <FieldError>{error.fields.role}</FieldError>}
          </FieldGroup>
        </FormSheetBody>
      </FormSheet>
    </>
  );
}
