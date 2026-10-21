import { useState } from "react";
import type { FormEvent, ReactElement } from "react";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { SettingsSection } from "@/components/app/SettingsSection";
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

/** Admin, Telegram: the global Telegram bot that every project can send alerts through. */
export function Telegram(): ReactElement {
  const res = useApi<{ bot: string | null }>("/telegram");
  const [token, setToken] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const bot = res.data?.bot ?? null;

  const save = async (value: string): Promise<void> => {
    const r = await api<{ bot: string | null }>("/admin/telegram", "PUT", { token: value });
    setToken("");
    res.reload();
    toast.success(r.bot ? `Saved. The global bot is @${r.bot}.` : "Global Telegram bot removed.");
  };
  const submit = async (e: FormEvent): Promise<void> => {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      await save(token);
    } catch (err) {
      setError(err instanceof ApiError ? (err.fields.token ?? err.message) : errorText(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <PageHeader title="Telegram" />
      {!res.data ? (
        <Skeleton className="h-64 rounded-xl" />
      ) : (
        <form onSubmit={(e) => void submit(e)}>
          <SettingsSection
            id="bot"
            title={bot ? `Global Telegram bot: @${bot}` : "Global Telegram bot"}
            description="One bot that every project can send alerts through. Create a bot with @BotFather, then paste its token here."
            footer={
              <Writable>
                {bot && (
                  <ConfirmDialog
                    trigger={
                      <Button type="button" variant="outline">
                        Remove bot
                      </Button>
                    }
                    title="Remove the global Telegram bot?"
                    description="Global Telegram cards stop sending until a bot is set again."
                    action="Remove bot"
                    onConfirm={() => save("")}
                  />
                )}
                <Button type="submit" disabled={busy || !token.trim()}>
                  {busy && <Spinner data-icon="inline-start" />}
                  {bot ? "Replace token" : "Save token"}
                </Button>
              </Writable>
            }
          >
            <Writable>
              <Field data-invalid={error ? true : undefined} className="max-w-md">
                <FieldLabel htmlFor="telegram-token">Bot token</FieldLabel>
                <Input
                  id="telegram-token"
                  type="password"
                  autoComplete="off"
                  className="font-mono"
                  placeholder="123456789:AA…"
                  aria-invalid={error ? true : undefined}
                  value={token}
                  onChange={(e) => {
                    setToken(e.target.value);
                    setError("");
                  }}
                />
                {error && <FieldError>{error}</FieldError>}
              </Field>
            </Writable>
          </SettingsSection>
        </form>
      )}
    </>
  );
}
