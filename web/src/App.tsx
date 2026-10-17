import type { ReactElement } from "react";
import { Route, Routes } from "react-router-dom";
import { AppLayout } from "@/layouts/AppLayout";
import { ProjectLayout } from "@/layouts/ProjectLayout";
import { SettingsLayout } from "@/layouts/SettingsLayout";
import { Alerts } from "@/pages/project/Alerts";
import { Allowlist } from "@/pages/project/Allowlist";
import { General } from "@/pages/project/General";
import { Heartbeat } from "@/pages/project/Heartbeat";
import { HeartbeatForm } from "@/pages/project/HeartbeatForm";
import { Incident } from "@/pages/project/Incident";
import { IncidentForm } from "@/pages/project/IncidentForm";
import { Incidents } from "@/pages/project/Incidents";
import { Heartbeats } from "@/pages/project/Heartbeats";
import { Keys } from "@/pages/project/Keys";
import { Monitor } from "@/pages/project/Monitor";
import { MonitorForm } from "@/pages/project/MonitorForm";
import { Monitors } from "@/pages/project/Monitors";
import { Overview } from "@/pages/project/Overview";
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
      </Route>
      <Route path="/projects/:slug" element={<ProjectLayout />}>
        <Route index element={<Overview />} />
        <Route path="monitors" element={<Monitors />} />
        <Route path="monitors/new" element={<MonitorForm key="new" />} />
        <Route path="monitors/:id" element={<Monitor />} />
        <Route path="monitors/:id/edit" element={<MonitorForm key="edit" />} />
        <Route path="heartbeats" element={<Heartbeats />} />
        <Route path="heartbeats/new" element={<HeartbeatForm key="new" />} />
        <Route path="heartbeats/:id" element={<Heartbeat />} />
        <Route path="heartbeats/:id/edit" element={<HeartbeatForm key="edit" />} />
        <Route path="incidents" element={<Incidents />} />
        <Route path="incidents/new" element={<IncidentForm />} />
        <Route path="incidents/:id" element={<Incident />} />
        <Route element={<SettingsLayout />}>
          <Route path="settings" element={<General />} />
          <Route path="settings/keys" element={<Keys />} />
          <Route path="settings/allowlist" element={<Allowlist />} />
          <Route path="alerts" element={<Alerts />} />
        </Route>
      </Route>
      <Route path="*" element={<NotFound />} />
    </Routes>
  );
}
