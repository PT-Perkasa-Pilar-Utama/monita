import type { ReactElement } from "react";
import { Search } from "@/components/icons";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import type { ListFilter } from "@/lib/filter";

export type Option = { value: string; label: string };

// Radix Select keeps no empty value, so "any" stands for an unset select.
const ANY = "any";

function FilterSelect({ label, value, options, onChange }: { label: string; value: string; options: Option[]; onChange: (v: string) => void }): ReactElement {
  return (
    <Select value={value || ANY} onValueChange={(v) => onChange(v === ANY ? "" : v)}>
      <SelectTrigger size="sm" aria-label={label} className="w-auto min-w-36 max-sm:flex-1">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          {options.map((o) => (
            <SelectItem key={o.value || ANY} value={o.value || ANY}>
              {o.label}
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  );
}

export const STATE_OPTIONS: Option[] = [
  { value: "", label: "Any state" },
  { value: "up", label: "Up" },
  { value: "down", label: "Down" },
  { value: "issue", label: "Issue" },
  { value: "maintenance", label: "Maintenance" },
  { value: "unknown", label: "No data yet" },
];

export const SEVERITY_OPTIONS: Option[] = [
  { value: "", label: "Any severity" },
  { value: "critical", label: "Critical" },
  { value: "warning", label: "Warning" },
];

/** A table band's search and its State and Severity selects. A select applies at once. */
export function ListFilters({
  noun,
  filter,
  onChange,
  stateOptions = STATE_OPTIONS,
  selects = true,
}: {
  noun: string;
  filter: ListFilter;
  onChange: (patch: Partial<ListFilter>) => void;
  stateOptions?: Option[];
  /** False for a list with search only. */
  selects?: boolean;
}): ReactElement {
  return (
    <>
      <div className="relative max-sm:w-full">
        <Search className="pointer-events-none absolute top-2 left-2.5 size-4 text-muted-foreground" />
        <Input
          type="search"
          value={filter.q}
          onChange={(e) => onChange({ q: e.target.value })}
          placeholder={`Search ${noun}`}
          aria-label={`Search ${noun}`}
          className="h-8 pl-8 text-[13px] sm:w-60"
        />
      </div>
      {selects && <FilterSelect label="State" value={filter.state} options={stateOptions} onChange={(state) => onChange({ state })} />}
      {selects && <FilterSelect label="Severity" value={filter.severity} options={SEVERITY_OPTIONS} onChange={(severity) => onChange({ severity })} />}
    </>
  );
}
