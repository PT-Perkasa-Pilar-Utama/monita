import type { ReactElement } from "react";
import { Password } from "@/pages/account/Password";
import { Profile } from "@/pages/account/Profile";
import { TwoFactor } from "@/pages/account/TwoFactor";
import { AccountLayout } from "@/layouts/AccountLayout";
import { Navigate, Route, Routes } from "react-router-dom";
import { AdminLayout } from "@/layouts/AdminLayout";
import { AppLayout, SignedInLayout } from "@/layouts/AppLayout";
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
import { Maintenance } from "@/pages/project/Maintenance";
import { Heartbeats } from "@/pages/project/Heartbeats";
import { Keys } from "@/pages/project/Keys";
import { Monitor } from "@/pages/project/Monitor";
import { Monitors } from "@/pages/project/Monitors";
import { Overview } from "@/pages/project/Overview";
import { DoneTickets } from "@/pages/project/DoneTickets";
import { Tickets } from "@/pages/project/Tickets";
import { History } from "@/pages/public/History";
import { Incidents as PublicIncidents } from "@/pages/public/Incidents";
import { Issues } from "@/pages/public/Issues";
import { PastUptime } from "@/pages/public/PastUptime";
import { Ticket } from "@/pages/public/Ticket";
import { Status } from "@/pages/public/Status";
import { Email } from "@/pages/admin/Email";
import { Telegram } from "@/pages/admin/Telegram";
import { RateLimits } from "@/pages/admin/RateLimits";
import { System } from "@/pages/admin/System";
import { Locations } from "@/pages/admin/Locations";
import { TimeZone } from "@/pages/admin/TimeZone";
import { Users } from "@/pages/admin/users/Users";
import { AllIncidents } from "@/pages/AllIncidents";
import { AllTickets } from "@/pages/AllTickets";
import { Invite } from "@/pages/Invite";
import { Changelog } from "@/pages/project/Changelog";
import { PublicChangelog } from "@/pages/public/Changelog";
import { Login } from "@/pages/Login";
import { NotFound } from "@/pages/NotFound";
import { Projects } from "@/pages/Projects";

/** Every page the binary serves; the server answers any path no route claims with this app. */
export default function App(): ReactElement {
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route path="/invite/:token" element={<Invite />} />
      <Route element={<SignedInLayout />}>
        <Route element={<AppLayout />}>
          <Route path="/" element={<Projects />} />
          <Route path="/incidents" element={<AllIncidents />} />
          <Route path="/tickets" element={<AllTickets />} />
          <Route path="/account" element={<Navigate to="/account/profile" replace />} />
          <Route element={<AccountLayout />}>
            <Route path="/account/profile" element={<Profile />} />
            <Route path="/account/password" element={<Password />} />
            <Route path="/account/two-factor" element={<TwoFactor />} />
          </Route>
          <Route element={<AdminLayout />}>
            <Route path="/admin/users" element={<Users />} />
            <Route path="/admin/limits" element={<RateLimits />} />
            <Route path="/admin/email" element={<Email />} />
            <Route path="/admin/telegram" element={<Telegram />} />
            <Route path="/admin/locations" element={<Locations />} />
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
          <Route path="maintenance" element={<Maintenance />} />
          <Route path="maintenance/new" element={<Maintenance creating />} />
          <Route path="incidents/maintenance" element={<Navigate to="../maintenance" replace />} />
          <Route path="incidents/:id" element={<Incident />} />
          <Route path="changelog" element={<Changelog />} />
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
      </Route>
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
      <Route path="*" element={<NotFound />} />
    </Routes>
  );
}
