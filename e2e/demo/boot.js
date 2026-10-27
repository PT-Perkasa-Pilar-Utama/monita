// Starts the online demo: the real app, with the service worker in sw.js answering its API from a
// recording. The app's script loads only once the worker controls the page, so no request reaches
// the network first. snapshot.ts puts this script in place of the app's module script.
(() => {
  const me = document.currentScript;
  const base = me.dataset.base;
  const entry = me.dataset.entry;
  const install = "https://github.com/PT-Perkasa-Pilar-Utama/monita#install";

  // The demo shows the pages, not the notes a first visit gets.
  try {
    localStorage.setItem("monita.2fa-note-closed:admin@monita.test", "1");
    localStorage.setItem("monita.whats-new-seen:admin@monita.test", "999.0.0");
  } catch {}

  // 404.html sends a page the site has no file for here, with its path in demo-to.
  const to = new URLSearchParams(location.search).get("demo-to");
  if (to && to.startsWith(`${base}/`)) history.replaceState(null, "", to);
  else if (location.pathname.length > base.length + 1 && location.pathname.endsWith("/")) {
    // GitHub Pages adds a slash to a folder's path; the app's routes have none.
    history.replaceState(null, "", location.pathname.replace(/\/+$/, "") + location.search + location.hash);
  }

  const ready = new Promise((done) => {
    if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", done, { once: true });
    else done();
  });

  const style = `position:fixed;z-index:60;left:16px;bottom:16px;max-width:calc(100vw - 32px);box-sizing:border-box;
padding:8px 14px;border-radius:999px;border:1px solid var(--border);background:var(--card);color:var(--foreground);
font:500 13px/1.4 Inter,system-ui,sans-serif;box-shadow:0 4px 16px rgb(0 0 0 / 0.12)`;
  const calm = `This demo runs on sample data and is read-only. <a href="${install}" style="color:var(--primary)">Install Monita</a> to watch your own systems.`;
  const banner = document.createElement("aside");
  banner.setAttribute("role", "note");
  banner.setAttribute("aria-live", "polite");
  banner.style.cssText = style;
  banner.innerHTML = calm;

  let timer;
  const flash = () => {
    banner.innerHTML = `The demo cannot save changes. <a href="${install}" style="color:var(--primary)">Install Monita</a> to try this.`;
    banner.style.borderColor = "var(--primary)";
    clearTimeout(timer);
    timer = setTimeout(() => {
      banner.innerHTML = calm;
      banner.style.borderColor = "var(--border)";
    }, 4000);
  };

  const start = async () => {
    await ready;
    document.body.append(banner);
    if (!("serviceWorker" in navigator)) {
      banner.textContent = "This browser has service workers turned off, which the demo needs. Open it in a regular window.";
      return;
    }
    const sw = navigator.serviceWorker;
    sw.addEventListener("message", (e) => {
      if (e.data && e.data.demo === "read-only") flash();
    });
    const reg = await sw.register(`${base}/sw.js`, { scope: `${base}/`, updateViaCache: "none" });
    if (!sw.controller) {
      await new Promise((done, fail) => {
        sw.addEventListener("controllerchange", done, { once: true });
        // A hard reload loads the page outside an active worker; it takes the page when asked.
        if (reg.active) reg.active.postMessage("claim");
        setTimeout(() => fail(new Error("the demo's service worker did not start")), 10_000);
      });
    }
    const app = document.createElement("script");
    app.type = "module";
    app.src = entry;
    document.head.append(app);
  };
  start().catch((e) => {
    console.error(e);
    banner.textContent = "The demo could not start in this browser. Reload the page to try again.";
  });
})();
