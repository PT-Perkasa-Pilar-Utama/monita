import { expect, test } from "bun:test";
import { highlight } from "@/lib/highlight";
import type { Kind, Lang } from "@/lib/highlight";

const kinds = (code: string, lang: Lang): [string, Kind][] => highlight(code, lang).filter((t) => t.kind !== "plain").map((t) => [t.text, t.kind]);

test("shell marks flags, strings and a trailing comment, and keeps the text", () => {
  const code = 'curl -fsS -m 10 "a # b" https://m.co/x # note';
  expect(kinds(code, "sh")).toEqual([
    ["-fsS", "flag"],
    ["-m", "flag"],
    ["10", "number"],
    ['"a # b"', "string"],
    ["# note", "comment"],
  ]);
  expect(highlight(code, "sh").map((t) => t.text).join("")).toBe(code);
});

test("JavaScript and Python keywords and comments", () => {
  expect(kinds("// hi\nconst x = await f(`a`);", "js")).toEqual([
    ["// hi", "comment"],
    ["const", "keyword"],
    ["await", "keyword"],
    ["`a`", "string"],
  ]);
  expect(kinds("try:\n    run()  # go\nexcept Exception:", "py")).toEqual([
    ["try", "keyword"],
    ["# go", "comment"],
    ["except", "keyword"],
  ]);
});

test("a word that holds a keyword is not one", () => {
  expect(kinds("iffy done_ fix", "sh")).toEqual([]);
});

test("a bare URL stays plain, port and all", () => {
  expect(kinds("curl http://localhost:7466/ping/x9 -m 10", "sh")).toEqual([
    ["-m", "flag"],
    ["10", "number"],
  ]);
});
