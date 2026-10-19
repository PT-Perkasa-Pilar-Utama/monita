/**
 * Turns an SVG rendered from the app's React components into a standalone file: every Tailwind
 * class becomes an inline style with a fixed colour, so the file looks the same anywhere.
 */

/** The light-theme values of the tokens the brand art uses (web/src/index.css `:root`). */
export const BRAND_COLORS: Record<string, string> = {
  primary: "#c4258f",
  "mascot-body": "#fbe4f2",
  "mascot-line": "#c4258f",
  "mascot-ink": "#1e1a1d",
  "mascot-shine": "#ffffff",
  "mascot-cheek": "#f2a7d4",
  up: "#23784e",
  input: "#c9cbce",
  "muted-foreground": "#7a7f87",
  muted: "#8a8f9840",
  foreground: "#16171a",
};

/** One class as CSS declarations, or null when the class means nothing in a standalone file. */
function declarations(c: string, colors: Record<string, string>): string[] | null {
  let m = c.match(/^(fill|stroke)-([a-z-]+?)(?:\/(\d+))?$/);
  if (m && colors[m[2]!]) {
    const out = [`${m[1]}:${colors[m[2]!]}`];
    if (m[3]) out.push(`${m[1]}-opacity:${Number(m[3]) / 100}`);
    return out;
  }
  if (c === "fill-none") return ["fill:none"];
  if ((m = c.match(/^animate-\[(.+)\]$/))) return [`animation:${m[1]!.replace(/_/g, " ")}`];
  if ((m = c.match(/^origin-\[(.+)\]$/))) return [`transform-origin:${m[1]!.replace(/_/g, " ")}`];
  if (c === "origin-center") return ["transform-origin:center"];
  if (c === "[transform-box:fill-box]") return ["transform-box:fill-box"];
  return null;
}

/** Inlines every class attribute. Throws on a class it does not know, so new art never ships
 * with a colour missing. */
export function inlineClasses(svg: string, colors: Record<string, string> = BRAND_COLORS): string {
  const inlined = svg.replace(/ class="([^"]*)"/g, (_m, cls: string) => {
    const style: string[] = [];
    for (const c of cls.split(/\s+/).filter(Boolean)) {
      const d = declarations(c, colors);
      if (d === null) throw new Error(`brand art: no standalone style for class "${c}"`);
      style.push(...d);
    }
    return ` style="${style.join(";")}"`;
  });
  // An element that also had a style attribute now has two; join them.
  return inlined.replace(/ style="([^"]*)" style="([^"]*)"/g, ' style="$1;$2"');
}

/** The keyframes the art animates with (web/src/index.css), stilled for reduced motion. */
export const KEYFRAMES = `@keyframes monita-blink{0%,94%,100%{transform:scaleY(1)}97%{transform:scaleY(.1)}}
@keyframes monita-beat{0%,100%{opacity:1}50%{opacity:.35}}
@keyframes monita-bounce{0%,100%{transform:translateY(0)}50%{transform:translateY(-22px)}}
@keyframes monita-toss{from{offset-distance:0%}to{offset-distance:100%}}
@keyframes monita-rise{0%{transform:translateY(0);opacity:0}15%{opacity:1}100%{transform:translateY(-70px);opacity:0}}
@keyframes monita-sway{0%,100%{transform:rotate(-6deg)}50%{transform:rotate(6deg)}}
@media (prefers-reduced-motion:reduce){*{animation:none!important}}`;
