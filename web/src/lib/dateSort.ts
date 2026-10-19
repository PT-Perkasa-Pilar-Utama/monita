// A date heading on a server-sorted list. The URL's `sort` is "newest", "oldest" or "", and ""
// means the list's own default order.
import type { Sort } from "@/lib/sort";

/** The heading's arrow. With `newestByDefault`, an empty sort already reads newest first. */
export function dateHead(by: string, param: string, newestByDefault: boolean): Sort {
  if (param === "oldest") return { by, dir: "asc" };
  if (param === "newest" || (param === "" && newestByDefault)) return { by, dir: "desc" };
  return null;
}

/** The sort after a click on the heading: newest, then oldest, then the list's own order. */
export function nextDate(param: string, newestByDefault: boolean): string {
  if (newestByDefault) return param === "oldest" ? "" : "oldest";
  if (param === "") return "newest";
  return param === "newest" ? "oldest" : "";
}
