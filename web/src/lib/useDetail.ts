import { useSearchParams } from "react-router-dom";
import { useApi, type Loaded } from "@/lib/useApi";

/** A monitor's or heartbeat's page, read with the chart range and history cursor in the URL. */
export function useDetail<T>(path: string): Loaded<T> & { older: (cursor: string) => string; newest: string | null } {
  const [params] = useSearchParams();
  const q = new URLSearchParams();
  for (const k of ["range", "before"]) {
    const v = params.get(k);
    if (v) q.set(k, v);
  }
  const res = useApi<T>(q.size ? `${path}?${q}` : path);
  const at = (before: string | null): string => {
    const next = new URLSearchParams(q);
    if (before) next.set("before", before);
    else next.delete("before");
    return next.size ? `?${next}` : "?";
  };
  return { ...res, older: (cursor) => at(cursor), newest: q.has("before") ? at(null) : null };
}
