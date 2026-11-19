// Headings on lists the server sorts, whose `sort` values are not a date. Each heading needs a
// value for both directions, or a second click asks for the order already shown.
import { dateHead, nextDate } from "@/lib/dateSort";
import type { Sort } from "@/lib/sort";

/** The Projects table's arrow: `name` is A to Z, `name_desc` Z to A; newest and oldest have no column. */
export function projectsHead(param: string): Sort {
  if (param === "name") return { by: "name", dir: "asc" };
  if (param === "name_desc") return { by: "name", dir: "desc" };
  return null;
}

/** The sort after a click on the Project heading: A to Z, Z to A, then newest first. */
export function nextProjectsSort(param: string): string {
  if (param === "name") return "name_desc";
  if (param === "name_desc") return "";
  return "name";
}

/** All incidents' arrow. The empty sort is critical first, so Alert level reads descending. */
export function incidentsHead(param: string): Sort {
  if (param === "") return { by: "severity", dir: "desc" };
  if (param === "warning_first") return { by: "severity", dir: "asc" };
  return dateHead("started", param, false);
}

/** The sort after a click on All incidents' `by` heading: Alert level flips; Started runs newest, oldest, then critical first. */
export function nextIncidentsSort(param: string, by: string): string {
  if (by === "severity") return param === "" ? "warning_first" : "";
  return nextDate(param === "warning_first" ? "" : param, false);
}
