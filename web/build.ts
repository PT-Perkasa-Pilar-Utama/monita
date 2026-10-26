import { readdir } from "node:fs/promises";
import { brotliCompressSync, constants, gzipSync } from "node:zlib";

// Builds the frontend into web/dist: hashed files under assets/, index.html for the team app and
// status.html for the public pages, which share the core in a chunk. Monita embeds the folder
// (src/platform/spa.rs): /status/ paths get status.html, every other page path index.html.

const OUT = "./dist";
await Bun.$`rm -rf ${OUT}`;

const result = await Bun.build({
  entrypoints: ["./src/main.tsx", "./src/public.tsx"],
  outdir: OUT,
  target: "browser",
  splitting: true,
  minify: true,
  naming: { entry: "assets/[name]-[hash].[ext]", chunk: "assets/[name]-[hash].[ext]", asset: "assets/[name]-[hash].[ext]" },
  define: { "process.env.NODE_ENV": JSON.stringify("production") },
});
if (!result.success) {
  for (const log of result.logs) console.error(log);
  process.exit(1);
}

const rel = (path: string): string => `/${path.slice(path.indexOf("assets/"))}`;
const entryOf = (name: string): string => {
  const out = result.outputs.find((o) => o.kind === "entry-point" && o.path.includes(`/assets/${name}-`) && o.path.endsWith(".js"));
  if (!out) {
    console.error(`build: no ${name} entry point in the output`);
    process.exit(1);
  }
  return out.path;
};

/** The chunks a file imports, followed through, so the page can ask for them all at once. */
async function chunksOf(path: string, seen = new Set<string>()): Promise<Set<string>> {
  const dir = path.slice(0, path.lastIndexOf("/") + 1);
  const text = await Bun.file(path).text();
  for (const m of text.matchAll(/(?:from|import)\s*["']\.\/([^"']+\.js)["']/g)) {
    const chunk = dir + m[1];
    if (seen.has(chunk)) continue;
    seen.add(chunk);
    await chunksOf(chunk, seen);
  }
  return seen;
}
// Tailwind's output is linked as its own file: its font URLs point at /static, which the
// bundler would try to resolve.
const cssText = await Bun.file("./src/globals.css").text();
const cssHash = new Bun.CryptoHasher("sha256").update(cssText).digest("hex").slice(0, 10);
const cssPath = `${OUT}/assets/app-${cssHash}.css`;
await Bun.write(cssPath, cssText);
const css = [{ path: cssPath }];

// theme.js runs before the first paint, so the page never flashes the wrong theme. Its URL
// carries a content hash: /static files are cached for a year in a release build.
const themeText = await Bun.file("../static/theme.js").text();
const themeHash = new Bun.CryptoHasher("sha256").update(themeText).digest("hex").slice(0, 8);
const page = async (entry: string): Promise<string> => {
  const preload = [...(await chunksOf(entry))].map((c) => `<link rel="modulepreload" href="${rel(c)}">`);
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<title>Monita</title>
<link rel="icon" href="/static/favicon.svg" type="image/svg+xml">
<link rel="preload" href="/static/fonts/inter-latin-wght.woff2" as="font" type="font/woff2" crossorigin>
<script src="/static/theme.js?v=${themeHash}"></script>
${css.map((c) => `<link rel="stylesheet" href="${rel(c.path)}">`).join("\n")}
${preload.join("\n")}
<script type="module" src="${rel(entry)}"></script>
</head>
<body>
<div id="root"></div>
</body>
</html>
`;
};
await Bun.write(`${OUT}/index.html`, await page(entryOf("main")));
await Bun.write(`${OUT}/status.html`, await page(entryOf("public")));

// Each file also ships as .br and .gz, which Monita sends to a browser that takes them
// (src/platform/spa.rs): the JS goes from 1 MB to about 240 KB.
for (const name of await readdir(`${OUT}/assets`)) {
  if (!/\.(js|css)$/.test(name)) continue;
  const path = `${OUT}/assets/${name}`;
  const raw = new Uint8Array(await Bun.file(path).arrayBuffer());
  await Bun.write(`${path}.br`, brotliCompressSync(raw, { params: { [constants.BROTLI_PARAM_QUALITY]: 11 } }));
  await Bun.write(`${path}.gz`, gzipSync(raw, { level: 9 }));
}
console.log(`web: ${result.outputs.length} files, entries ${rel(entryOf("main"))} and ${rel(entryOf("public"))}`);
