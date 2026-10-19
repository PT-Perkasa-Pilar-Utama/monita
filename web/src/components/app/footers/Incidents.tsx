import type { ReactElement } from "react";
import { ARM, Mascot } from "@/components/brand/Mascot";
import { Frame, TextLine, Window } from "./Frame";

const GLYPHS = (
  <>
    <path d="M20 20h24a4 4 0 0 1 4 4v12a4 4 0 0 1-4 4H32l-6 5v-5h-6a4 4 0 0 1-4-4V24a4 4 0 0 1 4-4Z" />
    <circle cx="130" cy="82" r="9" />
    <path d="M125.5 82l3 3 6-6" />
  </>
);

const STEPS: [string, boolean][] = [
  ["Investigating", false],
  ["Identified", false],
  ["Monitoring", false],
  ["Resolved", true],
];

/** Incidents: the team posts each step to the status page; Moni announces it, ending on Resolved. */
export function IncidentsFooter(): ReactElement {
  return (
    <Frame id="incidents" glyphs={GLYPHS}>
      <Window>
        <TextLine x={80} y={72} w={110} />
        <line x1="44" y1="104" x2="44" y2="164" strokeWidth="1.5" className="stroke-input" />
        {STEPS.map(([label, done], n) => {
          const y = 104 + n * 20;
          return (
            <g key={label}>
              <circle cx="44" cy={y} r="4" className={done ? "fill-up" : "fill-card stroke-muted-foreground"} strokeWidth="1.5" />
              <rect x="56" y={y - 8} width="84" height="16" rx="5" className={done ? "fill-up/15" : "fill-muted"} />
              <text x="98" y={y + 4} textAnchor="middle" className={done ? "fill-up text-[10px] font-semibold" : "fill-muted-foreground text-[10px] font-medium"}>
                {label}
              </text>
              <TextLine x={150} y={y - 4} w={done ? 70 : 90} />
            </g>
          );
        })}
      </Window>
      {/* The megaphone, held out to the left, and what it says. */}
      <g className="stroke-primary/60" strokeWidth="2.5" strokeLinecap="round" fill="none">
        <path d="M322 110 Q 314 124 322 138" className="animate-[monita-beat_1.6s_ease-in-out_infinite]" />
        <path d="M306 100 Q 292 124 306 148" className="animate-[monita-beat_1.6s_ease-in-out_0.3s_infinite]" />
      </g>
      <path d="M374 116 L340 102 L340 146 L374 132 Z" strokeWidth="2.5" strokeLinejoin="round" className="fill-card stroke-muted-foreground" />
      <rect x="374" y="114" width="8" height="20" rx="2" className="fill-muted-foreground" />
      <Mascot x={440} y={124} />
      <path d="M396 136 Q 388 138 382 128" strokeWidth="4" strokeLinecap="round" className={ARM} />
      <path d="M484 132 Q 496 140 494 152" strokeWidth="4" strokeLinecap="round" className={ARM} />
    </Frame>
  );
}
