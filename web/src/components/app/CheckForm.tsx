import type { ReactElement, ReactNode } from "react";
import { ChoiceCards } from "@/components/app/ChoiceCards";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldError, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import type { Choice, Severity } from "@/lib/types";

/** Critical or Warning, as two cards: what each one alerts by. */
export function SeverityField({ value, onChange }: { value: Severity; onChange: (s: Severity) => void }): ReactElement {
  return (
    <ChoiceCards
      id="sev"
      legend="Alert level"
      value={value}
      onChange={onChange}
      choices={[
        { value: "critical", title: "Critical", description: "For an outage that needs someone now, sent to every alert card that listens to critical." },
        { value: "warning", title: "Warning", description: "For a problem that can wait for working hours, sent to every alert card that listens to warning." },
      ]}
    />
  );
}

/** A select over a fixed list of seconds, such as a check interval or how late a job can be. */
export function ChoiceField({
  id,
  label,
  value,
  choices,
  onChange,
  error,
}: {
  id: string;
  label: string;
  value: number;
  choices: Choice[];
  onChange: (v: number) => void;
  error?: string | undefined;
}): ReactElement {
  return (
    <Field data-invalid={error ? true : undefined}>
      <FieldLabel htmlFor={id} required>
        {label}
      </FieldLabel>
      <Select value={String(value)} onValueChange={(v) => onChange(Number(v))}>
        <SelectTrigger id={id} aria-invalid={error ? true : undefined}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            {choices.map((c) => (
              <SelectItem key={c.value} value={String(c.value)}>
                {c.label}
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
      {error && <FieldError>{error}</FieldError>}
    </Field>
  );
}

/** The two switches a monitor or heartbeat has for a failure. Both start on. A monitor puts its
 * failure count first. */
export function WhenItFails({
  id,
  alert,
  autoIncident,
  onChange,
  children,
}: {
  id: string;
  alert: boolean;
  autoIncident: boolean;
  onChange: (k: "alert" | "auto_incident", v: boolean) => void;
  children?: ReactNode;
}): ReactElement {
  return (
    <FieldSet className="gap-3">
      <FieldLegend variant="label">When it fails</FieldLegend>
      {children}
      <Field orientation="horizontal">
        <Checkbox id={`${id}-alert`} checked={alert} onCheckedChange={(v) => onChange("alert", v === true)} />
        <FieldLabel htmlFor={`${id}-alert`} className="font-normal">
          Send alerts to the people in Alerts settings
        </FieldLabel>
      </Field>
      <Field orientation="horizontal">
        <Checkbox id={`${id}-auto`} checked={autoIncident} onCheckedChange={(v) => onChange("auto_incident", v === true)} />
        <FieldLabel htmlFor={`${id}-auto`} className="font-normal">
          Open an incident
        </FieldLabel>
      </Field>
    </FieldSet>
  );
}

/** Whether visitors see this monitor or heartbeat, as two cards near the top of its form, so the
 * choice is made on purpose. Its incidents reach the status page only when it is shown there. */
export function PublicField({ id, noun, checked, onChange }: { id: string; noun: "monitor" | "heartbeat"; checked: boolean; onChange: (v: boolean) => void }): ReactElement {
  return (
    <ChoiceCards
      id={`${id}-public`}
      legend="Who sees it"
      value={checked ? "public" : "private"}
      onChange={(v) => onChange(v === "public")}
      choices={[
        { value: "private", title: "Only your team", description: `The ${noun} and its incidents stay inside Monita.` },
        { value: "public", title: "Also the status page", description: `Visitors see this ${noun}, its uptime bars and its incidents on the public status page.` },
      ]}
    />
  );
}
