import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { SettingsPage, SettingsSection } from "@/components/app/SettingsSection";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { api, ApiError, errorText } from "@/lib/api";
import { toast } from "sonner";

type Fields = { current?: string; new?: string; repeat?: string };

/** User settings, Password: the current password, then the new one twice. */
export function Password(): ReactElement {
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [repeat, setRepeat] = useState("");
  const [errors, setErrors] = useState<Fields>({});
  const [busy, setBusy] = useState(false);

  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    if (next !== repeat) {
      setErrors({ repeat: "The two new passwords differ." });
      return;
    }
    setBusy(true);
    setErrors({});
    try {
      await api("/me/password", "POST", { current, new: next });
      setCurrent("");
      setNext("");
      setRepeat("");
      toast.success("Password changed, and your other sessions are signed out.");
    } catch (err) {
      if (err instanceof ApiError && Object.keys(err.fields).length > 0) setErrors(err.fields);
      else toast.error(errorText(err));
    } finally {
      setBusy(false);
    }
  };

  const field = (id: keyof Fields, label: string, value: string, set: (v: string) => void, autoComplete: string): ReactElement => (
    <Field data-invalid={errors[id] ? true : undefined}>
      <FieldLabel htmlFor={`pw-${id}`} required>
        {label}
      </FieldLabel>
      <Input
        id={`pw-${id}`}
        type="password"
        autoComplete={autoComplete}
        aria-required
        aria-invalid={errors[id] ? true : undefined}
        value={value}
        onChange={(e) => {
          set(e.target.value);
          setErrors((f) => ({ ...f, [id]: undefined }));
        }}
      />
      {errors[id] && <FieldError>{errors[id]}</FieldError>}
    </Field>
  );

  return (
    <SettingsPage title="Password">
      <form onSubmit={(e) => void submit(e)}>
        <SettingsSection
          id="password"
          title="Change your password"
          description="Your other sessions sign out when you save a new password of at least 8 characters."
          footer={
            <Button type="submit" disabled={busy || !current || !next || !repeat}>
              {busy && <Spinner data-icon="inline-start" />}
              Change password
            </Button>
          }
        >
          <FieldGroup className="max-w-md gap-5">
            {field("current", "Current password", current, setCurrent, "current-password")}
            {field("new", "New password", next, setNext, "new-password")}
            {field("repeat", "Repeat new password", repeat, setRepeat, "new-password")}
          </FieldGroup>
        </SettingsSection>
      </form>
    </SettingsPage>
  );
}
