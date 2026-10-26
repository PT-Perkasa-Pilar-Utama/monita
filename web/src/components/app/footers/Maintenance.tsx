import type { ReactElement } from "react";
import { ARM, Mascot } from "@/components/brand/Mascot";
import { tape } from "@/components/app/StateStamp";
import { Frame, TextLine, Window } from "./Frame";

const GLYPHS = (
  <>
    <path d="M22 40l12-12a6 6 0 0 1 8 8L30 48" />
    <circle cx="24" cy="46" r="3" />
    <path d="M118 66h28v20h-28ZM118 72h28" />
  </>
);

/** Maintenance: a planned window sits on the day's timeline as barrier tape, and Moni, wrench in
 * hand, works inside it while the checks wait. */
export function MaintenanceFooter(): ReactElement {
  return (
    <Frame id="maintenance" glyphs={GLYPHS}>
      <Window>
        <TextLine x={80} y={72} w={110} />
        {/* The day's timeline: hours pass, then the planned window. */}
        {Array.from({ length: 24 }, (_, n) => (
          <rect key={n} x={40 + n * 8} y="104" width="6" height="22" rx="1.5" className={n >= 13 && n <= 18 ? "fill-transparent" : "fill-up"} opacity={n >= 13 && n <= 18 ? 1 : 0.55 + (n % 5) * 0.09} />
        ))}
        <foreignObject x="144" y="100" width="48" height="30">
          <div className="h-full w-full rounded-md border-2 border-issue" style={tape("--issue", 40)} />
        </foreignObject>
        <TextLine x={40} y={142} w={96} />
        <TextLine x={40} y={158} w={140} />
      </Window>
      {/* A barrier on two legs, in front of the work. */}
      <g>
        <line x1="296" y1="150" x2="290" y2="180" strokeWidth="3" strokeLinecap="round" className="stroke-muted-foreground" />
        <line x1="356" y1="150" x2="362" y2="180" strokeWidth="3" strokeLinecap="round" className="stroke-muted-foreground" />
        <foreignObject x="282" y="136" width="88" height="18">
          <div className="h-full w-full rounded-sm border border-issue bg-card" style={tape("--issue", 70)} />
        </foreignObject>
      </g>
      {/* The wrench, raised in Moni's hand. */}
      <g className="stroke-muted-foreground" strokeWidth="5" strokeLinecap="round" fill="none">
        <path d="M374 112 L 356 92" />
        {/* An open jaw, its gap facing up and away from the handle. */}
        <path d="M343 77 A 9 9 0 1 0 353 77" strokeWidth="5" />
      </g>
      <Mascot x={440} y={124} />
      <path d="M396 136 Q 380 130 374 112" strokeWidth="4" strokeLinecap="round" className={ARM} />
      <path d="M484 132 Q 496 140 494 152" strokeWidth="4" strokeLinecap="round" className={ARM} />
    </Frame>
  );
}
