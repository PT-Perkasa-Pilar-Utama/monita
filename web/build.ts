export {};

// Builds the frontend into web/dist: hashed files under assets/, and index.html that names them.
// Monita embeds the folder (src/platform/spa.rs) and serves index.html for every page path.

const OUT = "./dist";
await Bun.$`rm -rf ${OUT}`;

const result = await Bun.build({
  entrypoints: ["./src/main.tsx"],
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
const entry = result.outputs.find((o) => o.kind === "entry-point" && o.path.endsWith(".js"));
// Tailwind's output is linked as its own file: its font URLs point at /static, which the
// bundler would try to resolve.
const cssText = await Bun.file("./src/globals.css").text();
const cssHash = new Bun.CryptoHasher("sha256").update(cssText).digest("hex").slice(0, 10);
const cssPath = `${OUT}/assets/app-${cssHash}.css`;
await Bun.write(cssPath, cssText);
const css = [{ path: cssPath }];
if (!entry) {
  console.error("build: no JS entry point in the output");
  process.exit(1);
}

// theme.js runs before the first paint, so the page never flashes the wrong theme. Its URL
// carries a content hash: /static files are cached for a year in a release build.
const themeText = await Bun.file("../static/theme.js").text();
const themeHash = new Bun.CryptoHasher("sha256").update(themeText).digest("hex").slice(0, 8);
const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<title>Monita</title>
<link rel="icon" href="/static/favicon.svg" type="image/svg+xml">
<link rel="preload" href="/static/fonts/inter-latin-wght.woff2" as="font" type="font/woff2" crossorigin>
<script src="/static/theme.js?v=${themeHash}"></script>
${css.map((c) => `<link rel="stylesheet" href="${rel(c.path)}">`).join("\n")}
<script type="module" src="${rel(entry.path)}"></script>
</head>
<body>
<div id="root"></div>
</body>
</html>
`;
await Bun.write(`${OUT}/index.html`, html);
console.log(`web: ${result.outputs.length} files, entry ${rel(entry.path)}`);
