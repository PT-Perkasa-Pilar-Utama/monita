import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { SeverityBadge } from "@/components/app/Tone";
import { Send } from "@/components/icons";
import { PageHeader } from "@/components/layout/Page";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardAction, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, errorText } from "@/lib/api";
import { useProject } from "@/lib/project";
import type { AlertRoute } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

function RouteCard({ slug, route }: { slug: string; route: AlertRoute }): ReactElement {
  const s = route.severity;
  const [emails, setEmails] = useState(route.emails.join("\n"));
  const [webhook, setWebhook] = useState(route.webhook_url ?? "");
  const [useWebhook, setUseWebhook] = useState(route.use_webhook);
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");

  const save = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    setNotice("");
    try {
      await api(`/projects/${slug}/alerts/${s}`, "PUT", { emails, webhook_url: webhook, use_webhook: useWebhook });
      toast.success(`${s === "critical" ? "Critical" : "Warning"} alerts saved.`);
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };
  const test = async (): Promise<void> => {
    setNotice("");
    try {
      const { sent_to: n } = await api<{ sent_to: number }>(`/projects/${slug}/alerts/${s}/test`, "POST");
      setNotice(
        n === 0
          ? "No recipients set for this severity."
          : n === 1
            ? "Test queued for 1 recipient. It arrives within a minute."
            : `Test queued for ${n} recipients. They arrive within a minute.`,
      );
    } catch (err) {
      toast.error(errorText(err));
    }
  };

  return (
    <Card id={s}>
      <CardHeader className="border-b">
        <CardTitle>
          <SeverityBadge severity={s} />
        </CardTitle>
        <CardAction>
          <Button variant="ghost" size="sm" onClick={() => void test()}>
            <Send data-icon="inline-start" />
            Send a test
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent>
        <form onSubmit={(e) => void save(e)} className="flex flex-col gap-5">
          {notice && (
            <Alert role="status">
              <AlertDescription>{notice}</AlertDescription>
            </Alert>
          )}
          <FieldGroup className="gap-5">
            <Field data-invalid={error?.fields.emails ? true : undefined}>
              <FieldLabel htmlFor={`emails-${s}`}>Email addresses</FieldLabel>
              <Textarea id={`emails-${s}`} rows={3} aria-invalid={error?.fields.emails ? true : undefined} value={emails} onChange={(e) => setEmails(e.target.value)} />
              {error?.fields.emails && <FieldError>{error.fields.emails}</FieldError>}
            </Field>
            <Field data-invalid={error?.fields.webhook_url ? true : undefined}>
              <FieldLabel htmlFor={`webhook-${s}`}>Webhook URL</FieldLabel>
              <Input id={`webhook-${s}`} type="url" aria-invalid={error?.fields.webhook_url ? true : undefined} value={webhook} onChange={(e) => setWebhook(e.target.value)} />
              {error?.fields.webhook_url && <FieldError>{error.fields.webhook_url}</FieldError>}
            </Field>
            <Field orientation="horizontal">
              <Checkbox id={`use-${s}`} checked={useWebhook} onCheckedChange={(v) => setUseWebhook(v === true)} />
              <FieldLabel htmlFor={`use-${s}`} className="font-normal">
                Send {s} alerts to the webhook
              </FieldLabel>
            </Field>
          </FieldGroup>
          <div>
            <Button type="submit" disabled={busy}>
              {busy && <Spinner data-icon="inline-start" />}
              Save {s} alerts
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  );
}

/** Settings, Alerts: who gets critical and warning alerts, by email and webhook. */
export function Alerts(): ReactElement {
  const { project: p } = useProject();
  const routes = useApi<AlertRoute[]>(`/projects/${p.slug}/alerts`);
  return (
    <>
      <PageHeader title="Alerts" />
      {routes.error && (
        <Alert variant="destructive">
          <AlertDescription>{routes.error.message}</AlertDescription>
        </Alert>
      )}
      {!routes.data ? (
        <Skeleton className="h-64 rounded-xl" />
      ) : (
        <div className="grid items-start gap-6 xl:grid-cols-2">
          {routes.data.map((r) => (
            <RouteCard key={r.severity} slug={p.slug} route={r} />
          ))}
        </div>
      )}
    </>
  );
}
