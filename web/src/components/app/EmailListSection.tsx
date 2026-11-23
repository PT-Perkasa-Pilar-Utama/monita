import { useState } from "react";
import type { ReactElement } from "react";
import { toast } from "sonner";
import { SettingsSection } from "@/components/app/SettingsSection";
import { Button } from "@/components/ui/button";
import { Field, FieldDescription, FieldError, FieldLabel } from "@/components/ui/field";
import { Spinner } from "@/components/ui/spinner";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, errorText } from "@/lib/api";

/** A list of addresses that gets a PDF (v1.2 Q59, Q63): a switch and the addresses, saved with
 * `PUT path {enabled, emails}`. The weekly and monthly reports and the post-mortems use it. */
export function EmailListSection({
  id,
  title,
  description,
  path,
  saved,
  onSaved,
  switchLabel,
}: {
  id: string;
  title: string;
  description: string;
  path: string;
  saved: { enabled: boolean; emails: string[] };
  onSaved: () => void;
  switchLabel: string;
}): ReactElement {
  const [enabled, setEnabled] = useState(saved.enabled);
  const [emails, setEmails] = useState(saved.emails.join("\n"));
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const changed = enabled !== saved.enabled || emails.trim() !== saved.emails.join("\n");

  const save = async (): Promise<void> => {
    setBusy(true);
    try {
      await api(path, "PUT", { enabled, emails });
      setError(null);
      onSaved();
      toast.success(`${title} saved.`);
    } catch (e) {
      if (e instanceof ApiError && e.fields["emails"]) setError(e.fields["emails"]);
      else toast.error(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <SettingsSection
      id={id}
      title={title}
      description={description}
      onSubmit={() => void save()}
      footer={
        <Button type="submit" disabled={busy || !changed}>
          {busy && <Spinner data-icon="inline-start" />}
          Save changes
        </Button>
      }
    >
      <div className="flex flex-col gap-5">
        <Field orientation="horizontal">
          <Switch id={`${id}-on`} checked={enabled} onCheckedChange={setEnabled} />
          <FieldLabel htmlFor={`${id}-on`}>{switchLabel}</FieldLabel>
        </Field>
        <Field data-invalid={error ? true : undefined} className="max-w-md">
          <FieldLabel htmlFor={`${id}-emails`}>Email addresses</FieldLabel>
          <Textarea
            id={`${id}-emails`}
            rows={3}
            aria-invalid={error ? true : undefined}
            value={emails}
            onChange={(e) => {
              setEmails(e.target.value);
              setError(null);
            }}
          />
          {error ? <FieldError>{error}</FieldError> : <FieldDescription>Enter one address a line, up to 20.</FieldDescription>}
        </Field>
      </div>
    </SettingsSection>
  );
}
