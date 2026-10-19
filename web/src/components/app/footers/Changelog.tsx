import type { ReactElement } from "react";
import { ARM, Mascot } from "@/components/brand/Mascot";
import { Frame, TextLine, Window } from "./Frame";

const GLYPHS = (
  <>
    <path d="M18 22h18l8 8-8 8H18Z" />
    <circle cx="24" cy="30" r="1.5" />
    <path d="M130 66v20M120 76h20" />
  </>
);

const CONFETTI: [number, number, number, string][] = [
  [292, 54, 20, "fill-mascot-line"],
  [316, 74, -30, "fill-up"],
  [338, 48, 45, "fill-muted-foreground"],
  [272, 82, -15, "fill-up"],
  [356, 70, 10, "fill-mascot-line"],
];

/** Changelog: a release goes out; Moni waves its flag over the notes. */
export function ChangelogFooter(): ReactElement {
  return (
    <Frame id="changelog" glyphs={GLYPHS}>
      <Window>
        <rect x="80" y="69" width="52" height="14" rx="4" className="fill-muted" />
        <text x="106" y="80" textAnchor="middle" className="fill-foreground font-mono text-[10px] font-semibold">
          v1.4.0
        </text>
        <TextLine x={140} y={72} w={80} />
        {[0, 1, 2, 3].map((n) => {
          const y = 104 + n * 18;
          const added = n < 3;
          return (
            <g key={n}>
              <path d={added ? `M44 ${y}h8M48 ${y - 4}v8` : `M44 ${y}h8`} strokeWidth="2" strokeLinecap="round" className={added ? "stroke-up" : "stroke-muted-foreground"} />
              <TextLine x={62} y={y - 4} w={[150, 120, 136, 96][n]!} />
            </g>
          );
        })}
      </Window>
      {CONFETTI.map(([x, y, r, cls]) => (
        <rect key={`${x}${y}`} x={x} y={y} width="8" height="4" rx="1" transform={`rotate(${r} ${x + 4} ${y + 2})`} className={cls} />
      ))}
      {/* The flag, on a pole in Moni's hand. */}
      <line x1="378" y1="134" x2="378" y2="52" strokeWidth="3" strokeLinecap="round" className="stroke-muted-foreground" />
      <path d="M378 54 L410 64 L378 74 Z" className="fill-mascot-line" />
      <Mascot x={440} y={124} />
      <path d="M396 136 Q 386 140 380 128" strokeWidth="4" strokeLinecap="round" className={ARM} />
      <path d="M484 132 Q 496 140 494 152" strokeWidth="4" strokeLinecap="round" className={ARM} />
    </Frame>
  );
}
