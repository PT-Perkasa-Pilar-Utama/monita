import type { ReactElement } from "react";
import { ARM, Mascot } from "@/components/brand/Mascot";
import { Frame } from "./Frame";

/** A ticket: a card with a notch on each side. */
const TICKET = "M0 0h30v6a4 4 0 0 0 0 8v6H0v-6a4 4 0 0 0 0-8Z";

const GLYPHS = (
  <>
    <path transform="translate(18 18)" d={TICKET} />
    <path d="M110 70v26M120 70v18M130 70v22" />
  </>
);

/** Card counts per column: Triage, Backlog, In progress, Done. */
const COLUMNS = [2, 3, 1, 3];

/** Tickets: a board of four columns; Moni carries a ticket over to Done. */
export function TicketsFooter(): ReactElement {
  return (
    <Frame id="tickets" glyphs={GLYPHS}>
      <rect x="24" y="64" width="232" height="116" rx="12" strokeWidth="1.5" className="fill-card stroke-input" />
      {COLUMNS.map((cards, c) => {
        const x = 32 + c * 56;
        const done = c === 3;
        return (
          <g key={c}>
            <rect x={x} y="72" width="48" height="100" rx="6" className="fill-paper" />
            <circle cx={x + 8} cy="82" r="3.5" strokeWidth="1.5" className={done ? "fill-up" : c === 0 ? "fill-none stroke-muted-foreground" : "fill-muted-foreground"} strokeDasharray={c === 0 ? "2 2" : undefined} />
            <rect x={x + 15} y="79" width="24" height="6" rx="3" className="fill-input" />
            {Array.from({ length: cards }, (_, n) => (
              <rect key={n} x={x + 4} y={92 + n * 22} width="40" height="16" rx="4" strokeWidth="1" className={done ? "fill-up/15 stroke-up/40" : "fill-card stroke-input"} />
            ))}
          </g>
        );
      })}
      <path d="M352 104 C 330 62, 270 56, 228 86" fill="none" strokeWidth="2.5" strokeLinecap="round" strokeDasharray="4 7" className="animate-[monita-ping_1.1s_linear_infinite] stroke-primary/60" />
      <path d="M236 80l-9 6 10 3" fill="none" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" className="stroke-primary/60" />
      <path transform="translate(352 104) rotate(-12)" d={TICKET} strokeWidth="2" className="fill-card stroke-muted-foreground" />
      <circle cx="367" cy="113" r="3" transform="rotate(-12 367 113)" className="fill-up" />
      <Mascot x={440} y={124} />
      <path d="M396 136 Q 384 140 378 122" strokeWidth="4" strokeLinecap="round" className={ARM} />
      <path d="M484 132 Q 496 140 494 152" strokeWidth="4" strokeLinecap="round" className={ARM} />
    </Frame>
  );
}
