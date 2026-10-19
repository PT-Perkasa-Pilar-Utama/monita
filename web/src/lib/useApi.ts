import { useCallback, useEffect, useState } from "react";
import { api, ApiError } from "@/lib/api";

/** How often a live page reads again: the shortest check interval. */
export const LIVE_MS = 30_000;

export type Loaded<T> = {
  data: T | undefined;
  error: ApiError | undefined;
  loading: boolean;
  reload: () => void;
  /** Reads again in place, for a Refresh button: the data stays until the new data lands. */
  refresh: () => void;
  refreshing: boolean;
};

/** Reads `path` once, and again on `reload()` or when `path` changes. `null` reads nothing.
 * With `pollMs`, it also reads again on that interval without setting `loading`, so a live list
 * refreshes in place instead of flashing its skeleton. */
export function useApi<T>(path: string | null, pollMs?: number): Loaded<T> {
  const [data, setData] = useState<T | undefined>(undefined);
  const [error, setError] = useState<ApiError | undefined>(undefined);
  const [loading, setLoading] = useState(path !== null);
  const [tick, setTick] = useState(0);
  const reload = useCallback(() => setTick((t) => t + 1), []);
  const [refreshing, setRefreshing] = useState(false);
  const refresh = useCallback(() => {
    if (path === null) return;
    setRefreshing(true);
    api<T>(path, "GET", undefined, "no-cache")
      .then((d) => {
        setData(d);
        setError(undefined);
      })
      .catch((e: unknown) => setError(e instanceof ApiError ? e : new ApiError(0, "network", "Monita did not answer. Check the connection.")))
      .finally(() => setRefreshing(false));
  }, [path]);

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

  useEffect(() => {
    if (path === null || !pollMs) return;
    let live = true;
    const id = setInterval(() => {
      // A hidden tab skips the read; the next one after it shows again catches up.
      if (document.hidden) return;
      api<T>(path, "GET", undefined, "no-cache")
        .then((d) => live && setData(d))
        .catch(() => undefined); // A missed refresh keeps the last data; the next one tries again.
    }, pollMs);
    return () => {
      live = false;
      clearInterval(id);
    };
  }, [path, pollMs]);

  return { data, error, loading, reload, refresh, refreshing };
}
