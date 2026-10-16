import { expect, test } from "bun:test";
import { readdirSync } from "node:fs";
import { join } from "node:path";
import { PROJECTS, SHARDS, inputsOf, shardsOfSpec } from "./shards";

test("every project and every spec belongs to a shard", () => {
  const sharded = new Set<string>(Object.values(SHARDS).flat());
  expect(PROJECTS.map((p) => p.name).filter((n) => !sharded.has(n))).toEqual([]);
  const specs = readdirSync(join(import.meta.dir, "..", "specs"))
    .filter((f) => f.endsWith(".spec.ts"))
    .map((f) => `e2e/specs/${f}`);
  expect(specs.length).toBeGreaterThan(0);
  expect(specs.filter((s) => shardsOfSpec(s).length === 0)).toEqual([]);
});

test("a shard ignores docs and the other shard's specs", () => {
  const tracked = ["src/app.rs", "docs/a.md", "README.md", "e2e/specs/smoke.spec.ts", "e2e/specs/accounts.spec.ts"];
  expect(inputsOf("smoke", tracked)).toEqual(["src/app.rs", "e2e/specs/smoke.spec.ts"]);
  expect(inputsOf("flows", tracked)).toEqual(["src/app.rs", "e2e/specs/accounts.spec.ts"]);
});
