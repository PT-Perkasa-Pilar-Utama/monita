import type { ReactElement } from "react";
import { ChoiceCards } from "@/components/app/ChoiceCards";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
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
        { value: "critical", title: "Critical", description: "For an outage that needs someone now. Alerts by email and webhook." },
        { value: "warning", title: "Warning", description: "For a problem that can wait for working hours. Alerts by email." },
      ]}
    />
  );
}

/** A select over a fixed list of seconds, such as a check interval or a grace time. */
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
