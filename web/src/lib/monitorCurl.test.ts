import { expect, test } from "bun:test";
import { monitorCurl, shellQuote } from "@/lib/monitorCurl";

const base = { url: "https://api.x/health", method: "GET", headers: "", body: null, timeout_s: 10, skip_tls_verify: false, expected_status: "2xx" };

test("a plain GET follows redirects and times out like the check", () => {
  expect(monitorCurl(base)).toBe(["curl -sS -o /dev/null", "-w '%{http_code} in %{time_total}s\\n'", "-m 10", "-L", "'https://api.x/health'"].join(" \\\n  "));
});

test("a POST sends its headers and body, quoted for the shell", () => {
  const c = monitorCurl({ ...base, method: "POST", headers: "Authorization: Bearer a:b\nX-Note: it's", body: '{"ping":1}', skip_tls_verify: true, expected_status: "301" });
  expect(c).toContain("-X POST");
  expect(c).toContain("-H 'Authorization: Bearer a:b'");
  expect(c).toContain("-H 'X-Note: it'\\''s'");
  expect(c).toContain(`--data-raw '{"ping":1}'`);
  expect(c).toContain("-k");
  expect(c).not.toContain("-L");
});

test("quoting survives a single quote", () => {
  expect(shellQuote("it's")).toBe("'it'\\''s'");
});
