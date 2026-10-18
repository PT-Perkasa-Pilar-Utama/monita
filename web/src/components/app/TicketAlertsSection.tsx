import { useState } from "react";
import type { ReactElement } from "react";
import { SettingsSection } from "@/components/app/SettingsSection";
import { Send } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { api, ApiError, errorText } from "@/lib/api";
import type { TicketAlerts } from "@/lib/types";
import { toast } from "sonner";

/** Ticket alerts: the owner's daily digest and a webhook per new ticket, each switched on its own. */
export function TicketAlertsSection({ slug, owner, initial, onSaved }: { slug: string; owner: string; initial: TicketAlerts; onSaved: (a: TicketAlerts) => void }): ReactElement {
  const [saved, setSaved] = useState(initial);
  const [digest, setDigest] = useState(initial.digest);
  const [useWebhook, setUseWebhook] = useState(initial.use_webhook);
  const [webhook, setWebhook] = useState(initial.webhook_url ?? "");
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const [testing, setTesting] = useState(false);
  const [notice, setNotice] = useState("");
  const changed = digest !== saved.digest || useWebhook !== saved.use_webhook || webhook.trim() !== (saved.webhook_url ?? "");

  const save = async (): Promise<void> => {
    setBusy(true);
    setError(null);
    setNotice("");
    try {
      await api(`/projects/${slug}/ticket-alerts`, "PUT", { digest, webhook_url: webhook, use_webhook: useWebhook });
      const next = { digest, webhook_url: webhook.trim() || null, use_webhook: useWebhook };
      setSaved(next);
      setWebhook(webhook.trim());
      onSaved(next);
      toast.success("Ticket alerts saved.");
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
      const { sent_to: n } = await api<{ sent_to: number }>(`/projects/${slug}/ticket-alerts/test`, "POST");
      setNotice(n === 0 ? "The webhook is off, so no test went out." : "Test sent to the webhook.");
    } catch (err) {
      toast.error(errorText(err));
    } finally {
      setTesting(false);
    }
  };

  return (
    <SettingsSection
      id="tickets"
      title="Ticket alerts"
      description="New bug reports from your users. A ticket your team files itself sends nothing."
      onSubmit={() => void save()}
      footerNote={changed ? "Save to test the new webhook." : notice}
      footer={
        <>
          <Button type="button" variant="outline" disabled={changed || testing || !saved.use_webhook} onClick={() => void test()}>
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
        <Field orientation="horizontal">
          <Checkbox id="ticket-digest" checked={digest} onCheckedChange={(v) => setDigest(v === true)} />
          <FieldLabel htmlFor="ticket-digest" className="font-normal">
            Email {owner} a daily list of new tickets in Triage
          </FieldLabel>
        </Field>
        <Field orientation="horizontal">
          <Checkbox id="ticket-webhook-on" checked={useWebhook} onCheckedChange={(v) => setUseWebhook(v === true)} />
          <FieldLabel htmlFor="ticket-webhook-on" className="font-normal">
            Send each new ticket to a webhook
          </FieldLabel>
        </Field>
        {useWebhook && (
          <Field data-invalid={error?.fields.webhook_url ? true : undefined}>
            <FieldLabel htmlFor="ticket-webhook" required>
              Webhook URL
            </FieldLabel>
            <Input
              id="ticket-webhook"
              type="url"
              placeholder="https://hooks.slack.com/services/…"
              aria-required
              aria-invalid={error?.fields.webhook_url ? true : undefined}
              value={webhook}
              onChange={(e) => setWebhook(e.target.value)}
            />
            {error?.fields.webhook_url && <FieldError>{error.fields.webhook_url}</FieldError>}
          </Field>
        )}
        {error && !error.fields.webhook_url && <p className="text-sm text-destructive">{error.message}</p>}
      </FieldGroup>
    </SettingsSection>
  );
}
