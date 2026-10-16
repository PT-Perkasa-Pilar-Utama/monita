/**
 * How many browser workers to run, read off the machine once at start. Pure, so it has a unit
 * test; the config reads `os` and passes the numbers in.
 *
 *   const capacity = { cpus: availableParallelism(), load1: loadavg()[0] ?? 0,
 *     totalGiB: totalmem() / 1024 ** 3, ci: process.env.CI !== undefined,
 *     override: process.env.E2E_WORKERS };
 *   const workers = workersFor(capacity);
 *   console.log(describeCapacity(capacity, workers));
 */
export type Capacity = {
  cpus: number;
  /** One-minute load average: what is already running, services included. */
  load1: number;
  totalGiB: number;
  ci: boolean;
  /** `E2E_WORKERS`, when set: a person's own answer wins over the rule. */
  override?: string | undefined;
};

function asked(override: string | undefined): number | null {
  if (override === undefined || !/^\d+$/.test(override)) return null;
  const count = Number(override);
  return count >= 1 && count <= 8 ? count : null;
}

/**
 * On CI the runner has nothing else to do: one per core, four at most. At home the services and
 * the person's own work share the cores: half of them, one fewer when the machine is busy, two at
 * most under 12 GiB of memory, never fewer than one.
 */
export function workersFor(capacity: Capacity): number {
  const wanted = asked(capacity.override);
  if (wanted !== null) return wanted;
  if (capacity.ci) return Math.max(1, Math.min(4, capacity.cpus));
  let workers = Math.floor(capacity.cpus / 2);
  if (capacity.load1 > capacity.cpus * 0.7) workers -= 1;
  if (capacity.totalGiB < 12) workers = Math.min(workers, 2);
  return Math.max(1, Math.min(4, workers));
}

/** One line for the console, so the choice is never a mystery. */
export function describeCapacity(capacity: Capacity, workers: number): string {
  const source = asked(capacity.override) === null ? "" : " (E2E_WORKERS)";
  return `e2e: ${capacity.cpus} cores, load ${capacity.load1.toFixed(1)}, ${capacity.totalGiB.toFixed(0)} GiB, ci=${capacity.ci ? "yes" : "no"} -> ${workers} workers${source}`;
}
