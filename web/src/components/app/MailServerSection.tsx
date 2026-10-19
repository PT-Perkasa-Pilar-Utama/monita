import { useState } from "react";
import type { ComponentProps, ReactElement } from "react";
import { SettingsSection } from "@/components/app/SettingsSection";
import { Plug } from "@/components/icons";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Spinner } from "@/components/ui/spinner";
import { api, ApiError, dropField, errorText } from "@/lib/api";
import type { MailView } from "@/lib/types";
import { toast } from "sonner";

const SECURITY: { value: string; label: string; port: string }[] = [
  { value: "starttls", label: "STARTTLS: start plain, then encrypt (usually port 587)", port: "587" },
  { value: "tls", label: "TLS from the start (usually port 465)", port: "465" },
  { value: "none", label: "No encryption, for a relay inside your network (usually port 25)", port: "25" },
];

type Form = { host: string; port: string; security: string; user: string; password: string; remove_password: boolean; from: string; skip_verify: boolean };

const SOURCE: Record<MailView["source"], string> = {
  saved: "Monita sends alerts, digests and invites through the server saved here.",
  env: "Monita uses MONITA_SMTP_URL from the server's settings file. Save a server here to replace it.",
  none: "No mail server is set, so Monita writes each email to its log. Save a server here to send them.",
};

