import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { CopyButton, CopyField } from "@/components/app/CopyField";
import { SettingsPage, SettingsSection } from "@/components/app/SettingsSection";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { api, ApiError, errorText } from "@/lib/api";
import type { TwoFactorSetup, TwoFactorStatus } from "@/lib/types";
import { useReloadMe } from "@/lib/session";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

/** A code field and its submit, for turning two-factor on or off. */
function CodeForm({ id, action, danger = false, onCode }: { id: string; action: string; danger?: boolean; onCode: (code: string) => Promise<void> }): ReactElement {
  const [code, setCode] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      await onCode(code);
    } catch (err) {
      setError(err instanceof ApiError ? (err.fields.code ?? err.message) : errorText(err));
    } finally {
      setBusy(false);
    }
  };
  return (
    <form onSubmit={(e) => void submit(e)} className="flex flex-col gap-3 sm:flex-row sm:items-start">
      <Field data-invalid={error ? true : undefined} className="sm:max-w-72">
        <FieldLabel htmlFor={id} className="sr-only">
          Code
        </FieldLabel>
        <Input id={id} autoComplete="one-time-code" className="font-mono" placeholder="Code from your app" aria-invalid={error ? true : undefined} value={code} onChange={(e) => setCode(e.target.value)} />
        {error && <FieldError>{error}</FieldError>}
      </Field>
      <Button type="submit" variant={danger ? "danger" : "default"} disabled={busy}>
        {busy && <Spinner data-icon="inline-start" />}
        {action}
      </Button>
    </form>
  );
}

/** User settings, Two-factor sign-in: a code from an authenticator app after the password. */
export function TwoFactor(): ReactElement {
  const status = useApi<TwoFactorStatus>("/me/two-factor");
  const reloadMe = useReloadMe();
  const [setup, setSetup] = useState<TwoFactorSetup | null>(null);
  const [codes, setCodes] = useState<string[] | null>(null);
  const s = status.data;

  const start = async (): Promise<void> => {
    try {
      setSetup(await api<TwoFactorSetup>("/me/two-factor", "POST"));
    } catch (err) {
      toast.error(errorText(err));
    }
  };
  const confirm = async (code: string): Promise<void> => {
    const r = await api<{ recovery_codes: string[] }>("/me/two-factor/confirm", "POST", { code });
    setSetup(null);
    setCodes(r.recovery_codes);
    status.reload();
    reloadMe();
  };
  const turnOff = async (code: string): Promise<void> => {
    await api("/me/two-factor/off", "POST", { code });
    toast.success("Two-factor sign-in is off.");
    status.reload();
    reloadMe();
  };

  return (
    <SettingsPage title="Two-factor sign-in">
      {!s ? (
        <Skeleton className="h-56 rounded-xl" />
      ) : codes ? (
        <SettingsSection id="codes" title="Save your recovery codes" description="Each code signs you in once when your phone is not at hand. Monita shows them only now.">
          <Alert>
            <AlertDescription>Store them in your password manager before you leave this page.</AlertDescription>
          </Alert>
          <ol id="recovery-codes" className="grid grid-cols-2 gap-2 font-mono text-sm sm:grid-cols-5">
            {codes.map((c) => (
              <li key={c} className="rounded-md border bg-paper px-2 py-1.5 text-center">
                {c}
              </li>
            ))}
          </ol>
          <div className="flex gap-2">
            <CopyButton value={codes.join("\n")} targetId="recovery-codes" label="Copy codes" />
            <Button type="button" onClick={() => setCodes(null)}>
              I saved them
            </Button>
          </div>
        </SettingsSection>
      ) : s.on ? (
        <SettingsSection id="on" title="Two-factor sign-in is on" description={`Monita asks for a code after your password. ${s.codes_left} of 10 recovery codes left.`}>
          <FieldGroup className="gap-3">
            <p className="text-sm text-muted-foreground">To turn it off, enter a code from your app or a recovery code.</p>
            <CodeForm id="off-code" action="Turn off" danger onCode={turnOff} />
          </FieldGroup>
        </SettingsSection>
      ) : setup ? (
        <SettingsSection id="setup" title="Add Monita to your authenticator app" description="Scan the code with your phone, then enter the 6 digits the app shows. This key works for 10 minutes.">
          <FieldGroup className="gap-4">
            <img id="totp-qr" src={`data:image/svg+xml;utf8,${encodeURIComponent(setup.qr)}`} alt="QR code for your authenticator app" className="w-fit rounded-md border" />
            <p className="text-sm text-muted-foreground">Cannot scan it? Type this key into the app, or open the link on the phone itself.</p>
            <CopyField id="totp-secret" value={setup.secret} />
            <Button asChild variant="outline" className="w-fit">
              <a href={setup.link}>Open in authenticator app</a>
            </Button>
            <CodeForm id="confirm-code" action="Turn on" onCode={confirm} />
          </FieldGroup>
        </SettingsSection>
      ) : (
        <SettingsSection
          id="off"
          title="Two-factor sign-in is off"
          description="With it on, Monita asks for a 6-digit code from an authenticator app after your password."
          footer={<Button onClick={() => void start()}>Turn on</Button>}
        >
          <p className="text-sm text-muted-foreground">Any app that reads time-based codes works, such as 1Password, Google Authenticator or Authy.</p>
        </SettingsSection>
      )}
    </SettingsPage>
  );
}
