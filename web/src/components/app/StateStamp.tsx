import type { CSSProperties, ReactElement } from "react";
import { Link } from "react-router-dom";
import { ago, duration, hhmm, nowSecs } from "@/lib/time";
import type { MaintenanceRef } from "@/lib/types";
import { cn } from "@/lib/utils";

/** Barrier stripes for a list row that is paused or in maintenance; nothing for any other state.
 * `covered` is a window over it: the row is in maintenance even when an open incident keeps the
 * state Down, because the state column says the state and the tape says what Monita is doing. */
export function rowTape(state: string, covered = false): CSSProperties | undefined {
  if (state === "paused") return tape("--muted-foreground", 10, "--row-tape-boost");
  if (state === "maintenance" || covered) return tape("--issue", 14, "--row-tape-boost");
  return undefined;
}

/** Diagonal stripes like barrier tape, in a state's own token. The boost token makes them stronger in dark mode. */
export function tape(token: string, strength = 22, boost = "--tape-boost"): CSSProperties {
  return {
    backgroundImage: `repeating-linear-gradient(-45deg, color-mix(in oklab, var(${token}) min(100%, calc(${strength}% * var(${boost}))), transparent) 0 10px, transparent 10px 20px)`,
  };
}

/**
 * A stamp under a monitor's or heartbeat's title while it is paused or in maintenance, so its
 * page never reads as a normal one: barrier stripes, the word, and one sentence on what it means.
 */
export function StateStamp({
  kind,
  pausedAt,
  maintenance,
  slug,
}: {
  kind: "monitor" | "heartbeat";
  pausedAt: number | null;
  maintenance: MaintenanceRef | null;
  slug: string;
}): ReactElement | null {
  if (pausedAt === null && maintenance === null) return null;
  const paused = pausedAt !== null;
  const token = paused ? "--muted-foreground" : "--issue";
  const left = maintenance ? Math.max(0, maintenance.ends_at - nowSecs()) : 0;
  return (
    <div role="status" className="overflow-hidden rounded-xl border bg-card">
      <div aria-hidden="true" className="h-2.5" style={tape(token)} />
      <div className="flex flex-wrap items-center gap-x-4 gap-y-2 px-5 py-3.5">
        <span className={cn("rounded-md border-2 px-2.5 py-0.5 text-sm font-bold tracking-[0.18em] uppercase", paused ? "border-muted-foreground text-muted-foreground" : "border-issue text-issue")}>
          {paused ? "Paused" : "In maintenance"}
        </span>
        <p className="min-w-0 flex-1 text-sm text-muted-foreground">
          {paused
            ? kind === "monitor"
              ? `Paused ${ago(pausedAt)}, so Monita checks nothing and sends no alerts until you resume it, and the status page leaves it out.`
              : `Paused ${ago(pausedAt)}, so Monita keeps its pings but watches no deadline and sends no alerts until you resume it.`
            : `${maintenance?.title} ends at ${hhmm(maintenance?.ends_at ?? 0)}, in ${duration(left)}, and until then a failure opens no incident and sends no alert.`}
        </p>
        {!paused && maintenance && (
          <Link to={`/projects/${slug}/maintenance`} className="text-sm text-link underline-offset-2 hover:underline">
            See the window
          </Link>
        )}
      </div>
      <div aria-hidden="true" className="h-2.5" style={tape(token)} />
    </div>
  );
}
