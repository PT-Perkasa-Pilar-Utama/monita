import { useState } from "react";
import type { ReactElement } from "react";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { DestinationFields } from "@/components/app/DestinationFields";
import type { Dest, DestKind } from "@/components/app/DestinationFields";
import { FormSheet, FormSheetBody } from "@/components/app/FormSheet";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldError, FieldGroup, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
import { Spinner } from "@/components/ui/spinner";
import { api, ApiError, dropField, errorText } from "@/lib/api";
import type { AlertCard, CardSection } from "@/lib/types";

/** What a custom card can send by. Global Telegram has its own switch above. A ticket never
 * resolves, so a ticket card has no PagerDuty. */
const CUSTOM_KINDS: Record<CardSection, DestKind[]> = {
  incidents: ["email", "json", "discord", "telegram", "ntfy", "pagerduty"],
  tickets: ["email", "json", "discord", "telegram", "ntfy"],
};

/** What each section's card listens to. */
const LISTENS: Record<CardSection, { key: "critical" | "warning" | "new_tickets" | "ticket_moves"; label: string }[]> = {
  incidents: [
    { key: "critical", label: "Critical alerts" },
    { key: "warning", label: "Warning alerts" },
  ],
  tickets: [
    { key: "new_tickets", label: "New tickets" },
    { key: "ticket_moves", label: "Ticket moves" },
  ],
};

/** Adds or edits one of the project's own alert cards. Delete sits at the left of the footer. */
export function AlertCardSheet({ slug, bot, section, card, open, onClose, onSaved }: { slug: string; bot: string | null; section: CardSection; card: AlertCard | null; open: boolean; onClose: () => void; onSaved: () => void }): ReactElement {
  const [dest, setDest] = useState<Dest>(() => ({
    kind: card && card.kind !== "owner_email" ? card.kind : "email",
    emails: card?.emails.join("\n") ?? "",
    url: card?.url ?? "",
    key: card?.key ?? "",
    title: card?.kind === "telegram_bot" ? card.name : "",
  }));
  // A new card listens to the first choice: critical alerts, or new tickets.
  const [levels, setLevels] = useState({
    critical: card?.critical ?? section === "incidents",
    warning: card?.warning ?? false,
    new_tickets: card?.new_tickets ?? section === "tickets",
    ticket_moves: card?.ticket_moves ?? false,
  });
  const [error, setError] = useState<ApiError | null>(null);
  const [busy, setBusy] = useState(false);
  const clear = (field: string): void => setError((e) => e && dropField(e, field));
  const fieldError = (f: string): string | undefined => error?.fields[f];

  const save = async (): Promise<void> => {
    setBusy(true);
    setError(null);
    try {
      const email = dest.kind === "email";
      // No name field: a global Telegram card is called after its chat, a card that keeps its kind
      // keeps its name, and the server names the rest after their kind.
      const name = dest.kind === "telegram_bot" ? (dest.title ?? "") : card && card.kind === dest.kind ? card.name : "";
      const body = { section, kind: dest.kind, name, ...levels, emails: email ? dest.emails : "", url: email ? "" : dest.url, key: email ? "" : dest.key };
      await (card ? api(`/projects/${slug}/alert-cards/${card.id}`, "PUT", body) : api(`/projects/${slug}/alert-cards`, "POST", body));
      onSaved();
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, "error", errorText(err)));
    } finally {
      setBusy(false);
    }
  };
  const listenField = section === "incidents" ? "levels" : "events";
  const unplaced = error && !["kind", listenField, "emails", "url", "key"].some((k) => error.fields[k]);

  return (
    <FormSheet open={open} onClose={onClose}>
      <FormSheetBody
        title={card ? `Edit ${card.name}` : section === "incidents" ? "Add alert card" : "Add ticket card"}
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
                description="Its alerts stop."
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
          <DestinationFields
            prefix="card"
            slug={slug}
            bot={bot}
            kinds={CUSTOM_KINDS[section]}
            value={dest}
            errors={{ kind: fieldError("kind"), emails: fieldError("emails"), url: fieldError("url"), key: fieldError("key") }}
            onChange={(patch, field) => {
              // The kind decides which fields the card needs. Old errors no longer apply.
              if (patch.kind && patch.kind !== dest.kind) setError(null);
              setDest((d) => ({ ...d, ...patch }));
              if (field) clear(field);
            }}
          />
          <FieldSet data-invalid={fieldError(listenField) ? true : undefined}>
            <FieldLegend variant="label">Listens to</FieldLegend>
            <div className="flex gap-6">
              {LISTENS[section].map(({ key, label }) => (
                <Field key={key} orientation="horizontal">
                  <Checkbox id={`card-${key}`} checked={levels[key]} onCheckedChange={(v) => (setLevels((x) => ({ ...x, [key]: v === true })), clear(listenField))} />
                  <FieldLabel htmlFor={`card-${key}`} className="font-normal">
                    {label}
                  </FieldLabel>
                </Field>
              ))}
            </div>
            {fieldError(listenField) && <FieldError>{fieldError(listenField)}</FieldError>}
          </FieldSet>
          {unplaced && <p className="text-sm text-destructive">{error.message}</p>}
        </FieldGroup>
      </FormSheetBody>
    </FormSheet>
  );
}
