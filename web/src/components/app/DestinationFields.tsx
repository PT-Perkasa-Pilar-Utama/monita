import { useState } from "react";
import type { ReactElement } from "react";
import { TelegramConnect } from "@/components/app/TelegramConnect";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { kindLabel } from "@/lib/alertCards";
import { webhookFields } from "@/lib/webhookFormats";
import type { WebhookFormat } from "@/lib/webhookFormats";

/** Where alerts go: an email list, or a webhook in one format. */
export type DestKind = "email" | WebhookFormat;
export type Dest = { kind: DestKind; emails: string; url: string; key: string };
export type DestErrors = { kind?: string | undefined; emails?: string | undefined; url?: string | undefined; key?: string | undefined };

/**
 * Picks where an alert card or ticket alerts send. With a global Telegram bot set up, a switch
 * comes first: Global Telegram connects a chat to that bot, Custom offers `kinds`. Then the fields
 * the kind needs.
 */
export function DestinationFields({ prefix, slug, bot, kinds, value, errors, onChange }: { prefix: string; slug: string; bot: string | null; kinds: DestKind[]; value: Dest; errors: DestErrors; onChange: (patch: Partial<Dest>, field?: keyof Dest) => void }): ReactElement {
  const global = value.kind === "telegram_bot";
  // The custom kind to go back to from Global Telegram.
  const [custom, setCustom] = useState<DestKind>(global ? kinds[0]! : value.kind);
  const [connected, setConnected] = useState("");
  const fields = value.kind === "email" ? null : webhookFields(value.kind);
  const id = (f: string): string => `${prefix}-${f}`;

  return (
    <>
      {(bot !== null || global) && (
        <Tabs value={global ? "global" : "custom"} onValueChange={(m) => onChange(m === "global" ? { kind: "telegram_bot" } : { kind: custom }, "kind")}>
          <TabsList aria-label="Where alerts go">
            <TabsTrigger value="global">Global Telegram</TabsTrigger>
            <TabsTrigger value="custom">Custom</TabsTrigger>
          </TabsList>
        </Tabs>
      )}
      {!global && (
        <Field data-invalid={errors.kind ? true : undefined}>
          <FieldLabel htmlFor={id("kind")}>Sends by</FieldLabel>
          <Select
            value={value.kind}
            onValueChange={(v) => {
              setCustom(v as DestKind);
              onChange({ kind: v as DestKind }, "kind");
            }}
          >
            <SelectTrigger id={id("kind")} aria-invalid={errors.kind ? true : undefined}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                {kinds.map((k) => (
                  <SelectItem key={k} value={k}>
                    {k === "json" ? "JSON (Slack, Google Chat, any receiver)" : kindLabel(k)}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>
          {errors.kind && <FieldError>{errors.kind}</FieldError>}
        </Field>
      )}
      {global && errors.kind && <FieldError>{errors.kind}</FieldError>}
      {global && bot === null && <p className="text-sm text-muted-foreground">No global Telegram bot is set up. Pick Custom.</p>}
      {value.kind === "email" && (
        <Field data-invalid={errors.emails ? true : undefined}>
          <FieldLabel htmlFor={id("emails")} required>
            Email addresses
          </FieldLabel>
          <Textarea id={id("emails")} rows={4} placeholder={"oncall@company.co.id\nlead@company.co.id"} aria-invalid={errors.emails ? true : undefined} value={value.emails} onChange={(e) => onChange({ emails: e.target.value }, "emails")} />
          {errors.emails && <FieldError>{errors.emails}</FieldError>}
        </Field>
      )}
      {fields?.url && (
        <Field data-invalid={errors.url ? true : undefined}>
          <FieldLabel htmlFor={id("url")} required>
            {fields.url.label}
          </FieldLabel>
          <Input id={id("url")} type="url" placeholder={fields.url.placeholder} aria-invalid={errors.url ? true : undefined} value={value.url} onChange={(e) => onChange({ url: e.target.value }, "url")} />
          {errors.url && <FieldError>{errors.url}</FieldError>}
        </Field>
      )}
      {fields?.key && (global ? bot !== null : true) && (
        <Field data-invalid={errors.key ? true : undefined}>
          <FieldLabel htmlFor={id("key")} required>
            {fields.key.label}
          </FieldLabel>
          <Input
            id={id("key")}
            className="font-mono"
            placeholder={fields.key.placeholder}
            aria-invalid={errors.key ? true : undefined}
            value={value.key}
            onChange={(e) => {
              onChange({ key: e.target.value }, "key");
              setConnected("");
            }}
          />
          {global && bot !== null && (
            <TelegramConnect
              slug={slug}
              bot={bot}
              onChat={(chat) => {
                onChange({ key: chat.chat_id }, "key");
                setConnected(`Connected to ${chat.title}.`);
              }}
            />
          )}
          {connected && <p role="status" className="text-sm text-muted-foreground">{connected}</p>}
          {errors.key && <FieldError>{errors.key}</FieldError>}
        </Field>
      )}
    </>
  );
}
