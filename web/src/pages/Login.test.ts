import { expect, test } from "bun:test";
import { nextPath } from "@/pages/Login";

const ORIGIN = "https://monita.example";

test("sign-in returns only to a path on this site", () => {
  expect(nextPath("/projects/shop/monitors?state=down", ORIGIN)).toBe("/projects/shop/monitors?state=down");
  expect(nextPath(null, ORIGIN)).toBe("/");
  expect(nextPath("https://evil.example/", ORIGIN)).toBe("/");
  expect(nextPath("//evil.example/", ORIGIN)).toBe("/");
  expect(nextPath("/\\evil.example/", ORIGIN)).toBe("/");
});

test("dot segments cannot turn the path into another host", () => {
  for (const next of ["/.//evil.example/x", "/..//evil.example", "/%2e//evil.example", "/a/..//evil.example"]) {
    expect(nextPath(next, ORIGIN)).toBe("/");
  }
  expect(nextPath("/projects/../projects/shop", ORIGIN)).toBe("/projects/shop");
});
