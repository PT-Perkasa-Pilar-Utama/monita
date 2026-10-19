import type { ReactElement } from "react";

/**
 * Moni, Monita's mascot: the mark's ring as a round body, its pulse as the mouth, an antenna that
 * listens. Drawn around (0, 0) with a radius of 46, for use inside a scene's SVG. Brand colour only;
 * a scene draws the arms and whatever Moni holds.
 */
export function Mascot({ x, y }: { x: number; y: number }): ReactElement {
  return (
    <g transform={`translate(${x} ${y})`}>
      <ellipse cx="-16" cy="50" rx="12" ry="6" className="fill-primary" />
      <ellipse cx="16" cy="50" rx="12" ry="6" className="fill-primary" />
      <line x1="0" y1="-46" x2="0" y2="-60" strokeWidth="3" strokeLinecap="round" className="stroke-primary" />
      <circle cx="0" cy="-64" r="5" className="animate-[monita-beat_2s_ease-in-out_infinite] fill-primary" />
      <circle r="46" className="fill-card" />
      <circle r="46" strokeWidth="4" className="fill-primary/10 stroke-primary" />
      <g className="origin-center animate-[monita-blink_5s_ease-in-out_infinite] [transform-box:fill-box]">
        <ellipse cx="-22" cy="-14" rx="5" ry="6.5" className="fill-foreground" />
        <ellipse cx="6" cy="-14" rx="5" ry="6.5" className="fill-foreground" />
      </g>
      <circle cx="-20.5" cy="-16.5" r="1.6" className="fill-card" />
      <circle cx="7.5" cy="-16.5" r="1.6" className="fill-card" />
      <ellipse cx="-32" cy="2" rx="6" ry="3.5" className="fill-primary/25" />
      <ellipse cx="18" cy="2" rx="6" ry="3.5" className="fill-primary/25" />
      <path d="M-20 10h5l3-7 6 12 3-5h5" fill="none" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" className="stroke-primary" />
    </g>
  );
}
