import type { ReactElement, ReactNode } from "react";
import { Mascot } from "@/components/brand/Mascot";

/** The scene's width in its own units; the edges fill the rest of the page width. */
const SCENE_W = 560;

/** One edge of the footer: the primitive's tile repeated, fading toward the scene. */
function Edge({ tile, side }: { tile: string; side: "left" | "right" }): ReactElement {
  return (
    <svg
      className={
        side === "left"
          ? "h-full min-w-0 flex-1 [mask-image:linear-gradient(to_right,black_20%,transparent)]"
          : "h-full min-w-0 flex-1 [mask-image:linear-gradient(to_left,black_20%,transparent)]"
      }
    >
      <rect width="100%" height="100%" fill={`url(#${tile})`} />
    </svg>
  );
}

/**
 * The full-width illustration under a primitive's pages (monitors first): a scene with Moni in the
 * middle, the primitive's pattern on both edges, on one ground line. Decoration only.
 */
export function PrimitiveFooter({ tile, defs, children }: { tile: string; defs: ReactNode; children: ReactNode }): ReactElement {
  return (
    <footer aria-hidden="true" className="relative mt-auto h-[200px] shrink-0 overflow-hidden border-t bg-paper max-sm:h-[150px]">
      {/* The tile's currentColor comes from here, where the pattern is defined. */}
      <svg className="absolute size-0 text-input">
        <defs>{defs}</defs>
      </svg>
      <div className="absolute inset-x-0 top-0 bottom-5 flex max-sm:bottom-[15px]">
        <Edge tile={tile} side="left" />
        <div className="w-[560px] shrink-0 max-sm:w-[420px]" />
        <Edge tile={tile} side="right" />
      </div>
      <div className="absolute inset-x-0 bottom-5 h-px bg-input max-sm:bottom-[15px]" />
      <svg viewBox={`0 0 ${SCENE_W} 200`} preserveAspectRatio="xMidYMax meet" className="relative mx-auto block h-full w-[560px] max-sm:w-[420px]">
        {children}
      </svg>
    </footer>
  );
}

/** Rows of day bars, mostly up: what a wall of monitors looks like from far away. */
const MONITOR_TILE = (
  // Offset so each row's bars stand on the ground line of the 180-unit-high edge.
  <pattern id="monitors-tile" y="26" width="96" height="40" patternUnits="userSpaceOnUse">
    {[18, 24, 14, 24, 20, 24, 10, 22].map((h, n) => (
      <rect key={n} x={n * 12 + 3} y={34 - h} width="6" height={h} rx="1.5" fill="currentColor" />
    ))}
    <rect x="63" y="10" width="6" height="24" rx="1.5" className="fill-up/40" />
  </pattern>
);

/** 24 hourly bars, all up; the newest one beats. */
function DayBars(): ReactElement {
  return (
    <g>
      {Array.from({ length: 24 }, (_, n) => (
        <rect
          key={n}
          x={40 + n * 8}
          y={138}
          width="6"
          height="26"
          rx="1.5"
          className={n === 23 ? "animate-[monita-beat_2s_ease-in-out_infinite] fill-up" : "fill-up"}
          opacity={n === 23 ? undefined : 0.55 + (n % 5) * 0.09}
        />
      ))}
    </g>
  );
}

/** Monitors: Moni checks an endpoint through a magnifying glass, on a timer, and it answers 200 OK. */
export function MonitorsFooter(): ReactElement {
  return (
    <PrimitiveFooter tile="monitors-tile" defs={MONITOR_TILE}>
      {/* The endpoint: a window standing on the ground line. */}
      <rect x="24" y="64" width="232" height="116" rx="12" strokeWidth="1.5" className="fill-card stroke-input" />
      <line x1="24" y1="88" x2="256" y2="88" strokeWidth="1.5" className="stroke-input" />
      {[40, 52, 64].map((cx) => (
        <circle key={cx} cx={cx} cy="76" r="3.5" className="fill-input" />
      ))}
      <rect x="80" y="71" width="120" height="10" rx="5" className="fill-input" />
      <rect x="40" y="102" width="60" height="22" rx="6" className="fill-up/15" />
      <text x="70" y="117" textAnchor="middle" className="fill-up font-mono text-[11px] font-semibold">
        200 OK
      </text>
      <rect x="110" y="109" width="84" height="8" rx="4" className="fill-input" />
      <DayBars />

      {/* The check travels from Moni's glass to the endpoint, on an interval. */}
      <path
        d="M333 108 C 310 92, 285 92, 260 112"
        fill="none"
        strokeWidth="2.5"
        strokeLinecap="round"
        strokeDasharray="4 7"
        className="animate-[monita-ping_1.1s_linear_infinite] stroke-primary/60"
      />
      <g className="stroke-muted-foreground" strokeWidth="2.5" strokeLinecap="round">
        <circle cx="296" cy="60" r="13" className="fill-card" />
        <line x1="296" y1="47" x2="296" y2="42" />
        <line x1="291" y1="41" x2="301" y2="41" />
        <line x1="296" y1="60" x2="296" y2="52" />
        <line x1="296" y1="60" x2="302" y2="64" />
      </g>

      {/* The glass, held out to the left. */}
      <line x1="362" y1="117" x2="375" y2="130" strokeWidth="6" strokeLinecap="round" className="stroke-muted-foreground" />
      <circle cx="349" cy="104" r="17" strokeWidth="4" className="fill-card stroke-muted-foreground" />
      <path d="M341 104l5 5 10-11" fill="none" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" className="stroke-up" />
      <Mascot x={440} y={124} />
      <path d="M396 136 Q 384 142 375 130" fill="none" strokeWidth="4" strokeLinecap="round" className="stroke-primary" />
      <path d="M484 132 Q 496 140 494 152" fill="none" strokeWidth="4" strokeLinecap="round" className="stroke-primary" />
    </PrimitiveFooter>
  );
}
