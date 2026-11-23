// Headings on lists the server sorts, whose `sort` values are not a date. Each heading needs a
// value for both directions, or a second click asks for the order already shown.
import { dateHead } from "@/lib/dateSort";
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

/** A Project heading's arrow: `project` is A to Z, `project_desc` Z to A. */
function projectHead(param: string): Sort {
  if (param === "project") return { by: "project", dir: "asc" };
  if (param === "project_desc") return { by: "project", dir: "desc" };
  return null;
}

/** A click on a Project heading: A to Z, Z to A, then the list's own order. */
function nextProject(param: string): string {
  if (param === "project") return "project_desc";
  if (param === "project_desc") return "";
  return "project";
}

/** All incidents' arrow. The empty sort is critical first, so Alert level reads descending. */
export function incidentsHead(param: string): Sort {
  if (param === "") return { by: "severity", dir: "desc" };
  if (param === "warning_first") return { by: "severity", dir: "asc" };
  return projectHead(param) ?? dateHead("started", param, false);
}

/** The sort after a click on All incidents' `by` heading. Alert level flips; Project runs A to Z,
 * Z to A, then critical first; Started runs newest, oldest, then critical first. */
export function nextIncidentsSort(param: string, by: string): string {
  if (by === "severity") return param === "" ? "warning_first" : "";
  if (by === "project") return nextProject(param);
  if (param === "newest") return "oldest";
  if (param === "oldest") return "";
  return "newest";
}

/** All tickets' arrow. The empty sort is newest first, so Reported reads descending. */
export function ticketsHead(param: string): Sort {
  return projectHead(param) ?? dateHead("reported", param, true);
}

/** The sort after a click on All tickets' `by` heading. Reported flips between newest and oldest
 * first; Project runs A to Z, Z to A, then newest first. */
export function nextTicketsSort(param: string, by: string): string {
  if (by === "project") return nextProject(param);
  return param === "" ? "oldest" : "";
}
