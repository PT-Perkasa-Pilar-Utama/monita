import type { ReactElement, ReactNode } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
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

/** The column beside a create or edit form: what the thing will do, in numbered sentences. */
export function FormAside({ title, steps, note }: { title: string; steps: ReactNode[]; note: string }): ReactElement {
  return (
    <Card className="bg-paper xl:sticky xl:top-[calc(var(--bar-h)+24px)]">
      <CardHeader>
        <CardTitle className="text-[15px]">{title}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <ol className="flex flex-col gap-3">
          {steps.map((s, i) => (
            <li key={i} className="flex gap-3">
              <span className="grid size-6 shrink-0 place-items-center rounded-full bg-primary/15 text-xs font-semibold">{i + 1}</span>
              <span className="text-foreground/90">{s}</span>
            </li>
          ))}
        </ol>
        <p className="text-xs text-muted-foreground">{note}</p>
      </CardContent>
    </Card>
  );
}
