import type { ReactElement } from "react";
import { Navigate, Route } from "react-router-dom";
import { PublicLayout } from "@/layouts/PublicLayout";
import { PublicChangelog } from "@/pages/public/Changelog";
import { History } from "@/pages/public/History";
import { Incidents as PublicIncidents } from "@/pages/public/Incidents";
import { Issues } from "@/pages/public/Issues";
import { PastUptime } from "@/pages/public/PastUptime";
import { Status } from "@/pages/public/Status";
import { Ticket } from "@/pages/public/Ticket";

/** A project's public pages. Both apps hold them: the team app, so its links open them in
 * place, and the public app, which a status page visitor downloads instead of the team's. */
export function publicRoutes(): ReactElement {
  return (
    <Route path="/status/:slug" element={<PublicLayout />}>
      <Route index element={<Status />} />
      <Route path="uptime" element={<PastUptime />} />
      <Route path="incidents" element={<PublicIncidents />} />
      <Route path="history" element={<History />} />
      <Route path="issues" element={<Issues />} />
      <Route path="ticket" element={<Ticket />} />
      <Route path="changelog/:version?" element={<PublicChangelog />} />
      <Route path="tickets" element={<Navigate to="../issues" replace />} />
      <Route path="report" element={<Navigate to="../ticket" replace />} />
      <Route path="mine" element={<Navigate to="../ticket" replace />} />
    </Route>
  );
}
