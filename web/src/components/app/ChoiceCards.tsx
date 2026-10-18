import type { ReactElement } from "react";
import { FieldLegend, FieldSet } from "@/components/ui/field";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { cn } from "@/lib/utils";

export type CardChoice<T extends string> = { value: T; title: string; description: string };

/**
 * A short set of options as cards: the name on its own line and one plain sentence under it.
 * The chosen card takes a dark border; the brand colour never marks a choice.
 */
export function ChoiceCards<T extends string>({
  id,
  legend,
  value,
  choices,
  onChange,
}: {
  id: string;
  legend: string;
  value: T;
  choices: CardChoice<T>[];
  onChange: (v: T) => void;
}): ReactElement {
  return (
    <FieldSet>
      <FieldLegend variant="label">{legend}</FieldLegend>
      <RadioGroup value={value} onValueChange={(v) => onChange(v as T)} className="grid gap-3 sm:grid-cols-2">
        {choices.map((c) => {
          const on = c.value === value;
          return (
            <label
              key={c.value}
              htmlFor={`${id}-${c.value}`}
              className={cn(
                "flex cursor-pointer items-start gap-3 rounded-lg border bg-card p-4 transition-colors hover:border-foreground/30",
                on && "border-foreground ring-1 ring-foreground",
              )}
            >
              <RadioGroupItem value={c.value} id={`${id}-${c.value}`} className="mt-0.5" />
              <span className="flex flex-col gap-1">
                <span className="text-sm leading-none font-medium">{c.title}</span>
                <span className="text-[13px] leading-snug text-muted-foreground">{c.description}</span>
              </span>
            </label>
          );
        })}
      </RadioGroup>
    </FieldSet>
  );
}
