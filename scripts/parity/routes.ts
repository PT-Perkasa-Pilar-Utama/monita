// Reads every route the server registers from src/, so the parity kit covers a new route
// without anyone adding it by hand. Run `bun scripts/parity/routes.ts` to print the list.

import { Glob } from "bun";

export type Route = { path: string; methods: string[]; file: string };

const ROOTS = ["/api/", "/ping/", "/static/", "/assets/", "/healthz", "/admin/", "/users/", "/projects/"];
// Routers built from these functions are nested under a project prefix (src/app.rs and each
// module's api.rs): `project_routes` under both the team UI and the key API, `key_routes` in
// projects only under the key API.
const NESTS: Record<string, string[]> = {
  project_routes: ["/api/ui/projects/{slug}", "/api/v1/projects/{slug}"],
  key_routes: ["/api/v1/projects/{slug}"],
};

/** The text of each `.route(...)` call, with the name of the function it sits in. */
function calls(src: string): { fn: string; text: string }[] {
  const out: { fn: string; text: string }[] = [];
  const fnAt: { at: number; name: string }[] = [...src.matchAll(/fn ([a-z_]+)\s*\(/g)].map((m) => ({ at: m.index!, name: m[1]! }));
  for (const m of src.matchAll(/\.route\(/g)) {
    let depth = 0;
    let end = m.index! + ".route".length;
    for (; end < src.length; end++) {
      if (src[end] === "(") depth++;
      else if (src[end] === ")" && --depth === 0) break;
    }
    const fn = fnAt.filter((f) => f.at < m.index!).at(-1)?.name ?? "";
    out.push({ fn, text: src.slice(m.index!, end + 1) });
  }
  return out;
}

export async function routes(root = new URL("../../", import.meta.url).pathname): Promise<Route[]> {
  const out: Route[] = [];
  for await (const file of new Glob("src/**/*.rs").scan(root)) {
    const src = await Bun.file(root + file).text();
    for (const { fn, text } of calls(src)) {
      const path = text.match(/"([^"]+)"/)?.[1];
      if (!path) throw new Error(`route without a path in ${file}: ${text.slice(0, 80)}`);
      const methods = [...new Set([...text.matchAll(/\b(get|post|put|delete|head|patch)\(/g)].map((x) => x[1]!.toUpperCase()))];
      const absolute = ROOTS.some((r) => path.startsWith(r));
      const prefixes = absolute ? [""] : NESTS[fn];
      if (!prefixes) throw new Error(`relative route ${path} in ${file} fn ${fn}: unknown nest`);
      for (const p of prefixes) out.push({ path: p + path, methods, file });
    }
  }
  return out.sort((a, b) => a.path.localeCompare(b.path));
}

if (import.meta.main) {
  const all = await routes();
  for (const r of all) console.log(`${r.methods.join(",").padEnd(16)} ${r.path}`);
  console.log(`${all.length} routes`);
}
