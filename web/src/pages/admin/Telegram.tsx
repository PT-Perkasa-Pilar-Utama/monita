import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { SettingsSection } from "@/components/app/SettingsSection";
import { StateBadge } from "@/components/app/Tone";
import { PageHeader } from "@/components/layout/Page";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { api, ApiError, errorText } from "@/lib/api";
import { Writable } from "@/lib/session";
import { useApi } from "@/lib/useApi";
import { toast } from "sonner";

/** The bot token field and its save button. */
function TokenForm({ action, onSaved, onCancel }: { action: string; onSaved: (token: string) => Promise<void>; onCancel?: () => void }): ReactElement {
  const [token, setToken] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      await onSaved(token);
      setToken("");
    } catch (err) {
      setError(err instanceof ApiError ? (err.fields.token ?? err.message) : errorText(err));
    } finally {
      setBusy(false);
    }
  };
  return (
    <form onSubmit={(e) => void submit(e)}>
      <Field data-invalid={error ? true : undefined}>
        <FieldLabel htmlFor="telegram-token">Bot token</FieldLabel>
        <div className="flex flex-col gap-3 sm:flex-row">
          <Input
            id="telegram-token"
            type="password"
            autoComplete="off"
            className="font-mono sm:max-w-md"
            placeholder="123456789:AA…"
            aria-invalid={error ? true : undefined}
            value={token}
            onChange={(e) => {
              setToken(e.target.value);
              setError("");
            }}
          />
          <Button type="submit" disabled={busy || !token.trim()} className="sm:w-fit">
            {busy && <Spinner data-icon="inline-start" />}
            {action}
          </Button>
          {onCancel && (
            <Button type="button" variant="ghost" onClick={onCancel} className="sm:w-fit">
              Cancel
            </Button>
          )}
        </div>
        {error && <FieldError>{error}</FieldError>}
      </Field>
    </form>
  );
}

/** Admin, Telegram: the global Telegram bot that every project can send alerts through. */
export function Telegram(): ReactElement {
  const res = useApi<{ bot: string | null }>("/telegram");
  const [replacing, setReplacing] = useState(false);
  const bot = res.data?.bot ?? null;

  const save = async (value: string): Promise<void> => {
    const r = await api<{ bot: string | null }>("/admin/telegram", "PUT", { token: value });
    setReplacing(false);
    res.reload();
    toast.success(r.bot ? `Saved. The global bot is @${r.bot}.` : "Global Telegram bot removed.");
  };

  return (
    <>
      <PageHeader title="Telegram" />
      {!res.data ? (
        <Skeleton className="h-64 rounded-xl" />
      ) : bot ? (
        <SettingsSection
          id="bot"
          title="Global Telegram bot"
          description="Every project can pick Global Telegram on its alert cards and connect a chat with a link."
          action={<StateBadge kind="up" label="Set up" />}
        >
          <div className="flex flex-col gap-5">
            <div className="flex flex-wrap items-center gap-3">
              <span className="font-mono text-lg font-semibold">@{bot}</span>
              <Button asChild variant="outline" size="sm">
                <a href={`https://t.me/${bot}`} target="_blank" rel="noreferrer">
                  Open in Telegram
                </a>
              </Button>
            </div>
            <Writable>
              {replacing ? (
                <TokenForm action="Save new token" onSaved={save} onCancel={() => setReplacing(false)} />
              ) : (
                <div className="flex gap-2">
                  <Button type="button" variant="outline" onClick={() => setReplacing(true)}>
                    Replace token
                  </Button>
                  <ConfirmDialog
                    trigger={
                      <Button type="button" variant="ghost" className="text-destructive">
                        Remove bot
                      </Button>
                    }
                    title="Remove the global Telegram bot?"
                    description="Global Telegram cards stop sending until a bot is set again."
                    action="Remove bot"
                    onConfirm={() => save("")}
                  />
                </div>
              )}
            </Writable>
          </div>
        </SettingsSection>
      ) : (
        <SettingsSection
          id="bot"
          title="Global Telegram bot"
          description="One bot that every project can send alerts through. Projects cannot pick Global Telegram until it is set."
          action={<StateBadge kind="unknown" label="Not set" />}
        >
          <div className="flex flex-col gap-5">
            <ol className="list-decimal space-y-1 pl-5 text-sm text-muted-foreground">
              <li>In Telegram, open @BotFather and send /newbot.</li>
              <li>Pick a name and a username for the bot.</li>
              <li>Copy the token BotFather sends, and paste it below.</li>
            </ol>
            <Writable>
              <TokenForm action="Save token" onSaved={save} />
            </Writable>
          </div>
        </SettingsSection>
      )}
    </>
  );
}
