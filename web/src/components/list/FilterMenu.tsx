import type { ReactElement } from "react";
import { ChevronDown } from "@/components/icons";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";

export type FilterOption = { value: string; label: string };

// FilterMenu is one filter in a list's band: a quiet button that names the
// filter, and a menu of choices. A set filter shows its choice on the button.
export function FilterMenu({
  label,
  heading,
  value,
  options,
  onChange,
}: {
  // The button text when nothing is chosen: "Owner".
  label: string;
  // The menu's heading: "Filter by owner".
  heading: string;
  // "" is the first option, the one that keeps every row.
  value: string;
  options: FilterOption[];
  onChange: (v: string) => void;
}): ReactElement {
  const chosen = value ? options.find((o) => o.value === value) : undefined;
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="sm" className={cn("max-w-56", chosen && "bg-accent text-foreground")}>
          <span className="truncate">{chosen ? chosen.label : label}</span>
          <ChevronDown data-icon="inline-end" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="max-h-80 w-56 overflow-y-auto">
        <DropdownMenuLabel>{heading}</DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuRadioGroup value={value} onValueChange={onChange}>
          {options.map((o) => (
            <DropdownMenuRadioItem key={o.value || "all"} value={o.value}>
              <span className="truncate">{o.label}</span>
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
