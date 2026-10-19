// A curl command that sends the same request as a monitor's check, to try it from a terminal.

import { parseHeaderLines } from "@/lib/checks";

export type CurlInput = { url: string; method: string; headers: string; body: string | null; timeout_s: number; skip_tls_verify: boolean; expected_status: string };

/** `it's` as `'it'\''s'`: safe inside a POSIX shell. */
export function shellQuote(s: string): string {
  return `'${s.replaceAll("'", `'\\''`)}'`;
}

/** The check as one curl command. It prints the status code and the time, as Monita sees them.
 * Redirects are followed unless the expected status names a 3xx, as the check does. */
export function monitorCurl(m: CurlInput): string {
  const parts = ["curl -sS -o /dev/null", `-w ${shellQuote("%{http_code} in %{time_total}s\\n")}`, `-m ${m.timeout_s}`];
  if (!m.expected_status.split(",").some((p) => p.trim().startsWith("3"))) parts.push("-L");
  if (m.skip_tls_verify) parts.push("-k");
  if (m.method === "HEAD") parts.push("-I");
  else if (m.method !== "GET") parts.push(`-X ${m.method}`);
  for (const [name, value] of parseHeaderLines(m.headers)) parts.push(`-H ${shellQuote(`${name}: ${value}`)}`);
  if (m.body && (m.method === "POST" || m.method === "PUT")) parts.push(`--data-raw ${shellQuote(m.body)}`);
  parts.push(shellQuote(m.url));
  return parts.join(" \\\n  ");
}
