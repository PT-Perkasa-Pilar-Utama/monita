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
      legend="Severity"
      value={value}
      onChange={onChange}
      choices={[
        { value: "critical", title: "Critical", description: "For an outage that needs someone now. Alerts by email, and a webhook when one is set." },
        { value: "warning", title: "Warning", description: "For a problem that can wait for working hours. Alerts by email." },
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

/** Whether visitors see this monitor or heartbeat. Its incidents reach the status page only when they do. */
export function PublicField({ id, noun, checked, onChange }: { id: string; noun: "monitor" | "heartbeat"; checked: boolean; onChange: (v: boolean) => void }): ReactElement {
  return (
    <FieldSet className="gap-3">
      <FieldLegend variant="label">Status page</FieldLegend>
      <Field orientation="horizontal">
        <Checkbox id={`${id}-public`} checked={checked} onCheckedChange={(v) => onChange(v === true)} />
        <FieldLabel htmlFor={`${id}-public`} className="font-normal">
          Show this {noun} and its incidents on the public status page
        </FieldLabel>
      </Field>
    </FieldSet>
  );
}
