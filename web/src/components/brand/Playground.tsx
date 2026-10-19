import type { ReactElement } from "react";
import { Mascot } from "@/components/brand/Mascot";

/** The ground line, in the scene's units. */
const GROUND = 206;

/** Where a Moni of this size stands so its feet touch the ground. */
const standY = (scale: number): number => GROUND - 56 * scale;

/** An arm in Moni's own units (Moni faces left there). */
function Arm({ d }: { d: string }): ReactElement {
  return <path d={d} fill="none" strokeWidth="5" strokeLinecap="round" className="stroke-mascot-line" />;
}

const ARM_DOWN = "M40 12Q54 22 52 36";

/** A four-pointed sparkle centred on (x, y). */
function Sparkle({ x, y, size, className, delay }: { x: number; y: number; size: number; className: string; delay: string }): ReactElement {
  const s = size;
  return (
    <path
      d={`M${x} ${y - s}Q${x + s * 0.2} ${y - s * 0.2} ${x + s} ${y}Q${x + s * 0.2} ${y + s * 0.2} ${x} ${y + s}Q${x - s * 0.2} ${y + s * 0.2} ${x - s} ${y}Q${x - s * 0.2} ${y - s * 0.2} ${x} ${y - s}Z`}
      style={{ animationDelay: delay }}
      className={`animate-[monita-beat_1.8s_ease-in-out_infinite] ${className}`}
    />
  );
}

function Cloud({ x, y }: { x: number; y: number }): ReactElement {
  return <path d={`M${x} ${y}a14 14 0 0 1 26-8a18 18 0 0 1 34 4a12 12 0 0 1 2 24h-60a12 12 0 0 1-2-20Z`} className="fill-muted" />;
}

const TOSS = "M571 116Q620 26 669 116";

/**
 * Moni and friends at play along the bottom of the sign-in page: a kite, a game of catch, a jump,
 * a tower of uptime blocks and bubbles. Decoration only; reduced motion stills it.
 */
export function Playground(): ReactElement {
  return (
    <svg aria-hidden="true" viewBox="0 0 1200 220" preserveAspectRatio="xMidYMax slice" className="block h-[160px] w-full sm:aspect-[1200/220] sm:h-auto">
      <Cloud x={70} y={46} />
      <Cloud x={980} y={34} />
      <line x1="0" y1={GROUND} x2="1200" y2={GROUND} strokeWidth="1.5" className="stroke-input" />

      {/* A kite with the mark on it. */}
      <path d="M212 120Q252 108 262 64" fill="none" strokeWidth="1.5" className="stroke-muted-foreground" />
      <g className="origin-[262px_64px] animate-[monita-sway_3s_ease-in-out_infinite]">
        <path d="M264 18L286 42L264 66L242 42Z" strokeWidth="2.5" strokeLinejoin="round" className="fill-mascot-body stroke-mascot-line" />
        <path d="M249 44h4l3-6 5 11 3-5h4" fill="none" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="stroke-mascot-line" />
        <path d="M264 66q-7 10 0 18q7 8 0 16" fill="none" strokeWidth="1.5" className="stroke-muted-foreground" />
        <path d="M258 82l6 3-6 3ZM270 98l-6 3 6 3Z" className="fill-mascot-cheek" />
      </g>
      <Mascot x={170} y={standY(0.75)} scale={0.75} flip happy>
        <Arm d="M-34 -20Q-50 -40 -56 -58" />
        <Arm d={ARM_DOWN} />
      </Mascot>

      {/* A jump for joy. */}
      <ellipse cx="360" cy={GROUND} rx="30" ry="4" className="fill-input" />
      <g className="animate-[monita-bounce_1.2s_ease-in-out_infinite]">
        <Mascot x={360} y={standY(0.8)} scale={0.8} happy>
          <Arm d="M-38 -18Q-58 -36 -52 -58" />
          <Arm d="M38 -18Q58 -36 52 -58" />
        </Mascot>
      </g>
      <Sparkle x={300} y={92} size={9} className="fill-mascot-line" delay="0s" />
      <Sparkle x={424} y={72} size={7} className="fill-up" delay="0.6s" />
      <Sparkle x={414} y={124} size={5} className="fill-mascot-cheek" delay="1.1s" />

      {/* A game of catch, and a small one cheering it on. */}
      <Mascot x={520} y={standY(0.85)} scale={0.85} flip happy>
        <Arm d="M-36 -14Q-56 -26 -60 -42" />
        <Arm d={ARM_DOWN} />
      </Mascot>
      <Mascot x={720} y={standY(0.85)} scale={0.85} happy>
        <Arm d="M-36 -14Q-56 -26 -60 -42" />
        <Arm d={ARM_DOWN} />
      </Mascot>
      <g style={{ offsetPath: `path("${TOSS}")` }} className="animate-[monita-toss_1.4s_ease-in-out_infinite_alternate]">
        <circle r="10" strokeWidth="2" className="fill-mascot-cheek stroke-mascot-line" />
        <path d="M-9 -3q9 6 18 0" fill="none" strokeWidth="1.5" className="stroke-mascot-line" />
      </g>
      <Mascot x={620} y={standY(0.45)} scale={0.45}>
        <Arm d="M-38 -18Q-58 -36 -52 -58" />
        <Arm d="M38 -18Q58 -36 52 -58" />
      </Mascot>

      {/* A tower of uptime blocks, one more on the way. */}
      {[0.6, 0.7, 0.8, 0.9].map((o, n) => (
        <rect key={n} x="940" y={GROUND - 14 - n * 15} width="48" height="14" rx="3" opacity={o} className="fill-up" />
      ))}
      <rect x="926" y="104" width="48" height="14" rx="3" transform="rotate(-8 950 111)" className="fill-up" />
      <Mascot x={880} y={standY(0.8)} scale={0.8} flip>
        <Arm d="M-36 -10Q-60 -30 -64 -50" />
        <Arm d={ARM_DOWN} />
      </Mascot>

      {/* Bubbles. */}
      <line x1="1104" y1="138" x2="1114" y2="124" strokeWidth="2.5" strokeLinecap="round" className="stroke-muted-foreground" />
      <circle cx="1120" cy="116" r="9" fill="none" strokeWidth="2" className="stroke-muted-foreground" />
      {[
        [1134, 100, 7, "0s"],
        [1150, 86, 10, "0.7s"],
        [1128, 78, 6, "1.4s"],
        [1156, 112, 5, "2.1s"],
      ].map(([cx, cy, r, delay]) => (
        <circle
          key={String(delay)}
          cx={cx}
          cy={cy}
          r={r}
          strokeWidth="1.5"
          style={{ animationDelay: String(delay) }}
          className="animate-[monita-rise_2.8s_ease-out_infinite] fill-mascot-body/40 stroke-mascot-line/60"
        />
      ))}
      <Mascot x={1060} y={standY(0.75)} scale={0.75} flip happy>
        <Arm d="M-34 -8Q-52 -18 -58 -34" />
        <Arm d={ARM_DOWN} />
      </Mascot>
    </svg>
  );
}

/** Moni peeking over the top edge of a card, holding on with both hands. Place it in a relative
 * parent; it sits above the edge. */
export function PeekingMoni({ className }: { className?: string }): ReactElement {
  return (
    <svg aria-hidden="true" viewBox="0 0 100 46" className={className}>
      <Mascot x={50} y={50} scale={0.62} />
      <ellipse cx="24" cy="44" rx="8" ry="5" className="fill-mascot-line" />
      <ellipse cx="76" cy="44" rx="8" ry="5" className="fill-mascot-line" />
    </svg>
  );
}
