// The probe a location runs, as the Locations page shows it once (technical-specs 14).

/** The shell lines that start `monita probe` for one location, the token in a variable so it stays out of `ps`. */
export function probeCommand(server: string, token: string): string {
  return [`export MONITA_PROBE_TOKEN='${token}'`, `./monita probe --server ${server}`].join("\n");
}

const IMAGE = "ghcr.io/pt-perkasa-pilar-utama/monita:1";
const RELEASES = "https://github.com/PT-Perkasa-Pilar-Utama/monita/releases/latest/download";

/** The three ways to start a location's probe on its machine, ready to paste: in Docker, as a
 * Linux service that starts with the machine, or once in a terminal. */
export function setupCommands(server: string, token: string): { docker: string; linux: string; once: string } {
  return {
    docker: [
      "docker run -d --name monita-probe --restart unless-stopped \\",
      `  -e MONITA_PROBE_SERVER=${server} \\`,
      `  -e MONITA_PROBE_TOKEN='${token}' \\`,
      `  ${IMAGE} probe`,
    ].join("\n"),
    linux: [
      `curl -fsSL ${RELEASES}/monita_linux_$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/').tar.gz | tar -xz monita`,
      "sudo install -m 0755 monita /usr/local/bin/monita",
      `sudo MONITA_PROBE_TOKEN='${token}' monita probe install --server ${server}`,
    ].join("\n"),
    once: probeCommand(server, token),
  };
}

/** Running or not, in words: a probe silent for two minutes is not running. */
export function probeState(at: number | null, now: number): { running: boolean; label: string } {
  const seen = lastSeen(at, now);
  if (!seen.stale) return { running: true, label: "Running" };
  return { running: false, label: at === null ? "Not running yet" : `Not running, last seen ${seen.text}` };
}

/** `Never`, or how long ago the probe last called, and whether that is long enough to worry. */
export function lastSeen(at: number | null, now: number): { text: string; stale: boolean } {
  if (at === null) return { text: "Never", stale: true };
  const s = Math.max(0, now - at);
  const text = s < 60 ? "Just now" : s < 3600 ? `${Math.floor(s / 60)}m ago` : s < 86_400 ? `${Math.floor(s / 3600)}h ago` : `${Math.floor(s / 86_400)}d ago`;
  return { text, stale: s >= 120 };
}

/** The quorum after the places change: never above the number of places, never below 1. Text the user is still typing stays as typed while it fits. */
export function clampQuorum(quorum: string, places: number): string {
  const n = Number.parseInt(quorum, 10);
  if (Number.isNaN(n)) return quorum;
  return String(Math.min(Math.max(n, 1), places));
}

/** Whether a place's newest result still counts toward the verdict: two intervals and 30 seconds (technical-specs 14.7). */
export function fresh(checkedAt: number, now: number, intervalS: number): boolean {
  return checkedAt >= now - (2 * intervalS + 30);
}
