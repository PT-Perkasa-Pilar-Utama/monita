import { useSearchParams } from "react-router-dom";
import { nextSort, parseSort, type Sort } from "@/lib/sort";

/** A table's order in the URL (`?sort=&dir=`), beside its filters. Returns the order and a heading click. */
export function useSort(allowed: readonly string[]): [Sort, (by: string) => void] {
  const [params, setParams] = useSearchParams();
  const sort = parseSort(params.get("sort"), params.get("dir"), allowed);
  const click = (by: string): void => {
    const next = nextSort(sort, by);
    setParams(
      (p) => {
        const out = new URLSearchParams(p);
        out.delete("sort");
        out.delete("dir");
        if (next) {
          out.set("sort", next.by);
          out.set("dir", next.dir);
        }
        return out;
      },
      { replace: true },
    );
  };
  return [sort, click];
}
