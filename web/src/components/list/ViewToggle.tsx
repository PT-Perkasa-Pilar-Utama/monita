import type { ReactElement } from "react";
import { ChartLine, LayoutGrid, List } from "@/components/icons";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import type { ViewMode } from "@/lib/useViewMode";

// ViewToggle switches a list page between a chart, cards and a table. A click on the
// open view keeps it open: Radix reports that as an empty value.
export function ViewToggle({ value, onChange, chart = false }: { value: ViewMode; onChange: (v: ViewMode) => void; chart?: boolean }): ReactElement {
  return (
    <ToggleGroup
      type="single"
      variant="outline"
      size="default"
      value={value}
      onValueChange={(v) => v && onChange(v as ViewMode)}
      aria-label="View"
    >
      {chart && (
        <ToggleGroupItem value="chart" aria-label="Chart view" title="Chart view">
          <ChartLine />
        </ToggleGroupItem>
      )}
      <ToggleGroupItem value="card" aria-label="Card view" title="Card view">
        <LayoutGrid />
      </ToggleGroupItem>
      <ToggleGroupItem value="list" aria-label="List view" title="List view">
        <List />
      </ToggleGroupItem>
    </ToggleGroup>
  );
}
