// The demo is a snapshot of the rendered pages, without the app's script. The theme switch still
// works; any other button or a link the demo cannot follow says so in the banner.
(() => {
  const banner = document.querySelector(".demo-banner");
  if (!banner) return;
  const original = banner.innerHTML;
  // A page saved with a panel open carries the page that closing it leads to (snapshot.ts).
  // Its X, Cancel, the backdrop and Escape go there. `target` null means the Escape key.
  const closePanel = (target) => {
    const to = document.querySelector("a[data-demo-close]");
    if (!to) return false;
    if (target) {
      const button = target.closest('[role="dialog"] button');
      const text = button ? button.textContent.trim() : "";
      if (!target.closest('[data-slot="sheet-overlay"], [data-slot="dialog-overlay"]') && text !== "Close" && text !== "Cancel") return false;
    }
    location.href = to.href;
    return true;
  };
  let timer;
  const flash = () => {
    banner.innerHTML =
      '<strong>Demo</strong> Read-only: this action needs a running Monita. <a href="https://github.com/PT-Perkasa-Pilar-Utama/monita#install">Install it</a>.';
    banner.classList.add("is-flash");
    clearTimeout(timer);
    timer = setTimeout(() => {
      banner.innerHTML = original;
      banner.classList.remove("is-flash");
    }, 3500);
  };
  document.addEventListener("click", (e) => {
    if (!(e.target instanceof Element)) return;
    // The public pages have their own theme button; signed-in pages keep it in the account menu,
    // which needs the app's script, so the banner carries one too.
    const theme = e.target.closest('button[aria-label^="Switch to the"], [data-demo-theme]');
    if (theme) {
      const dark = document.documentElement.classList.toggle("dark");
      document.documentElement.classList.toggle("light", !dark);
      try {
        localStorage.setItem("monita-theme", dark ? "dark" : "light");
      } catch {}
      return;
    }
    if (closePanel(e.target)) {
      e.preventDefault();
      return;
    }
    if (e.target.closest('a[href="#demo-off"], button')) {
      e.preventDefault();
      flash();
    }
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") closePanel(null);
  });
  document.addEventListener("submit", (e) => {
    e.preventDefault();
    flash();
  });
})();
