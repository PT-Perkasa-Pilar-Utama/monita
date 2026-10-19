import { expect, test } from "bun:test";
import { adminPage } from "@/lib/adminPage";

test("a stored admin page opens again", () => {
  expect(adminPage("/admin/email")).toBe("/admin/email");
});

test("nothing stored, or a path that is not an admin page, opens User management", () => {
  expect(adminPage("")).toBe("/admin/users");
  expect(adminPage("/admin/settings")).toBe("/admin/users");
  expect(adminPage("/projects/x")).toBe("/admin/users");
});
