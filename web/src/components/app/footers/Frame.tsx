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
      <svg className="absolute size-0 text-muted-foreground">
        <defs>
          <pattern id="footer-dots" width="16" height="16" patternUnits="userSpaceOnUse">
            <circle cx="8" cy="8" r="1.2" fill="currentColor" opacity="0.3" />
          </pattern>
          <pattern id={`${id}-glyphs`} width="180" height="120" patternUnits="userSpaceOnUse" fill="none" stroke="currentColor" strokeOpacity="0.45" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round">
            {glyphs}
          </pattern>
          {/* Arrowheads for a scene's flow lines: brand for what Monita sends, green for an answer. */}
          <marker id="arrow-brand" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="12" markerHeight="12" markerUnits="userSpaceOnUse" orient="auto-start-reverse">
            <path d="M1 1L9 5L1 9" fill="none" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="stroke-primary" />
          </marker>
          <marker id="arrow-up" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="12" markerHeight="12" markerUnits="userSpaceOnUse" orient="auto-start-reverse">
            <path d="M1 1L9 5L1 9" fill="none" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="stroke-up" />
          </marker>
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

/** A dashed flow line, drawn from where the flow starts to where it lands, with an arrowhead at
 * the end. The dashes march the same way. `answer` makes it green, for a reply. */
export function Flow({ d, answer = false }: { d: string; answer?: boolean }): ReactElement {
  return (
    <path
      d={d}
      fill="none"
      strokeWidth="2.5"
      strokeLinecap="round"
      strokeDasharray="4 7"
      markerEnd={answer ? "url(#arrow-up)" : "url(#arrow-brand)"}
      className={`animate-[monita-ping_1.1s_linear_infinite] ${answer ? "stroke-up/70" : "stroke-primary/70"}`}
    />
  );
}

/** A grey bar that stands for a line of text. */
export function TextLine({ x, y, w }: { x: number; y: number; w: number }): ReactElement {
  return <rect x={x} y={y} width={w} height="8" rx="4" className="fill-input" />;
}
