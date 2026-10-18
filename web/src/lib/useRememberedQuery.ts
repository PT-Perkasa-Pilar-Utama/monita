import { useEffect } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { restoreQuery } from "@/lib/page";
import { readLocal, writeLocal } from "@/lib/store";

/**
 * A list's query string (`?q=&page=`), kept in the URL and remembered in this browser. A visit
 * without one opens the query used last, so coming back to the list lands on the same page.
 * Returns the query to read now and a setter that remembers before it navigates.
 */
export function useRememberedQuery(key: string): [string, (search: string, replace: boolean) => void] {
  const { search: inUrl } = useLocation();
  const navigate = useNavigate();
  const search = restoreQuery(inUrl, readLocal(key));

  useEffect(() => {
    if (search !== inUrl) navigate({ search }, { replace: true });
    else writeLocal(key, search);
  }, [key, search, inUrl, navigate]);

  const set = (next: string, replace: boolean): void => {
    writeLocal(key, next);
    navigate({ search: next }, { replace, preventScrollReset: replace });
  };
  return [search, set];
}
