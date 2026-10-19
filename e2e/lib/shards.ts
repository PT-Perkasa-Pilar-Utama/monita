/**
 * The suite's projects and the CI shards that split them. Plain data with no runtime import, so
 * `shard-key.ts` can read it with bare Bun before a package is installed.
 * `playwright.config.ts` takes its projects from `projectsFor(process.env.E2E_SHARD)`.
 */
export type ProjectSpec = { name: string; testMatch: RegExp; dependencies?: string[] };

export const PROJECTS: ProjectSpec[] = [
  // Reads only: health, static pages, the design system.
  { name: "smoke", testMatch: /\/smoke\.spec\.ts$/ },
  // Story flows. Each test creates its own project with a unique slug, so they run in parallel.
  { name: "flows", testMatch: /\/(projects|checks|heartbeats|incidents|status|tickets|home|changelog|footers)\.spec\.ts$/ },
  // Accounts, roles, admin pages and project settings: who may do what, and the forms that set it.
  { name: "people", testMatch: /\/(accounts|admin|supervisor|settings)\.spec\.ts$/ },
];

/** One CI job per shard, on its own runner with a fresh Monita and database. */
export const SHARDS = {
  smoke: ["smoke"],
  flows: ["flows"],
  people: ["people"],
} as const satisfies Record<string, readonly string[]>;

export type Shard = keyof typeof SHARDS;

export function isShard(name: string): name is Shard {
  return Object.hasOwn(SHARDS, name);
}

function nearestInShard(name: string, inShard: ReadonlySet<string>): string[] {
  const project = PROJECTS.find((one) => one.name === name);
  return (project?.dependencies ?? []).flatMap((dependency) =>
    inShard.has(dependency) ? [dependency] : nearestInShard(dependency, inShard)
  );
}

/** Every project for a full run, or one shard's projects with the chain order kept inside it. */
export function projectsFor(shard: string | undefined): ProjectSpec[] {
  if (shard === undefined || shard === "") return PROJECTS;
  if (!isShard(shard)) throw new Error(`E2E_SHARD=${shard} is not one of ${Object.keys(SHARDS).join(", ")}`);
  const inShard = new Set<string>(SHARDS[shard]);
  return PROJECTS.filter((project) => inShard.has(project.name)).map((project) => ({
    ...project,
    dependencies: [...new Set(nearestInShard(project.name, inShard))],
  }));
}

const SPEC = /^e2e\/specs\/[^/]+\.spec\.ts$/;

/** The shards whose projects run a spec file, by its path from the repository root. */
export function shardsOfSpec(path: string): Shard[] {
  const projects = new Set(PROJECTS.filter((p) => p.testMatch.test(`/${path}`)).map((p) => p.name));
  return Object.keys(SHARDS)
    .filter(isShard)
    .filter((shard) => SHARDS[shard].some((name) => projects.has(name)));
}

/** The tracked files a shard's result depends on: all but other shards' specs, docs and Markdown. */
export function inputsOf(shard: Shard, tracked: readonly string[]): string[] {
  return tracked.filter((path) => {
    if (path.startsWith("docs/") || path.endsWith(".md")) return false;
    if (!SPEC.test(path)) return true;
    return shardsOfSpec(path).includes(shard);
  });
}
