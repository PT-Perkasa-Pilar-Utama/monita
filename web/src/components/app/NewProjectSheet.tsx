import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { useNavigate } from "react-router-dom";
import { IconPicker } from "@/components/app/IconPicker";
import { UserPicker } from "@/components/app/UserPicker";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { FormSheet, FormSheetBody } from "@/components/app/FormSheet";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, errorText } from "@/lib/api";
import { useMe } from "@/lib/session";
import type { Owner, Project } from "@/lib/types";

/** New project: a name, then for a super admin the owner, and an optional description and icon. */
export function NewProjectSheet({
  open,
  onOpenChange,
  owners,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  owners: Owner[];
}): ReactElement {
  const me = useMe();
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const [owner, setOwner] = useState(me.email);
  const [description, setDescription] = useState("");
  const [icon, setIcon] = useState("");
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const p = await api<Project>("/projects", "POST", { name, owner_email: owner, description, icon });
      onOpenChange(false);
      navigate(`/projects/${p.slug}/monitors`);
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };
  const fieldError = (f: string): string | undefined => error?.fields[f];
  const general = error && Object.keys(error.fields).length === 0 ? error.message : "";

  return (
    <FormSheet open={open} onClose={() => onOpenChange(false)}>
      <FormSheetBody
        title="New project"
        onSubmit={(e) => void submit(e)}
          footer={
            <div className="ml-auto flex gap-2">
              <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
                Cancel
              </Button>
              <Button type="submit" disabled={busy}>
                {busy && <Spinner data-icon="inline-start" />}
                Create project
              </Button>
            </div>
          }
      >
        <div className="flex flex-col gap-5">
          {general && (
            <Alert variant="destructive">
              <AlertDescription>{general}</AlertDescription>
            </Alert>
          )}
          <FieldGroup className="gap-5">
            <Field data-invalid={fieldError("name") ? true : undefined}>
              <FieldLabel htmlFor="project-name" required>
                Name
              </FieldLabel>
              <Input
                id="project-name"
                autoFocus
                aria-required
                aria-invalid={fieldError("name") ? true : undefined}
                maxLength={80}
                placeholder="Payments Gateway"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
              {fieldError("name") && <FieldError>{fieldError("name")}</FieldError>}
            </Field>
            {me.is_admin && (
              <UserPicker label="Owner" users={owners} value={owner} onChange={setOwner} error={fieldError("owner_email")} />
            )}
            <Field data-invalid={fieldError("description") ? true : undefined}>
              <FieldLabel htmlFor="project-description">Description</FieldLabel>
              <Textarea
                id="project-description"
                rows={3}
                maxLength={280}
                placeholder="What this system does and who uses it."
                aria-invalid={fieldError("description") ? true : undefined}
                value={description}
                onChange={(e) => setDescription(e.target.value)}
              />
              {fieldError("description") && <FieldError>{fieldError("description")}</FieldError>}
            </Field>
            <IconPicker value={icon} onChange={setIcon} error={fieldError("icon")} />
          </FieldGroup>
        </div>
      </FormSheetBody>
    </FormSheet>
  );
}
