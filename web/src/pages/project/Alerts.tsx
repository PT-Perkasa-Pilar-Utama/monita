import { useState } from "react";
import type { ReactElement } from "react";
import { AlertsGuide } from "@/components/app/AlertsGuide";
import { AlertSwitch } from "@/components/app/AlertSwitch";
import { SettingsPage, SettingsSection } from "@/components/app/SettingsSection";
import { TicketAlertsSection } from "@/components/app/TicketAlertsSection";
import { Send } from "@/components/icons";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import type { AlertRoute, TicketAlerts } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

const COPY = {
  critical: {
    title: "Critical alerts",
    description: "Monitors and heartbeats set to critical alert these recipients when an incident opens and when it resolves.",
  },
  warning: {
    title: "Warning alerts",
    description: "Monitors and heartbeats set to warning alert these recipients when an incident opens and when it resolves.",
  },
} as const;

function testNotice(n: number): string {
  if (n === 0) return "No saved recipients get this test.";
  return n === 1 ? "Test sent to 1 recipient." : `Test sent to ${n} recipients.`;
}

function RouteSection({ slug, route }: { slug: string; route: AlertRoute }): ReactElement {
  const s = route.severity;
  const [saved, setSaved] = useState(route);
  const [emails, setEmails] = useState(route.emails.join("\n"));
  const [webhook, setWebhook] = useState(route.webhook_url ?? "");
  const [useWebhook, setUseWebhook] = useState(route.use_webhook);
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const [testing, setTesting] = useState(false);
  const [notice, setNotice] = useState("");
  const [enabled, setEnabled] = useState(route.enabled);
  const changed = emails.trim() !== saved.emails.join("\n") || webhook.trim() !== (saved.webhook_url ?? "") || useWebhook !== saved.use_webhook;

  const save = async (): Promise<void> => {
    setBusy(true);
    setError(null);
    setNotice("");
    try {
      await api(`/projects/${slug}/alerts/${s}`, "PUT", { emails, webhook_url: webhook, use_webhook: useWebhook });
      const list = emails.split("\n").map((e) => e.trim()).filter(Boolean);
      setSaved({ severity: s, emails: list, webhook_url: webhook.trim() || null, use_webhook: useWebhook, enabled });
      setEmails(list.join("\n"));
      setWebhook(webhook.trim());
      toast.success(`${COPY[s].title} saved.`);
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };
  const test = async (): Promise<void> => {
    setTesting(true);
    setNotice("");
    try {
      const { sent_to: n } = await api<{ sent_to: number }>(`/projects/${slug}/alerts/${s}/test`, "POST");
      setNotice(testNotice(n));
    } catch (err) {
      toast.error(errorText(err));
    } finally {
      setTesting(false);
    }
  };

  return (
    <SettingsSection
      id={s}
      title={COPY[s].title}
      description={COPY[s].description}
      action={<AlertSwitch slug={slug} route={s} name={COPY[s].title} on={enabled} onChange={setEnabled} />}
      onSubmit={() => void save()}
      footerNote={!enabled ? "Off. Nothing is sent. The recipients stay for when you turn it back on." : changed ? "Save to test the new recipients." : notice}
      footer={
        <>
          <Button type="button" variant="outline" disabled={!enabled || changed || testing} onClick={() => void test()}>
            {testing ? <Spinner data-icon="inline-start" /> : <Send data-icon="inline-start" />}
            Send a test
          </Button>
          <Button type="submit" disabled={busy || !changed}>
            {busy && <Spinner data-icon="inline-start" />}
            Save changes
          </Button>
        </>
      }
    >
      <fieldset disabled={!enabled} className="contents">
      <FieldGroup className="gap-5">
        <Field data-invalid={error?.fields.emails ? true : undefined}>
          <FieldLabel htmlFor={`emails-${s}`}>Email addresses, one per line</FieldLabel>
          <Textarea
            id={`emails-${s}`}
            rows={3}
            placeholder={"oncall@company.co.id\nlead@company.co.id"}
            aria-invalid={error?.fields.emails ? true : undefined}
            value={emails}
            onChange={(e) => setEmails(e.target.value)}
          />
          {error?.fields.emails && <FieldError>{error.fields.emails}</FieldError>}
        </Field>
        <Field orientation="horizontal">
          <Checkbox id={`use-${s}`} checked={useWebhook} onCheckedChange={(v) => setUseWebhook(v === true)} />
          <FieldLabel htmlFor={`use-${s}`} className="font-normal">
            Also send to a webhook
          </FieldLabel>
        </Field>
        {useWebhook && (
          <Field data-invalid={error?.fields.webhook_url ? true : undefined}>
            <FieldLabel htmlFor={`webhook-${s}`} required>
              Webhook URL
            </FieldLabel>
            <Input
              id={`webhook-${s}`}
              type="url"
              placeholder="https://hooks.slack.com/services/…"
              aria-invalid={error?.fields.webhook_url ? true : undefined}
              value={webhook}
              onChange={(e) => setWebhook(e.target.value)}
            />
            {error?.fields.webhook_url && <FieldError>{error.fields.webhook_url}</FieldError>}
          </Field>
        )}
        {error && !error.fields.emails && !error.fields.webhook_url && <p className="text-sm text-destructive">{error.message}</p>}
      </FieldGroup>
      </fieldset>
    </SettingsSection>
  );
}

/** Settings, Alerts: who gets critical, warning and ticket alerts. The guide opens beside it. */
export function Alerts(): ReactElement {
  const { project: p } = useProject();
  const routes = useApi<AlertRoute[]>(`/projects/${p.slug}/alerts`);
  const tickets = useApi<TicketAlerts>(`/projects/${p.slug}/ticket-alerts`);
  const [ticketHook, setTicketHook] = useState<TicketAlerts | null>(null);
  const t = ticketHook ?? tickets.data;
  const webhooks: Record<string, string | null> = {
    ...Object.fromEntries((routes.data ?? []).map((r) => [r.severity, r.use_webhook ? r.webhook_url : null])),
    ticket: t?.use_webhook || t?.webhook_moves ? t.webhook_url : null,
  };
  return (
    <SettingsPage
      title="Alerts"
      actions={<AlertsGuide slug={p.slug} webhooks={webhooks} />}
      toc={[
        { id: "critical", label: "Critical alerts" },
        { id: "warning", label: "Warning alerts" },
        { id: "tickets", label: "Ticket alerts" },
      ]}
    >
      {routes.error && (
        <Alert variant="destructive">
          <AlertDescription>{routes.error.message}</AlertDescription>
        </Alert>
      )}
      {!routes.data ? (
        <>
          <Skeleton className="h-72 rounded-xl" />
          <Skeleton className="h-72 rounded-xl" />
        </>
      ) : (
        routes.data.map((r) => <RouteSection key={r.severity} slug={p.slug} route={r} />)
      )}
      {tickets.data ? (
        <TicketAlertsSection slug={p.slug} owner={p.owner_email} initial={tickets.data} onSaved={setTicketHook} />
      ) : (
        <Skeleton className="h-56 rounded-xl" />
      )}
    </SettingsPage>
  );
}
