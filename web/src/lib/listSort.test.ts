import { expect, test } from "bun:test";
import { incidentsHead, nextIncidentsSort, nextProjectsSort, nextTicketsSort, projectsHead, ticketsHead } from "@/lib/listSort";

test("the Project heading runs A to Z, Z to A, then newest first", () => {
  expect(nextProjectsSort("")).toBe("name");
  expect(nextProjectsSort("oldest")).toBe("name");
  expect(nextProjectsSort("name")).toBe("name_desc");
  expect(nextProjectsSort("name_desc")).toBe("");
  expect(projectsHead("name")).toEqual({ by: "name", dir: "asc" });
  expect(projectsHead("name_desc")).toEqual({ by: "name", dir: "desc" });
  expect(projectsHead("")).toBeNull();
  expect(projectsHead("oldest")).toBeNull();
});

test("Alert level flips between critical first and warning first", () => {
  expect(incidentsHead("")).toEqual({ by: "severity", dir: "desc" });
  expect(nextIncidentsSort("", "severity")).toBe("warning_first");
  expect(incidentsHead("warning_first")).toEqual({ by: "severity", dir: "asc" });
  expect(nextIncidentsSort("warning_first", "severity")).toBe("");
  expect(nextIncidentsSort("newest", "severity")).toBe("");
});

test("Started runs newest, oldest, then critical first, from any Alert level order", () => {
  expect(nextIncidentsSort("", "started")).toBe("newest");
  expect(nextIncidentsSort("warning_first", "started")).toBe("newest");
  expect(nextIncidentsSort("newest", "started")).toBe("oldest");
  expect(nextIncidentsSort("oldest", "started")).toBe("");
  expect(incidentsHead("newest")).toEqual({ by: "started", dir: "desc" });
  expect(incidentsHead("oldest")).toEqual({ by: "started", dir: "asc" });
});

test("Project on All incidents runs A to Z, Z to A, then critical first, and every heading leaves it", () => {
  expect(nextIncidentsSort("", "project")).toBe("project");
  expect(incidentsHead("project")).toEqual({ by: "project", dir: "asc" });
  expect(nextIncidentsSort("project", "project")).toBe("project_desc");
  expect(incidentsHead("project_desc")).toEqual({ by: "project", dir: "desc" });
  expect(nextIncidentsSort("project_desc", "project")).toBe("");
  expect(nextIncidentsSort("project", "started")).toBe("newest");
  expect(nextIncidentsSort("project_desc", "severity")).toBe("");
  expect(nextIncidentsSort("newest", "project")).toBe("project");
});

test("All tickets sorts by Reported or by Project, and each heading takes over from the other", () => {
  expect(ticketsHead("")).toEqual({ by: "reported", dir: "desc" });
  expect(ticketsHead("oldest")).toEqual({ by: "reported", dir: "asc" });
  expect(ticketsHead("project")).toEqual({ by: "project", dir: "asc" });
  expect(ticketsHead("project_desc")).toEqual({ by: "project", dir: "desc" });
  expect(nextTicketsSort("", "reported")).toBe("oldest");
  expect(nextTicketsSort("oldest", "reported")).toBe("");
  expect(nextTicketsSort("project", "reported")).toBe("");
  expect(nextTicketsSort("", "project")).toBe("project");
  expect(nextTicketsSort("project", "project")).toBe("project_desc");
  expect(nextTicketsSort("project_desc", "project")).toBe("");
  expect(nextTicketsSort("oldest", "project")).toBe("project");
});
