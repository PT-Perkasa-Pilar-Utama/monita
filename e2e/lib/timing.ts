/**
 * A Playwright reporter: wall time per project (first start to last end) and the summed test time
 * inside it. Prints a table, and appends it to the GitHub Actions job summary.
 *
 *   reporter: [["list"], ["html", { open: "never" }], ["./e2e/lib/timing.ts"]],
 */
import { appendFileSync } from "node:fs";
import type { Reporter, TestCase, TestResult } from "@playwright/test/reporter";

export type Run = { project: string; start: number; duration: number };
export type ProjectTime = { project: string; tests: number; wallMs: number; busyMs: number };

export function projectTimes(runs: readonly Run[]): ProjectTime[] {
  const byProject = new Map<string, Run[]>();
  for (const run of runs) byProject.set(run.project, [...(byProject.get(run.project) ?? []), run]);
  return [...byProject].map(([project, list]) => {
    const start = Math.min(...list.map((run) => run.start));
    const end = Math.max(...list.map((run) => run.start + run.duration));
    return { project, tests: list.length, wallMs: end - start, busyMs: list.reduce((sum, run) => sum + run.duration, 0) };
  });
}

const seconds = (ms: number): string => (ms / 1000).toFixed(1);

export function timingTable(times: readonly ProjectTime[], totalMs: number): string {
  const rows = [...times]
    .sort((a, b) => b.wallMs - a.wallMs)
    .map((t) => `| ${t.project} | ${t.tests} | ${seconds(t.wallMs)} | ${seconds(t.busyMs)} |`);
  return ["### Browser end-to-end time per project", "", "| Project | Tests | Wall s | Busy s |", "| --- | ---: | ---: | ---: |", ...rows, "", `Whole run: ${seconds(totalMs)} s.`, ""].join("\n");
}

export default class TimingReporter implements Reporter {
  private readonly runs: Run[] = [];
  private begun = Date.now();

  onBegin(): void {
    this.begun = Date.now();
  }

  onTestEnd(test: TestCase, result: TestResult): void {
    if (result.status === "skipped") return;
    this.runs.push({ project: test.parent.project()?.name ?? "default", start: result.startTime.getTime(), duration: result.duration });
  }

  onEnd(): void {
    const table = timingTable(projectTimes(this.runs), Date.now() - this.begun);
    process.stdout.write(`\n${table}`);
    const summary = process.env["GITHUB_STEP_SUMMARY"];
    if (summary !== undefined && summary !== "") appendFileSync(summary, table);
  }
}
