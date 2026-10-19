// Starting points a tech lead copies into a job, built from its heartbeat's ping URL.

export type Snippet = { title: string; code: string };

/** Commands for success and failure, and a script that runs a job and reports either. */
export function pingSnippets(url: string): Snippet[] {
  const ok = `curl -fsS -m 10 --retry 3 ${url}`;
  const fail = `${url}/fail`;
  return [
    { title: "When the job succeeds", code: `${ok} > /dev/null` },
    { title: "When it fails, with the reason", code: `curl -fsS -m 10 --data-raw "disk full on /backup" ${fail} > /dev/null` },
    {
      title: "A script that runs the job and reports either",
      code: [
        "#!/bin/sh",
        "# Runs the job, then tells Monita how it went.",
        "# On failure it sends the job's last output as the reason.",
        'if output=$(/path/to/your-job.sh 2>&1); then',
        `  ${ok} > /dev/null`,
        "else",
        `  printf '%s' "$output" | tail -c 1000 | curl -fsS -m 10 --data-binary @- ${fail} > /dev/null`,
        "fi",
      ].join("\n"),
    },
  ];
}
