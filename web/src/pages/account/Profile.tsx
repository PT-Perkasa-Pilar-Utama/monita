import { useState } from "react";
import type { ReactElement } from "react";
import { IconPicker } from "@/components/app/IconPicker";
import { SettingsPage, SettingsSection } from "@/components/app/SettingsSection";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { api, ApiError, errorText } from "@/lib/api";
import { avatarUrl } from "@/lib/avatar";
import { useMe, useReloadMe } from "@/lib/session";
import { toast } from "sonner";

/** User settings, Profile: the picture the top bar shows for this user. */
export function Profile(): ReactElement {
  const me = useMe();
  const reloadMe = useReloadMe();
  const [value, setValue] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const current = avatarUrl(me.id, me.avatar_at);

  const send = async (avatar: string, done: string): Promise<void> => {
    setBusy(true);
    setError("");
    try {
      await api("/me/avatar", "PUT", { avatar });
      setValue("");
      reloadMe();
      toast.success(done);
    } catch (err) {
      if (err instanceof ApiError && err.fields.avatar) setError(err.fields.avatar);
      else toast.error(errorText(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <SettingsPage title="Profile">
      <SettingsSection
        id="picture"
        title="Picture"
        description="The top bar shows it in place of your initial, and Monita crops it to a square."
        footer={
          <>
            {current && !value && (
              <Button variant="outline" disabled={busy} onClick={() => void send("", "Picture removed.")}>
                Remove picture
              </Button>
            )}
            <Button disabled={busy || !value} onClick={() => void send(value, "Picture saved.")}>
              {busy && <Spinner data-icon="inline-start" />}
              Save picture
            </Button>
          </>
        }
      >
        <IconPicker
          label="Picture"
          own
          value={value}
          current={current}
          error={error || undefined}
          onChange={(v) => {
            setValue(v);
            setError("");
          }}
        />
      </SettingsSection>
    </SettingsPage>
  );
}
