import { expect, test } from "bun:test";
import { incidentsHead, nextIncidentsSort, nextProjectsSort, projectsHead } from "@/lib/listSort";

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
