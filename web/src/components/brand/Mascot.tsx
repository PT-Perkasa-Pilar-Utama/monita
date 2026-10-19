import type { ReactElement, ReactNode } from "react";

/**
 * Moni, Monita's mascot: the mark's ring as a round body, its pulse as the mouth, an antenna that
 * listens. Drawn around (0, 0) with a radius of 46, for use inside a scene's SVG. Its colours are
 * the `--mascot-*` tokens, the same in both themes, so Moni never changes face in dark mode.
 * A scene draws the arms (`ARM` classes) and whatever Moni holds, beside it or as `children` in
 * Moni's own units, where Moni faces left. `happy` closes the eyes into smiling arcs.
 */
export function Mascot({
  x,
  y,
  scale = 1,
  happy = false,
  flip = false,
  children,
}: {
  x: number;
  y: number;
  scale?: number;
  happy?: boolean;
  /** Faces right instead of left. */
  flip?: boolean;
  children?: ReactNode;
}): ReactElement {
  return (
    <g transform={`translate(${x} ${y}) scale(${flip ? -scale : scale} ${scale})`}>
      <ellipse cx="-16" cy="50" rx="12" ry="6" className="fill-mascot-line" />
      <ellipse cx="16" cy="50" rx="12" ry="6" className="fill-mascot-line" />
      <line x1="0" y1="-46" x2="0" y2="-60" strokeWidth="3" strokeLinecap="round" className="stroke-mascot-line" />
      <circle cx="0" cy="-64" r="5" className="animate-[monita-beat_2s_ease-in-out_infinite] fill-mascot-line" />
      <circle r="46" strokeWidth="4" className="fill-mascot-body stroke-mascot-line" />
      {happy ? (
        <g fill="none" strokeWidth="3.5" strokeLinecap="round" className="stroke-mascot-ink">
          <path d="M-28 -12q6-8 12 0" />
          <path d="M0 -12q6-8 12 0" />
        </g>
      ) : (
        <>
          <g className="origin-center animate-[monita-blink_5s_ease-in-out_infinite] [transform-box:fill-box]">
            <ellipse cx="-22" cy="-14" rx="5" ry="6.5" className="fill-mascot-ink" />
            <ellipse cx="6" cy="-14" rx="5" ry="6.5" className="fill-mascot-ink" />
          </g>
          <circle cx="-20.5" cy="-16.5" r="1.6" className="fill-mascot-shine" />
          <circle cx="7.5" cy="-16.5" r="1.6" className="fill-mascot-shine" />
        </>
      )}
      <ellipse cx="-32" cy="2" rx="6" ry="3.5" className="fill-mascot-cheek" />
      <ellipse cx="18" cy="2" rx="6" ry="3.5" className="fill-mascot-cheek" />
      <path d="M-20 10h5l3-7 6 12 3-5h5" fill="none" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" className="stroke-mascot-line" />
      {children}
    </g>
  );
}

/** Classes for an arm drawn as a path beside Moni. */
export const ARM = "fill-none stroke-mascot-line";
