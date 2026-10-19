import type { ReactElement } from "react";
import { Navigate, Route, Routes } from "react-router-dom";
import { AdminLayout } from "@/layouts/AdminLayout";
import { AppLayout } from "@/layouts/AppLayout";
import { ProjectLayout } from "@/layouts/ProjectLayout";
import { PublicLayout } from "@/layouts/PublicLayout";
import { SettingsLayout } from "@/layouts/SettingsLayout";
import { Alerts } from "@/pages/project/Alerts";
import { Allowlist } from "@/pages/project/Allowlist";
import { General } from "@/pages/project/General";
import { Secrets } from "@/pages/project/Secrets";
import { StatusPageSettings } from "@/pages/project/StatusPageSettings";
import { Heartbeat } from "@/pages/project/Heartbeat";
import { Incident } from "@/pages/project/Incident";
import { Incidents } from "@/pages/project/Incidents";
import { Heartbeats } from "@/pages/project/Heartbeats";
import { Keys } from "@/pages/project/Keys";
import { Monitor } from "@/pages/project/Monitor";
import { Monitors } from "@/pages/project/Monitors";
import { Overview } from "@/pages/project/Overview";
import { DoneTickets } from "@/pages/project/DoneTickets";
import { Tickets } from "@/pages/project/Tickets";
import { History } from "@/pages/public/History";
import { Incidents as PublicIncidents } from "@/pages/public/Incidents";
import { KnownIssues } from "@/pages/public/KnownIssues";
import { Mine } from "@/pages/public/Mine";
import { Report } from "@/pages/public/Report";
import { Status } from "@/pages/public/Status";
import { Email } from "@/pages/admin/Email";
import { RateLimits } from "@/pages/admin/RateLimits";
import { System } from "@/pages/admin/System";
import { TimeZone } from "@/pages/admin/TimeZone";
import { Users } from "@/pages/admin/users/Users";
import { Invite } from "@/pages/Invite";
import { Login } from "@/pages/Login";
import { NotFound } from "@/pages/NotFound";
import { Projects } from "@/pages/Projects";

/** Every page the binary serves; the server answers any path no route claims with this app. */
export default function App(): ReactElement {
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route path="/invite/:token" element={<Invite />} />
      <Route element={<AppLayout />}>
        <Route path="/" element={<Projects />} />
        <Route element={<AdminLayout />}>
          <Route path="/admin/users" element={<Users />} />
          <Route path="/admin/limits" element={<RateLimits />} />
          <Route path="/admin/email" element={<Email />} />
          <Route path="/admin/time-zone" element={<TimeZone />} />
          <Route path="/admin/system" element={<System />} />
          <Route path="/admin/settings" element={<Navigate to="/admin/system" replace />} />
        </Route>
      </Route>
      <Route path="/projects/:slug" element={<ProjectLayout />}>
        <Route index element={<Overview />} />
        <Route path="monitors" element={<Monitors />} />
        <Route path="monitors/new" element={<Monitors creating />} />
        <Route path="monitors/:id" element={<Monitor />} />
        <Route path="monitors/:id/edit" element={<Monitor editing />} />
        <Route path="heartbeats" element={<Heartbeats />} />
        <Route path="heartbeats/new" element={<Heartbeats creating />} />
        <Route path="heartbeats/:id" element={<Heartbeat />} />
        <Route path="heartbeats/:id/edit" element={<Heartbeat editing />} />
        <Route path="incidents" element={<Incidents />} />
        <Route path="incidents/new" element={<Incidents creating />} />
        <Route path="incidents/:id" element={<Incident />} />
        <Route path="tickets/done" element={<DoneTickets />} />
        <Route path="tickets/:number?" element={<Tickets />} />
        <Route element={<SettingsLayout />}>
          <Route path="settings" element={<General />} />
          <Route path="settings/status-page" element={<StatusPageSettings />} />
          <Route path="settings/keys" element={<Keys />} />
          <Route path="settings/secrets" element={<Secrets />} />
          <Route path="settings/allowlist" element={<Allowlist />} />
          <Route path="settings/alerts" element={<Alerts />} />
        </Route>
        <Route path="alerts" element={<Navigate to="../settings/alerts" replace />} />
      </Route>
      <Route path="/status/:slug" element={<PublicLayout />}>
        <Route index element={<Status />} />
        <Route path="incidents" element={<PublicIncidents />} />
        <Route path="history" element={<History />} />
        <Route path="tickets" element={<KnownIssues />} />
        <Route path="report" element={<Report />} />
        <Route path="mine" element={<Mine />} />
      </Route>
      <Route path="*" element={<NotFound />} />
    </Routes>
  );
}
