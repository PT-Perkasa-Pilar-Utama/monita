import type { ReactElement } from "react";
import { Route, Routes } from "react-router-dom";
import { AppLayout } from "@/layouts/AppLayout";
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
      <Route path="*" element={<NotFound />} />
    </Routes>
  );
}
