import { expect, test } from "bun:test";
import { pingSnippets } from "@/lib/pingSnippets";

test("success calls the URL, failure calls its /fail with a body", () => {
  const [ok, fail, script] = pingSnippets("https://m.co/ping/abc");
  expect(ok?.code).toBe("curl -fsS -m 10 --retry 3 https://m.co/ping/abc > /dev/null");
  expect(fail?.code).toContain("--data-raw");
  expect(fail?.code).toContain(" https://m.co/ping/abc/fail ");
  expect(script?.code).toContain("--data-binary @- https://m.co/ping/abc/fail");
  expect(script?.code.startsWith("#!/bin/sh\n")).toBe(true);
});
