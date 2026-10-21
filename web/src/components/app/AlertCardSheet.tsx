import { useState } from "react";
import type { ReactElement } from "react";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { FormSheet, FormSheetBody } from "@/components/app/FormSheet";
import { TelegramConnect } from "@/components/app/TelegramConnect";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldError, FieldGroup, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { api, ApiError, dropField, errorText } from "@/lib/api";
import { kindLabel } from "@/lib/alertCards";
import type { AlertCard, CardKind } from "@/lib/types";
import { formatsFor, webhookFields } from "@/lib/webhookFormats";
import type { WebhookFormat } from "@/lib/webhookFormats";

/** The form's values; `card` is null for a new card. */
function initial(card: AlertCard | null) {
  return {
    kind: (card?.kind ?? "email") as Exclude<CardKind, "owner_email">,
    name: card?.name ?? kindLabel("email"),
    critical: card?.critical ?? true,
    warning: card?.warning ?? false,
    emails: card?.emails.join("\n") ?? "",
    url: card?.url ?? "",
    key: card?.key ?? "",
  };
}

/**
 * Adds or edits one of the project's own alert cards: what sends (email or a webhook format), a
 * name, the levels it listens to, and what that kind needs. Delete sits at the left of the footer.
 */
export function AlertCardSheet({ slug, bot, card, open, onClose, onSaved }: { slug: string; bot: string | null; card: AlertCard | null; open: boolean; onClose: () => void; onSaved: () => void }): ReactElement {
  const [f, setF] = useState(() => initial(card));
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const [connected, setConnected] = useState("");
  const set = (patch: Partial<typeof f>, field?: string): void => {
    setF((v) => ({ ...v, ...patch }));
    if (field) setError((e) => e && dropField(e, field));
  };
  const fields = f.kind === "email" ? null : webhookFields(f.kind as WebhookFormat);
  const kinds = [{ value: "email", label: "Email" }, ...formatsFor(bot, (card?.kind ?? "json") as WebhookFormat)];
  const fieldError = (name: string): string | undefined => error?.fields[name];

  const save = async (): Promise<void> => {
    setBusy(true);
    setError(null);
    try {
      const body = { ...f, emails: f.kind === "email" ? f.emails : "", url: f.kind === "email" ? "" : f.url, key: f.kind === "email" ? "" : f.key };
      await (card ? api(`/projects/${slug}/alert-cards/${card.id}`, "PUT", body) : api(`/projects/${slug}/alert-cards`, "POST", body));
      onSaved();
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };
  const unplaced = error && !["kind", "name", "levels", "emails", "url", "key"].some((k) => error.fields[k]);

  return (
    <FormSheet open={open} onClose={onClose}>
      <FormSheetBody
        title={card ? `Edit ${card.name}` : "Add alert card"}
        onSubmit={() => void save()}
        footer={
          <>
            {card && (
              <ConfirmDialog
                trigger={
                  <Button type="button" variant="ghost" className="text-destructive">
                    Delete
                  </Button>
                }
                title={`Delete ${card.name}?`}
                description="Its alerts stop. Monitors and heartbeats keep their alert level."
                action="Delete card"
                onConfirm={async () => {
                  await api(`/projects/${slug}/alert-cards/${card.id}`, "DELETE");
                  onSaved();
                }}
              />
            )}
            <span className="flex-1" />
            <Button type="button" variant="ghost" onClick={onClose}>
              Cancel
            </Button>
            <Button type="submit" disabled={busy}>
              {busy && <Spinner data-icon="inline-start" />}
              {card ? "Save changes" : "Add card"}
            </Button>
          </>
        }
      >
        <FieldGroup className="gap-5">
          <Field data-invalid={fieldError("kind") ? true : undefined}>
            <FieldLabel htmlFor="card-kind">Sends by</FieldLabel>
            <Select
              value={f.kind}
              onValueChange={(v) => {
                const kind = v as typeof f.kind;
                // A name still equal to the old kind's label follows the new kind.
                set({ kind, name: !f.name || f.name === kindLabel(f.kind) ? kindLabel(kind) : f.name });
                // The kind decides which fields the card needs; old errors no longer apply.
                setError(null);
              }}
            >
              <SelectTrigger id="card-kind" aria-invalid={fieldError("kind") ? true : undefined}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  {kinds.map((k) => (
                    <SelectItem key={k.value} value={k.value}>
                      {k.label}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
            {fieldError("kind") && <FieldError>{fieldError("kind")}</FieldError>}
          </Field>
          <Field data-invalid={fieldError("name") ? true : undefined}>
            <FieldLabel htmlFor="card-name" required>
              Name
            </FieldLabel>
            <Input id="card-name" maxLength={60} placeholder="On call" aria-invalid={fieldError("name") ? true : undefined} value={f.name} onChange={(e) => set({ name: e.target.value }, "name")} />
            {fieldError("name") && <FieldError>{fieldError("name")}</FieldError>}
          </Field>
          <FieldSet data-invalid={fieldError("levels") ? true : undefined}>
            <FieldLegend variant="label">Listens to</FieldLegend>
            <div className="flex gap-6">
              {(["critical", "warning"] as const).map((l) => (
                <Field key={l} orientation="horizontal">
                  <Checkbox id={`card-${l}`} checked={f[l]} onCheckedChange={(v) => set({ [l]: v === true }, "levels")} />
                  <FieldLabel htmlFor={`card-${l}`} className="font-normal">
                    {l === "critical" ? "Critical alerts" : "Warning alerts"}
                  </FieldLabel>
                </Field>
              ))}
            </div>
            {fieldError("levels") && <FieldError>{fieldError("levels")}</FieldError>}
          </FieldSet>
          {f.kind === "email" && (
            <Field data-invalid={fieldError("emails") ? true : undefined}>
              <FieldLabel htmlFor="card-emails" required>
                Email addresses
              </FieldLabel>
              <Textarea id="card-emails" rows={4} placeholder={"oncall@company.co.id\nlead@company.co.id"} aria-invalid={fieldError("emails") ? true : undefined} value={f.emails} onChange={(e) => set({ emails: e.target.value }, "emails")} />
              {fieldError("emails") && <FieldError>{fieldError("emails")}</FieldError>}
            </Field>
          )}
          {fields?.url && (
            <Field data-invalid={fieldError("url") ? true : undefined}>
              <FieldLabel htmlFor="card-url" required>
                {fields.url.label}
              </FieldLabel>
              <Input id="card-url" type="url" placeholder={fields.url.placeholder} aria-invalid={fieldError("url") ? true : undefined} value={f.url} onChange={(e) => set({ url: e.target.value }, "url")} />
              {fieldError("url") && <FieldError>{fieldError("url")}</FieldError>}
            </Field>
          )}
          {fields?.key && (
            <Field data-invalid={fieldError("key") ? true : undefined}>
              <FieldLabel htmlFor="card-key" required>
                {fields.key.label}
              </FieldLabel>
              <Input id="card-key" className="font-mono" placeholder={fields.key.placeholder} aria-invalid={fieldError("key") ? true : undefined} value={f.key} onChange={(e) => {
                set({ key: e.target.value }, "key");
                setConnected("");
              }} />
              {f.kind === "telegram_bot" && bot && (
                <TelegramConnect slug={slug} bot={bot} onChat={(chat) => {
                  set({ key: chat.chat_id }, "key");
                  setConnected(`Connected to ${chat.title}.`);
                }} />
              )}
              {connected && <p role="status" className="text-sm text-muted-foreground">{connected}</p>}
              {fieldError("key") && <FieldError>{fieldError("key")}</FieldError>}
            </Field>
          )}
          {unplaced && <p className="text-sm text-destructive">{error.message}</p>}
        </FieldGroup>
      </FormSheetBody>
    </FormSheet>
  );
}
