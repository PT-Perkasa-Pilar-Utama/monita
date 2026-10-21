// A curl command that sends the same request as a monitor's check, to try it from a terminal.

import { envName, parseHeaderLines } from "@/lib/checks";

export type CurlInput = {
  url: string;
  method: string;
  headers: string;
  body: string | null;
  timeout_s: number;
  skip_tls_verify: boolean;
  expected_status: string;
  auth_kind?: string;
  auth_secret_name?: string | null;
  auth_user?: string | null;
  auth_header?: string | null;
};

/** `it's` as `'it'\''s'`: safe inside a POSIX shell. */
export function shellQuote(s: string): string {
  return `'${s.replaceAll("'", `'\\''`)}'`;
}

/** Text safe inside a double-quoted shell word, where a `$NAME` beside it still expands. */
function inDoubleQuotes(s: string): string {
  return s.replace(/["\\$`]/g, "\\$&");
}

/** The sign-in part, with the secret as a shell variable the reader sets first. */
function authPart(m: CurlInput): string | null {
  const v = `$${envName(m.auth_secret_name ?? null)}`;
  switch (m.auth_kind) {
    case "bearer":
      return `-H "Authorization: Bearer ${v}"`;
    case "basic":
      return `-u "${inDoubleQuotes(m.auth_user ?? "")}:${v}"`;
    case "header":
      return `-H "${inDoubleQuotes(m.auth_header ?? "")}: ${v}"`;
    default:
      return null;
  }
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
  const auth = authPart(m);
  if (auth) parts.push(auth);
  if (m.body && (m.method === "POST" || m.method === "PUT")) parts.push(`--data-raw ${shellQuote(m.body)}`);
  parts.push(shellQuote(m.url));
  const command = parts.join(" \\\n  ");
  return auth ? `# Set the secret first: export ${envName(m.auth_secret_name ?? null)}='…'\n${command}` : command;
}

/** The check as a terminal command: curl for HTTP, `nc` for a TCP port, `getent` for a DNS
 * name, which asks the same system resolver that Monita asks, and one `ping` for a host. */
export function monitorCommand(m: CurlInput & { kind: string }): string {
  if (m.kind === "tcp") {
    const at = m.url.lastIndexOf(":");
    const host = m.url.slice(0, at).replace(/^\[(.*)\]$/, "$1");
    return `nc -vz -w ${m.timeout_s} ${shellQuote(host)} ${m.url.slice(at + 1)}`;
  }
  if (m.kind === "dns") return `getent ahosts ${shellQuote(m.url)}`;
  if (m.kind === "ping") return `ping -c 1 ${shellQuote(m.url)}`;
  return monitorCurl(m);
}
