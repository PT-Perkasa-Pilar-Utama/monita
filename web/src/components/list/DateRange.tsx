import type { ReactElement } from "react";
import { Input } from "@/components/ui/input";

/** A From and a To day for a list the server filters by time. Each is `YYYY-MM-DD` on the viewer's clock, or "" for no bound. */
export function DateRange({ from, to, onChange }: { from: string; to: string; onChange: (patch: { from?: string; to?: string }) => void }): ReactElement {
  return (
    <div className="flex items-center gap-1.5 max-sm:w-full">
      <Input type="date" aria-label="From" value={from} max={to || undefined} onChange={(e) => onChange({ from: e.target.value })} className="h-8 w-auto text-[13px] max-sm:flex-1" />
      <span className="text-sm text-muted-foreground">to</span>
      <Input type="date" aria-label="To" value={to} min={from || undefined} onChange={(e) => onChange({ to: e.target.value })} className="h-8 w-auto text-[13px] max-sm:flex-1" />
    </div>
  );
}
