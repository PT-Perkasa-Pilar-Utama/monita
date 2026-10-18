import type { ReactElement } from "react";
import { Field, FieldError, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import type { Choice, Severity } from "@/lib/types";

/** Critical or Warning, as two cards: what each one alerts by. */
export function SeverityField({ value, onChange }: { value: Severity; onChange: (s: Severity) => void }): ReactElement {
  return (
    <FieldSet>
      <FieldLegend variant="label">Severity</FieldLegend>
      <RadioGroup value={value} onValueChange={(v) => onChange(v as Severity)} className="grid gap-3 sm:grid-cols-2">
        {(
          [
            ["critical", "Critical", "email and webhook"],
            ["warning", "Warning", "email only"],
          ] as const
        ).map(([v, title, how]) => (
          <FieldLabel key={v} htmlFor={`sev-${v}`}>
            <Field orientation="horizontal">
              <RadioGroupItem value={v} id={`sev-${v}`} />
              <span>
                <strong className="font-semibold">{title}</strong> <span className="text-muted-foreground">{how}</span>
              </span>
            </Field>
          </FieldLabel>
        ))}
      </RadioGroup>
    </FieldSet>
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
