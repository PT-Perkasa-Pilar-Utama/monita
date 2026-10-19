// Starting points a tech lead copies into a job, built from its heartbeat's ping URL.

import type { Lang } from "@/lib/highlight";

export type Snippet = { title: string; code: string };

export const LANGS: { lang: Lang; label: string }[] = [
  { lang: "sh", label: "Shell" },
  { lang: "js", label: "Node.js" },
  { lang: "py", label: "Python" },
];

/** Commands for success and failure, and code that runs a job and reports either, in `lang`. */
export function pingSnippets(url: string, lang: Lang = "sh"): Snippet[] {
  const fail = `${url}/fail`;
  const titles = ["When the job succeeds", "When it fails, with the reason", "Run the job and report either"] as const;
  const code: Record<Lang, [string, string, string]> = {
    sh: [
      `curl -fsS -m 10 --retry 3 ${url} > /dev/null`,
      `curl -fsS -m 10 --data-raw "disk full on /backup" ${fail} > /dev/null`,
      [
        "#!/bin/sh",
        "# Runs the job, then tells Monita how it went.",
        "# On failure it sends the job's last output as the reason.",
        "if output=$(/path/to/your-job.sh 2>&1); then",
        `  curl -fsS -m 10 --retry 3 ${url} > /dev/null`,
        "else",
        `  printf '%s' "$output" | tail -c 1000 | curl -fsS -m 10 --data-binary @- ${fail} > /dev/null`,
        "fi",
      ].join("\n"),
    ],
    js: [
      `await fetch("${url}", { signal: AbortSignal.timeout(10_000) });`,
      `await fetch("${fail}", { method: "POST", body: "disk full on /backup", signal: AbortSignal.timeout(10_000) });`,
      [
        "// Node 18 or later. Runs the job, then tells Monita how it went.",
        "// On failure it sends the error as the reason, then fails as before.",
        `const PING = "${url}";`,
        "try {",
        "  await runJob();",
        "  await fetch(PING, { signal: AbortSignal.timeout(10_000) });",
        "} catch (err) {",
        "  const reason = String(err?.stack ?? err).slice(-1000);",
        "  await fetch(`${PING}/fail`, { method: \"POST\", body: reason, signal: AbortSignal.timeout(10_000) });",
        "  throw err;",
        "}",
      ].join("\n"),
    ],
    py: [
      `import urllib.request\n\nurllib.request.urlopen("${url}", timeout=10)`,
      `import urllib.request\n\nurllib.request.urlopen("${fail}", data=b"disk full on /backup", timeout=10)`,
      [
        "# Runs the job, then tells Monita how it went.",
        "# On failure it sends the traceback as the reason, then fails as before.",
        "import traceback",
        "import urllib.request",
        "",
        `PING = "${url}"`,
        "",
        "try:",
        "    run_job()",
        "    urllib.request.urlopen(PING, timeout=10)",
        "except Exception:",
        "    reason = traceback.format_exc()[-1000:].encode()",
        '    urllib.request.urlopen(PING + "/fail", data=reason, timeout=10)',
        "    raise",
      ].join("\n"),
    ],
  };
  return code[lang].map((c, n) => ({ title: titles[n] ?? "", code: c }));
}
