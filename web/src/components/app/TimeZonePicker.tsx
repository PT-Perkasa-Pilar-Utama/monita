import { useEffect, useId, useMemo, useRef, useState } from "react";
import type { KeyboardEvent, ReactElement } from "react";
import { Popover } from "radix-ui";
import { Check, ChevronDownIcon } from "@/components/icons";
import { Input } from "@/components/ui/input";
import { matches } from "@/lib/filter";
import { zone } from "@/lib/time";
import { cn } from "@/lib/utils";

/** An offset in minutes east of UTC and the places that use it. */
export type Zone = [number, string];

const label = ([min, places]: Zone): string => `${zone(min)}: ${places}`;

/**
 * Pick an offset by typing a place or an offset, the way `UserPicker` picks a person: the field
 * filters the rows under it, the first match is highlighted, and Enter or a click picks it.
 */
export function TimeZonePicker({
  id,
  zones,
  value,
  onChange,
}: {
  id: string;
  zones: Zone[];
  value: number;
  onChange: (min: number) => void;
}): ReactElement {
  const listId = `${useId()}-list`;
  const listRef = useRef<HTMLUListElement>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState<string | null>(null);
  const [active, setActive] = useState(0);
  const shown = useMemo(() => zones.filter(([min, places]) => matches(query ?? "", zone(min), places)), [zones, query]);
  const current = zones.find(([min]) => min === value);

  useEffect(() => {
    if (open) listRef.current?.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: "nearest" });
  }, [open, active]);

  const show = (): void => {
    setQuery(null);
    setActive(Math.max(0, zones.findIndex(([min]) => min === value)));
    setOpen(true);
  };

  const close = (): void => {
    setQuery(null);
    setOpen(false);
  };

  const pick = ([min]: Zone): void => {
    onChange(min);
    close();
  };

  const onKey = (e: KeyboardEvent<HTMLInputElement>): void => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      if (!open) return show();
      const step = e.key === "ArrowDown" ? 1 : -1;
      setActive((a) => (shown.length ? (a + step + shown.length) % shown.length : 0));
    } else if (e.key === "Enter" && open && shown[active]) {
      e.preventDefault();
      pick(shown[active]);
    } else if (e.key === "Escape" && open) {
      e.preventDefault();
      close();
    }
  };

  // The list renders in a portal: a settings card clips its overflow.
  return (
    <Popover.Root open={open} onOpenChange={(o) => !o && close()}>
      <Popover.Anchor className="relative">
        <Input
          id={id}
          role="combobox"
          aria-expanded={open}
          aria-controls={listId}
          aria-autocomplete="list"
          aria-activedescendant={open && shown[active] ? `${listId}-${active}` : undefined}
          autoComplete="off"
          spellCheck={false}
          placeholder="Search a city or an offset"
          className="pr-9"
          value={query ?? (current ? label(current) : zone(value))}
          onFocus={(e) => {
            e.target.select();
            show();
          }}
          onClick={() => !open && show()}
          onBlur={close}
          onChange={(e) => {
            setQuery(e.target.value);
            setActive(0);
            setOpen(true);
          }}
          onKeyDown={onKey}
        />
        <ChevronDownIcon className="pointer-events-none absolute top-2.5 right-3 size-4 text-muted-foreground opacity-50" />
      </Popover.Anchor>
      <Popover.Portal>
        <Popover.Content
          role="presentation"
          side="bottom"
          align="start"
          sideOffset={4}
          onOpenAutoFocus={(e) => e.preventDefault()}
          onCloseAutoFocus={(e) => e.preventDefault()}
          className="z-50 w-(--radix-popover-trigger-width)"
        >
          <ul
            ref={listRef}
            id={listId}
            role="listbox"
            aria-label="Time zones"
            className="max-h-[min(18rem,var(--radix-popover-content-available-height))] overflow-y-auto rounded-md border bg-popover p-1 text-popover-foreground shadow-md"
          >
            {shown.length === 0 && <li className="px-2 py-1.5 text-sm text-muted-foreground">No place or offset matches.</li>}
            {shown.map((z, i) => (
              <li
                key={z[0]}
                id={`${listId}-${i}`}
                data-index={i}
                role="option"
                aria-selected={z[0] === value}
                onMouseDown={(e) => e.preventDefault()}
                onMouseEnter={() => setActive(i)}
                onClick={() => pick(z)}
                className={cn(
                  "flex cursor-pointer items-center gap-2 rounded-md py-1.5 pr-2 pl-2 text-sm",
                  i === active && "bg-accent text-accent-foreground",
                )}
              >
                <span className="flex-1 truncate">{label(z)}</span>
                {z[0] === value && <Check className="size-4 text-muted-foreground" />}
              </li>
            ))}
          </ul>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
