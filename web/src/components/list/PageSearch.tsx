import type { ReactElement } from "react";
import { Search } from "@/components/icons";
import { Input } from "@/components/ui/input";

// PageSearch is the search beside a page's title, for a page that is a report
// rather than one table: it narrows every card on the page at once.
export function PageSearch({
  value,
  onChange,
  placeholder,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder: string;
}): ReactElement {
  return (
    <div className="relative w-full sm:w-64">
      <Search className="absolute top-2.5 left-2.5 size-4 text-muted-foreground" />
      <Input
        type="search"
        aria-label={placeholder}
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="pl-8"
      />
    </div>
  );
}
