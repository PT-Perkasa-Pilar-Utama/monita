import type { ReactElement } from "react";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldError, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { clampQuorum } from "@/lib/locations";

/** Where a monitor checks from: this server always, and any locations; and how many must fail (technical-specs 14). */
export function CheckFrom({
  locations,
  ids,
  quorum,
  onChange,
  err,
}: {
  locations: { id: number; name: string }[];
  ids: number[];
  quorum: string;
  onChange: (ids: number[], quorum: string) => void;
  err: (k: string) => string | undefined;
}): ReactElement {
  const places = 1 + ids.length;
  const toggle = (id: number, on: boolean): void => {
    const next = on ? [...ids, id] : ids.filter((i) => i !== id);
    onChange(next, clampQuorum(quorum, 1 + next.length));
  };
  return (
    <FieldSet className="gap-3">
      <FieldLegend>Check from</FieldLegend>
      <Field orientation="horizontal">
        <Checkbox id="m-place-here" checked disabled />
        <FieldLabel htmlFor="m-place-here" className="font-normal">
          This server
        </FieldLabel>
      </Field>
      {locations.map((l) => (
        <Field key={l.id} orientation="horizontal">
          <Checkbox id={`m-place-${l.id}`} checked={ids.includes(l.id)} onCheckedChange={(v) => toggle(l.id, v === true)} />
          <FieldLabel htmlFor={`m-place-${l.id}`} className="font-normal">
            {l.name}
          </FieldLabel>
        </Field>
      ))}
      {err("location_ids") && <FieldError>{err("location_ids")}</FieldError>}
      {places > 1 && (
        <Field data-invalid={err("quorum") ? true : undefined}>
          <FieldLabel htmlFor="m-quorum" className="font-normal">
            A check counts as failed when
          </FieldLabel>
          <div className="flex items-center gap-3">
            <Input id="m-quorum" type="number" inputMode="numeric" min={1} max={places} className="w-20" aria-invalid={err("quorum") ? true : undefined} value={quorum} onChange={(e) => onChange(ids, e.target.value)} />
            <span className="text-sm text-muted-foreground">of the {places} places fail</span>
          </div>
          {err("quorum") && <FieldError>{err("quorum")}</FieldError>}
        </Field>
      )}
    </FieldSet>
  );
}
