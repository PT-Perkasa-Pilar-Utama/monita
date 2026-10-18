import { useCallback, useEffect, useState } from "react";
import { api, ApiError } from "@/lib/api";

export type Loaded<T> = {
  data: T | undefined;
  error: ApiError | undefined;
  loading: boolean;
  reload: () => void;
};

/** Reads `path` once, and again on `reload()` or when `path` changes. `null` reads nothing. */
export function useApi<T>(path: string | null): Loaded<T> {
  const [data, setData] = useState<T | undefined>(undefined);
  const [error, setError] = useState<ApiError | undefined>(undefined);
  const [loading, setLoading] = useState(path !== null);
  const [tick, setTick] = useState(0);
  const reload = useCallback(() => setTick((t) => t + 1), []);

  useEffect(() => {
    if (path === null) return;
    let live = true;
    setLoading(true);
    // A reload asks the server again, past a public page's 30-second cache.
    api<T>(path, "GET", undefined, tick > 0 ? "no-cache" : undefined)
      .then((d) => {
        if (!live) return;
        setData(d);
        setError(undefined);
      })
      .catch((e: unknown) => {
        if (!live) return;
        setError(e instanceof ApiError ? e : new ApiError(0, "network", "Monita did not answer. Check the connection."));
      })
      .finally(() => {
        if (live) setLoading(false);
      });
    return () => {
      live = false;
    };
  }, [path, tick]);

  return { data, error, loading, reload };
}
