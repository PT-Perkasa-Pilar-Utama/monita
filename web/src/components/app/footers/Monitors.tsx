import type { ReactElement } from "react";
import { ARM, Mascot } from "@/components/brand/Mascot";
import { Frame, TextLine, Window } from "./Frame";

const GLYPHS = (
  <>
    <circle cx="30" cy="28" r="9" />
    <path d="M37 35l7 7" />
    <circle cx="120" cy="78" r="9" />
    <path d="M115.5 78l3 3 6-6" />
  </>
);

/** Monitors: Moni checks a website through a magnifying glass, on a timer, and it answers 200 OK. */
export function MonitorsFooter(): ReactElement {
  return (
    <Frame id="monitors" glyphs={GLYPHS}>
      <Window>
        <TextLine x={80} y={72} w={120} />
        <rect x="40" y="102" width="60" height="22" rx="6" className="fill-up/15" />
        <text x="70" y="117" textAnchor="middle" className="fill-up font-mono text-[11px] font-semibold">
          200 OK
        </text>
        <TextLine x={110} y={109} w={84} />
        {Array.from({ length: 24 }, (_, n) => (
          <rect
            key={n}
            x={40 + n * 8}
            y="138"
            width="6"
            height="26"
            rx="1.5"
            className={n === 23 ? "animate-[monita-beat_2s_ease-in-out_infinite] fill-up" : "fill-up"}
            opacity={n === 23 ? undefined : 0.55 + (n % 5) * 0.09}
          />
        ))}
      </Window>
      <path d="M333 108 C 310 92, 285 92, 260 112" fill="none" strokeWidth="2.5" strokeLinecap="round" strokeDasharray="4 7" className="animate-[monita-ping_1.1s_linear_infinite] stroke-primary/60" />
      <g className="stroke-muted-foreground" strokeWidth="2.5" strokeLinecap="round">
        <circle cx="296" cy="60" r="13" className="fill-card" />
        <line x1="296" y1="47" x2="296" y2="42" />
        <line x1="291" y1="41" x2="301" y2="41" />
        <line x1="296" y1="60" x2="296" y2="52" />
        <line x1="296" y1="60" x2="302" y2="64" />
      </g>
      <line x1="362" y1="117" x2="375" y2="130" strokeWidth="6" strokeLinecap="round" className="stroke-muted-foreground" />
      <circle cx="349" cy="104" r="17" strokeWidth="4" className="fill-card stroke-muted-foreground" />
      <path d="M341 104l5 5 10-11" fill="none" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" className="stroke-up" />
      <Mascot x={440} y={124} />
      <path d="M396 136 Q 384 142 375 130" strokeWidth="4" strokeLinecap="round" className={ARM} />
      <path d="M484 132 Q 496 140 494 152" strokeWidth="4" strokeLinecap="round" className={ARM} />
    </Frame>
  );
}
