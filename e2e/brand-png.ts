// Rasterises docs/assets/brand/*.svg into PNGs with transparent backgrounds, beside the SVGs.
// The page loads Inter from static/fonts, so the wordmark in the logo PNGs is the real face.
// Run: make brand (after web/scripts/brand.tsx has written the SVGs).
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { chromium } from "playwright";

const ROOT = join(import.meta.dir, "..");
const DIR = join(ROOT, "docs/assets/brand");
const font = readFileSync(join(ROOT, "static/fonts/inter-latin-wght.woff2")).toString("base64");

/** The widths to export each file at, in pixels. */
function widths(name: string): number[] {
  if (name.startsWith("moni-friends")) return [1200, 2400];
  if (name.startsWith("logo")) return [608, 1216];
  return [512, 1024];
}

const browser = await chromium.launch({ channel: "chrome", headless: true });
try {
  const page = await browser.newPage();
  for (const name of readdirSync(DIR).filter((f) => f.endsWith(".svg")).sort()) {
    // Inline, so the SVG's text uses the @font-face of the page around it.
    const svg = readFileSync(join(DIR, name), "utf8").replace(/ width="\d+" height="\d+"/, ' width="100%"');
    for (const w of widths(name)) {
      await page.setContent(`<style>@font-face{font-family:Inter;src:url(data:font/woff2;base64,${font}) format("woff2");font-weight:100 900}html,body{margin:0;background:transparent}#art{width:${w}px}#art svg{display:block}</style><div id="art">${svg}</div>`);
      await page.evaluate(() => document.fonts.ready);
      const out = join(DIR, name.replace(/\.svg$/, `-${w}.png`));
      await page.locator("#art").screenshot({ path: out, omitBackground: true });
      console.log(`brand: ${name} -> ${out.slice(ROOT.length + 1)}`);
    }
  }
} finally {
  await browser.close();
}
