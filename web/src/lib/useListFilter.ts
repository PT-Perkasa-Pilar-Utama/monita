import { useSearchParams } from "react-router-dom";
import type { ListFilter } from "@/lib/filter";

/** A list's filter, kept in the URL (`?q=&state=&severity=`) so a link or Back restores it. */
export function useListFilter(): [ListFilter, (patch: Partial<ListFilter>) => void, () => void] {
  const [params, setParams] = useSearchParams();
  const f: ListFilter = { q: params.get("q") ?? "", state: params.get("state") ?? "", severity: params.get("severity") ?? "" };
  // The filter's own keys change; anything else in the URL, such as the sort, stays.
  const write = (next: ListFilter): void => {
    const out = new URLSearchParams(params);
    for (const [k, v] of Object.entries(next)) {
      if (v) out.set(k, v);
      else out.delete(k);
    }
    setParams(out, { replace: true });
  };
  return [f, (patch) => write({ ...f, ...patch }), () => write({ q: "", state: "", severity: "" })];
}
