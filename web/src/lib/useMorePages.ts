import { useState } from "react";
import { api, errorText } from "@/lib/api";
import type { ColumnPage, Stage } from "@/lib/types";
import { toast } from "sonner";

/** Later pages of a board's columns, read one Show more at a time. `reset` drops them. */
export function useMorePages<T>(base: string): {
  more: T[];
  busy: Stage | null;
  load: (stage: Stage) => Promise<void>;
  reset: () => void;
} {
  const [more, setMore] = useState<T[]>([]);
  const [pages, setPages] = useState<Partial<Record<Stage, number>>>({});
  const [busy, setBusy] = useState<Stage | null>(null);
  const load = async (stage: Stage): Promise<void> => {
    const next = (pages[stage] ?? 1) + 1;
    setBusy(stage);
    try {
      const r = await api<ColumnPage<T>>(`${base}?stage=${stage}&page=${next}`);
      setMore((m) => [...m, ...r.tickets]);
      setPages((p) => ({ ...p, [stage]: next }));
    } catch (err) {
      toast.error(errorText(err));
    } finally {
      setBusy(null);
    }
  };
  const reset = (): void => {
    setMore([]);
    setPages({});
  };
  return { more, busy, load, reset };
}
