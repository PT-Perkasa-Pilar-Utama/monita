import { expect, test } from "bun:test";
import { statusTemplate } from "@/lib/statusTemplate";

const script = (html: string): string => html.slice(html.indexOf("<script>") + 8, html.lastIndexOf("</script>"));

test("the template calls this Monita's public status JSON for this project", () => {
  const html = statusTemplate("https://status.example.com", "payments-ab2cd3");
  expect(html).toContain('const MONITA = "https://status.example.com";');
  expect(html).toContain('const SLUG = "payments-ab2cd3";');
  expect(html).toContain('"/api/v1/public/status/"');
});

test("its script parses, and it never writes API text as markup", () => {
  const html = statusTemplate("https://status.example.com", "payments");
  expect(() => new Function(script(html))).not.toThrow();
  expect(html).not.toContain("innerHTML");
  expect(html).not.toContain("insertAdjacentHTML");
});

test("an origin or slug cannot close the script tag", () => {
  const html = statusTemplate("https://x.example.com", '</script><script>alert(1)</script>');
  expect(html.match(/<\/script>/g)?.length).toBe(1);
  expect(() => new Function(script(html))).not.toThrow();
});
