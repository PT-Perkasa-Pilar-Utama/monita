import { useState } from "react";
import type { ReactElement } from "react";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { api, errorText } from "@/lib/api";
import { toast } from "sonner";

/** Turns one alert setting on or off at once. Off keeps its recipients. */
export function AlertSwitch({ slug, route, name, on, onChange }: { slug: string; route: "critical" | "warning" | "ticket"; name: string; on: boolean; onChange: (on: boolean) => void }): ReactElement {
  const [busy, setBusy] = useState(false);
  const flip = async (next: boolean): Promise<void> => {
    setBusy(true);
    onChange(next);
    try {
      await api(`/projects/${slug}/alerts/${route}/enabled`, "PUT", { enabled: next });
      toast.success(`${name} are ${next ? "on" : "off"}.`);
    } catch (err) {
      onChange(!next);
      toast.error(errorText(err));
    } finally {
      setBusy(false);
    }
  };
  const id = `switch-${route}`;
  return (
    <div className="flex items-center gap-2">
      <Label htmlFor={id} className="w-6 text-[13px] font-medium text-muted-foreground">
        {on ? "On" : "Off"}
      </Label>
      <Switch id={id} checked={on} disabled={busy} onCheckedChange={(v) => void flip(v)} aria-label={`${name} on`} />
    </div>
  );
}
