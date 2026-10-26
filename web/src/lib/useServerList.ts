import { useEffect, useState } from "react";
import { listQuery, pageParam } from "@/lib/page";
import { useRememberedQuery } from "@/lib/useRememberedQuery";

/** A server-read list's URL: search, filters, sort and page. */
export type ServerListParams = { q: string; state: string; severity: string; sort: string; page: number };

/**
 * A list the server pages and filters. The query sits in the URL and is remembered in this
 * browser, so Back and a return visit land on the same rows. The search waits for a pause in
 * typing before it asks the server.
 */
export function useServerList(key: string, active = true): {
  params: ServerListParams;
  query: string;
  draft: string;
  setDraft: (q: string) => void;
  set: (patch: Partial<ServerListParams>) => void;
  clear: () => void;
  filtered: boolean;
} {
  const [search, setSearch] = useRememberedQuery(key, active);
  const u = new URLSearchParams(search);
  const params: ServerListParams = {
    q: u.get("q") ?? "",
    state: u.get("state") ?? "",
    severity: u.get("severity") ?? "",
    sort: u.get("sort") ?? "",
    page: pageParam(u.get("page")),
  };
  const [draft, setDraft] = useState(params.q);
  // A change of filter starts at page 1; a change of page keeps the filters.
  const set = (patch: Partial<ServerListParams>): void => setSearch(listQuery({ ...params, page: 1, ...patch }), patch.page === undefined);
  useEffect(() => {
    if (draft.trim() === params.q.trim()) return;
    const t = setTimeout(() => set({ q: draft }), 250);
    return () => clearTimeout(t);
  }, [draft, params.q]);
  useEffect(() => {
    if (draft.trim() !== params.q.trim()) setDraft(params.q);
  }, [params.q]);
  const clear = (): void => {
    setDraft("");
    setSearch(listQuery({ q: "", sort: params.sort, page: 1 }), true);
  };
  return { params, query: listQuery(params), draft, setDraft, set, clear, filtered: params.q !== "" || params.state !== "" || params.severity !== "" };
}
