// Writes the brand files in docs/assets/brand from the app's own components, so the logo and Moni
// in the docs, the README and anyone's slides are the ones the app draws. Run: make brand.
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import type { ReactElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { Mark } from "@/components/brand/Mark";
import { Mascot } from "@/components/brand/Mascot";
import { PlaygroundScene } from "@/components/brand/Playground";
import { ChangelogFooter } from "@/components/app/footers/Changelog";
import { HeartbeatsFooter } from "@/components/app/footers/Heartbeats";
import { IncidentsFooter } from "@/components/app/footers/Incidents";
import { MonitorsFooter } from "@/components/app/footers/Monitors";
import { TicketsFooter } from "@/components/app/footers/Tickets";
import { BRAND_COLORS, inlineClasses, KEYFRAMES, THEMED_COLORS } from "./brandSvg";

const OUT = join(import.meta.dir, "../../docs/assets/brand");
const BRAND = BRAND_COLORS["primary"]!;
const BRAND_ON_DARK = "#f25cbf";
const INK = "#16171a";
const PAPER = "#f7f8f8";

/** An arm in Moni's own units, where Moni faces left. */
function Arm({ d }: { d: string }): ReactElement {
  return <path d={d} fill="none" strokeWidth="5" strokeLinecap="round" className="stroke-mascot-line" />;
}
const DOWN_LEFT = "M-40 12Q-54 22 -52 36";
const DOWN_RIGHT = "M40 12Q54 22 52 36";
const UP_LEFT = "M-38 -18Q-58 -36 -52 -58";
const UP_RIGHT = "M38 -18Q58 -36 52 -58";

/** The inner markup of a rendered `<svg>` or `<g>`. */
function inner(el: ReactElement): string {
  return renderToStaticMarkup(el).replace(/^<svg[^>]*>|<\/svg>$/g, "");
}

function file(name: string, viewBox: string, width: number, height: number, body: string, label: string, animate = false): void {
  const style = animate ? `<style>${KEYFRAMES}</style>` : "";
  let svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${viewBox}" width="${width}" height="${height}" role="img" aria-label="${label}">${style}${body}</svg>\n`;
  // A single pose is a still image: no blinking or beating antenna.
  if (!animate) svg = svg.replace(/animation:[^;"]*;?/g, "").replace(/ style=""/g, "");
  writeFileSync(join(OUT, name), svg);
  console.log(`brand: ${name}`);
}

/** The mark in one colour: its strokes take the colour of `currentColor`. */
function mark(color: string): string {
  // The mark's own <svg> sets fill="none"; its paths need it once the <svg> is gone.
  return `<g fill="none" style="color:${color}">${inner(<Mark />)}</g>`;
}

/** The mark and the name, as the app's header sets them. */
function lockup(markColor: string, wordColor: string): string {
  return `<g transform="translate(0 4) scale(2)">${mark(markColor)}</g><text x="50" y="35" fill="${wordColor}" font-family="Inter, -apple-system, 'Segoe UI', system-ui, sans-serif" font-size="30" font-weight="600" letter-spacing="-0.75">Monita</text>`;
}

function moni(pose: ReactElement): string {
  return inlineClasses(renderToStaticMarkup(pose));
}

mkdirSync(OUT, { recursive: true });
file("mark.svg", "0 0 20 20", 256, 256, mark(BRAND), "Monita");
file("mark-on-dark.svg", "0 0 20 20", 256, 256, mark(BRAND_ON_DARK), "Monita");
file("mark-white.svg", "0 0 20 20", 256, 256, mark("#ffffff"), "Monita");
file("logo.svg", "0 0 152 48", 304, 96, lockup(BRAND, INK), "Monita");
file("logo-on-dark.svg", "0 0 152 48", 304, 96, lockup(BRAND_ON_DARK, PAPER), "Monita");

const MONI_BOX = "-74 -80 148 142";
file("moni.svg", MONI_BOX, 296, 284, moni(<Mascot x={0} y={0}><Arm d={DOWN_LEFT} /><Arm d={DOWN_RIGHT} /></Mascot>), "Moni, the Monita mascot");
file("moni-happy.svg", MONI_BOX, 296, 284, moni(<Mascot x={0} y={0} happy><Arm d={UP_LEFT} /><Arm d={UP_RIGHT} /></Mascot>), "Moni, happy, arms up");
file("moni-wave.svg", MONI_BOX, 296, 284, moni(<Mascot x={0} y={0} flip><Arm d={UP_LEFT} /><Arm d={DOWN_RIGHT} /></Mascot>), "Moni, waving");
file("moni-peek.svg", "-74 -80 148 78", 296, 156, moni(<Mascot x={0} y={0} />) + `<ellipse cx="-40" cy="-4" rx="12" ry="8" style="fill:${BRAND}"/><ellipse cx="40" cy="-4" rx="12" ry="8" style="fill:${BRAND}"/>`, "Moni, peeking over an edge");
file("moni-friends.svg", "0 0 1200 230", 1200, 230, inlineClasses(inner(<svg><PlaygroundScene /></svg>)) + `<line x1="0" y1="219.5" x2="1200" y2="219.5" style="stroke:${BRAND_COLORS["input"]};stroke-width:1.5"/>`, "Moni and friends at play: a kite, a game of catch, a jump, uptime blocks and bubbles", true);

/**
 * A primitive's footer scene as a 560 by 200 file, with its ground line and arrowheads. The
 * landing page inlines these, so the colours are themed (THEMED_COLORS) and the ids carry the
 * primitive's name.
 */
function scene(kind: string, el: ReactElement, label: string): void {
  const html = renderToStaticMarkup(el);
  const markers = html.match(/<marker[\s\S]*?<\/marker>/g) ?? [];
  const body = /<svg viewBox="0 0 560 200"[^>]*>([\s\S]*)<\/svg><\/footer>$/.exec(html)?.[1];
  if (!body || markers.length === 0) throw new Error(`brand: the ${kind} scene changed shape`);
  const ground = '<line x1="0" y1="180.75" x2="560" y2="180.75" stroke-width="1.5" class="stroke-input"/>';
  const art = `<defs>${markers.join("")}</defs>${ground}${body}`.replace(/(id="|url\(#)arrow-/g, `$1${kind}-arrow-`);
  file(`scene-${kind}.svg`, "0 0 560 200", 560, 200, inlineClasses(art, THEMED_COLORS), label, true);
}

scene("monitors", <MonitorsFooter />, "Moni checks a website on a timer, and it answers 200 OK");
scene("heartbeats", <HeartbeatsFooter />, "Moni hears a scheduled job ping on time");
scene("incidents", <IncidentsFooter />, "Moni follows an incident from open to resolved and sends the alert");
scene("tickets", <TicketsFooter />, "Moni sorts a bug report on the ticket board");
scene("changelog", <ChangelogFooter />, "Moni publishes a release on the changelog");