/** The mail server form on the System page. The password never comes back from the server. */
export function MailServerSection({ mail, onSaved }: { mail: MailView; onSaved: () => void }): ReactElement {
  const initial: Form = {
    host: mail.host ?? "",
    port: mail.port ? String(mail.port) : "587",
    security: mail.security ?? "starttls",
    user: mail.user ?? "",
    password: "",
    remove_password: false,
    from: mail.from,
    skip_verify: mail.skip_verify,
  };
  const [form, setForm] = useState(initial);
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const [testing, setTesting] = useState(false);
  const [tested, setTested] = useState<{ ok: boolean; server?: string; error?: string } | null>(null);
  const err = (k: string): string | undefined => error?.fields[k];
  const set = <K extends keyof Form>(k: K, v: Form[K]): void => {
    setForm((f) => ({ ...f, [k]: v }));
    setError((e) => e && dropField(e, k));
    setTested(null);
  };
  const pickSecurity = (s: string): void => {
    // A port still on another choice's usual value follows the new choice.
    const usual = SECURITY.some((x) => x.port === form.port);
    setForm((f) => ({ ...f, security: s, port: usual ? (SECURITY.find((x) => x.value === s)?.port ?? f.port) : f.port, skip_verify: s === "none" ? false : f.skip_verify }));
  };
  const changed = JSON.stringify(form) !== JSON.stringify(initial);
  const save = async (): Promise<void> => {
    setBusy(true);
    setError(null);
    try {
      await api("/admin/mail", "PUT", form);
      toast.success("Mail server saved. The next email uses it.");
      onSaved();
    } catch (e) {
      setError(e instanceof ApiError ? e : new ApiError(0, "error", errorText(e)));
    } finally {
      setBusy(false);
    }
  };
  const test = async (): Promise<void> => {
    setTesting(true);
    setError(null);
    setTested(null);
    try {
      setTested(await api<{ ok: boolean; server?: string; error?: string }>("/admin/mail/test-connection", "POST", form));
    } catch (e) {
      setError(e instanceof ApiError ? e : new ApiError(0, "error", errorText(e)));
    } finally {
      setTesting(false);
    }
  };
  const clear = async (): Promise<void> => {
    setBusy(true);
    try {
      await api("/admin/mail", "DELETE");
      toast.success("Saved mail server removed.");
      onSaved();
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const field = (k: keyof Form, label: string, input: ReactElement, required = false, className?: string): ReactElement => (
    <Field data-invalid={err(k) ? true : undefined} className={className}>
      <FieldLabel htmlFor={`mail-${k}`} required={required}>
        {label}
      </FieldLabel>
      {input}
      {err(k) && <FieldError>{err(k)}</FieldError>}
    </Field>
  );
  const text = (k: "host" | "port" | "user" | "from", extra: Partial<ComponentProps<typeof Input>> = {}): ReactElement => (
    <Input id={`mail-${k}`} aria-invalid={err(k) ? true : undefined} value={form[k]} onChange={(e) => set(k, e.target.value)} {...extra} />
  );
  return (
    <SettingsSection
      id="mail-server"
      title="Mail server"
      description={SOURCE[mail.source]}
      onSubmit={() => void save()}
      footerNote={error && Object.keys(error.fields).length === 0 ? <span className="text-destructive">{error.message}</span> : undefined}
      footer={
        <>
          {mail.source === "saved" && (
            <Button type="button" variant="ghost" disabled={busy} onClick={() => void clear()}>
              Remove saved server
            </Button>
          )}
          <Button type="button" variant="secondary" disabled={testing || busy} onClick={() => void test()}>
            {testing ? <Spinner data-icon="inline-start" /> : <Plug data-icon="inline-start" />}
            Test connection
          </Button>
          <Button type="submit" disabled={busy || !changed}>
            {busy && <Spinner data-icon="inline-start" />}
            Save changes
          </Button>
        </>
      }
    >
      <FieldGroup className="gap-5">
        <div className="grid gap-5 sm:grid-cols-[minmax(0,1fr)_8rem]">
          {field("host", "Server", text("host", { placeholder: "mail.company.co.id", className: "font-mono", "aria-required": true }), true)}
          {field("port", "Port", text("port", { inputMode: "numeric", "aria-required": true }), true)}
        </div>
        {field(
          "security",
          "How Monita connects",
          <Select value={form.security} onValueChange={pickSecurity}>
            <SelectTrigger id="mail-security" aria-invalid={err("security") ? true : undefined}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                {SECURITY.map((s) => (
                  <SelectItem key={s.value} value={s.value}>
                    {s.label}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>,
          true,
        )}
        <div className="grid gap-5 sm:grid-cols-2">
          {field("user", "User name", text("user", { autoComplete: "off" }))}
          {field(
            "password",
            "Password",
            <Input
              id="mail-password"
              type="password"
              autoComplete="new-password"
              placeholder={mail.has_password ? "Leave blank to keep the saved password" : undefined}
              disabled={form.remove_password}
              value={form.password}
              onChange={(e) => set("password", e.target.value)}
            />,
          )}
        </div>
        {mail.has_password && (
          <Field orientation="horizontal">
            <Checkbox id="mail-remove" checked={form.remove_password} onCheckedChange={(v) => set("remove_password", v === true)} />
            <FieldLabel htmlFor="mail-remove" className="font-normal">
              Remove the saved password
            </FieldLabel>
          </Field>
        )}
        {field("from", "Sends from", text("from", { placeholder: "Monita <monita@company.co.id>", "aria-required": true }), true)}
        <Field orientation="horizontal" data-disabled={form.security === "none" ? true : undefined}>
          <Checkbox id="mail-skip" checked={form.skip_verify} disabled={form.security === "none"} onCheckedChange={(v) => set("skip_verify", v === true)} />
          <FieldLabel htmlFor="mail-skip" className="font-normal">
            The server's certificate may be self-signed
          </FieldLabel>
        </Field>
        {tested && (
          <Alert role="status" variant={tested.ok ? "default" : "destructive"}>
            <AlertTitle>{tested.ok ? `Connected to ${tested.server} and signed in.` : "Monita could not connect."}</AlertTitle>
            <AlertDescription className={tested.ok ? undefined : "font-mono text-[13px] break-all"}>
              {tested.ok ? (changed ? "Save changes to use these settings." : "These are the saved settings.") : tested.error}
            </AlertDescription>
          </Alert>
        )}
      </FieldGroup>
    </SettingsSection>
  );
}
