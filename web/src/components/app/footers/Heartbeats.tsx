import type { ReactElement } from "react";
import { ARM, Mascot } from "@/components/brand/Mascot";
import { Flow, Frame, TextLine, Window } from "./Frame";

/** A heart about 20 units wide, centred on (x, y). */
function Heart({ x, y, scale = 1, className }: { x: number; y: number; scale?: number; className: string }): ReactElement {
  return <path transform={`translate(${x} ${y}) scale(${scale})`} d="M0 8C-12 0-9-10 0-4C9-10 12 0 0 8Z" className={className} />;
}

const GLYPHS = (
  <>
    <path d="M30 34C18 26 21 16 30 22C39 16 42 26 30 34Z" />
    <path d="M100 82h10l4-9 6 18 4-9h10" />
    <circle cx="150" cy="30" r="8" />
    <path d="M150 25v5l3 2" />
  </>
);

/** Heartbeats: a scheduled job runs, then calls Monita; Moni catches each beat. */
export function HeartbeatsFooter(): ReactElement {
  return (
    <Frame id="heartbeats" glyphs={GLYPHS}>
      <Window>
        <TextLine x={80} y={72} w={100} />
        <text x="40" y="114" className="fill-muted-foreground font-mono text-[11px]">
          $ ./nightly-backup.sh
        </text>
        <TextLine x={40} y={126} w={150} />
        <rect x="40" y="142" width="64" height="22" rx="6" className="fill-up/15" />
        <text x="72" y="157" textAnchor="middle" className="fill-up font-mono text-[11px] font-semibold">
          exit 0
        </text>
      </Window>
      {/* The schedule that runs the job. */}
      <g className="stroke-muted-foreground" strokeWidth="2.5" strokeLinecap="round">
        <circle cx="244" cy="64" r="15" className="fill-card" />
        <line x1="244" y1="64" x2="244" y2="55" />
        <line x1="244" y1="64" x2="251" y2="68" />
      </g>
      {/* The job calls Monita when it finishes. */}
      <Flow d="M262 132 C 296 94, 326 92, 350 106" answer />
      <Heart x={304} y={92} scale={0.7} className="fill-up/70" />
      <Heart x={366} y={112} scale={1.4} className="animate-[monita-beat_1.2s_ease-in-out_infinite] fill-up" />
      <Mascot x={440} y={124} />
      <path d="M396 136 Q 384 140 376 124" strokeWidth="4" strokeLinecap="round" className={ARM} />
      <path d="M484 132 Q 496 140 494 152" strokeWidth="4" strokeLinecap="round" className={ARM} />
    </Frame>
  );
}
