import { useEffect, type ReactElement } from "react";
import { Routes, Route, useLocation } from "react-router-dom";
import { mount } from "@/mount";
import { NotFound } from "@/pages/NotFound";
import { publicRoutes } from "@/PublicRoutes";

/** A link out of the public pages loads the team app from the server. Under /status/ the server
 * sends this app again, so such a path is a 404 here instead of a reload loop. */
function Leave(): ReactElement | null {
  const { pathname, search, hash } = useLocation();
  const outside = !pathname.startsWith("/status/");
  useEffect(() => {
    if (outside) window.location.assign(pathname + search + hash);
  }, [outside, pathname, search, hash]);
  return outside ? null : <NotFound />;
}

mount(
  <Routes>
    {publicRoutes()}
    <Route path="*" element={<Leave />} />
  </Routes>,
);
