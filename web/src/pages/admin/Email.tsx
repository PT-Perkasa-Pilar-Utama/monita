import { useState } from "react";
import type { ReactElement } from "react";
import { Facts } from "@/components/app/Facts";
import { MailServerSection } from "@/components/app/MailServerSection";
import { SettingsSection } from "@/components/app/SettingsSection";
import { StateBadge } from "@/components/app/Tone";
import { PageHeader } from "@/components/layout/Page";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Send } from "@/components/icons";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { api, ApiError, dropField, errorText } from "@/lib/api";
import { useMe } from "@/lib/session";
import { ago } from "@/lib/time";
import type { SystemView } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { cn } from "@/lib/utils";

type Sent = { ok: boolean; to: string; source: "saved" | "env" | "none"; error?: string };

/** Sends one email now through the mail server in use, to any address, and says what happened. */
function TestSendSection(): ReactElement {
  const me = useMe();
  const [to, setTo] = useState(me.email);
  const [busy, setBusy] = useState(false);
  const [sent, setSent] = useState<Sent | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const send = async (): Promise<void> => {
    setBusy(true);
    setSent(null);
    setError(null);
    try {
      setSent(await api<Sent>("/admin/mail/test-send", "POST", { to }));
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };
  return (
    <SettingsSection
      id="test-send"
      title="Send a test email"
      description="Monita sends it now through the mail server in use, so you see the result here."
      onSubmit={() => void send()}
      footer={
        <Button type="submit" variant="secondary" disabled={busy}>
          {busy ? <Spinner data-icon="inline-start" /> : <Send data-icon="inline-start" />}
          Send test email
        </Button>
      }
    >
      <FieldGroup className="gap-5">
        <Field data-invalid={error?.fields.to ? true : undefined} className="max-w-md">
          <FieldLabel htmlFor="test-to" required>
            Send it to
          </FieldLabel>
          <Input
            id="test-to"
            type="email"
            aria-required
            aria-invalid={error?.fields.to ? true : undefined}
            value={to}
            onChange={(e) => {
              setTo(e.target.value);
              setError((x) => x && dropField(x, "to"));
            }}
          />
          {error?.fields.to && <FieldError>{error.fields.to}</FieldError>}
        </Field>
        {sent && (
          <Alert role="status" variant={sent.ok ? "default" : "destructive"}>
            <AlertTitle>
              {!sent.ok ? `The test email to ${sent.to} did not go out.` : sent.source === "none" ? "No mail server is set, so the test went to Monita's log." : `Sent to ${sent.to}.`}
            </AlertTitle>
            <AlertDescription className={sent.ok ? undefined : "font-mono text-[13px] break-all"}>
              {!sent.ok ? sent.error : sent.source === "none" ? "Save a mail server above to send real email." : "Check that inbox, and its spam folder."}
            </AlertDescription>
          </Alert>
        )}
        {error && !error.fields.to && <p className="text-sm text-destructive">{error.message}</p>}
      </FieldGroup>
    </SettingsSection>
  );
}

function DeliverySection({ d }: { d: SystemView }): ReactElement {
  const e = d.email;
  const counts = [
    [e.sent, `Sent in ${d.email_days} days`, false],
    [e.waiting, "Waiting or retrying", false],
    [e.failed, "Gave up", e.failed > 0],
  ] as const;
  return (
    <SettingsSection
      title="Email delivery"
      description={d.mail.server ? "Alerts, digests and invites that went out in the last days." : "No mail server is set, so Monita writes each email to its log."}
      action={d.mail.server ? <StateBadge kind="up" label="Sending" /> : <StateBadge kind="issue" label="Log only" />}
    >
      <div className="flex flex-col gap-5">
        <Facts
          rows={[
            ["Mail server", <span className="font-mono text-[13px]">{d.mail.server ?? "Not set"}</span>],
            ["Last sent", e.last_sent_at ? ago(e.last_sent_at) : "Nothing sent yet"],
          ]}
        />
        <dl className="grid grid-cols-3 divide-x rounded-lg border">
          {counts.map(([n, label, bad]) => (
            <div key={label} className="flex flex-col gap-0.5 px-4 py-3">
              <dt className="text-xs text-muted-foreground">{label}</dt>
              <dd className={cn("text-lg font-semibold tabular-nums", bad && "text-down")}>{n.toLocaleString()}</dd>
            </div>
          ))}
        </dl>
        {e.last_error && (
          <Alert variant="destructive">
            <AlertTitle>Last error, {ago(e.last_error[1])}</AlertTitle>
            <AlertDescription className="font-mono text-[13px] break-all">{e.last_error[0]}</AlertDescription>
          </Alert>
        )}
      </div>
    </SettingsSection>
  );
}

/** Admin, Email: the mail server alerts go out through, and what went out. */
export function Email(): ReactElement {
  const res = useApi<SystemView>("/admin/system");
  const d = res.data;
  return (
    <>
      <PageHeader title="Email" />
      {!d ? (
        <Skeleton className="h-96 rounded-xl" />
      ) : (
        <div className="flex flex-col gap-6">
          <MailServerSection key={JSON.stringify(d.mail)} mail={d.mail} onSaved={res.reload} />
          <TestSendSection />
          <DeliverySection d={d} />
        </div>
      )}
    </>
  );
}
