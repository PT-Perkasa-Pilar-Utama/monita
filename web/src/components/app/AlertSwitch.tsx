import { useState } from "react";
import type { ReactElement } from "react";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { api, errorText } from "@/lib/api";
import { useCanEdit } from "@/lib/session";
import { toast } from "sonner";

/** Turns one alert card, or ticket alerts, on or off at once. Off keeps its settings. */
export function AlertSwitch({ path, id, name, on, onChange }: { path: string; id: string; name: string; on: boolean; onChange: (on: boolean) => void }): ReactElement {
  const canEdit = useCanEdit();
  const [busy, setBusy] = useState(false);
  const flip = async (next: boolean): Promise<void> => {
    setBusy(true);
    onChange(next);
    try {
      await api(path, "PUT", { enabled: next });
      toast.success(`${name}: ${next ? "on" : "off"}.`);
    } catch (err) {
      onChange(!next);
      toast.error(errorText(err));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="flex items-center gap-2">
      <Label htmlFor={id} className="w-6 text-[13px] font-medium text-muted-foreground">
        {on ? "On" : "Off"}
      </Label>
      <Switch id={id} checked={on} disabled={busy || !canEdit} onCheckedChange={(v) => void flip(v)} aria-label={`${name} on`} />
    </div>
  );
}
