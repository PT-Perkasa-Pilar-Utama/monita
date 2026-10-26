import { useState } from "react";
import type { ReactElement } from "react";
import { ConfirmDialog } from "@/components/app/ConfirmDialog";
import { Pause, Play } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { api, errorText } from "@/lib/api";
import { Writable } from "@/lib/session";
import { toast } from "sonner";

/** A monitor's or heartbeat's own actions in its page header: pause or resume it (v1.1 Q38). `path` is its API path, such as `/projects/pay/monitors/3`. */
export function SourceActions({ kind, path, paused, onChange }: { kind: "monitor" | "heartbeat"; path: string; paused: boolean; onChange: () => void }): ReactElement {
  const [busy, setBusy] = useState(false);
  const set = async (on: boolean): Promise<void> => {
    setBusy(true);
    try {
      await api(`${path}/paused`, "PUT", { paused: on });
      toast.success(on ? `The ${kind} is paused.` : `The ${kind} runs again.`);
      onChange();
    } catch (err) {
      toast.error(errorText(err));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Writable>
      {paused ? (
        <Button type="button" variant="secondary" disabled={busy} onClick={() => void set(false)}>
          {busy ? <Spinner data-icon="inline-start" /> : <Play data-icon="inline-start" />}
          Resume
        </Button>
      ) : (
        <ConfirmDialog
          trigger={
            <Button type="button" variant="secondary">
              <Pause data-icon="inline-start" />
              Pause
            </Button>
          }
          title={`Pause this ${kind}?`}
          description={
            kind === "monitor"
              ? "Monita stops checking it and sends no alerts until you resume it, closes any open incident, and hides it from the status page in the meantime."
              : "Monita keeps its pings but stops watching its deadline and sends no alerts until you resume it, closes any open incident, and hides it from the status page in the meantime."
          }
          action="Pause"
          onConfirm={() => set(true)}
        />
      )}
    </Writable>
  );
}
