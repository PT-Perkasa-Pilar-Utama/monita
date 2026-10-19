import type { ReactElement } from "react";
import { Checkbox } from "@/components/ui/checkbox";
import { FieldLegend, FieldSet } from "@/components/ui/field";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { cn } from "@/lib/utils";

export type CardChoice<T extends string> = {
  value: T;
  title: string;
  description: string;
  // Why the choice is off right now; the card shows it in place of the description.
  unavailable?: string | undefined;
};

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
          const off = c.unavailable !== undefined;
          return (
            <label
              key={c.value}
              htmlFor={`${id}-${c.value}`}
              className={cn(
                "flex cursor-pointer items-start gap-3 rounded-lg border bg-card p-4 transition-colors hover:border-foreground/30",
                on && "border-foreground ring-1 ring-foreground",
                off && "cursor-not-allowed border-dashed bg-transparent hover:border-border",
              )}
            >
              <RadioGroupItem value={c.value} id={`${id}-${c.value}`} disabled={off} className="mt-0.5" />
              <span className="flex flex-col gap-1">
                <span className={cn("text-sm leading-none font-medium", off && "text-muted-foreground")}>{c.title}</span>
                <span className="text-[13px] leading-snug text-muted-foreground">{c.unavailable ?? c.description}</span>
              </span>
            </label>
          );
        })}
      </RadioGroup>
    </FieldSet>
  );
}

export type CheckCard<K extends string> = {
  key: K;
  title: string;
  description: string;
  /** Always on; the card shows it ticked and cannot change it. */
  locked?: boolean;
};

/** Options that turn on and off independently, as cards in a grid that wraps as the screen narrows.
 * A ticked card takes the same dark border as a chosen `ChoiceCards` card. */
export function CheckCards<K extends string>({
  id,
  legend,
  cards,
  value,
  onChange,
}: {
  id: string;
  legend: string;
  cards: CheckCard<K>[];
  value: Record<K, boolean>;
  onChange: (k: K, on: boolean) => void;
}): ReactElement {
  return (
    <FieldSet>
      <FieldLegend className="sr-only">{legend}</FieldLegend>
      <div className="grid grid-cols-[repeat(auto-fill,minmax(min(100%,13rem),1fr))] gap-3">
        {cards.map((c) => {
          const on = c.locked === true || value[c.key];
          return (
            <label
              key={c.key}
              htmlFor={`${id}-${c.key}`}
              className={cn(
                "flex cursor-pointer items-start gap-3 rounded-lg border bg-card p-4 transition-colors hover:border-foreground/30",
                on && "border-foreground ring-1 ring-foreground",
                c.locked && "cursor-default hover:border-foreground",
              )}
            >
              <Checkbox id={`${id}-${c.key}`} checked={on} disabled={c.locked} onCheckedChange={(v) => onChange(c.key, v === true)} className="mt-0.5" />
              <span className="flex flex-col gap-1">
                <span className="text-sm leading-none font-medium">{c.title}</span>
                <span className="text-[13px] leading-snug text-muted-foreground">{c.description}</span>
              </span>
            </label>
          );
        })}
      </div>
    </FieldSet>
  );
}
