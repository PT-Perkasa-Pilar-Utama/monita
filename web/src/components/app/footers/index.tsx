import type { ReactElement } from "react";
import type { Primitive } from "@/lib/primitiveFooter";
import { ChangelogFooter } from "./Changelog";
import { HeartbeatsFooter } from "./Heartbeats";
import { IncidentsFooter } from "./Incidents";
import { MaintenanceFooter } from "./Maintenance";
import { MonitorsFooter } from "./Monitors";
import { TicketsFooter } from "./Tickets";

const FOOTERS: Record<Primitive, () => ReactElement> = {
  monitors: MonitorsFooter,
  heartbeats: HeartbeatsFooter,
  incidents: IncidentsFooter,
  maintenance: MaintenanceFooter,
  tickets: TicketsFooter,
  changelog: ChangelogFooter,
};

/** The footer illustration for a primitive's pages. */
export function PrimitiveFooter({ kind }: { kind: Primitive }): ReactElement {
  const Footer = FOOTERS[kind];
  return <Footer />;
}
