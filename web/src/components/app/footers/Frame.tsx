import type { ReactElement, ReactNode } from "react";

/** One edge of the footer: a faint dot grid and the primitive's glyphs, fading toward the scene. */
function Edge({ id, side }: { id: string; side: "left" | "right" }): ReactElement {
  return (
    <svg className={`h-full min-w-0 flex-1 ${side === "left" ? "footer-edge-left" : "footer-edge-right"}`}>
      <rect width="100%" height="100%" fill="url(#footer-dots)" />
      <rect width="100%" height="100%" fill={`url(#${id}-glyphs)`} />
    </svg>
  );
}

/**
 * The full-width illustration under a primitive's pages: a 560 by 200 scene with Moni in the
 * middle, standing on the ground line at y 180, and a quiet texture on both edges. `glyphs` are
 * the contents of a 180 by 120 tile, two or three outline shapes in `currentColor`.
 * Decoration only.
 */
export function Frame({ id, glyphs, children }: { id: string; glyphs: ReactNode; children: ReactNode }): ReactElement {
  return (
    <footer aria-hidden="true" className="relative mt-auto h-[200px] shrink-0 overflow-hidden border-t bg-paper max-sm:h-[150px]">
      {/* The tiles' currentColor comes from here, where the patterns are defined. */}
      <svg className="absolute size-0 text-input">
        <defs>
          <pattern id="footer-dots" width="16" height="16" patternUnits="userSpaceOnUse">
            <circle cx="8" cy="8" r="1" fill="currentColor" />
          </pattern>
          <pattern id={`${id}-glyphs`} width="180" height="120" patternUnits="userSpaceOnUse" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
            {glyphs}
          </pattern>
        </defs>
      </svg>
      <div className="absolute inset-x-0 top-0 bottom-5 flex max-sm:bottom-[15px]">
        <Edge id={id} side="left" />
        <div className="w-[560px] shrink-0 max-sm:w-[420px]" />
        <Edge id={id} side="right" />
      </div>
      <div className="absolute inset-x-0 bottom-5 h-px bg-input max-sm:bottom-[15px]" />
      <svg viewBox="0 0 560 200" preserveAspectRatio="xMidYMax meet" className="relative mx-auto block h-full w-[560px] max-sm:w-[420px]">
        {children}
      </svg>
    </footer>
  );
}

/** A window standing on the ground line at the left of a scene: a card with a title bar. */
export function Window({ children }: { children?: ReactNode }): ReactElement {
  return (
    <g>
      <rect x="24" y="64" width="232" height="116" rx="12" strokeWidth="1.5" className="fill-card stroke-input" />
      <line x1="24" y1="88" x2="256" y2="88" strokeWidth="1.5" className="stroke-input" />
      {[40, 52, 64].map((cx) => (
        <circle key={cx} cx={cx} cy="76" r="3.5" className="fill-input" />
      ))}
      {children}
    </g>
  );
}

/** A grey bar that stands for a line of text. */
export function TextLine({ x, y, w }: { x: number; y: number; w: number }): ReactElement {
  return <rect x={x} y={y} width={w} height="8" rx="4" className="fill-input" />;
}
