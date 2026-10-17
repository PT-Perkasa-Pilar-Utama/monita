import { expect, test } from "bun:test";
import { dataUrlBytes } from "@/lib/icon";

test("a data URL's size is the decoded bytes, not the text length", () => {
  expect(dataUrlBytes("data:image/png;base64,AAAA")).toBe(3);
  expect(dataUrlBytes(`data:image/webp;base64,${"A".repeat(4 * 1000)}`)).toBe(3000);
});
