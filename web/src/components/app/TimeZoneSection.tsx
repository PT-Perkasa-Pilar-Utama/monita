import { useState } from "react";
import type { ReactElement } from "react";
import { SettingsSection } from "@/components/app/SettingsSection";
import { Button } from "@/components/ui/button";
import { Field, FieldLabel } from "@/components/ui/field";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Spinner } from "@/components/ui/spinner";
import { api, errorText } from "@/lib/api";
import { digestNote, stamp, zone } from "@/lib/time";
import { toast } from "sonner";

/** Offsets real places use, in minutes east of UTC, with a few places for each. */
const ZONES: [number, string][] = [
  [-720, "Baker Island"],
  [-660, "American Samoa"],
  [-600, "Hawaii"],
  [-540, "Alaska"],
  [-480, "Los Angeles"],
  [-420, "Denver"],
  [-360, "Mexico City, Chicago"],
  [-300, "New York, Bogotá"],
  [-240, "Santiago, Halifax"],
  [-210, "Newfoundland"],
  [-180, "São Paulo, Buenos Aires"],
  [-120, "South Georgia"],
  [-60, "Azores"],
  [0, "London in winter, Reykjavík"],
  [60, "Lagos, Paris in winter"],
  [120, "Cairo, Johannesburg"],
  [180, "Moscow, Riyadh, Nairobi"],
  [210, "Tehran"],
  [240, "Dubai"],
  [270, "Kabul"],
  [300, "Karachi, Tashkent"],
  [330, "India"],
  [345, "Nepal"],
  [360, "Dhaka, Almaty"],
  [390, "Myanmar"],
  [420, "Jakarta (WIB), Bangkok, Hanoi"],
  [480, "Makassar (WITA), Singapore, Manila"],
  [540, "Jayapura (WIT), Tokyo, Seoul"],
  [570, "Darwin"],
  [600, "Brisbane, Port Moresby"],
  [630, "Lord Howe Island"],
  [660, "Solomon Islands"],
  [720, "Fiji, New Zealand in winter"],
  [765, "Chatham Islands"],
  [780, "Tonga"],
  [840, "Kiribati Line Islands"],
];

/** The UTC offset every page shows times in. A fixed offset: no daylight saving. */
export function TimeZoneSection({ initial, digestHour }: { initial: number; digestHour: number }): ReactElement {
  const [saved, setSaved] = useState(initial);
  const [value, setValue] = useState(initial);
  const [busy, setBusy] = useState(false);
  const save = async (): Promise<void> => {
    setBusy(true);
    try {
      await api("/admin/time-zone", "PUT", { utc_offset_min: value });
      setSaved(value);
      toast.success(`Times now show in ${zone(value)}, so the page is reloading.`);
      window.location.reload();
    } catch (err) {
      toast.error(errorText(err));
    } finally {
      setBusy(false);
    }
  };
  return (
    <SettingsSection
      id="time-zone"
      title="Times on every page"
      description="The public status page shows its times in this offset and starts its days at its midnight, while the team's own pages show every time on each person's own clock."
      footerNote={`Now it is ${stamp(Math.floor(Date.now() / 1000), value)}. ${digestNote(digestHour, value)}`}
      footer={
        <Button onClick={() => void save()} disabled={busy || value === saved}>
          {busy && <Spinner data-icon="inline-start" />}
          Save changes
        </Button>
      }
    >
      <Field className="max-w-md">
        <FieldLabel htmlFor="tz">Show times in</FieldLabel>
        <Select value={String(value)} onValueChange={(v) => setValue(Number(v))}>
          <SelectTrigger id="tz">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              {ZONES.map(([min, places]) => (
                <SelectItem key={min} value={String(min)}>
                  {zone(min)}: {places}
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
      </Field>
    </SettingsSection>
  );
}
