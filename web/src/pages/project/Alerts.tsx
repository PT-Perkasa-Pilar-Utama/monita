import { useState } from "react";
import type { ReactElement } from "react";
import { SettingsPage, SettingsSection } from "@/components/app/SettingsSection";
import { SeverityBadge } from "@/components/app/Tone";
import { Send } from "@/components/icons";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { api, ApiError, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import type { AlertRoute } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

const COPY = {
  critical: {
    title: "Critical alerts",
    description: "Monitors and heartbeats set to critical alert these recipients when an incident opens and when it resolves.",
  },
  warning: {
    title: "Warning alerts",
    description: "Monitors and heartbeats set to warning alert these recipients. A new ticket also calls the webhook.",
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
  const changed = emails.trim() !== saved.emails.join("\n") || webhook.trim() !== (saved.webhook_url ?? "") || useWebhook !== saved.use_webhook;

  const save = async (): Promise<void> => {
    setBusy(true);
    setError(null);
    setNotice("");
    try {
      await api(`/projects/${slug}/alerts/${s}`, "PUT", { emails, webhook_url: webhook, use_webhook: useWebhook });
      const list = emails.split("\n").map((e) => e.trim()).filter(Boolean);
      setSaved({ severity: s, emails: list, webhook_url: webhook.trim() || null, use_webhook: useWebhook });
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
      onSubmit={() => void save()}
      footerNote={changed ? "Save to test the new recipients." : notice}
      footer={
        <>
          <Button type="button" variant="outline" disabled={changed || testing} onClick={() => void test()}>
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
    </SettingsSection>
  );
}

type Preview = {
  event: string;
  label: string;
  severity: "critical" | "warning";
  email: { subject: string; body: string } | null;
  webhook: unknown;
};

const CODE = "overflow-x-auto rounded-lg border bg-paper p-4 font-mono text-[13px] leading-relaxed whitespace-pre-wrap break-words";

/** Critical against warning, and when either fires. */
function HowAlertsWork(): ReactElement {
  return (
    <SettingsSection
      id="how"
      title="How alerts work"
      description="You choose critical or warning on each monitor and heartbeat. Its incidents alert that severity's recipients."
    >
      <div className="flex flex-col gap-4 text-sm">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Severity</TableHead>
              <TableHead>Use it for</TableHead>
              <TableHead>Sends by default</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            <TableRow>
              <TableCell><SeverityBadge severity="critical" /></TableCell>
              <TableCell className="whitespace-normal">An outage that needs someone now, such as the checkout or the login.</TableCell>
              <TableCell>Email and webhook</TableCell>
            </TableRow>
            <TableRow>
              <TableCell><SeverityBadge severity="warning" /></TableCell>
              <TableCell className="whitespace-normal">A problem that can wait for working hours, such as a slow report or a nightly job.</TableCell>
              <TableCell>Email</TableCell>
            </TableRow>
          </TableBody>
        </Table>
        <p className="text-muted-foreground">
          An incident opens after 2 failed checks in a row, a missed heartbeat or a fail ping, and closes on the next good one. Each recipient gets one alert when it opens and
          one when it closes. A manual incident sends no alert; it shows on the status page. A new ticket calls the warning webhook only.
        </p>
      </div>
    </SettingsSection>
  );
}

/** Each kind of alert as Monita sends it, built by the server. */
function WhatRecipientsGet({ slug, webhooks }: { slug: string; webhooks: Record<string, string | null> }): ReactElement {
  const previews = useApi<Preview[]>(`/projects/${slug}/alerts-preview`);
  const [kind, setKind] = useState("incident.opened");
  const p = previews.data?.find((x) => x.event === kind);
  return (
    <SettingsSection id="preview" title="What recipients get" description="Sample alerts for this project. The incident and the ticket are made up.">
      {!previews.data || !p ? (
        <Skeleton className="h-64" />
      ) : (
        <div className="flex flex-col gap-5">
          <ToggleGroup type="single" variant="outline" value={kind} onValueChange={(v) => v && setKind(v)} className="flex-wrap" aria-label="Alert">
            {previews.data.map((x) => (
              <ToggleGroupItem key={x.event} value={x.event} className="px-3">
                {x.label}
              </ToggleGroupItem>
            ))}
          </ToggleGroup>
          <div className="flex flex-col gap-2">
            <h3 className="text-sm font-semibold">Email</h3>
            {p.email ? (
              <div className={CODE}>
                <span className="text-muted-foreground">Subject: </span>
                {p.email.subject}
                {"\n\n"}
                {p.email.body}
              </div>
            ) : (
              <p className="text-sm text-muted-foreground">This alert sends no email.</p>
            )}
          </div>
          <div className="flex flex-col gap-2">
            <h3 className="text-sm font-semibold">Webhook</h3>
            <div className={CODE}>
              <span className="text-muted-foreground">POST </span>
              {webhooks[p.severity] ?? "your webhook URL"}
              {"\n"}
              <span className="text-muted-foreground">Content-Type: application/json</span>
              {"\n\n"}
              {JSON.stringify(p.webhook, null, 2)}
            </div>
            <p className="text-sm text-muted-foreground">
              Goes to the {p.severity} webhook when it is on. <code className="font-mono">at</code> is Unix seconds, UTC.
            </p>
          </div>
        </div>
      )}
    </SettingsSection>
  );
}

function Delivery(): ReactElement {
  return (
    <SettingsSection id="delivery" title="Webhook delivery" description="Alerts leave from a queue, so a slow email server or webhook never delays a check.">
      <ul className="flex list-disc flex-col gap-2 pl-5 text-sm">
        <li>Monita sends a POST with a JSON body. Slack and Google Chat show its <code className="font-mono">text</code> field.</li>
        <li>A 2xx answer within 10 seconds counts as delivered. A redirect counts as a failure.</li>
        <li>A failed call is tried again after 1, 5 and 15 minutes. After the fourth failure Monita gives up and writes the error to its log.</li>
        <li>The webhook URL is a secret. Monita leaves it out of its logs and error messages.</li>
      </ul>
    </SettingsSection>
  );
}

/** Settings, Alerts: who gets critical and warning alerts, and what they get. */
export function Alerts(): ReactElement {
  const { project: p } = useProject();
  const routes = useApi<AlertRoute[]>(`/projects/${p.slug}/alerts`);
  const webhooks = Object.fromEntries((routes.data ?? []).map((r) => [r.severity, r.use_webhook ? r.webhook_url : null]));
  return (
    <SettingsPage
      title="Alerts"
      toc={[
        { id: "how", label: "How alerts work" },
        { id: "critical", label: "Critical alerts" },
        { id: "warning", label: "Warning alerts" },
        { id: "preview", label: "What recipients get" },
        { id: "delivery", label: "Webhook delivery" },
      ]}
    >
      <HowAlertsWork />
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
      <WhatRecipientsGet slug={p.slug} webhooks={webhooks} />
      <Delivery />
    </SettingsPage>
  );
}
