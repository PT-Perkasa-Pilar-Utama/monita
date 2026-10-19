import { expect, test } from "bun:test";
import { appliesTo, availableActions, planAction } from "@/lib/users";
import type { AdminUserRow } from "@/lib/types";

const u = (email: string, role: AdminUserRow["role"], status: AdminUserRow["status"]): AdminUserRow => ({ id: email.length, email, role, status, created_at: 0 });
const me = "admin@x.co";
const self = u(me, "super_admin", "active");
const tl = u("rina@x.co", "tl", "active");
const admin = u("budi@x.co", "super_admin", "invited");
const gone = u("old@x.co", "tl", "disabled");
const sup = u("sari@x.co", "supervisor", "active");

test("nobody acts on their own account", () => {
  for (const a of ["make_tl", "reset", "disable", "enable"] as const) expect(appliesTo(a, self, me)).toBe(false);
});

test("a role changes only to one of the other two, and only on a live account", () => {
  expect(appliesTo("make_admin", tl, me)).toBe(true);
  expect(appliesTo("make_tl", tl, me)).toBe(false);
  expect(appliesTo("make_supervisor", tl, me)).toBe(true);
  expect(appliesTo("make_tl", admin, me)).toBe(true);
  expect(appliesTo("make_supervisor", sup, me)).toBe(false);
  expect(appliesTo("make_tl", sup, me)).toBe(true);
  expect(appliesTo("make_admin", gone, me)).toBe(false);
});

test("a disabled account can only be enabled", () => {
  expect(availableActions([gone], me)).toEqual(["enable"]);
  expect(availableActions([tl], me)).toEqual(["make_admin", "make_supervisor", "reset", "disable"]);
});

test("a batch runs on the accounts it fits and counts the rest", () => {
  const plan = planAction("disable", [self, tl, gone], me);
  expect(plan.targets.map((x) => x.email)).toEqual(["rina@x.co"]);
  expect(plan.skipped).toBe(2);
});
