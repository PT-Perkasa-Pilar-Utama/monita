import { useSearchParams } from "react-router-dom";
import type { ListFilter } from "@/lib/filter";

/** A list's filter, kept in the URL (`?q=&state=&severity=`) so a link or Back restores it. */
export function useListFilter(): [ListFilter, (patch: Partial<ListFilter>) => void, () => void] {
  const [params, setParams] = useSearchParams();
  const f: ListFilter = { q: params.get("q") ?? "", state: params.get("state") ?? "", severity: params.get("severity") ?? "" };
  const set = (patch: Partial<ListFilter>): void => {
    const next = { ...f, ...patch };
    const out = new URLSearchParams();
    for (const [k, v] of Object.entries(next)) if (v) out.set(k, v);
    setParams(out, { replace: true });
  };
  return [f, set, () => setParams(new URLSearchParams(), { replace: true })];
}
