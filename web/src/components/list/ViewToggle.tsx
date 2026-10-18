import type { ReactElement } from "react";
import { LayoutGrid, List } from "@/components/icons";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import type { ViewMode } from "@/lib/useViewMode";

// ViewToggle switches a list page between cards and a table. A click on the
// open view keeps it open: Radix reports that as an empty value.
export function ViewToggle({ value, onChange }: { value: ViewMode; onChange: (v: ViewMode) => void }): ReactElement {
  return (
    <ToggleGroup
      type="single"
      variant="outline"
      size="default"
      value={value}
      onValueChange={(v) => v && onChange(v as ViewMode)}
      aria-label="View"
    >
      <ToggleGroupItem value="card" aria-label="Card view" title="Card view">
        <LayoutGrid />
      </ToggleGroupItem>
      <ToggleGroupItem value="list" aria-label="List view" title="List view">
        <List />
      </ToggleGroupItem>
    </ToggleGroup>
  );
}
